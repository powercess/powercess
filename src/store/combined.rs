//! 组合存储：将多个 DeviceStore 合并为一个，实现静态设备与数据库设备的共存。
//!
//! 使用场景：
//! - 主数据源为 SQLite/PostgreSQL 数据库
//! - 同时在 config.toml 中声明额外的静态设备
//!
//! 示例配置：
//! ```toml
//! [store]
//! type = "sqlite"
//! path = "/var/lib/powercess/powercess.db"
//!
//! [[devices]]  # 这些设备会与数据库中的设备合并
//! mac = "AA:BB:CC:DD:EE:FF"
//! name = "临时测试设备"
//! label = "测试"
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use crate::error::AppResult;
use crate::model::DeviceInfo;
use crate::store::DeviceStore;

/// 组合存储：合并多个 DeviceStore 的设备列表。
///
/// 设备去重规则：以 MAC 地址为准，后加入的 store 优先（可覆盖前面的）。
pub struct CombinedStore {
    stores: Vec<Arc<dyn DeviceStore>>,
}

impl CombinedStore {
    /// 创建空的组合存储
    pub fn new() -> Self {
        Self { stores: Vec::new() }
    }

    /// 添加一个 DeviceStore
    pub fn add(mut self, store: Arc<dyn DeviceStore>) -> Self {
        self.stores.push(store);
        self
    }

    /// 从已有的 store 列表创建
    pub fn from_stores(stores: Vec<Arc<dyn DeviceStore>>) -> Self {
        Self { stores }
    }
}

impl Default for CombinedStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DeviceStore for CombinedStore {
    /// 合并所有 store 的设备列表，按 MAC 地址去重。
    ///
    /// 去重规则：相同 MAC 的设备，后添加的 store 中的定义会覆盖前面的。
    /// 这样设计是为了让用户可以用静态设备覆盖数据库中的设备信息。
    async fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
        let mut devices_by_mac: std::collections::HashMap<String, DeviceInfo> =
            std::collections::HashMap::new();

        for store in &self.stores {
            let devices = store.list_devices().await?;
            for device in devices {
                let mac = device.mac.clone();
                if devices_by_mac.contains_key(&mac) {
                    info!(
                        "[CombinedStore] 设备 MAC {} 重复，使用后续 store 的定义",
                        mac
                    );
                }
                devices_by_mac.insert(mac, device);
            }
        }

        Ok(devices_by_mac.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockStore {
        devices: Vec<DeviceInfo>,
    }

    #[async_trait]
    impl DeviceStore for MockStore {
        async fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
            Ok(self.devices.clone())
        }
    }

    #[tokio::test]
    async fn test_combined_store_merges_devices() {
        let store1 = Arc::new(MockStore {
            devices: vec![DeviceInfo {
                mac: "AA:BB:CC:DD:EE:FF".to_string(),
                name: "设备1".to_string(),
                label: Some("标签1".to_string()),
            }],
        });

        let store2 = Arc::new(MockStore {
            devices: vec![DeviceInfo {
                mac: "11:22:33:44:55:66".to_string(),
                name: "设备2".to_string(),
                label: None,
            }],
        });

        let combined = CombinedStore::new()
            .add(store1)
            .add(store2);

        let devices = combined.list_devices().await.unwrap();
        assert_eq!(devices.len(), 2);
    }

    #[tokio::test]
    async fn test_combined_store_dedup_by_mac() {
        let store1 = Arc::new(MockStore {
            devices: vec![DeviceInfo {
                mac: "AA:BB:CC:DD:EE:FF".to_string(),
                name: "原始名称".to_string(),
                label: Some("原始标签".to_string()),
            }],
        });

        let store2 = Arc::new(MockStore {
            devices: vec![DeviceInfo {
                mac: "AA:BB:CC:DD:EE:FF".to_string(),
                name: "覆盖名称".to_string(),
                label: Some("覆盖标签".to_string()),
            }],
        });

        let combined = CombinedStore::new()
            .add(store1)
            .add(store2);

        let devices = combined.list_devices().await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "覆盖名称");
    }
}