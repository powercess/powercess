//! 设备数据层：定义 `DeviceStore` trait，提供多种后端实现。
//!
//! - `StaticStore`  — 从配置文件硬编码设备列表（无需数据库）
//! - `SqliteStore`  — 读取 SQLite 中的设备表（feature: store-sqlite）
//! - `PostgresStore`— 读取 PostgreSQL 中的设备表（feature: store-postgres）
//! - `CombinedStore`— 组合多个存储，支持静态设备与数据库设备共存
//!
//! ## 静态设备与数据库共存
//!
//! 当 `store.type` 为 sqlite 或 postgres 时，`config.toml` 中的 `[[devices]]`
//! 会与数据库中的设备合并，实现补充设备的功能。

use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{AppConfig, StoreConfig};
use crate::error::AppResult;
use crate::model::DeviceInfo;

pub mod combined;
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
///
/// 当 `store.type` 为 sqlite 或 postgres 时，会自动合并 `config.toml` 中的
/// 静态设备（`[[devices]]`），实现数据库设备与静态设备的共存。
pub async fn create_store(cfg: &AppConfig) -> AppResult<Arc<dyn DeviceStore>> {
    // 检查是否有静态设备需要合并
    let has_static_devices = !cfg.devices.is_empty();

    match &cfg.store {
        StoreConfig::Static => {
            // 纯静态模式
            Ok(Arc::new(static_store::StaticStore::new(cfg.devices.clone())))
        }

        #[cfg(feature = "store-sqlite")]
        StoreConfig::Sqlite { path } => {
            let db_store: Arc<dyn DeviceStore> =
                Arc::new(sqlite::SqliteStore::connect(path).await?);

            if has_static_devices {
                // 合并数据库设备 + 静态设备
                let static_store: Arc<dyn DeviceStore> =
                    Arc::new(static_store::StaticStore::new(cfg.devices.clone()));
                Ok(Arc::new(combined::CombinedStore::new()
                    .add(db_store)
                    .add(static_store)))
            } else {
                Ok(db_store)
            }
        }

        #[cfg(feature = "store-postgres")]
        StoreConfig::Postgres { url } => {
            let db_store: Arc<dyn DeviceStore> =
                Arc::new(postgres::PostgresStore::connect(url).await?);

            if has_static_devices {
                // 合并数据库设备 + 静态设备
                let static_store: Arc<dyn DeviceStore> =
                    Arc::new(static_store::StaticStore::new(cfg.devices.clone()));
                Ok(Arc::new(combined::CombinedStore::new()
                    .add(db_store)
                    .add(static_store)))
            } else {
                Ok(db_store)
            }
        }

        // 编译时未开启对应 feature 时的友好提示
        #[allow(unreachable_patterns)]
        other => Err(crate::error::AppError::Config(format!(
            "存储后端 {other:?} 未在编译时启用，请检查 Cargo feature"
        ))),
    }
}
