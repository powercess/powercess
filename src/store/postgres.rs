//! PostgreSQL 设备数据层（feature: store-postgres）
//!
//! 依赖 `database/timescaledb/init.sql` 初始化后的完整 Schema：
//! - `devices`          — 设备实例（字段 mac_address、name、location、is_active、is_deleted）
//! - `raw_measurements` — TimescaleDB Hypertable，payload 为 JSONB

use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

use crate::error::{AppError, AppResult};
use crate::model::DeviceInfo;
use crate::store::DeviceStore;

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(url: &str) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        info!("[Store] PostgreSQL 已连接");
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl DeviceStore for PostgresStore {
    async fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
        #[derive(sqlx::FromRow)]
        struct DeviceRow {
            mac: String,
            name: String,
            label: Option<String>,
        }

        // 新 schema：mac_address（UNIQUE TEXT）、location 对应旧 label
        // 仅返回激活且未软删除的设备
        let rows: Vec<DeviceRow> = sqlx::query_as(
            "SELECT mac_address AS mac, name, location AS label \
             FROM devices \
             WHERE is_active AND NOT is_deleted \
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| DeviceInfo {
                mac: r.mac,
                name: r.name,
                label: r.label,
            })
            .collect())
    }
}
