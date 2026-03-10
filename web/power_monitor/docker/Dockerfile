# 构建阶段
FROM node:20-alpine AS builder

WORKDIR /app

# 复制 package 文件
COPY web/power_monitor/package*.json ./

# 安装依赖
RUN npm ci

# 复制源代码
COPY web/power_monitor/ ./

# 构建应用
RUN npm run build

# 生产阶段 - 使用 nginx 托管静态文件
FROM nginx:alpine AS production

# 复制构建产物到 nginx
COPY --from=builder /app/.output/public /usr/share/nginx/html

# 复制 nginx 配置
COPY web/power_monitor/docker/nginx.conf /etc/nginx/conf.d/default.conf

# 复制入口脚本
COPY web/power_monitor/docker/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

# 暴露端口
EXPOSE 80

ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]