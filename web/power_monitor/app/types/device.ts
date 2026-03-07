/**
 * 受监控的 BLE 功率计设备信息
 * 对应后端 model::DeviceInfo
 */
export interface DeviceInfo {
  /** BLE MAC 地址（大写，冒号分隔，如 "12:10:37:4C:47:47"） */
  mac: string;
  /** 人类可读名称 */
  name: string;
  /** 可选位置/备注标签 */
  label?: string | null;
}
