// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2025-07-15",
  ssr: false,
  devtools: { enabled: true },
  modules: ["@nuxt/eslint", "@nuxt/image", "@nuxt/ui"],
  css: ["~/assets/css/main.css"],

  nitro: {
    // 信任 X-Forwarded-Proto 等头部，解决反向代理后的协议识别问题
    // 设置为 true 表示信任所有直接连接的代理（适合内网穿透场景）
    // 如果出于安全考虑，也可以指定具体的 IP 数组，例如：['127.0.0.1', '172.16.0.0/12']
    xForwardedHeaders: {
      trustProxy: true,
    },
  },

  /**
   * 运行时配置
   *
   * 运行容器时传入环境变量：
   *   podman run -e NUXT_PUBLIC_API_BASE=http://xxx -e NUXT_PUBLIC_WS_BASE=ws://xxx ...
   */
  runtimeConfig: {
    public: {
      /**
       * Rust 后端 HTTP 基地址
       */
      apiBase: process.env.NUXT_PUBLIC_API_BASE || "",
      /**
       * Rust 后端 WebSocket 基地址
       */
      wsBase: process.env.NUXT_PUBLIC_WS_BASE || "",
    },
  },

  /**
   * Vite 开发代理
   * 将 /api 、/health 、/ws 请求转发到 Rust 后端，彻底解决开发模式 CORS 问题
   */
  vite: {
    server: {
      proxy: {
        "/health": {
          target: "http://localhost:3030",
          changeOrigin: true,
        },
        "/api": {
          target: "http://localhost:3030",
          changeOrigin: true,
        },
        "/ws": {
          target: "ws://localhost:3030",
          changeOrigin: true,
          ws: true,
        },
      },
    },
  },
});
