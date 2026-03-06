# powercess 集成文档

本文档说明以下内容：

1. **外部应用如何消费** powercess 提供的实时功率数据（HTTP REST API）
2. **WebSocket 实时推送**：订阅所有设备或指定设备的实时数据流
3. **powercess 如何对接数据库**，从 SQLite / PostgreSQL 读取受监控的蓝牙设备列表

---

## 一、HTTP REST API

程序启动后在 `config.toml` 中 `reporter.http_bind`（默认 `0.0.0.0:8080`）地址提供 HTTP 服务。

> API 全部为只读的 `GET` 请求，无需鉴权，支持跨域（CORS 已开放）。

### 1.1 健康检查

```
GET /health
```

| 场景     | 状态码   |
| -------- | -------- |
| 服务正常 | `200 OK` |

适合 Kubernetes / Docker healthcheck 或反向代理探针使用。

---

### 1.2 获取受监控设备列表

```
GET /api/devices
```

**响应示例**

```json
[
  {
    "mac": "12:10:37:4C:47:47",
    "name": "德力西功率计-1",
    "label": "实验室插线板"
  }
]
```

| 字段    | 类型           | 说明                           |
| ------- | -------------- | ------------------------------ |
| `mac`   | string         | BLE MAC 地址（大写，冒号分隔） |
| `name`  | string         | 设备名称                       |
| `label` | string \| null | 位置/备注标签                  |

---

### 1.3 获取所有设备的最新测量值

```
GET /api/measurements
```

**响应示例**

```json
[
  {
    "device_mac": "12:10:37:4C:47:47",
    "recorded_at": "2026-03-06T08:59:11.174633Z",
    "voltage_v": 220.341,
    "current_a": 1.234,
    "power_w": 271.5,
    "frequency_hz": 50.0,
    "power_factor": 0.98,
    "pf_type": "感性",
    "energy_kwh": 12.345,
    "uptime_secs": 86400
  }
]
```

> 内存中只保留每台设备**最新一次**的测量值（每秒刷新）。

---

### 1.4 获取指定设备的最新测量值

```
GET /api/measurements/{mac}
```

`{mac}` 大小写均可，如 `12:10:37:4C:47:47` 或 `12:10:37:4c:47:47`。

| 场景              | 状态码          |
| ----------------- | --------------- |
| 找到数据          | `200 OK` + JSON |
| 设备未在线/未采集 | `404 Not Found` |

**响应字段说明**

| 字段           | 类型   | 单位         | 说明                       |
| -------------- | ------ | ------------ | -------------------------- |
| `device_mac`   | string | —            | 设备 MAC 地址              |
| `recorded_at`  | string | RFC 3339 UTC | 本次采集时刻               |
| `voltage_v`    | float  | V            | 电压                       |
| `current_a`    | float  | A            | 电流                       |
| `power_w`      | float  | W            | 有功功率                   |
| `frequency_hz` | float  | Hz           | 电网频率                   |
| `power_factor` | float  | —            | 功率因数（绝对值，0~1）    |
| `pf_type`      | string | —            | `"感性"` / `"容性"` / `""` |
| `energy_kwh`   | float  | kWh          | 累计用电量                 |
| `uptime_secs`  | int    | s            | 设备累计通电时间           |

---

### 1.5 客户端代码示例

**Python**

```python
import requests

resp = requests.get("http://192.168.1.100:8080/api/measurements/12:10:37:4C:47:47")
if resp.status_code == 200:
    data = resp.json()
    print(f"功率: {data['power_w']} W，电压: {data['voltage_v']} V")
```

**curl**

```bash
# 全部设备
curl http://localhost:8080/api/measurements | jq

# 指定设备
curl http://localhost:8080/api/measurements/12:10:37:4C:47:47 | jq
```

**JavaScript / fetch**

```js
const res = await fetch("http://localhost:8080/api/measurements");
const data = await res.json();
data.forEach((d) => console.log(d.device_mac, d.power_w, "W"));
```

**轮询建议**  
程序每 1 秒采集一次（可在 `config.toml` 调整），外部客户端轮询间隔建议 ≥ 1 秒，避免无意义请求。若需实时推送，请使用下一节的 WebSocket 接口。

---

## 二、WebSocket 实时推送 API

WebSocket 接口与 HTTP REST API 共用同一端口（默认 `8080`）。连接建立后服务端会：

1. **立即推送快照**：当前内存中已有的最新测量值
2. **持续实时推送**：每次 BLE 采集到新数据后立即下发

每条消息均为 JSON 字符串，字段与 REST `/api/measurements` 完全一致。

---

### 2.1 订阅所有设备的实时数据

```
WS  ws://<host>:8080/ws/measurements
```

**JavaScript 示例**

