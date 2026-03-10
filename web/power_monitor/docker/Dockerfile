# 构建阶段
FROM docker.io/oven/bun:1 AS builder

WORKDIR /app

# 复制 package 文件
COPY web/power_monitor/package.json ./

# 安装所有依赖
RUN bun install

# 复制源代码
COPY web/power_monitor/ ./

# 构建应用
RUN bun run build

# 生产阶段 - 使用 Node.js 运行
FROM docker.io/node:20-alpine AS production

WORKDIR /app

# 复制构建产物
COPY --from=builder /app/.output ./.output

# 暴露端口
EXPOSE 3000

# 设置环境变量
ENV HOST=0.0.0.0
ENV PORT=3000
ENV NODE_ENV=production

# 启动 Nuxt 服务
CMD ["node", ".output/server/index.mjs"]