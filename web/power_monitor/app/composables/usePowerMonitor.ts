/**
 * usePowerMonitor
 *
 * 主数据 composable：聚合设备列表 + 实时测量数据
 *
 * 流程：
 * 1. 组件挂载时通过 REST 拉取设备列表 + 最新测量快照
 * 2. 建立 WebSocket 连接，实时更新 measurements Map
 *
 * 组件只需调用此 composable，从中读取响应式状态即可，无需关心数据来源。
 */

import type { DeviceInfo, MeasurementMsg } from "~/types";
import { fetchDevices, fetchHealth } from "~/api/devices";
import { fetchAllMeasurements } from "~/api/measurements";
import { useApiBase } from "~/api/client";
import { usePowerWebSocket } from "./usePowerWebSocket";

export function usePowerMonitor() {
  // ── 在 composable 顶层（setup 上下文）读取配置 ──────────────────────────────
  const apiBase = useApiBase();

  // ── 状态 ────────────────────────────────────────────────────────────────────

  const devices = ref<DeviceInfo[]>([]);
  /** key = device_mac（大写），value = 最新测量值 */
  const measurements = ref<Map<string, MeasurementMsg>>(new Map());
  const isLoading = ref(true);
  const backendOnline = ref<boolean | null>(null);
  const fetchError = ref<string | null>(null);
  /** 最近一次数据更新时间 */
  const lastUpdatedAt = ref<Date | null>(null);

  // ── 初始数据拉取 ────────────────────────────────────────────────────────────

  async function loadInitialData() {
    isLoading.value = true;
    fetchError.value = null;

    try {
      // 健康检查
      backendOnline.value = await fetchHealth(apiBase);
    } catch {
      backendOnline.value = false;
      fetchError.value = "后端离线或无法连接";
      isLoading.value = false;
      return;
    }

    try {
      const [devList, snapshots] = await Promise.all([
        fetchDevices(apiBase),
        fetchAllMeasurements(apiBase),
      ]);

      devices.value = devList;

      const map = new Map<string, MeasurementMsg>();
      for (const m of snapshots) {
        map.set(m.device_mac, m);
      }
      measurements.value = map;
      lastUpdatedAt.value = new Date();
    } catch (e) {
      fetchError.value = e instanceof Error ? e.message : "数据加载失败";
    } finally {
      isLoading.value = false;
    }
  }

  // ── WebSocket 实时更新 ───────────────────────────────────────────────────────

  const {
    status: wsStatus,
    lastError: wsError,
    reconnect,
  } = usePowerWebSocket({
    onMessage(msg) {
      measurements.value = new Map(measurements.value).set(msg.device_mac, msg);
      lastUpdatedAt.value = new Date();
    },
  });

  // ── 计算属性 ────────────────────────────────────────────────────────────────

  /** 按设备列表顺序返回带测量值的设备，无快照的设备仍显示（measurement 为 undefined） */
  const deviceMeasurements = computed(() =>
    devices.value.map((dev) => ({
      device: dev,
      measurement: measurements.value.get(dev.mac.toUpperCase()),
    })),
  );

  /** 统计数据 */
  const stats = computed(() => {
    const vals = Array.from(measurements.value.values());
    return {
      totalDevices: devices.value.length,
      activeDevices: vals.length,
      totalPowerW: vals.reduce((sum, m) => sum + m.power_w, 0),
      totalEnergyKwh: vals.reduce((sum, m) => sum + m.energy_kwh, 0),
    };
  });

  // ── 生命周期 ────────────────────────────────────────────────────────────────

  onMounted(() => loadInitialData());

  return {
    // 数据
    devices: readonly(devices),
    measurements: readonly(measurements),
    deviceMeasurements,
    stats,
    // 状态
    isLoading: readonly(isLoading),
    backendOnline: readonly(backendOnline),
    fetchError: readonly(fetchError),
    lastUpdatedAt: readonly(lastUpdatedAt),
    wsStatus,
    wsError,
    // 方法
    refresh: loadInitialData,
    reconnect,
  };
}
