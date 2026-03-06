//! 统一错误类型

use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("BLE 错误: {0}")]
    Ble(#[from] btleplug::Error),

    #[error("设备 {mac} 未找到")]
    DeviceNotFound { mac: String },

    #[error("CRC 校验失败：期望 {expected}，实际 {actual}")]
    CrcMismatch { expected: i16, actual: i16 },

    #[error("帧格式错误: {0}")]
    FrameFormat(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("响应超时")]
    Timeout,

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = std::result::Result<T, AppError>;
