//! BLE 帧协议：德力西功率计私有协议
//!
//! 帧格式（大端）：
//!   Header(2) | CMD(2) | Length(2) | Data(Length*2-8 字节) | CRC(2)
//!   Length = 整帧 word 数（含 Header / CMD / Length / CRC 自身）
//!
//! CRC = ~(Σ words) & 0xFFFF（有符号 i16 存储）

use chrono::Utc;
use tracing::{debug, warn};

use crate::error::{AppError, AppResult};
use crate::model::{Measurement, PfType};

// ── BLE UUID ─────────────────────────────────────────────────────────────────

use uuid::{uuid, Uuid};
pub const SERVICE_UUID: Uuid = uuid!("0000ff00-0000-1000-8000-00805f9b34fb");
pub const WRITE_CHAR_UUID: Uuid = uuid!("0000ff02-0000-1000-8000-00805f9b34fb");
pub const NOTIFY_CHAR_UUID: Uuid = uuid!("0000ff01-0000-1000-8000-00805f9b34fb");

// ── CRC ───────────────────────────────────────────────────────────────────────

fn calc_crc(words: &[u16]) -> i16 {
    let sum: u32 = words.iter().map(|&w| w as u32).sum();
    ((!sum) as u16) as i16
}

// ── 发送帧构建 ────────────────────────────────────────────────────────────────

/// 构建 0xF001 实时查询帧（8 字节，无数据段）。
pub fn build_f001_query() -> Vec<u8> {
    let header: u16 = 0xEB90;
    let cmd: u16 = 0xF001;
    let length: u16 = 4; // 4 word = 8 字节
    let crc = calc_crc(&[header, cmd, length]);

    let mut frame = Vec::with_capacity(8);
    frame.extend_from_slice(&header.to_be_bytes());
    frame.extend_from_slice(&cmd.to_be_bytes());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&crc.to_be_bytes());

    debug!("[TX] F001 查询: {} ({} B)", fmt_hex(&frame), frame.len());
    frame
}

// ── 响应帧解析 ────────────────────────────────────────────────────────────────

/// 解析 0xF001 响应帧，返回 `Measurement`。
pub fn parse_f001_response(device_mac: &str, data: &[u8]) -> AppResult<Measurement> {
    debug!("[RX] 原始 ({} B): {}", data.len(), fmt_hex(data));

    if data.len() < 8 {
        return Err(AppError::FrameFormat(format!(
            "帧太短：{} 字节（最少需要 8）",
            data.len()
        )));
    }

    let header = u16::from_be_bytes([data[0], data[1]]);
    let cmd = u16::from_be_bytes([data[2], data[3]]);
    let length = u16::from_be_bytes([data[4], data[5]]);
    let total_bytes = length as usize * 2;
    let data_len = total_bytes.saturating_sub(8);

    debug!(
        "[HDR] Header=0x{header:04X} CMD=0x{cmd:04X} Length={length} \
         => 整帧={total_bytes}B 数据段={data_len}B"
    );

    if data.len() < total_bytes {
        return Err(AppError::FrameFormat(format!(
            "帧长度不足：期望 {total_bytes} B，实际 {} B",
            data.len()
        )));
    }

    let raw = &data[6..6 + data_len];
    let crc_off = 6 + data_len;
    let recv_crc = i16::from_be_bytes([data[crc_off], data[crc_off + 1]]);

    // CRC 校验
    let mut words: Vec<u16> = vec![header, cmd, length];
    for chunk in raw.chunks_exact(2) {
        words.push(u16::from_be_bytes([chunk[0], chunk[1]]));
    }
    let calc = calc_crc(&words);
    debug!("[CRC] 接收={recv_crc} 计算={calc}");
    if calc != recv_crc {
        warn!("[CRC] 校验失败");
        return Err(AppError::CrcMismatch {
            expected: calc,
            actual: recv_crc,
        });
    }

    // 字段解析（最少 24 字节数据段）
    if data_len < 24 {
        return Err(AppError::FrameFormat(format!(
            "数据段太短：{data_len} B（最少 24 B）"
        )));
    }

    let voltage_raw = i32::from_be_bytes(raw[0..4].try_into().unwrap());
    let current_raw = i32::from_be_bytes(raw[4..8].try_into().unwrap());
    let power_raw = i32::from_be_bytes(raw[8..12].try_into().unwrap());
    let freq_raw = i16::from_be_bytes(raw[12..14].try_into().unwrap());
    let pf_raw = i16::from_be_bytes(raw[14..16].try_into().unwrap());
    let energy_raw = i32::from_be_bytes(raw[16..20].try_into().unwrap());
    let time_sec = i32::from_be_bytes(raw[20..24].try_into().unwrap());

    let voltage = voltage_raw as f64 * 0.001;
    let current = current_raw as f64 * 0.001;
    let power = power_raw as f64 * 0.001;
    let frequency = freq_raw as f64 * 0.1;
    let mut pf_value = pf_raw as f64 * 0.01;
    let energy = energy_raw as f64 * 0.001;

    let pf_display = (pf_value * 100.0).round() / 100.0;
    let pf_type = if pf_display == 0.0 || pf_display == 1.0 {
        PfType::Resistive
    } else if pf_display == -1.0 {
        pf_value = pf_value.abs();
        PfType::Resistive
    } else if pf_value > 0.0 {
        PfType::Inductive
    } else {
        pf_value = pf_value.abs();
        PfType::Capacitive
    };

    if data_len > 24 {
        debug!("[EXT] 额外 {} B: {}", data_len - 24, fmt_hex(&raw[24..]));
    }

    Ok(Measurement {
        recorded_at: Utc::now(),
        device_mac: device_mac.to_string(),
        voltage,
        current,
        power,
        frequency,
        power_factor: pf_value,
        pf_type,
        energy,
        uptime_secs: time_sec,
    })
}

// ── 工具函数 ──────────────────────────────────────────────────────────────────

pub fn fmt_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
