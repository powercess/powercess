/**
 * 设备相关 REST API
 *
 * GET /health          — 健康检查
 * GET /api/devices     — 受监控设备列表
 */

import type { DeviceInfo } from "~/types";
import { apiFetch } from "./client";

/**
 * 健康检查
 * @param base HTTP 基础地址（由 composable 顶层读取后传入）
 * @returns 后端正常返回 true，异常 throw
 */
export async function fetchHealth(base: string): Promise<boolean> {
  await apiFetch<void>(base, "/health");
  return true;
}

/**
 * 获取受监控设备列表
 * @param base HTTP 基础地址
 */
export async function fetchDevices(base: string): Promise<DeviceInfo[]> {
  return apiFetch<DeviceInfo[]>(base, "/api/devices");
}
