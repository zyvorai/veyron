use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Device type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Sensor,
    Actuator,
    Gateway,
    Camera,
    Controller,
    Custom(String),
}

/// Device status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Online,
    Offline,
    Inactive,
    Error,
}

/// IoT device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDevice {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub status: DeviceStatus,
    pub firmware_version: String,
    pub edge_node_id: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub attributes: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl IoTDevice {
    pub fn new(id: impl Into<String>, name: impl Into<String>, device_type: DeviceType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            device_type,
            status: DeviceStatus::Offline,
            firmware_version: "1.0.0".to_string(),
            edge_node_id: None,
            last_seen: Utc::now(),
            attributes: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_firmware(mut self, version: impl Into<String>) -> Self {
        self.firmware_version = version.into();
        self
    }

    pub fn assign_to_node(mut self, node_id: impl Into<String>) -> Self {
        self.edge_node_id = Some(node_id.into());
        self
    }

    pub fn set_status(&mut self, status: DeviceStatus) {
        self.status = status;
        self.last_seen = Utc::now();
    }

    pub fn add_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    pub fn is_online(&self) -> bool {
        self.status == DeviceStatus::Online
    }

    pub fn is_assigned(&self) -> bool {
        self.edge_node_id.is_some()
    }
}

/// Device manager
pub struct DeviceManager {
    devices: HashMap<String, IoTDevice>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, device: IoTDevice) -> String {
        let id = device.id.clone();
        self.devices.insert(id.clone(), device);
        id
    }

    pub fn get_device(&self, id: &str) -> Option<&IoTDevice> {
        self.devices.get(id)
    }

    pub fn get_device_mut(&mut self, id: &str) -> Option<&mut IoTDevice> {
        self.devices.get_mut(id)
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn devices_by_type(&self, device_type: &DeviceType) -> Vec<&IoTDevice> {
        self.devices
            .values()
            .filter(|d| &d.device_type == device_type)
            .collect()
    }

    pub fn devices_by_node(&self, node_id: &str) -> Vec<&IoTDevice> {
        self.devices
            .values()
            .filter(|d| d.edge_node_id.as_deref() == Some(node_id))
            .collect()
    }

    pub fn online_devices(&self) -> Vec<&IoTDevice> {
        self.devices.values().filter(|d| d.is_online()).collect()
    }

    pub fn unassigned_devices(&self) -> Vec<&IoTDevice> {
        self.devices.values().filter(|d| !d.is_assigned()).collect()
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iot_device() {
        let device = IoTDevice::new("dev-001", "Temperature Sensor", DeviceType::Sensor);

        assert_eq!(device.id, "dev-001");
        assert_eq!(device.name, "Temperature Sensor");
        assert_eq!(device.device_type, DeviceType::Sensor);
        assert_eq!(device.status, DeviceStatus::Offline);
    }

    #[test]
    fn test_device_with_firmware() {
        let device = IoTDevice::new("dev-001", "Sensor", DeviceType::Sensor).with_firmware("2.1.0");

        assert_eq!(device.firmware_version, "2.1.0");
    }

    #[test]
    fn test_device_assign_to_node() {
        let device =
            IoTDevice::new("dev-001", "Sensor", DeviceType::Sensor).assign_to_node("edge-node-1");

        assert_eq!(device.edge_node_id, Some("edge-node-1".to_string()));
        assert!(device.is_assigned());
    }

    #[test]
    fn test_device_set_status() {
        let mut device = IoTDevice::new("dev-001", "Sensor", DeviceType::Sensor);

        device.set_status(DeviceStatus::Online);
        assert_eq!(device.status, DeviceStatus::Online);
        assert!(device.is_online());
    }

    #[test]
    fn test_device_add_attribute() {
        let mut device = IoTDevice::new("dev-001", "Sensor", DeviceType::Sensor);

        device.add_attribute("location", "warehouse-1");
        device.add_attribute("unit", "celsius");

        assert_eq!(device.attributes.len(), 2);
        assert_eq!(
            device.attributes.get("location"),
            Some(&"warehouse-1".to_string())
        );
    }

    #[test]
    fn test_device_manager() {
        let mut manager = DeviceManager::new();

        let device = IoTDevice::new("dev-001", "Sensor", DeviceType::Sensor);
        let id = manager.add_device(device);

        assert_eq!(manager.device_count(), 1);
        assert!(manager.get_device(&id).is_some());
    }

    #[test]
    fn test_manager_devices_by_type() {
        let mut manager = DeviceManager::new();

        manager.add_device(IoTDevice::new("dev-001", "Temp", DeviceType::Sensor));
        manager.add_device(IoTDevice::new("dev-002", "Cam", DeviceType::Camera));
        manager.add_device(IoTDevice::new("dev-003", "Humidity", DeviceType::Sensor));

        let sensors = manager.devices_by_type(&DeviceType::Sensor);
        assert_eq!(sensors.len(), 2);
    }

    #[test]
    fn test_manager_devices_by_node() {
        let mut manager = DeviceManager::new();

        manager.add_device(
            IoTDevice::new("dev-001", "S1", DeviceType::Sensor).assign_to_node("node-1"),
        );
        manager.add_device(
            IoTDevice::new("dev-002", "S2", DeviceType::Sensor).assign_to_node("node-2"),
        );
        manager.add_device(
            IoTDevice::new("dev-003", "S3", DeviceType::Sensor).assign_to_node("node-1"),
        );

        let node1_devices = manager.devices_by_node("node-1");
        assert_eq!(node1_devices.len(), 2);
    }

    #[test]
    fn test_manager_online_devices() {
        let mut manager = DeviceManager::new();

        let mut device1 = IoTDevice::new("dev-001", "S1", DeviceType::Sensor);
        device1.set_status(DeviceStatus::Online);

        let device2 = IoTDevice::new("dev-002", "S2", DeviceType::Sensor);

        manager.add_device(device1);
        manager.add_device(device2);

        let online = manager.online_devices();
        assert_eq!(online.len(), 1);
    }

    #[test]
    fn test_manager_unassigned_devices() {
        let mut manager = DeviceManager::new();

        manager.add_device(IoTDevice::new("dev-001", "S1", DeviceType::Sensor));
        manager.add_device(
            IoTDevice::new("dev-002", "S2", DeviceType::Sensor).assign_to_node("node-1"),
        );

        let unassigned = manager.unassigned_devices();
        assert_eq!(unassigned.len(), 1);
    }

    #[test]
    fn test_device_type_equality() {
        assert_eq!(DeviceType::Sensor, DeviceType::Sensor);
        assert_ne!(DeviceType::Sensor, DeviceType::Camera);
    }

    #[test]
    fn test_device_status_equality() {
        assert_eq!(DeviceStatus::Online, DeviceStatus::Online);
        assert_ne!(DeviceStatus::Online, DeviceStatus::Offline);
    }
}
