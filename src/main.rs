//! powercess — 德力西功率计 BLE 实时监控系统
//!
//! 架构：
//!   config  ─► store (StaticStore / SQLite / PostgreSQL)
//!           ─► reporter (DB + HTTP API)
//!           ─► monitor::run_all (每台设备独立 tokio task)
//!
//! 用法：
//!   powercess -c config.toml                  # 使用指定配置文件启动
//!   powercess --config /etc/powercess.toml    # 同上
//!   powercess                                 # 显示帮助信息

mod ble;
mod config;
mod error;
mod model;
mod monitor;
mod reporter;
mod store;

use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::{AppConfig, StoreConfig};
use crate::reporter::MultiReporter;

/// powercess — 德力西功率计 BLE 实时监控系统
#[derive(Parser, Debug)]
#[command(name = "powercess")]
#[command(version)]
#[command(about = "德力西功率计 BLE 实时监控系统", long_about = None)]
struct Args {
    /// 配置文件路径 (如: config.toml)
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // 如果没有指定配置文件，显示帮助信息并退出
    let config_path = match args.config {
        Some(path) => path,
        None => {
            println!("powercess v{} - 德力西功率计 BLE 实时监控系统\n", env!("CARGO_PKG_VERSION"));
            println!("用法:");
            println!("  powercess -c <CONFIG_FILE>    使用指定配置文件启动");
            println!("  powercess --config <FILE>     同上");
            println!("\n示例:");
            println!("  powercess -c config.toml");
            println!("  powercess -c /etc/powercess/config.toml");
            println!("\n更多帮助:");
            println!("  powercess --help");
            println!("  powercess --version");
            std::process::exit(0);
        }
    };

    // 检查配置文件是否存在
    if !Path::new(&config_path).exists() {
        eprintln!("[ERROR] 配置文件不存在: {config_path}");
        std::process::exit(1);
    }

    // ── 加载配置 ─────────────────────────────────────────────────────
    let cfg = AppConfig::load(Some(&config_path)).unwrap_or_else(|e| {
        eprintln!("[WARN] 读取 {} 失败（{e}），使用默认配置", config_path);
        AppConfig::default()
    });

    // ── 初始化结构化日志 ─────────────────────────────────────────────────────
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cfg.app.log_level)),
        )
        .init();

    info!("powercess v{} 启动，配置文件: {}", env!("CARGO_PKG_VERSION"), config_path);

    if let Err(e) = run(cfg).await {
        error!("程序异常退出: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cfg: AppConfig) -> anyhow::Result<()> {
    // ── 1. 初始化设备数据层 ──────────────────────────────────────────────────
    let device_store = store::create_store(&cfg)
        .await
        .context("初始化设备数据层失败")?;

    let devices = device_store
        .list_devices()
        .await
        .context("获取设备列表失败")?;

    info!("共 {} 台设备需要监控", devices.len());
    for d in &devices {
        info!("  - {} ({}) label={:?}", d.name, d.mac, d.label);
    }

    // ── 2. 构建上报链 ────────────────────────────────────────────────────────
    let mut reporters: Vec<Arc<dyn reporter::Reporter>> = Vec::new();

    // SQLite 数据库上报
    #[cfg(feature = "store-sqlite")]
    if cfg.reporter.db_enabled {
        if let StoreConfig::Sqlite { ref path } = cfg.store {
            use reporter::database::SqliteDatabaseReporter;
            use store::sqlite::SqliteStore;

            let sqlite = SqliteStore::connect(path).await?;
            reporters.push(Arc::new(SqliteDatabaseReporter::new(
                sqlite.pool().clone(),
            )));
            info!("[Reporter] SQLite 数据库上报已启用: {path}");
        }
    }

    // PostgreSQL 数据库上报（写入 raw_measurements Hypertable）
    #[cfg(feature = "store-postgres")]
    if cfg.reporter.db_enabled {
        if let StoreConfig::Postgres { ref url } = cfg.store {
            use reporter::database::PostgresDatabaseReporter;
            use store::postgres::PostgresStore;

            let pg = PostgresStore::connect(url).await?;
            reporters.push(Arc::new(PostgresDatabaseReporter::new(
                pg.pool().clone(),
            )));
            info!("[Reporter] PostgreSQL 数据库上报已启用 (raw_measurements)");
        }
    }

    // HTTP REST API 上报
    // 先于 monitor 启动，确保端口已 bind 后再开始 BLE 任务
    #[allow(unused_mut)]
    let mut http_handle: Option<tokio::task::JoinHandle<()>> = None;

    #[cfg(feature = "reporter-http")]
    {
        use dashmap::DashMap;
        use reporter::http::make_reporter;

        if cfg.reporter.http_enabled {
            let latest = Arc::new(DashMap::new());
            let (tx, http_reporter) = make_reporter(latest.clone());
            reporters.push(Arc::new(http_reporter));

            let bind = cfg.reporter.http_bind.clone();
            let devs = devices.clone();
            // 先 bind 端口，再启动 monitor，保证 HTTP 先于 BLE 就绪
            let listener = tokio::net::TcpListener::bind(&bind)
                .await
                .with_context(|| format!("HTTP 端口 {bind} 绑定失败"))?;
            info!("[Reporter] HTTP API + WebSocket 已启用: http://{bind}");

            let handle = tokio::spawn(async move {
                if let Err(e) =
                    reporter::http::serve(listener, latest, devs, tx).await
                {
                    error!("[HTTP] 服务异常: {e}");
                }
            });
            http_handle = Some(handle);
        }
    }

    let reporter: Arc<dyn reporter::Reporter> = Arc::new(MultiReporter::new(reporters));

    // ── 3. 启动监控调度器（独立 task，不阻塞主流程） ──────────────────────────
    let monitor_handle = tokio::spawn(async move {
        monitor::run_all(devices, cfg.app, reporter).await;
    });

    // ── 4. 等待退出信号（Ctrl-C / SIGTERM），两者任一退出则关闭 ──────────────
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("收到 Ctrl-C，正在退出...");
        }
        _ = monitor_handle => {
            // 所有设备 task 全部退出（不应发生，仅作保底）
            error!("监控任务意外退出");
        }
        // 若 HTTP 未启用，http_handle 为 None，此分支永不匹配
        Some(res) = async { if let Some(h) = http_handle { Some(h.await) } else { None } } => {
            if let Err(e) = res {
                error!("HTTP task panic: {e}");
            }
        }
    }

    Ok(())
}