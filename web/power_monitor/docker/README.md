# Power Monitor Docker 部署

## 快速开始

### 使用 Podman

```bash
# 构建镜像
podman build -f web/power_monitor/docker/Dockerfile -t power-monitor .

# 运行容器
podman run -d \
  -p 3000:3000 \
  -e NUXT_PUBLIC_API_BASE=http://server.taten.org:5090 \
  -e NUXT_PUBLIC_WS_BASE=ws://server.taten.org:5090 \
  --name power-monitor \
  power-monitor
```

### 使用 Docker Compose

```bash
cd web/power_monitor/docker
podman-compose up -d
```

## 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `NUXT_PUBLIC_API_BASE` | 后端 HTTP API 地址 | 空 |
| `NUXT_PUBLIC_WS_BASE` | WebSocket 地址 | 空 |