//! 设备数据层：定义 `DeviceStore` trait，提供三种后端实现。
//!
//! - `StaticStore`  — 从配置文件硬编码设备列表（无需数据库）
//! - `SqliteStore`  — 读取 SQLite 中的设备表（feature: store-sqlite）  
//! - `PostgresStore`— 读取 PostgreSQL 中的设备表（feature: store-postgres）

use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{AppConfig, StoreConfig};
use crate::error::AppResult;
use crate::model::DeviceInfo;

pub mod static_store;

#[cfg(feature = "store-sqlite")]
pub mod sqlite;

#[cfg(feature = "store-postgres")]
pub mod postgres;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// 设备存储抽象：返回需要监控的设备列表。
#[async_trait]
pub trait DeviceStore: Send + Sync {
    /// 返回所有需要监控的设备。
    async fn list_devices(&self) -> AppResult<Vec<DeviceInfo>>;
}

// ── 工厂函数 ──────────────────────────────────────────────────────────────────

/// 根据配置创建对应的 `DeviceStore` 实例。
pub async fn create_store(cfg: &AppConfig) -> AppResult<Arc<dyn DeviceStore>> {
    match &cfg.store {
        StoreConfig::Static => {
            let store: Arc<dyn DeviceStore> =
                Arc::new(static_store::StaticStore::new(cfg.devices.clone()));
            Ok(store)
        }

        #[cfg(feature = "store-sqlite")]
        StoreConfig::Sqlite { path } => {
            let store: Arc<dyn DeviceStore> =
                Arc::new(sqlite::SqliteStore::connect(path).await?);
            Ok(store)
        }

        #[cfg(feature = "store-postgres")]
        StoreConfig::Postgres { url } => {
            let store: Arc<dyn DeviceStore> =
                Arc::new(postgres::PostgresStore::connect(url).await?);
            Ok(store)
        }

        // 编译时未开启对应 feature 时的友好提示
        #[allow(unreachable_patterns)]
        other => Err(crate::error::AppError::Config(format!(
            "存储后端 {other:?} 未在编译时启用，请检查 Cargo feature"
        ))),
    }
}
