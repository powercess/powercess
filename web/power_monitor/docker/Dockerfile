# 构建阶段
FROM oven/bun:1 AS builder

WORKDIR /app

# 复制 package 文件
COPY web/power_monitor/package.json ./

# 安装依赖
RUN bun install

# 复制源代码
COPY web/power_monitor/ ./

# 构建应用
RUN bun run build

# 生产阶段
FROM oven/bun:1-slim AS production

WORKDIR /app

# 复制构建产物
COPY --from=builder /app/.output ./.output
COPY --from=builder /app/package.json ./

# 暴露端口
EXPOSE 3000

# 启动 Nuxt 服务
CMD ["bun", "run", ".output/server/index.mjs"]