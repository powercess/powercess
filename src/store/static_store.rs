//! 静态设备列表：设备信息直接来自 `config.toml` 的 `[[devices]]` 节。

use async_trait::async_trait;

use crate::error::AppResult;
use crate::model::DeviceInfo;
use crate::store::DeviceStore;

pub struct StaticStore {
    devices: Vec<DeviceInfo>,
}

impl StaticStore {
    pub fn new(devices: Vec<DeviceInfo>) -> Self {
        Self { devices }
    }
}

#[async_trait]
impl DeviceStore for StaticStore {
    async fn list_devices(&self) -> AppResult<Vec<DeviceInfo>> {
        Ok(self.devices.clone())
    }
}
