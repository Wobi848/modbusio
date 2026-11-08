//! Sites: the devices found or added at one plant, with the name the user gave each, its profile
//! and what modbusio learnt about it. One JSON file per site in the data folder.

use serde::{Deserialize, Serialize};

use crate::access::Limits;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub devices: Vec<SiteDevice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteDevice {
    /// The device's key (`serial:COM3:9600:8E1/5`).
    pub key: String,
    #[serde(default)]
    pub name: String,
    /// The id of its profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

impl SiteDevice {
    pub fn new(key: String) -> SiteDevice {
        SiteDevice {
            key,
            name: String::new(),
            profile: None,
            limits: Limits::default(),
            vendor: None,
            product: None,
            revision: None,
        }
    }
}

impl Site {
    pub fn device(&self, key: &str) -> Option<&SiteDevice> {
        self.devices.iter().find(|device| device.key == key)
    }

    pub fn device_mut(&mut self, key: &str) -> Option<&mut SiteDevice> {
        self.devices.iter_mut().find(|device| device.key == key)
    }

    /// Adds a device unless it is there; returns whether it was new.
    pub fn add(&mut self, device: SiteDevice) -> bool {
        if self.device(&device.key).is_some() {
            return false;
        }
        self.devices.push(device);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_is_added_once() {
        let mut site = Site {
            id: "a".into(),
            name: "A".into(),
            devices: Vec::new(),
        };
        assert!(site.add(SiteDevice::new("tcp:10.0.0.5:502/1".into())));
        assert!(!site.add(SiteDevice::new("tcp:10.0.0.5:502/1".into())));
        assert_eq!(site.devices.len(), 1);
    }

    #[test]
    fn an_old_site_file_without_limits_loads() {
        let site: Site = serde_json::from_str(
            r#"{"id":"a","name":"A","devices":[{"key":"tcp:10.0.0.5:502/1"}]}"#,
        )
        .expect("loads");
        assert_eq!(site.devices[0].limits, Limits::default());
    }
}
