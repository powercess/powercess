# Power Monitor Docker 部署

## 文件说明

| 文件 | 说明 |
|------|------|
| `Dockerfile` | 多阶段构建镜像定义 |
| `docker-compose.yml` | Docker Compose 配置 |
| `nginx.conf` | Nginx 静态文件服务配置 |
| `docker-entrypoint.sh` | 容器启动脚本，动态注入环境变量 |
| `.env.example` | 环境变量示例文件 |

## 快速开始

### 使用 Docker Compose

```bash
cd web/power_monitor/docker

# 复制并编辑环境变量
cp .env.example .env

# 启动服务
docker-compose up -d
```

### 使用 Docker 直接构建

```bash
# 从项目根目录构建
docker build -f web/power_monitor/docker/Dockerfile -t power-monitor .

# 运行容器
docker run -d \
  -p 3000:80 \
  -e NUXT_PUBLIC_API_BASE=http://your-server:5090 \
  -e NUXT_PUBLIC_WS_BASE=ws://your-server:5090 \
  power-monitor
```

## 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `NUXT_PUBLIC_API_BASE` | 后端 HTTP API 地址 | `http://localhost:5090` |
| `NUXT_PUBLIC_WS_BASE` | WebSocket 地址 | `ws://localhost:5090` |