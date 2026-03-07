# 设备信息录入指南

powercess 支持三种设备数据源后端（由 `config.toml` 中 `store.type` 决定），
不同后端的录入方式和字段规范不同，本文档逐一说明。

---

## 一、字段规范（所有后端通用）

程序内部使用 `DeviceInfo` 结构体描述一台受监控设备，包含三个字段：

| 字段    | 类型             | 必填 | 说明                                                              |
| ------- | ---------------- | ---- | ----------------------------------------------------------------- |
| `mac`   | `String`         | ✅   | BLE MAC 地址；**大写字母 + 冒号分隔**，如 `12:10:37:4C:47:47`     |
| `name`  | `String`         | ✅   | 设备人类可读名称，在 HTTP API 响应和日志中展示                    |
| `label` | `Option<String>` | ❌   | 可选的位置/备注标签（SQLite/静态）；PostgreSQL 对应 `location` 列 |

### MAC 地址格式要求

- 6 组十六进制数，冒号分隔
- **必须全大写**，如 `AA:BB:CC:DD:EE:FF`
- PostgreSQL `devices` 表有 CHECK 约束：`^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$`（大小写均可，但程序读取后会调用 `.to_uppercase()` 统一处理）

---

## 二、Backend 1：Static（无数据库，推荐嵌入式场景）

`store.type = "static"` 时设备列表直接写在 `config.toml` 中，重启后生效。

### 2.1 配置方式

在 `config.toml` 末尾追加 `[[devices]]` 节，每台设备一节：

```toml
[store]
type = "static"

[[devices]]
mac   = "12:10:37:4C:47:47"
name  = "德力西功率计-A"
label = "实验室主插座"      # 可省略

[[devices]]
mac   = "AA:BB:CC:DD:EE:FF"
name  = "德力西功率计-B"
label = "服务器机柜"
```

### 2.2 注意事项

- `[[devices]]` 节必须在 `[store]` 节之后，否则 TOML 解析可能报错。
- `label` 字段省略时，HTTP API 中对应字段为 `null`。
- MAC 地址重复不会报错，但同一 MAC 会被扫描两次，应避免。

---

## 三、Backend 2：SQLite（轻量持久化）

`store.type = "sqlite"` 时程序在启动时自动建表（若不存在），并从表中读取设备列表。

### 3.1 config.toml 配置

```toml
[store]
type = "sqlite"
path = "/var/lib/powercess/powercess.db"   # 默认值：powercess.db（当前目录）
```

### 3.2 表结构（自动创建）

```sql
CREATE TABLE IF NOT EXISTS devices (
    mac   TEXT PRIMARY KEY,
    name  TEXT NOT NULL,
    label TEXT
);
```

### 3.3 插入设备

使用任意 SQLite 客户端（`sqlite3`命令行、DB Browser for SQLite 等）执行：

```sql
-- 添加单台设备
INSERT INTO devices (mac, name, label)
VALUES ('12:10:37:4C:47:47', '德力西功率计-A', '实验室主插座');

-- 添加多台设备
INSERT INTO devices (mac, name, label) VALUES
  ('AA:BB:CC:DD:EE:FF', '德力西功率计-B', '服务器机柜'),
  ('11:22:33:44:55:66', '德力西功率计-C', NULL);
```

### 3.4 更新 / 删除设备

```sql
-- 修改名称
UPDATE devices SET name = '新名称', label = '新位置' WHERE mac = '12:10:37:4C:47:47';

-- 删除（程序运行期间不会自动重载，需重启生效）
DELETE FROM devices WHERE mac = 'AA:BB:CC:DD:EE:FF';
```

> **变更生效**：SQLite 后端在启动时一次性读取设备列表，修改后需**重启程序**。

---

## 四、Backend 3：PostgreSQL / TimescaleDB（生产推荐）

`store.type = "postgres"` 时程序从已存在的 `devices` 表中读取 `is_active = true AND is_deleted = false` 的设备。**表结构不自动创建，需先执行 `database/timescaledb/init.sql`。**

### 4.1 config.toml 配置

```toml
[store]
type = "postgres"
url  = "postgres://user:password@host:5432/dbname"
```

### 4.2 前置依赖：device_types 表

`devices` 表有外键 `device_type_id -> device_types(id)`，必须**先**在 `device_types` 中建立设备类型，再创建设备实例。