```javascript
const ws = new WebSocket("ws://192.168.1.100:8080/ws/measurements");

ws.onopen = () => console.log("已连接，等待数据…");

ws.onmessage = (event) => {
  const m = JSON.parse(event.data);
  console.log(`${m.device_mac}  ${m.power_w} W  ${m.voltage_v} V`);
};

ws.onerror = (e) => console.error("WS 错误", e);
ws.onclose = () => console.log("连接关闭");
```

**Python 示例**

```python
import asyncio
import json
import websockets

async def main():
    uri = "ws://192.168.1.100:8080/ws/measurements"
    async with websockets.connect(uri) as ws:
        async for raw in ws:
            m = json.loads(raw)
            print(f"{m['device_mac']}  {m['power_w']:.1f} W  {m['voltage_v']:.1f} V")

asyncio.run(main())
```

---

### 2.2 订阅指定设备的实时数据

```
WS  ws://<host>:8080/ws/measurements/{mac}
```

`{mac}` 大小写均可，如 `12:10:37:4C:47:47`。

连接建立后，只会推送该 MAC 对应设备的数据，适合多设备场景下精准订阅。

**JavaScript 示例**

```javascript
const mac = "12:10:37:4C:47:47";
const ws = new WebSocket(`ws://192.168.1.100:8080/ws/measurements/${mac}`);

ws.onmessage = (event) => {
  const m = JSON.parse(event.data);
  console.log(`功率: ${m.power_w} W  电流: ${m.current_a} A`);
};
```

**Python 示例**

```python
import asyncio, json, websockets

async def main():
    mac = "12:10:37:4C:47:47"
    uri = f"ws://192.168.1.100:8080/ws/measurements/{mac}"
    async with websockets.connect(uri) as ws:
        async for raw in ws:
            m = json.loads(raw)
            print(f"功率={m['power_w']:.1f}W  电压={m['voltage_v']:.1f}V  能耗={m['energy_kwh']:.3f}kWh")

asyncio.run(main())
```

---

### 2.3 消息字段说明

WebSocket 消息与 REST `/api/measurements` 字段完全一致：

| 字段           | 类型   | 单位 | 说明                                         |
| -------------- | ------ | ---- | -------------------------------------------- |
| `device_mac`   | string | —    | BLE MAC 地址（大写，冒号分隔）               |
| `recorded_at`  | string | —    | ISO 8601 UTC 时间戳（采集时刻）              |
| `voltage_v`    | number | V    | 电压                                         |
| `current_a`    | number | A    | 电流                                         |
| `power_w`      | number | W    | 功率                                         |
| `frequency_hz` | number | Hz   | 频率                                         |
| `power_factor` | number | —    | 功率因数（0~1）                              |
| `pf_type`      | string | —    | 功率因数类型：`"感性"` / `"容性"` / `"纯阻"` |
| `energy_kwh`   | number | kWh  | 累计能耗                                     |
| `uptime_secs`  | number | s    | 设备运行时长                                 |

---

### 2.4 行为说明

| 行为      | 说明                                                                     |
| --------- | ------------------------------------------------------------------------ |
| 断线重连  | 客户端自行实现，服务端无状态                                             |
| 无数据时  | 服务端保持连接等待，不发送任何消息                                       |
| 设备离线  | 已下发的最新快照保留；重新上线后继续推送                                 |
| 背压保护  | 服务端广播队列容量 64 条；客户端消费过慢时会跳过旧数据（记录 warn 日志） |
| Ping/Pong | 服务端自动响应客户端的 Ping 帧                                           |

---

## 三、数据库对接：管理蓝牙设备列表

powercess 支持三种设备数据后端，通过 `config.toml` 的 `[store]` 节切换。

### 2.1 方式一：静态配置（默认，无需数据库）

适合设备固定不变的场景，设备列表直接写在 `config.toml` 中，重启后生效。

```toml
[store]
type = "static"

[[devices]]
mac   = "12:10:37:4C:47:47"
name  = "德力西功率计-1"
label = "实验室插线板"

[[devices]]
mac   = "AA:BB:CC:DD:EE:FF"
name  = "德力西功率计-2"
label = "服务器机柜"
```

---

### 2.2 方式二：SQLite（轻量本地数据库）

适合设备列表需要动态增删、但不想部署独立数据库服务的场景（树莓派推荐）。

**1. 修改 `config.toml`**

```toml
[store]
type = "sqlite"
path = "/var/lib/powercess/powercess.db"   # 数据库文件路径

[reporter]
db_enabled = true   # 同时将测量值写入数据库
```

**2. 编译时启用 feature**

```bash
cargo build --release --features store-sqlite
```

> 默认已包含 `store-sqlite`，无需额外指定。

**3. 程序首次启动将自动建表**，表结构如下：

```sql
-- 设备表（程序从此表读取需要监控的设备）
CREATE TABLE IF NOT EXISTS devices (
    mac   TEXT PRIMARY KEY,      -- "12:10:37:4C:47:47"
    name  TEXT NOT NULL,         -- "德力西功率计-1"
    label TEXT                   -- 可为 NULL
);

