//! SQLite 设备数据层（feature: store-sqlite）
//!
//! 表结构（自动迁移）：
//! ```sql
//! CREATE TABLE IF NOT EXISTS devices (
//!     mac   TEXT PRIMARY KEY,
//!     name  TEXT NOT NULL,
//!     label TEXT
//! );
//! ```

use async_trait::async_trait;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

use crate::error::{AppError, AppResult};
use crate::model::DeviceInfo;
use crate::store::DeviceStore;

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn connect(path: &str) -> AppResult<Self> {
        let url = format!("sqlite://{path}?mode=rwc");
        let pool = SqlitePoolOptions::new()
            .max_connections(4)         // 树莓派 4：限制连接数
            .connect(&url)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 自动建表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
                mac   TEXT PRIMARY KEY,
                name  TEXT NOT NULL,
                label TEXT
            );
            CREATE TABLE IF NOT EXISTS measurements (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                device_mac  TEXT    NOT NULL,
                recorded_at TEXT    NOT NULL,
                voltage     REAL    NOT NULL,
                current_a   REAL    NOT NULL,
                power       REAL    NOT NULL,
                frequency   REAL    NOT NULL,
                power_factor REAL   NOT NULL,
                pf_type     TEXT    NOT NULL,
                energy      REAL    NOT NULL,
                uptime_secs INTEGER NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        info!("[Store] SQLite 已连接: {path}");
        Ok(Self { pool })
    }

    /// 暴露连接池，用于 `DatabaseReporter` 写入测量值
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl DeviceStore for SqliteStore {
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
