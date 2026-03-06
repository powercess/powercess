//! 核心数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── 设备描述 ──────────────────────────────────────────────────────────────────

/// 一台受监控的 BLE 功率计设备
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// BLE MAC 地址（大写，冒号分隔，如 "12:10:37:4C:47:47"）
    pub mac: String,
    /// 人类可读名称
    pub name: String,
    /// 可选位置/备注标签
    pub label: Option<String>,
}

impl DeviceInfo {
    pub fn mac_upper(&self) -> String {
        self.mac.to_uppercase()
    }
}

// ── 功率因数类型 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PfType {
    Inductive,  // 感性
    Capacitive, // 容性
    Resistive,  // 纯阻性 / 满功率因数
}

impl PfType {
    pub fn label(&self) -> &'static str {
        match self {
            PfType::Inductive => "感性",
            PfType::Capacitive => "容性",
            PfType::Resistive => "",
        }
    }
}

// ── 测量值 ────────────────────────────────────────────────────────────────────

/// 一次完整的功率计读数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    /// 采集时刻（UTC）
    pub recorded_at: DateTime<Utc>,
    /// 设备 MAC
    pub device_mac: String,

    /// 电压（V）
    pub voltage: f64,
    /// 电流（A）
    pub current: f64,
    /// 有功功率（W）
    pub power: f64,
    /// 频率（Hz）
    pub frequency: f64,
    /// 功率因数（绝对值，正数）
    pub power_factor: f64,
    /// 功率因数类型
    pub pf_type: PfType,
    /// 累计用电量（kWh）
    pub energy: f64,
    /// 设备累计通电时间（秒）
    pub uptime_secs: i32,
}

impl Measurement {
    /// 通电时间格式化为 "X 小时 Y 分钟"
    #[allow(dead_code)]
    pub fn uptime_human(&self) -> String {
        let h = self.uptime_secs / 3600;
        let m = (self.uptime_secs % 3600) / 60;
        format!("{h} 小时 {m} 分钟")
    }
}
