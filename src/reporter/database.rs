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
//
// 目标表：raw_measurements（TimescaleDB Hypertable）
//   collected_at  TIMESTAMPTZ  — 采集时刻（Hypertable 分区键）
//   device_id     UUID         — 通过 mac_address 子查询获取
//   reporter_id   UUID         — 本地运行无 reporter 注册，填 NULL
//   payload       JSONB        — 6 个标准字段 + pf_type + uptime_secs
//
// 若 mac_address 未在 devices 表中命中（设备未注册），子查询返回空行，
// INSERT 不执行任何写入（静默跳过），并通过 warn 日志记录。

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
        use serde_json::json;

        // 构造符合 init.sql 注释约定的 JSONB payload
        let payload = json!({
            "voltage":      m.voltage,
            "current":      m.current,
            "power":        m.power,
            "energy_kwh":   m.energy,      // 里程表式累计读数（kWh）
            "frequency":    m.frequency,
            "power_factor": m.power_factor,
            "pf_type":      format!("{:?}", m.pf_type),
            "uptime_secs":  m.uptime_secs,
        });

        // 用 CTE 子查询将 mac_address 解析为 device_id（UUID）;
        // 若设备未注册则子查询为空，INSERT 不写入任何行（ON CONFLICT DO NOTHING
        // 同时处理同一 collected_at + device_id 的重复上报）。
        let rows_affected = sqlx::query(
            r#"
            WITH dev AS (
                SELECT id
                FROM   devices
                WHERE  mac_address = $3
                  AND  NOT is_deleted
                LIMIT  1
            )
            INSERT INTO raw_measurements (collected_at, device_id, reporter_id, payload)
            SELECT $1, dev.id, NULL::uuid, $2
            FROM   dev
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(m.recorded_at)          // $1  TIMESTAMPTZ
        .bind(sqlx::types::Json(&payload)) // $2  JSONB
        .bind(&m.device_mac)          // $3  TEXT (mac_address)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if rows_affected.rows_affected() == 0 {
            tracing::warn!(
                "[DB] 设备 {} 未在 devices 表中注册，测量值已跳过",
                m.device_mac
            );
        } else {
            debug!("[DB] 已写入 raw_measurements: {}", m.device_mac);
        }

        Ok(())
    }
}
