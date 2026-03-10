//! 配置加载：运行时读取 `config.toml`，支持环境变量覆盖（前缀 `POWERCESS__`）。
//!
//! 加载优先级（后者覆盖前者）：
//!   1. Rust const 默认值（兖底，保证缺字段不 panic）
//!   2. 运行目录的 `config.toml`（动态配置，改后重启生效，无需重新编译）
//!   3. `POWERCESS__` 环境变量（非交互式部署覆盖）

use crate::model::DeviceInfo;
use config_crate::{Config, Environment, File};
use serde::{Deserialize, Serialize};

// ── 默认值常量（兖底，config.toml 注释与此保持一致）────────────────────────
const DEFAULT_POLL_INTERVAL_SECS:   u64   = 1;
const DEFAULT_WORKER_THREADS:       usize = 2;
const DEFAULT_SCAN_TIMEOUT_SECS:    u64   = 15;
const DEFAULT_RETRY_INTERVAL_SECS:  u64   = 30;
const DEFAULT_MAX_BLE_CONNECTIONS:  usize = 5;

fn default_poll_interval()      -> u64    { DEFAULT_POLL_INTERVAL_SECS }
fn default_worker_threads()     -> usize  { DEFAULT_WORKER_THREADS }
fn default_log_level()          -> String { "info".into() }
fn default_scan_timeout()       -> u64    { DEFAULT_SCAN_TIMEOUT_SECS }
fn default_retry_interval()     -> u64    { DEFAULT_RETRY_INTERVAL_SECS }
fn default_max_ble_connections() -> usize  { DEFAULT_MAX_BLE_CONNECTIONS }
fn default_sqlite_path()    -> String { "powercess.db".into() }
fn default_http_bind()      -> String { "0.0.0.0:8080".into() }
fn bool_true()              -> bool   { true }
fn bool_false()             -> bool   { false }

// ── 顶层配置结构 ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub app: AppSettings,
    pub store: StoreConfig,
    pub reporter: ReporterConfig,
    /// 仅 store.r#type = "static" 时生效
    #[serde(default)]
    pub devices: Vec<DeviceInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppSettings {
    /// 采集间隔（秒）
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    /// tokio worker 线程数（树莓派建议 2）
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    /// 日志级别（trace/debug/info/warn/error）
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// BLE 扫描单次最长时间（秒）
    #[serde(default = "default_scan_timeout")]
    pub scan_timeout_secs: u64,
    /// 连接失败后，重试间隔（秒）
    #[serde(default = "default_retry_interval")]
    pub retry_interval_secs: u64,
    /// 同时允许保持的 BLE 连接数上限（受限于蓝牙适配器硬件，通常 5~7）
    #[serde(default = "default_max_ble_connections")]
    pub max_ble_connections: usize,
}

// ── 存储后端配置 ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoreConfig {
    /// 静态列表（硬编码在 config.toml 的 [[devices]] 节）
    Static,
    /// SQLite（feature: store-sqlite）
    Sqlite {
        /// 数据库文件路径
        #[serde(default = "default_sqlite_path")]
        path: String,
    },
    /// PostgreSQL（feature: store-postgres）
    Postgres {
        /// 连接 URL，如 "postgres://user:pass@host/db"
        url: String,
    },
}

// ── 上报配置 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReporterConfig {
    /// 是否启用 HTTP API
    #[serde(default = "bool_true")]
    pub http_enabled: bool,
    /// HTTP 监听地址
    #[serde(default = "default_http_bind")]
    pub http_bind: String,
    /// 是否将测量值写入数据库
    #[serde(default = "bool_false")]
    pub db_enabled: bool,
}

// ── 加载入口 ──────────────────────────────────────────────────────────────────

impl AppConfig {
    /// 运行时加载配置。
    /// 当配置文件不存在时，所有字段回落到 Rust const 默认值。
    ///
    /// # 参数
    /// - `config_path`: 可选的配置文件路径。若为 None，则尝试从运行目录加载 `config.toml`
    pub fn load(config_path: Option<&str>) -> anyhow::Result<Self> {
        let mut builder = Config::builder();

        // 层 1：配置文件（可缺失）
        match config_path {
            Some(path) => {
                // 用户指定的配置文件路径
                builder = builder.add_source(File::with_name(path).required(false));
            }
            None => {
                // 默认从运行目录加载 config.toml
                builder = builder.add_source(File::with_name("config").required(false));
            }
        }

        // 层 2：环境变量覆盖
        let cfg = builder
            .add_source(
                Environment::with_prefix("POWERCESS")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        Ok(cfg.try_deserialize()?)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                poll_interval_secs:  DEFAULT_POLL_INTERVAL_SECS,
                worker_threads:      DEFAULT_WORKER_THREADS,
                log_level:           "info".into(),
                scan_timeout_secs:   DEFAULT_SCAN_TIMEOUT_SECS,
                retry_interval_secs: DEFAULT_RETRY_INTERVAL_SECS,
                max_ble_connections: DEFAULT_MAX_BLE_CONNECTIONS,
            },
            store: StoreConfig::Static,
            reporter: ReporterConfig {
                http_enabled: true,
                http_bind:    "0.0.0.0:8080".into(),
                db_enabled:   false,
            },
            devices: vec![],
        }
    }
}
