//! 数据上报层：定义 `Reporter` trait，支持多路上报。
//!
//! - `DatabaseReporter` — 将测量值持久化到数据库
//! - `HttpReporter`     — 在内存中保存最新值，并提供 HTTP REST API
//! - `MultiReporter`    — 将上报分发到多个 Reporter（组合模式）

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppResult;
use crate::model::{DeviceInfo, Measurement};

pub mod database;

#[cfg(feature = "reporter-http")]
pub mod http;

// ── Trait ─────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait Reporter: Send + Sync {
    /// 上报一次测量结果。
    async fn report(&self, device: &DeviceInfo, m: &Measurement) -> AppResult<()>;
}

// ── 组合上报 ──────────────────────────────────────────────────────────────────

/// 将同一份测量结果依次分发给内部所有 Reporter。
pub struct MultiReporter {
    reporters: Vec<Arc<dyn Reporter>>,
}

impl MultiReporter {
    pub fn new(reporters: Vec<Arc<dyn Reporter>>) -> Self {
        Self { reporters }
    }
}

#[async_trait]
impl Reporter for MultiReporter {
    async fn report(&self, device: &DeviceInfo, m: &Measurement) -> AppResult<()> {
        for r in &self.reporters {
            if let Err(e) = r.report(device, m).await {
                tracing::warn!("[Reporter] 上报失败: {e}");
            }
        }
        Ok(())
    }
}
