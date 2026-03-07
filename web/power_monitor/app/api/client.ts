/**
 * API 基础客户端
 *
 * useApiBase / useWsBase 是 Vue composable，必须在 setup 或 composable 顶层调用。
 * apiFetch 是纯函数，接受 base URL 参数，供 composable 透传使用。
 */

/** 获取 HTTP 基础地址（必须在 Vue setup 上下文中调用） */
export function useApiBase(): string {
  const config = useRuntimeConfig();
  return config.public.apiBase as string;
}

/**
 * 获取 WebSocket 基础地址（必须在 Vue setup 上下文中调用）
 * 当 runtimeConfig.public.wsBase 为空字符串时（开发代理场景），
 * 自动根据当前页面协议推导：http→ws，https→wss，路径前缀 /ws 由代理处理
 */
export function useWsBase(): string {
  const config = useRuntimeConfig();
  const configured = config.public.wsBase as string;
  if (configured) return configured;

  // 客户端动态推导（服务端渲染阶段返回空，WS 连接只在 onMounted 后发起）
  if (import.meta.client) {
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${window.location.host}`;
  }
  return "";
}

/**
 * 基础 fetch 包装（纯函数，base 由调用方在顶层读取后传入）
 * @param base  HTTP 基础地址，如 'http://localhost:3030'
 * @param path  接口路径，如 '/api/devices'
 * @param opts  $fetch 选项
 */
export function apiFetch<T>(
  base: string,
  path: string,
  opts?: Parameters<typeof $fetch>[1],
) {
  return $fetch<T>(`${base}${path}`, opts);
}
