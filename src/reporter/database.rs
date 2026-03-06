//! 数据库上报：将每次测量值持久化到 SQLite 或 PostgreSQL。

use async_trait::async_trait;
use tracing::debug;

use crate::error::{AppError, AppResult};
use crate::model::{DeviceInfo, Measurement};
use crate::reporter::Reporter;

// ── SQLite 实现 ───────────────────────────────────────────────────────────────

#[cfg(feature = "store-sqlite")]
pub struct SqliteDatabaseReporter {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "store-sqlite")]
impl SqliteDatabaseReporter {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "store-sqlite")]
#[async_trait]
impl Reporter for SqliteDatabaseReporter {
    async fn report(&self, _device: &DeviceInfo, m: &Measurement) -> AppResult<()> {
        let recorded_at = m.recorded_at.to_rfc3339();
        let pf_type = format!("{:?}", m.pf_type);

        sqlx::query(
            "INSERT INTO measurements \
             (device_mac, recorded_at, voltage, current_a, power, \
              frequency, power_factor, pf_type, energy, uptime_secs) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&m.device_mac)
        .bind(&recorded_at)
        .bind(m.voltage)
        .bind(m.current)
        .bind(m.power)
        .bind(m.frequency)
        .bind(m.power_factor)
        .bind(&pf_type)
        .bind(m.energy)
        .bind(m.uptime_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        debug!("[DB] 已写入测量值: {}", m.device_mac);
        Ok(())
    }
}

// ── PostgreSQL 实现 ───────────────────────────────────────────────────────────

#[cfg(feature = "store-postgres")]
pub struct PostgresDatabaseReporter {
    pool: sqlx::PgPool,
}

#[cfg(feature = "store-postgres")]
impl PostgresDatabaseReporter {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "store-postgres")]
#[async_trait]
impl Reporter for PostgresDatabaseReporter {
    async fn report(&self, _device: &DeviceInfo, m: &Measurement) -> AppResult<()> {
        let pf_type = format!("{:?}", m.pf_type);

        sqlx::query(
            "INSERT INTO measurements \
             (device_mac, recorded_at, voltage, current_a, power, \
              frequency, power_factor, pf_type, energy, uptime_secs) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&m.device_mac)
        .bind(m.recorded_at)
        .bind(m.voltage)
        .bind(m.current)
        .bind(m.power)
        .bind(m.frequency)
        .bind(m.power_factor)
        .bind(&pf_type)
        .bind(m.energy)
        .bind(m.uptime_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        debug!("[DB] 已写入测量值: {}", m.device_mac);
        Ok(())
    }
}
