//! 监控调度器：为每台设备独立启动 tokio task，实现自动重连与周期上报。
//!
//! 设计原则（树莓派 4 优化）：
//!   - 每台设备一个 task，避免单 BLE 会话阻塞全局
//!   - **共享单个 Adapter**，避免重复初始化
//!   - **扫描阶段用 Semaphore(1) 互斥**：同一时刻只有一个 task 在 start_scan/stop_scan，
//!     防止某 task 的 stop_scan 把其他 task 正在进行的扫描也一并停掉。
//!   - **连接阶段用 Semaphore(N) 限流**：N = config.app.max_ble_connections（默认 5），
//!     防止超过适配器硬件并发上限而触发 le-connection-abort-by-local。
//!     permit 在 polling_session 全程持有，会话结束（断连/重试）时自动释放槽位。
//!   - 各 task 错开 500ms 启动，避免全体同时抢扫描锁
//!   - 断连 / 超时后 exponential backoff 重试，最长等待 `max_retry_secs`

use std::sync::Arc;
use std::time::Duration;

use btleplug::platform::Adapter;
use tokio::sync::Semaphore;
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::ble::client::{get_adapter, polling_session, scan_for_device};
use crate::config::AppSettings;
use crate::model::DeviceInfo;
use crate::reporter::Reporter;

/// 单台设备的监控任务（永不退出，内部自动重连）。
async fn monitor_device(
    device: DeviceInfo,
    settings: Arc<AppSettings>,
    reporter: Arc<dyn Reporter>,
    adapter: Arc<Adapter>,
    scan_lock: Arc<Semaphore>,
    conn_lock: Arc<Semaphore>,
) {
    let mac = device.mac_upper();
    let mut retry_delay = settings.retry_interval_secs;

    loop {
        info!("[Monitor] 启动监控任务: {} ({})", device.name, mac);

        // ── 1. 获取扫描许可（同一时刻只允许一个 task 执行 start/stop_scan）──
        //    持有 permit 期间执行扫描，找到设备后 permit 自动 drop，其他 task 即可开始扫描。
        let peripheral = {
            let _permit = scan_lock.acquire().await
                .expect("scan_lock Semaphore 已关闭");

            // ── 2. 扫描目标设备 ─────────────────────────────────────────────
            match scan_for_device(&adapter, &mac, settings.scan_timeout_secs).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    warn!("[Monitor] 扫描超时，未找到 {mac}，{retry_delay}s 后重试");
                    // permit 在此 drop，释放扫描锁
                    time::sleep(Duration::from_secs(retry_delay)).await;
                    retry_delay = (retry_delay * 2).min(300);
                    continue;
                }
                Err(e) => {
                    error!("[Monitor] 扫描失败 {mac}: {e}，{retry_delay}s 后重试");
                    time::sleep(Duration::from_secs(retry_delay)).await;
                    continue;
                }
            }
            // _permit 在此 drop → 下一台设备可以开始扫描
        };

        // 找到设备则重置退避
        retry_delay = settings.retry_interval_secs;

        // ── 3. 获取连接槽许可（限制适配器并发连接数，防止硬件上限错误）────────
        //    permit 在整个 polling_session 生命周期内持有；
        //    会话结束（断连/出错）后 _conn_permit 自动 drop，释放槽位。
        let _conn_permit = match Arc::clone(&conn_lock).acquire_owned().await {
            Ok(p) => p,
            Err(_) => {
                error!("[Monitor] 连接信号量已关闭: {mac}");
                return;
            }
        };
        info!(
            "[Monitor] 占用连接槽（剩余 {} 槽）: {mac}",
            conn_lock.available_permits()
        );

        // ── 4. 建立会话并周期轮询 ────────────────────────────────────────────
        let dev_clone = device.clone();
        let rep_clone = reporter.clone();
        let poll_secs = settings.poll_interval_secs;

        let result = polling_session(&device, peripheral, poll_secs, move |m| {
            let reporter = rep_clone.clone();
            let dev = dev_clone.clone();
            async move {
                // 正常采集仅 debug 级别，不刷屏
                debug!(
                    "[Data] {} V={:.3} A={:.3} W={:.3} PF={:.2} E={:.3}kWh",
                    dev.mac_upper(),
                    m.voltage, m.current, m.power, m.power_factor, m.energy
                );
                // 上报（DB / HTTP 等）
                if let Err(e) = reporter.report(&dev, &m).await {
                    error!("[Monitor] 上报失败 {}: {e}", dev.mac_upper());
                }
            }
        })
        .await;

        match result {
            Ok(()) => {
                warn!("[Monitor] 会话正常结束 {mac}，{retry_delay}s 后重连");
            }
            Err(e) => {
                error!("[Monitor] 会话异常 {mac}: {e}，{retry_delay}s 后重连");
            }
        }
        time::sleep(Duration::from_secs(retry_delay)).await;
    }
}

/// 启动所有设备的监控任务（扫描串行、连接并发）。
pub async fn run_all(
    devices: Vec<DeviceInfo>,
    settings: AppSettings,
    reporter: Arc<dyn Reporter>,
) {
    if devices.is_empty() {
        warn!("[Monitor] 设备列表为空，无任务启动");
        return;
    }

    // ── 获取全局共享 Adapter（避免各 task 重复初始化）────────────────────────
    let adapter = match get_adapter().await {
        Ok(a) => Arc::new(a),
        Err(e) => {
            error!("[Monitor] 无法获取 BLE 适配器，程序无法运行: {e}");
            return;
        }
    };

    // ── 扫描互斥信号量：同一时刻只有 1 个 task 可执行 start/stop_scan ────────
    let scan_lock = Arc::new(Semaphore::new(1));

    // ── 连接限流信号量：最多同时保持 N 条 BLE 会话 ───────────────────────────
    let max_conn = settings.max_ble_connections.max(1); // 至少保留 1 个槽位
    info!("[Monitor] BLE 同时连接上限: {max_conn}");
    let conn_lock = Arc::new(Semaphore::new(max_conn));

    let settings = Arc::new(settings);

    let mut handles = Vec::with_capacity(devices.len());
    for (i, device) in devices.into_iter().enumerate() {
        let s = settings.clone();
        let r = reporter.clone();
        let a = adapter.clone();
        let lock = scan_lock.clone();
        let clk  = conn_lock.clone();

        let handle = tokio::spawn(async move {
            // 各 task 错开 500ms 启动，避免全体同时竞争扫描锁
            if i > 0 {
                time::sleep(Duration::from_millis(500 * i as u64)).await;
            }
            monitor_device(device, s, r, a, lock, clk).await;
        });
        handles.push(handle);
    }

    // 等待所有任务（它们内部是无限循环，除非进程被终止）
    for h in handles {
        let _ = h.await;
    }
}
