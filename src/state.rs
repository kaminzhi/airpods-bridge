use serde::Serialize;
use std::sync::{Arc, Mutex};

pub type SharedState = Arc<Mutex<AirPodsState>>;

#[derive(Serialize, Clone, Debug)]
pub struct BatteryInfo {
    pub level: Option<u8>,
    pub charging: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct AirPodsState {
    pub device_name: String,
    pub model_name: String,
    pub anc_mode: Option<String>,
    pub connected: bool,
    pub left: BatteryInfo,
    pub right: BatteryInfo,
    pub case: BatteryInfo,
    #[serde(skip)]
    pub session_fd: Option<i32>,
    #[serde(skip)]
    pub seq: u8,
    #[serde(skip)]
    pub last_hash: String,
}

impl Default for AirPodsState {
    fn default() -> Self {
        Self {
            device_name: "AirPods".into(),
            model_name: "Unknown".into(),
            anc_mode: None,
            connected: false,
            left: BatteryInfo { level: None, charging: false },
            right: BatteryInfo { level: None, charging: false },
            case: BatteryInfo { level: None, charging: false },
            session_fd: None,
            seq: 0x01,
            last_hash: String::new(),
        }
    }
}

impl AirPodsState {
    pub fn print_json(&mut self) {
        let current = serde_json::to_string(self).unwrap_or_default();
        if self.last_hash != current {
            println!("{}", current);
            self.last_hash = current;
        }
    }
}
