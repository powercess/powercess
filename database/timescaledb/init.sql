-- =============================================================================
-- Powercess 电力管理系统 - 数据库初始化脚本
-- 依赖：PostgreSQL 15+ / TimescaleDB 2.9+（Hierarchical CA 需要 2.9+）
-- 说明：本脚本保留所有历史数据，不设置自动删除策略
-- =============================================================================

-- 启用扩展
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS "pgcrypto";  -- 提供 gen_random_uuid() 和 crypt()

-- =============================================================================
-- 1. 工具函数：自动更新 updated_at
-- =============================================================================
CREATE OR REPLACE FUNCTION fn_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- 2. 设备类型模板
-- =============================================================================
CREATE TABLE device_types (
    id            SERIAL      PRIMARY KEY,
    name          TEXT        NOT NULL UNIQUE,
    description   TEXT,
    protocol_spec JSONB,      -- 协议规格（BLE UUID、寄存器地址等）
    data_schema   JSONB       NOT NULL,  -- 字段定义（用于前端动态渲染）
    is_deleted    BOOLEAN     NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_device_types_updated_at
BEFORE UPDATE ON device_types
FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- =============================================================================
-- 3. 树莓派上报节点（Reporter）
--    每台树莓派对应一条记录，持有 API Key 用于上报认证
-- =============================================================================
CREATE TABLE reporters (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT        NOT NULL,
    description  TEXT,
    api_key_hash TEXT        NOT NULL UNIQUE,  -- SHA-256 后存储，明文由管理员分发
    ip_address   INET,                          -- 最后一次上报的 IP（信息性）
    last_seen    TIMESTAMPTZ,
    is_active    BOOLEAN     NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_reporters_updated_at
BEFORE UPDATE ON reporters
FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- =============================================================================
-- 4. 设备实例
-- =============================================================================
CREATE TABLE devices (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name           TEXT        NOT NULL,
    mac_address    TEXT        NOT NULL UNIQUE
                               CHECK (mac_address ~ '^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$'),
    device_type_id INT         NOT NULL REFERENCES device_types(id) ON DELETE RESTRICT,
    reporter_id    UUID        REFERENCES reporters(id) ON DELETE SET NULL,
    location       TEXT,
    is_active      BOOLEAN     NOT NULL DEFAULT true,
    is_deleted     BOOLEAN     NOT NULL DEFAULT false,  -- 软删除，不级联清除历史数据
    last_seen      TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_devices_type_active ON devices(device_type_id, is_active) WHERE NOT is_deleted;
CREATE INDEX idx_devices_reporter    ON devices(reporter_id)               WHERE NOT is_deleted;

CREATE TRIGGER trg_devices_updated_at
BEFORE UPDATE ON devices
FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- =============================================================================
-- 5. Web 系统用户
-- =============================================================================
CREATE TYPE user_role AS ENUM ('admin', 'operator', 'viewer');

CREATE TABLE users (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT        NOT NULL UNIQUE
                              CHECK (email ~ '^[^@\s]+@[^@\s]+\.[^@\s]+$'),
    password_hash TEXT        NOT NULL,   -- bcrypt / argon2 哈希，由 Rust 后端处理
    display_name  TEXT        NOT NULL,
    role          user_role   NOT NULL DEFAULT 'viewer',
    is_active     BOOLEAN     NOT NULL DEFAULT true,
    is_deleted    BOOLEAN     NOT NULL DEFAULT false,  -- 软删除，保留审计外键引用
    last_login    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_users_updated_at
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- =============================================================================
-- 6. 原始测量数据（Hypertable）
--    payload 期望键：voltage, current, power, energy_kwh, frequency, power_factor
--    energy_kwh 为里程表式累计读数（单调递增），不是增量
-- =============================================================================
CREATE TABLE raw_measurements (
    collected_at TIMESTAMPTZ NOT NULL,
    device_id    UUID        NOT NULL REFERENCES devices(id) ON DELETE RESTRICT,
    reporter_id  UUID        REFERENCES reporters(id) ON DELETE SET NULL,
    payload      JSONB       NOT NULL,
    raw_frame    BYTEA,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collected_at, device_id)
);

-- 转为 Hypertable，按 collected_at 分区，每 chunk 覆盖 1 天
SELECT create_hypertable(
    'raw_measurements',
    'collected_at',
    chunk_time_interval => INTERVAL '1 day'
);

-- 压缩配置（旧数据节省存储，但永久保留，不启用 retention policy）
ALTER TABLE raw_measurements
SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'device_id',
    timescaledb.compress_orderby   = 'collected_at DESC'
);

-- 7 天后自动压缩旧 chunk（压缩 ≠ 删除，数据仍可查询）
SELECT add_compression_policy('raw_measurements', INTERVAL '7 days');

CREATE INDEX idx_raw_device_time ON raw_measurements (device_id, collected_at DESC);

-- =============================================================================
-- 7. 小时聚合（Continuous Aggregate）
--
--    energy_kwh 是里程表读数，正确做法：MAX - MIN 得到该小时内实际耗电增量
--    若设备重启导致计数器归零，delta 可能为负，业务层应过滤
-- =============================================================================
CREATE MATERIALIZED VIEW hourly_aggregates
WITH (timescaledb.continuous) AS
SELECT
    device_id,
    time_bucket(INTERVAL '1 hour', collected_at)            AS hour_bucket,
    AVG((payload->>'voltage')::NUMERIC)                     AS avg_voltage,
    MIN((payload->>'voltage')::NUMERIC)                     AS min_voltage,
    MAX((payload->>'voltage')::NUMERIC)                     AS max_voltage,
    AVG((payload->>'current')::NUMERIC)                     AS avg_current,
    MIN((payload->>'current')::NUMERIC)                     AS min_current,
    MAX((payload->>'current')::NUMERIC)                     AS max_current,
    AVG((payload->>'power')::NUMERIC)                       AS avg_power,
    MAX((payload->>'power')::NUMERIC)                       AS max_power,
    -- 里程表式耗电：小时末读数 - 小时初读数 = 本小时实际耗电增量
    (MAX((payload->>'energy_kwh')::NUMERIC)
     - MIN((payload->>'energy_kwh')::NUMERIC))              AS energy_delta_kwh,
    AVG((payload->>'frequency')::NUMERIC)                   AS avg_frequency,
    AVG((payload->>'power_factor')::NUMERIC)                AS avg_power_factor,
    COUNT(*)                                                AS record_count
FROM raw_measurements
GROUP BY device_id, time_bucket(INTERVAL '1 hour', collected_at);
-- 注意：Continuous Aggregate 不支持 ORDER BY，排序在查询时指定

-- 刷新策略：每 1 小时刷新，往回看 3 小时以覆盖网络延迟到达的数据
SELECT add_continuous_aggregate_policy('hourly_aggregates',
    start_offset      => INTERVAL '3 hours',
    end_offset        => INTERVAL '10 minutes',
    schedule_interval => INTERVAL '1 hour'
);

CREATE INDEX idx_hourly_pkey ON hourly_aggregates (device_id, hour_bucket);

-- =============================================================================
-- 8. 日聚合（Hierarchical Continuous Aggregate，基于 hourly_aggregates）
--    需要 TimescaleDB 2.9+
-- =============================================================================
CREATE MATERIALIZED VIEW daily_aggregates
WITH (timescaledb.continuous) AS
SELECT
    device_id,
    time_bucket(INTERVAL '1 day', hour_bucket)               AS day_bucket,  -- 保留 TIMESTAMPTZ，时区转换由应用层处理
    -- 日耗电量 = 各小时增量之和
    SUM(energy_delta_kwh)                                   AS energy_delta_kwh,
    AVG(avg_power)                                          AS avg_power,
    MAX(max_power)                                          AS peak_power,    -- 日最大功率
    MAX(max_voltage)                                        AS peak_voltage,  -- 日最高电压（命名与含义一致）
    MIN(min_voltage)                                        AS min_voltage,
    AVG(avg_power_factor)                                   AS avg_power_factor,
    AVG(avg_frequency)                                      AS avg_frequency,
    -- 有数据上报的小时数（粗略在线时长），类型为 BIGINT，不做无意义的 *1.0 转换
    COUNT(*)                                                AS uptime_hours,
    SUM(record_count)                                       AS total_records
FROM hourly_aggregates
GROUP BY device_id, time_bucket(INTERVAL '1 day', hour_bucket);
-- 注意：Continuous Aggregate 不支持 ORDER BY，排序在查询时指定

-- 刷新策略：每天刷新，start 与 end 之差需覆盖至少 2 个 day bucket
-- end_offset=1h（接近当前），start_offset=4 days → 窗口约 4 天，满足 >=2 bucket 要求
SELECT add_continuous_aggregate_policy('daily_aggregates',
    start_offset      => INTERVAL '4 days',
    end_offset        => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 day'
);

CREATE INDEX idx_daily_pkey ON daily_aggregates (device_id, day_bucket);

-- =============================================================================
-- 9. 告警规则配置
-- =============================================================================
CREATE TYPE alert_severity  AS ENUM ('info', 'warning', 'critical');
CREATE TYPE alert_condition AS ENUM ('gt', 'lt', 'gte', 'lte', 'eq', 'neq');

CREATE TABLE alert_rules (
    id          UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT            NOT NULL,
    device_id   UUID            REFERENCES devices(id) ON DELETE RESTRICT,
                                -- NULL 表示适用于所有设备
    metric      TEXT            NOT NULL,  -- 对应 payload 中的键，如 'voltage'、'power'
    condition   alert_condition NOT NULL,
    threshold   NUMERIC         NOT NULL,
    severity    alert_severity  NOT NULL DEFAULT 'warning',
    is_active   BOOLEAN         NOT NULL DEFAULT true,
    is_deleted  BOOLEAN         NOT NULL DEFAULT false,  -- 软删除，保留历史告警事件的关联
    created_by  UUID            REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_rules_device ON alert_rules(device_id) WHERE is_active AND NOT is_deleted;

CREATE TRIGGER trg_alert_rules_updated_at
BEFORE UPDATE ON alert_rules
FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- =============================================================================
-- 10. 告警事件（Hypertable，时序记录）
-- =============================================================================
CREATE TABLE alert_events (
    triggered_at  TIMESTAMPTZ    NOT NULL,
    alert_rule_id UUID           NOT NULL REFERENCES alert_rules(id) ON DELETE RESTRICT,  -- 规则删除前必须先处理事件
    device_id     UUID           NOT NULL REFERENCES devices(id) ON DELETE RESTRICT,       -- 设备使用软删除，不会真正删除行
    metric        TEXT           NOT NULL,
    actual_value  NUMERIC        NOT NULL,
    threshold     NUMERIC        NOT NULL,
    severity      alert_severity NOT NULL,
    message       TEXT,
    acknowledged  BOOLEAN        NOT NULL DEFAULT false,
    ack_by        UUID           REFERENCES users(id) ON DELETE SET NULL,
    ack_at        TIMESTAMPTZ,
    PRIMARY KEY (triggered_at, alert_rule_id)
);

SELECT create_hypertable(
    'alert_events',
    'triggered_at',
    chunk_time_interval => INTERVAL '7 days'
);

CREATE INDEX idx_alert_events_device  ON alert_events (device_id, triggered_at DESC);
CREATE INDEX idx_alert_events_unacked ON alert_events (acknowledged, triggered_at DESC)
    WHERE NOT acknowledged;