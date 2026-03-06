//! HTTP + WebSocket 上报层（feature: reporter-http）
//!
//! REST 接口：
//!   GET  /health                        — 健康检查
//!   GET  /api/devices                   — 受监控设备列表
//!   GET  /api/measurements              — 所有设备最新数据（快照）
//!   GET  /api/measurements/{mac}        — 指定设备最新数据（快照）
//!
//! WebSocket 接口（实时推送，每次采集后立即下发）：
//!   WS   /ws/measurements               — 订阅所有设备的实时数据
//!   WS   /ws/measurements/{mac}         — 订阅指定设备的实时数据
//!
//! 连接建立后，服务端先推送已有快照，随后实时推送每次新采集数据。

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{debug, info, warn};

use crate::error::AppResult;
use crate::model::{DeviceInfo, Measurement};
use crate::reporter::Reporter;

// ── 类型别名 ──────────────────────────────────────────────────────────────────

/// 最新快照缓存（key = MAC 大写）
pub type LatestStore = Arc<DashMap<String, MeasurementMsg>>;

/// 广播频道容量：树莓派 4 上保留 64 条即可
const BROADCAST_CAPACITY: usize = 64;

// ── 消息格式（REST + WS 共用） ─────────────────────────────────────────────────

/// 对外暴露的测量值 JSON 结构（REST 响应 / WebSocket 推送两用）
#[derive(Serialize, Clone, Debug)]
pub struct MeasurementMsg {
    pub device_mac:   String,
    pub recorded_at:  String,
    pub voltage_v:    f64,
    pub current_a:    f64,
    pub power_w:      f64,
    pub frequency_hz: f64,
    pub power_factor: f64,
    pub pf_type:      String,
    pub energy_kwh:   f64,
    pub uptime_secs:  i32,
}

impl From<&Measurement> for MeasurementMsg {
    fn from(m: &Measurement) -> Self {
        Self {
            device_mac:   m.device_mac.clone(),
            recorded_at:  m.recorded_at.to_rfc3339(),
            voltage_v:    m.voltage,
            current_a:    m.current,
            power_w:      m.power,
            frequency_hz: m.frequency,
            power_factor: m.power_factor,
            pf_type:      m.pf_type.label().to_string(),
            energy_kwh:   m.energy,
            uptime_secs:  m.uptime_secs,
        }
    }
}

// ── HttpReporter（写端） ──────────────────────────────────────────────────────

pub struct HttpReporter {
    latest: LatestStore,
    tx:     broadcast::Sender<Arc<MeasurementMsg>>,
}

impl HttpReporter {
    pub fn new(latest: LatestStore, tx: broadcast::Sender<Arc<MeasurementMsg>>) -> Self {
        Self { latest, tx }
    }
}

#[async_trait]
impl Reporter for HttpReporter {
    async fn report(&self, _device: &DeviceInfo, m: &Measurement) -> AppResult<()> {
        let msg = Arc::new(MeasurementMsg::from(m));
        self.latest.insert(m.device_mac.clone(), (*msg).clone());
        let _ = self.tx.send(msg); // 无订阅者时返回 Err，忽略
        Ok(())
    }
}

// ── axum 共享状态 ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    latest:  LatestStore,
    devices: Arc<Vec<DeviceInfo>>,
    tx:      broadcast::Sender<Arc<MeasurementMsg>>,
}

// ── 公共工厂 ──────────────────────────────────────────────────────────────────

/// 创建广播频道，返回 `(Sender, HttpReporter)`。
/// 调用方把 Sender 传给 `serve()`，把 HttpReporter 注册到上报链。
pub fn make_reporter(
    latest: LatestStore,
) -> (broadcast::Sender<Arc<MeasurementMsg>>, HttpReporter) {
    let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
    let reporter = HttpReporter::new(latest, tx.clone());
    (tx, reporter)
}

/// 启动 HTTP/WebSocket 服务。
/// `listener` 由调用方预先 bind（确保端口在 BLE 任务启动前已就绪）。
pub async fn serve(
    listener: tokio::net::TcpListener,
    latest:   LatestStore,
    devices:  Vec<DeviceInfo>,
    tx:       broadcast::Sender<Arc<MeasurementMsg>>,
) -> anyhow::Result<()> {
    let state = AppState {
        latest,
        devices: Arc::new(devices),
        tx,
    };

    let app = Router::new()
        // REST
        .route("/health",                 get(health))
        .route("/api/devices",            get(list_devices))
        .route("/api/measurements",       get(all_measurements))
        .route("/api/measurements/{mac}", get(get_measurement))
        // WebSocket
        .route("/ws/measurements",        get(ws_all))
        .route("/ws/measurements/{mac}",  get(ws_single))
        .layer(CorsLayer::permissive())
        .with_state(state);

    info!("[HTTP] 服务就绪: http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

// ── REST 处理器 ───────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn list_devices(State(s): State<AppState>) -> Json<Vec<DeviceInfo>> {
    Json(s.devices.as_ref().clone())
}

async fn all_measurements(State(s): State<AppState>) -> Json<Vec<MeasurementMsg>> {
    Json(s.latest.iter().map(|e| e.value().clone()).collect())
}

async fn get_measurement(
    State(s):  State<AppState>,
    Path(mac): Path<String>,
) -> Result<Json<MeasurementMsg>, StatusCode> {
    match s.latest.get(&mac.to_uppercase()) {
        Some(m) => Ok(Json(m.value().clone())),
        None    => Err(StatusCode::NOT_FOUND),
    }
}

// ── WebSocket 处理器 ──────────────────────────────────────────────────────────

/// 订阅所有设备的实时数据
async fn ws_all(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, s, None))
}

/// 订阅指定设备的实时数据
async fn ws_single(
    ws:        WebSocketUpgrade,
    State(s):  State<AppState>,
    Path(mac): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, s, Some(mac.to_uppercase())))
}

/// 通用 WebSocket 会话：
///   1. 先推送当前快照（立即可见历史最新值）
///   2. 订阅广播频道，实时转发新数据直到客户端断开
async fn handle_ws(socket: WebSocket, state: AppState, filter_mac: Option<String>) {
    let (mut sink, mut stream) = socket.split();

    // ── 第一步：推送快照 ──────────────────────────────────────────────────────
    let snapshots: Vec<MeasurementMsg> = match &filter_mac {
        None      => state.latest.iter().map(|e| e.value().clone()).collect(),
        Some(mac) => state.latest.get(mac).map(|e| e.value().clone()).into_iter().collect(),
    };
    for snap in snapshots {
        let Ok(json) = serde_json::to_string(&snap) else { continue };
        if sink.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    // ── 第二步：实时推送广播 ──────────────────────────────────────────────────
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if let Some(ref mac) = filter_mac {
                            if msg.device_mac != *mac { continue; }
                        }
                        let Ok(json) = serde_json::to_string(&*msg) else { continue };
                        if sink.send(Message::Text(json.into())).await.is_err() {
                            debug!("[WS] 客户端断开");
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("[WS] 广播滞后，跳过 {n} 条消息");
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sink.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}
