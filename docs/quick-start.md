# powercess 启动指南

本文档说明如何启动和运行 powercess 程序。

## 快速开始

### 基本用法

```bash
# 使用指定配置文件启动
./powercess -c config.toml

# 或使用长参数形式
./powercess --config config.toml

# 使用绝对路径
./powercess -c /etc/powercess/config.toml
```

### 不带参数运行

如果不带任何参数运行程序，将显示帮助信息：

```bash
./powercess
```

输出示例：
```
powercess v1.3.1 - 德力西功率计 BLE 实时监控系统

用法:
  powercess -c <CONFIG_FILE>    使用指定配置文件启动
  powercess --config <FILE>     同上

示例:
  powercess -c config.toml
  powercess -c /etc/powercess/config.toml

更多帮助:
  powercess --help
  powercess --version
```

### 查看版本和帮助

```bash
# 查看版本
./powercess --version

# 查看详细帮助
./powercess --help
```

## 配置文件

### 配置文件位置

配置文件默认命名为 `config.toml`，可以放置在任意位置。启动时通过 `-c` 参数指定路径：

```bash
# 当前目录
./powercess -c config.toml

# /etc 目录
./powercess -c /etc/powercess/config.toml

# 用户目录
./powercess -c ~/powercess/config.toml
```

### 配置文件不存在

如果指定的配置文件不存在，程序将报错退出：

```
[ERROR] 配置文件不存在: /path/to/config.toml
```

### 配置文件格式

详见项目根目录的 `config.toml` 示例文件，主要包含以下部分：

- `[app]` - 应用程序设置（采集间隔、日志级别等）
- `[store]` - 数据存储配置（static/sqlite/postgres）
- `[reporter]` - 上报配置（HTTP API、数据库写入）
- `[[devices]]` - 静态设备列表

### 静态设备与数据库设备共存

powercess 支持**静态设备与数据库设备同时使用**。当 `store.type` 设置为 `sqlite` 或 `postgres` 时，`config.toml` 中的 `[[devices]]` 会自动与数据库中的设备合并。

**示例配置：**

```toml
[store]
type = "sqlite"
path = "/var/lib/powercess/powercess.db"

# reporter 配置...

# 这些设备会与数据库中的设备合并
[[devices]]
mac   = "AA:BB:CC:DD:EE:FF"
name  = "临时测试设备"
label = "测试插座"

[[devices]]
mac   = "11:22:33:44:55:66"
name  = "新设备"
label = "待录入数据库"
```

**合并规则：**

1. 数据库设备 + 静态设备 = 最终设备列表
2. 如果 MAC 地址重复，静态设备的定义会覆盖数据库中的记录
3. 可用于临时测试新设备，或将设备信息覆盖

**使用场景：**

| 场景 | 说明 |
|------|------|
| 临时测试 | 在录入数据库前先在配置文件中测试新设备 |
| 信息覆盖 | 用静态配置覆盖数据库中的设备名称/标签 |
| 混合管理 | 部分设备在数据库，部分设备在配置文件 |

## 下载与安装

### 从 GitHub Releases 下载

