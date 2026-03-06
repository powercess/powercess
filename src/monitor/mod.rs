//! 监控调度器：为每台设备独立启动 tokio task，实现自动重连与周期上报。
//!
//! 设计原则（树莓派 4 优化）：
//!   - 每台设备一个 task，避免单 BLE 会话阻塞全局
//!   - 扫描阶段共用一个 Adapter，避免重复初始化
//!   - 断连 / 超时后 exponential backoff 重试，最长等待 `max_retry_secs`

use std::sync::Arc;
use std::time::Duration;

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
) {
    let mac = device.mac_upper();
    let mut retry_delay = settings.retry_interval_secs;

    loop {
        info!("[Monitor] 启动监控任务: {} ({})", device.name, mac);

        // ── 1. 获取蓝牙适配器 ───────────────────────────────────────────────
        let adapter = match get_adapter().await {
            Ok(a) => a,
            Err(e) => {
                error!("[Monitor] 无法获取 BLE 适配器: {e}，{retry_delay}s 后重试");
                time::sleep(Duration::from_secs(retry_delay)).await;
                continue;
            }
        };

        // ── 2. 扫描目标设备 ─────────────────────────────────────────────────
        let peripheral = match scan_for_device(
            &adapter,
            &mac,
            settings.scan_timeout_secs,
        )
        .await
        {
            Ok(Some(p)) => p,
            Ok(None) => {
                warn!("[Monitor] 扫描超时，未找到 {mac}，{retry_delay}s 后重试");
                time::sleep(Duration::from_secs(retry_delay)).await;
                retry_delay = (retry_delay * 2).min(300); // 最长等 5 分钟
                continue;
            }
            Err(e) => {
                error!("[Monitor] 扫描失败 {mac}: {e}，{retry_delay}s 后重试");
                time::sleep(Duration::from_secs(retry_delay)).await;
                continue;
            }
        };

        // 找到设备则重置退避
        retry_delay = settings.retry_interval_secs;

        // ── 3. 建立会话并周期轮询 ────────────────────────────────────────────
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

/// 启动所有设备的监控任务（并发执行，互不干扰）。
pub async fn run_all(
    devices: Vec<DeviceInfo>,
    settings: AppSettings,
    reporter: Arc<dyn Reporter>,
) {
    if devices.is_empty() {
        warn!("[Monitor] 设备列表为空，无任务启动");
        return;
    }

    let settings = Arc::new(settings);

    let mut handles = Vec::with_capacity(devices.len());
    for device in devices {
        let s = settings.clone();
        let r = reporter.clone();
        let handle = tokio::spawn(async move {
            monitor_device(device, s, r).await;
        });
        handles.push(handle);
    }

    // 等待所有任务（它们内部是无限循环，除非进程被终止）
    for h in handles {
        let _ = h.await;
    }
}