```sql
-- 针对德力西功率计的设备类型示例
-- data_schema 描述 payload 字段，供前端动态渲染；可根据实际字段调整
INSERT INTO device_types (name, description, data_schema)
VALUES (
    '德力西功率计',
    '通过 BLE 采集电压、电流、功率等数据',
    '{
        "fields": [
            {"key": "voltage",      "label": "电压",     "unit": "V"},
            {"key": "current",      "label": "电流",     "unit": "A"},
            {"key": "power",        "label": "有功功率", "unit": "W"},
            {"key": "energy_kwh",   "label": "累计用电", "unit": "kWh"},
            {"key": "frequency",    "label": "频率",     "unit": "Hz"},
            {"key": "power_factor", "label": "功率因数", "unit": ""},
            {"key": "pf_type",      "label": "负载性质", "unit": ""},
            {"key": "uptime_secs",  "label": "通电时长", "unit": "s"}
        ]
    }'::jsonb
);
```

查询刚插入的类型 ID：

```sql
SELECT id, name FROM device_types;
```

### 4.3 插入设备实例

```sql
-- 将 <type_id> 替换为上一步 SELECT 得到的 device_types.id
INSERT INTO devices (name, mac_address, device_type_id, location)
VALUES
    ('德力西功率计-A', '12:10:37:4C:47:47', <type_id>, '实验室主插座'),
    ('德力西功率计-B', 'AA:BB:CC:DD:EE:FF', <type_id>, '服务器机柜');
```

`id`、`created_at`、`updated_at`、`is_active`、`is_deleted` 均有默认值，无需手动填写。

### 4.4 字段说明

| 列名             | 类型      | 必填 | 说明                                                           |
| ---------------- | --------- | ---- | -------------------------------------------------------------- |
| `name`           | `TEXT`    | ✅   | 设备名称                                                       |
| `mac_address`    | `TEXT`    | ✅   | BLE MAC 地址，格式由 CHECK 约束保证，全局唯一                  |
| `device_type_id` | `INT`     | ✅   | 外键 `device_types.id`，决定 payload/前端渲染模板              |
| `location`       | `TEXT`    | ❌   | 位置备注；对应程序内 `DeviceInfo.label`                        |
| `reporter_id`    | `UUID`    | ❌   | 绑定到指定 reporter 节点；`NULL` 表示任意节点均可读取          |
| `is_active`      | `BOOLEAN` | —    | 默认 `true`；设为 `false` 则程序不扫描该设备                   |
| `is_deleted`     | `BOOLEAN` | —    | 默认 `false`；软删除，历史测量数据保留，程序不读取已软删除设备 |

### 4.5 停用 / 软删除设备

```sql
-- 停用（不再采集，但保留历史数据和设备记录）
UPDATE devices SET is_active = false WHERE mac_address = 'AA:BB:CC:DD:EE:FF';

-- 软删除（同时停用并标记为已删除）
UPDATE devices SET is_active = false, is_deleted = true WHERE mac_address = 'AA:BB:CC:DD:EE:FF';
```

> **变更生效**：PostgreSQL 后端同样在**启动时**一次性读取设备列表，修改后需**重启程序**。

---

## 五、上报写入规范（PostgreSQL）

当 `reporter.db_enabled = true` 且后端为 PostgreSQL 时，程序将每次测量结果写入 `raw_measurements` 表，**写入的 `device_id` 通过 `mac_address` 在运行时解析**。

若某台设备的 MAC 地址未在 `devices` 表中注册（或已软删除），该次测量值将被**静默丢弃**，并在日志中输出 `WARN` 级别提示：

```
[DB] 设备 AA:BB:CC:DD:EE:FF 未在 devices 表中注册，测量值已跳过
```

确保设备在 `devices` 表中注册后重启程序即可恢复写入。

---

## 六、快速验证

### 检查 Static / SQLite

启动程序后，日志中应出现：

```
共 N 台设备需要监控
  • 德力西功率计-A (12:10:37:4C:47:47) label=Some("实验室主插座")
```

### 检查 PostgreSQL

```bash
# 确认程序能读取到设备
curl http://localhost:8080/api/devices

# 确认测量数据写入（db_enabled = true 时）
psql -c "SELECT device_id, collected_at, payload FROM raw_measurements ORDER BY collected_at DESC LIMIT 5;"
```
