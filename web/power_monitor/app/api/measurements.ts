/**
 * 测量值相关 REST API
 *
 * GET /api/measurements         — 所有设备最新数据快照
 * GET /api/measurements/{mac}   — 指定设备最新数据快照
 */

import type { MeasurementMsg } from "~/types";
import { apiFetch } from "./client";

/**
 * 获取所有设备的最新测量快照
 * @param base HTTP 基础地址
 */
export async function fetchAllMeasurements(
  base: string,
): Promise<MeasurementMsg[]> {
  return apiFetch<MeasurementMsg[]>(base, "/api/measurements");
}

/**
 * 获取指定设备的最新测量快照
 * @param base HTTP 基础地址
 * @param mac  设备 MAC 地址（大写，冒号分隔）
 */
export async function fetchMeasurement(
  base: string,
  mac: string,
): Promise<MeasurementMsg> {
  return apiFetch<MeasurementMsg>(
    base,
    `/api/measurements/${encodeURIComponent(mac)}`,
  );
}
