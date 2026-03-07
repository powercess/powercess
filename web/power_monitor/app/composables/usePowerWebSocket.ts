/**
 * usePowerWebSocket
 *
 * 封装对后端 WebSocket 接口的订阅逻辑：
 *   WS /ws/measurements         — 订阅所有设备的实时数据
 *   WS /ws/measurements/{mac}   — 订阅指定设备的实时数据
 *
 * 特性：
 * - 连接建立后服务端先推送快照，随后实时推送新采集数据
 * - 组件卸载时自动断连
 * - 提供可响应的 status、lastError
 */

import type { MeasurementMsg, WsStatus } from "~/types";

export interface UsePowerWebSocketOptions {
  /** 过滤特定设备 MAC；不传则订阅全部设备 */
  mac?: string;
  /** 收到新消息时的回调 */
  onMessage: (msg: MeasurementMsg) => void;
}

export function usePowerWebSocket(options: UsePowerWebSocketOptions) {
  const status = ref<WsStatus>("closed");
  const lastError = ref<string | null>(null);

  // 在 composable 顶层（Vue setup 上下文）读取配置，获取 Ref 而非立即求值
  const config = useRuntimeConfig();

  let ws: WebSocket | null = null;

  function resolveWsBase(): string {
    const configured = config.public.wsBase as string;
    if (configured) return configured;
    // 开发代理场景：根据当前页面协议推导 ws:// / wss://
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${window.location.host}`;
  }

  function connect() {
    if (import.meta.server) return;

    const wsBase = resolveWsBase();
    const path = options.mac
      ? `/ws/measurements/${encodeURIComponent(options.mac)}`
      : "/ws/measurements";
    const url = `${wsBase}${path}`;

    ws = new WebSocket(url);
    status.value = "connecting";
    lastError.value = null;

    ws.onopen = () => {
      status.value = "open";
    };

    ws.onmessage = (event: MessageEvent<string>) => {
      try {
        const msg = JSON.parse(event.data) as MeasurementMsg;
        options.onMessage(msg);
      } catch (e) {
        console.warn("[PowerWS] JSON 解析失败:", e);
      }
    };

    ws.onerror = () => {
      status.value = "error";
      lastError.value = "连接异常";
    };

    ws.onclose = () => {
      status.value = "closed";
    };
  }

  function disconnect() {
    ws?.close();
    ws = null;
  }

  onMounted(() => connect());
  onBeforeUnmount(() => disconnect());

  return {
    /** WebSocket 当前连接状态 */
    status: readonly(status),
    /** 最近一次错误信息（无则 null） */
    lastError: readonly(lastError),
    /** 手动重连 */
    reconnect: () => {
      disconnect();
      connect();
    },
  };
}
