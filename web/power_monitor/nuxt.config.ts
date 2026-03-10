// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2025-07-15",
  ssr: false,
  devtools: { enabled: true },
  modules: ["@nuxt/eslint", "@nuxt/image", "@nuxt/ui"],
  css: ["~/assets/css/main.css"],

  /**
   * 运行时配置
   *
   * 开发模式：使用 Vite 代理转发
   * 生产环境：容器启动时通过 docker-entrypoint.sh 替换占位符
   *
   * 运行容器时传入环境变量：
   *   docker run -e NUXT_PUBLIC_API_BASE=http://xxx -e NUXT_PUBLIC_WS_BASE=ws://xxx ...
   */
  runtimeConfig: {
    public: {
      /**
       * Rust 后端 HTTP 基地址
       * 生产环境使用占位符，容器启动时替换
       */
      apiBase: process.env.NUXT_PUBLIC_API_BASE || "__NUXT_PUBLIC_API_BASE__",
      /**
       * Rust 后端 WebSocket 基地址
       * 生产环境使用占位符，容器启动时替换
       */
      wsBase: process.env.NUXT_PUBLIC_WS_BASE || "__NUXT_PUBLIC_WS_BASE__",
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