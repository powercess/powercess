// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2025-07-15",
  devtools: { enabled: true },
  modules: ["@nuxt/eslint", "@nuxt/image", "@nuxt/ui"],
  css: ["~/assets/css/main.css"],

  /**
   * 运行时配置
   *
   * 开发模式： apiBase 留空（请求经过 Vite 代理转发），wsBase 留空（客户端动态推导）
   * 生产环境覆盖：
   *   NUXT_PUBLIC_API_BASE=http://192.168.1.x:3030
   *   NUXT_PUBLIC_WS_BASE=ws://192.168.1.x:3030
   */
  runtimeConfig: {
    public: {
      /**
       * Rust 后端 HTTP 基地址
       * 开发模式置空字符串——请求经 Vite devProxy 转发，起到跳过 CORS 的作用
       * 生产环境通过环境变量设置完整地址
       */
      apiBase: "http://127.0.0.1:8080",
      /**
       * Rust 后端 WebSocket 基地址
       * 留空时客户端动态推导（ws:// 毴当前页面协议）
       */
      wsBase: "ws://127.0.0.1:8080",
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
