//! BLE 客户端：封装扫描、连接、订阅通知、发送查询、接收响应的完整生命周期。

use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::error::{AppError, AppResult};
use crate::model::{DeviceInfo, Measurement};

use super::protocol::{
    build_f001_query, parse_f001_response, NOTIFY_CHAR_UUID, WRITE_CHAR_UUID,
};

// ── BLE 管理 ──────────────────────────────────────────────────────────────────

/// 获取系统首个蓝牙适配器
pub async fn get_adapter() -> AppResult<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("未找到蓝牙适配器")))
}

// ── 扫描 ──────────────────────────────────────────────────────────────────────

/// 扫描并返回 MAC 匹配的外设；`timeout_secs` 内未找到则返回 `None`。
pub async fn scan_for_device(
    adapter: &Adapter,
    target_mac: &str,
    timeout_secs: u64,
) -> AppResult<Option<Peripheral>> {
    let mac_upper = target_mac.to_uppercase();

    // 注意：BlueZ（Linux/树莓派）不支持按 Service UUID 过滤广播包，
    // 许多设备不在广播包中携带 Service UUID，导致过滤后扫描结果为空。
    // 使用空 ScanFilter 扫描所有设备，由后续 MAC 地址匹配完成定位。
    adapter.start_scan(ScanFilter::default()).await?;
    debug!("[BLE] 开始扫描（最长 {timeout_secs}s），目标 {mac_upper}");

    let mut found: Option<Peripheral> = None;
    let steps = timeout_secs * 10; // 每 100ms 检查一次
    for _ in 0..steps {
        time::sleep(Duration::from_millis(100)).await;
        for p in adapter.peripherals().await? {
            if let Some(props) = p.properties().await? {
                if props.address.to_string().to_uppercase() == mac_upper {
                    let name = props.local_name.unwrap_or_else(|| "Unknown".into());
                    info!("[BLE] 找到设备: {name} @ {mac_upper}");
                    found = Some(p);
                    break;
                }
            }
        }
        if found.is_some() {
            break;
        }
    }

    adapter.stop_scan().await?;
    Ok(found)
}

// ── 周期轮询会话 ──────────────────────────────────────────────────────────────

/// 连接到 `device`，每隔 `poll_secs` 秒查询一次，
/// 将结果通过 `on_measurement` 回调传出；遇到断连则返回错误由调用方重试。
pub async fn polling_session<F, Fut>(
    device_info: &DeviceInfo,
    peripheral: Peripheral,
    poll_secs: u64,
    on_measurement: F,
) -> AppResult<()>
where
    F: Fn(Measurement) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ()> + Send,
{
    let mac = device_info.mac_upper();

    // 连接
    peripheral.connect().await.map_err(|e| {
        error!("[BLE] 连接 {mac} 失败: {e}");
        AppError::Ble(e)
    })?;
    info!("[BLE] 已连接: {mac}");

    // 发现服务与特征
    peripheral.discover_services().await?;
    let chars = peripheral.characteristics();

    let notify_char = chars
        .iter()
        .find(|c| c.uuid == NOTIFY_CHAR_UUID)
        .ok_or_else(|| {
            AppError::FrameFormat(format!("未找到通知特征 {NOTIFY_CHAR_UUID}"))
        })?
        .clone();

    let write_char = chars
        .iter()
        .find(|c| c.uuid == WRITE_CHAR_UUID)
        .ok_or_else(|| {
            AppError::FrameFormat(format!("未找到写特征 {WRITE_CHAR_UUID}"))
        })?
        .clone();

    // 启用通知
    peripheral.subscribe(&notify_char).await?;
    info!("[BLE] 已启用通知: {mac}");

    // 通知流（subscribe 后立即获取，避免漏包）
    let mut notif_stream = peripheral.notifications().await?;

    loop {
        // ── 发送查询 ──────────────────────────────────────────────────────────
        let query = build_f001_query();
        if let Err(e) = peripheral
            .write(&write_char, &query, WriteType::WithoutResponse)
            .await
        {
            error!("[BLE] 写入失败 {mac}: {e}");
            break;
        }
        debug!("[BLE] 已发送 F001 查询: {mac}");

        // ── 等待响应（最多 5 秒） ─────────────────────────────────────────────
        match time::timeout(Duration::from_secs(5), notif_stream.next()).await {
            Ok(Some(notif)) => {
                debug!("[BLE] 收到通知 ({} B): {mac}", notif.value.len());
                match parse_f001_response(&mac, &notif.value) {
                    Ok(m) => {
                        on_measurement(m).await;
                    }
                    Err(e) => {
                        warn!("[BLE] 解析失败 {mac}: {e}");
                    }
                }
            }
            Ok(None) => {
                error!("[BLE] 通知流关闭: {mac}");
                break; // 断连，由外层重试
            }
            Err(_) => {
                warn!("[BLE] 等待响应超时: {mac}");
                // 超时不断连，继续下一轮查询
            }
        }

        // ── 等待下一个调度周期 ────────────────────────────────────────────────
        time::sleep(Duration::from_secs(poll_secs)).await;
    }

    // 清理（尽力而为）
    let _ = peripheral.unsubscribe(&notify_char).await;
    let _ = peripheral.disconnect().await;
    info!("[BLE] 已断开: {mac}");

    Ok(())
}
