use notify_rust::{Timeout, Urgency};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Urgency")]
enum UrgencyDef {
    Low = 0,
    Normal = 1,
    Critical = 2,
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Timeout")]
pub enum TimeoutDef {
    Default,
    Never,
    Milliseconds(u32),
}

#[derive(Serialize, Deserialize)]
pub struct NotifConfig {
    #[serde(with = "UrgencyDef")]
    pub urgency: Urgency,
    pub summary: String,
    pub body: Option<String>,
    pub icon: Option<String>,
    #[serde(with = "TimeoutDef")]
    pub timeout: Timeout,
}

impl Default for NotifConfig {
    fn default() -> Self {
        NotifConfig {
            urgency: Urgency::Normal,
            summary: "%v".to_string(),
            body: None,
            icon: None,
            timeout: Timeout::from(5000),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BatteryConfig {
    pub appname: String,
    pub critical: NotifConfig,
    pub low: NotifConfig,
    pub full: NotifConfig,
    pub charging: NotifConfig,
    pub discharging: NotifConfig,
    pub crit_percentage: u8,
    pub low_percentage: u8,
    pub full_percentage: u8,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        BatteryConfig {
            appname: "battery".to_string(),
            critical: NotifConfig {
                urgency: Urgency::Critical,
                summary: "Battery".to_string(),
                body: Some("Battery Critical".to_string()),
                timeout: Timeout::Never,
                ..Default::default()
            },
            low: NotifConfig {
                summary: "Battery".to_string(),
                body: Some("Battery Low".to_string()),
                ..Default::default()
            },
            full: NotifConfig {
                summary: "Battery".to_string(),
                body: Some("Battery Full".to_string()),
                ..Default::default()
            },
            charging: NotifConfig {
                summary: "Battery".to_string(),
                body: Some("Plugged".to_string()),
                ..Default::default()
            },
            discharging: NotifConfig {
                summary: "Battery".to_string(),
                body: Some("Unplugged".to_string()),
                ..Default::default()
            },
            crit_percentage: 5,
            low_percentage: 20,
            full_percentage: 100,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AudioConfig {
    pub appname: String,
    pub volume: NotifConfig,
    pub mute: NotifConfig,
}

impl Default for AudioConfig {
    fn default() -> Self {
        AudioConfig {
            appname: "volume".to_string(),
            volume: NotifConfig {
                summary: "Volume".to_string(),
                body: Some("%v".to_string()),
                ..Default::default()
            },
            mute: NotifConfig {
                summary: "Volume".to_string(),
                body: Some("%v".to_string()),
                ..Default::default()
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BrightnessConfig {
    pub appname: String,
    pub notification: NotifConfig,
}

impl Default for BrightnessConfig {
    fn default() -> Self {
        BrightnessConfig {
            appname: "brightness".to_string(),
            notification: NotifConfig {
                summary: "Brightness".to_string(),
                ..Default::default()
            },
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub battery: BatteryConfig,
    pub audio: AudioConfig,
    pub brightness: BrightnessConfig,
}

// impl Default for Config {
//     fn default() -> Self {
//         Config {
//             battery: BatteryConfig::default(),
//             audio: AudioConfig::default(),
//             brightness: BrightnessConfig::default(),
//         }
//     }
// }