-- 测量值表（当 reporter.db_enabled = true 时写入）
CREATE TABLE IF NOT EXISTS measurements (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    device_mac   TEXT    NOT NULL,
    recorded_at  TEXT    NOT NULL,   -- RFC 3339
    voltage      REAL    NOT NULL,   -- V
    current_a    REAL    NOT NULL,   -- A
    power        REAL    NOT NULL,   -- W
    frequency    REAL    NOT NULL,   -- Hz
    power_factor REAL    NOT NULL,
    pf_type      TEXT    NOT NULL,   -- "Inductive"/"Capacitive"/"Resistive"
    energy       REAL    NOT NULL,   -- kWh
    uptime_secs  INTEGER NOT NULL
);
```

**4. 添加/删除设备**

```bash
# 添加设备（程序重启后生效）
sqlite3 /var/lib/powercess/powercess.db \
  "INSERT INTO devices VALUES ('AA:BB:CC:DD:EE:FF', '二号功率计', '走廊插座');"

# 删除设备
sqlite3 /var/lib/powercess/powercess.db \
  "DELETE FROM devices WHERE mac = 'AA:BB:CC:DD:EE:FF';"

# 查询历史数据
sqlite3 /var/lib/powercess/powercess.db \
  "SELECT recorded_at, power, energy FROM measurements
   WHERE device_mac = '12:10:37:4C:47:47'
   ORDER BY id DESC LIMIT 10;"
```

---

### 2.3 方式三：PostgreSQL（生产级数据库）

适合已有 PostgreSQL 基础设施、或需要多实例共享设备列表的场景。

**1. 修改 `config.toml`**

```toml
[store]
type = "postgres"
url  = "postgres://powercess:password@db-host:5432/powercess_db"

[reporter]
db_enabled = true
```

**2. 编译时启用 feature**

```bash
cargo build --release --features store-postgres
```

**3. 手动执行建表 SQL**（PostgreSQL 不自动建表）

```sql
CREATE TABLE IF NOT EXISTS devices (
    mac   TEXT PRIMARY KEY,
    name  TEXT NOT NULL,
    label TEXT
);

CREATE TABLE IF NOT EXISTS measurements (
    id           BIGSERIAL   PRIMARY KEY,
    device_mac   TEXT        NOT NULL,
    recorded_at  TIMESTAMPTZ NOT NULL,
    voltage      FLOAT8      NOT NULL,
    current_a    FLOAT8      NOT NULL,
    power        FLOAT8      NOT NULL,
    frequency    FLOAT8      NOT NULL,
    power_factor FLOAT8      NOT NULL,
    pf_type      TEXT        NOT NULL,
    energy       FLOAT8      NOT NULL,
    uptime_secs  INT         NOT NULL
);

-- 建议加索引，加速按设备和时间查询
CREATE INDEX IF NOT EXISTS idx_meas_mac_time
    ON measurements (device_mac, recorded_at DESC);
```

**4. 添加设备**

```sql
INSERT INTO devices (mac, name, label)
VALUES ('AA:BB:CC:DD:EE:FF', '二号功率计', '走廊插座');
```

**5. 查询示例**

```sql
-- 某设备最近 1 小时均值
SELECT
    date_trunc('minute', recorded_at) AS minute,
    AVG(power)   AS avg_power_w,
    AVG(voltage) AS avg_voltage_v
FROM measurements
WHERE device_mac = '12:10:37:4C:47:47'
  AND recorded_at >= NOW() - INTERVAL '1 hour'
GROUP BY 1
ORDER BY 1;
```

---

## 四、典型部署架构

```
┌──────────────────────────────────────────┐
│               树莓派 4                    │
│                                          │
│  ┌─────────────┐    BLE    ┌──────────┐  │
│  │ powercess   │◄─────────►│ 功率计   │  │
│  │             │           └──────────┘  │
│  │  :8080 HTTP │                         │
│  └──────┬──────┘                         │
│         │ SQLite (本地)                   │
│         └─► /var/lib/powercess/db        │
└──────────────────────────────────────────┘
         │ HTTP GET /api/measurements
         ▼
┌─────────────────────┐    ┌──────────────────┐
│  Grafana / 自研前端  │    │  Python 脚本/    │
│  数据可视化          │    │  Home Assistant  │
└─────────────────────┘    └──────────────────┘
```

---

## 五、环境变量覆盖（无需修改配置文件）

所有 `config.toml` 字段均可通过 `POWERCESS__` 前缀的环境变量覆盖，层级用 `__` 分隔：

```bash
# 修改采集间隔为 5 秒
POWERCESS__APP__POLL_INTERVAL_SECS=5 ./powercess

# 切换到 PostgreSQL
POWERCESS__STORE__TYPE=postgres \
POWERCESS__STORE__URL="postgres://user:pass@host/db" \
./powercess

# 修改 HTTP 监听端口
POWERCESS__REPORTER__HTTP_BIND="0.0.0.0:9090" ./powercess
```