访问 [Releases 页面](https://github.com/your-repo/powercess/releases) 下载对应平台和数据库后端的二进制文件。

文件命名格式：
```
powercess-v<VERSION>-<TARGET>-<FEATURE>.tar.gz
```

| 字段 | 说明 | 示例 |
|------|------|------|
| VERSION | 版本号 | v1.3.1 |
| TARGET | 目标平台 | x86_64-unknown-linux-gnu, aarch64-apple-darwin |
| FEATURE | 数据库后端 | sqlite, postgres |

### 选择正确的版本

根据您的操作系统和数据库需求选择：

#### 按操作系统

| 操作系统 | 架构 | 目标平台 |
|----------|------|----------|
| Linux (x86_64) | 64位 | x86_64-unknown-linux-gnu |
| Linux (ARM64) | 树莓派 4/5, Apple Silicon Mac | aarch64-unknown-linux-gnu |
| Linux (ARMv7) | 树莓派 2/3/4 (32位) | armv7-unknown-linux-gnueabihf |
| Linux (x86) | 32位 | i686-unknown-linux-gnu |
| macOS (Apple Silicon) | M1/M2/M3 | aarch64-apple-darwin |

#### 按数据库后端

| Feature | 说明 | 适用场景 |
|---------|------|----------|
| sqlite | SQLite 数据库 | 树莓派、嵌入式设备、单机部署 |
| postgres | PostgreSQL 数据库 | 生产环境、多实例共享数据 |

### 安装步骤

```bash
# 1. 下载压缩包
wget https://github.com/your-repo/powercess/releases/download/v1.3.1/powercess-v1.3.1-x86_64-unknown-linux-gnu-sqlite.tar.gz

# 2. 解压
tar xzf powercess-v1.3.1-x86_64-unknown-linux-gnu-sqlite.tar.gz

# 3. 查看内容
ls
# config.toml  powercess

# 4. 赋予执行权限
chmod +x powercess

# 5. 修改配置文件（根据实际需求）
vim config.toml

# 6. 启动程序
./powercess -c config.toml
```

## 环境变量覆盖

所有配置项都可以通过环境变量覆盖，格式为 `POWERCESS__<SECTION>__<KEY>`：

```bash
# 修改采集间隔
POWERCESS__APP__POLL_INTERVAL_SECS=5 ./powercess -c config.toml

# 修改 HTTP 监听端口
POWERCESS__REPORTER__HTTP_BIND="0.0.0.0:9090" ./powercess -c config.toml

# 切换数据库类型（需要对应 feature 版本）
POWERCESS__STORE__TYPE=sqlite \
POWERCESS__STORE__PATH="/var/lib/powercess/powercess.db" \
./powercess -c config.toml
```

## 日志级别

通过 `RUST_LOG` 环境变量控制日志输出：

```bash
# 详细调试日志
RUST_LOG=debug ./powercess -c config.toml

# 仅错误日志
RUST_LOG=error ./powercess -c config.toml

# 特定模块日志
RUST_LOG=powercess::ble=trace ./powercess -c config.toml
```

## Systemd 服务配置（推荐）

创建 systemd 服务文件实现开机自启和后台运行：

```bash
# 创建服务文件
sudo vim /etc/systemd/system/powercess.service
```

内容如下：

```ini
[Unit]
Description=Powercess BLE Power Monitor
After=network.target bluetooth.target

[Service]
Type=simple
User=powercess
Group=powercess
WorkingDirectory=/opt/powercess
ExecStart=/opt/powercess/powercess -c /opt/powercess/config.toml
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

启用服务：

```bash
# 创建用户
sudo useradd -r -s /bin/false powercess

# 创建目录
sudo mkdir -p /opt/powercess
sudo chown powercess:powercess /opt/powercess

# 复制文件
sudo cp powercess config.toml /opt/powercess/
sudo chmod +x /opt/powercess/powercess

# 启用并启动服务
sudo systemctl daemon-reload
sudo systemctl enable powercess
sudo systemctl start powercess

# 查看状态
sudo systemctl status powercess

# 查看日志
sudo journalctl -u powercess -f
```

## Docker 运行

```bash
# 构建镜像
docker build -t powercess .

# 运行容器
docker run -d \
  --name powercess \
  --privileged \
  --network host \
  -v /path/to/config.toml:/app/config.toml:ro \
  powercess -c /app/config.toml
```

> 注意：BLE 蓝牙功能需要 `--privileged` 或正确配置 Bluetooth 设备权限。

## 常见问题

### 1. 配置文件找不到

```
[ERROR] 配置文件不存在: config.toml
```

**解决方案**：确保使用 `-c` 参数指定正确的配置文件路径。

### 2. 蓝牙权限不足

```
Error: Permission denied while accessing Bluetooth adapter
```

**解决方案**：
- 确保用户在 `bluetooth` 用户组：`sudo usermod -aG bluetooth $USER`
- 或使用 root 权限运行

### 3. 端口被占用

```
Error: HTTP 端口 0.0.0.0:8080 绑定失败: Address already in use
```

**解决方案**：
- 修改 `config.toml` 中的 `reporter.http_bind`
- 或通过环境变量覆盖：`POWERCESS__REPORTER__HTTP_BIND="0.0.0.0:8081"`

### 4. 数据库连接失败

**SQLite 版本**：
```
Error: 初始化设备数据层失败: Unable to open database file
```

**解决方案**：确保数据库文件目录存在且有写入权限。

**PostgreSQL 版本**：
```
Error: 初始化设备数据层失败: connection refused
```

**解决方案**：检查数据库连接 URL 是否正确，确保 PostgreSQL 服务正在运行。