/**
 * 测量值消息结构
 * 对应后端 reporter::http::MeasurementMsg（REST 响应 / WebSocket 推送两用）
 */
export interface MeasurementMsg {
  /** 设备 MAC 地址（大写） */
  device_mac: string;
  /** 采集时刻（ISO 8601 字符串） */
  recorded_at: string;
  /** 电压 (V) */
  voltage_v: number;
  /** 电流 (A) */
  current_a: number;
  /** 有功功率 (W) */
  power_w: number;
  /** 频率 (Hz) */
  frequency_hz: number;
  /** 功率因数（绝对值，0–1） */
  power_factor: number;
  /** 功率因数类型（感性 / 容性 / 空字符串） */
  pf_type: string;
  /** 累计用电量 (kWh) */
  energy_kwh: number;
  /** 设备累计通电时间 (秒) */
  uptime_secs: number;
}

/** WebSocket 连接状态 */
export type WsStatus = "connecting" | "open" | "closed" | "error";
