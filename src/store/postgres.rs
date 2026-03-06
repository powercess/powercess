//! PostgreSQL 设备数据层（feature: store-postgres）
//!
//! 在使用前请先手动执行建表 SQL：
//! ```sql
//! CREATE TABLE IF NOT EXISTS devices (
//!     mac   TEXT PRIMARY KEY,
//!     name  TEXT NOT NULL,
//!     label TEXT
//! );
//! CREATE TABLE IF NOT EXISTS measurements (
//!     id           BIGSERIAL PRIMARY KEY,
//!     device_mac   TEXT      NOT NULL,
//!     recorded_at  TIMESTAMPTZ NOT NULL,
//!     voltage      FLOAT8    NOT NULL,
//!     current_a    FLOAT8    NOT NULL,
//!     power        FLOAT8    NOT NULL,
//!     frequency    FLOAT8    NOT NULL,
//!     power_factor FLOAT8    NOT NULL,
//!     pf_type      TEXT      NOT NULL,
//!     energy       FLOAT8    NOT NULL,
//!     uptime_secs  INT       NOT NULL
//! );
//! ```

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

        let rows: Vec<DeviceRow> = sqlx::query_as(
            "SELECT mac, name, label FROM devices",
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
