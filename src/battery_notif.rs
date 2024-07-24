use notify_rust::{Notification, NotificationHandle, Timeout, Urgency};

#[derive(Debug)]
pub enum BatteryEvent {
    Poll,
    Plugged,
    Unplugged,
}

struct BatteryNotif {
    handle: Option<NotificationHandle>,
    urgency: Urgency,
    summary: String,
    body: Option<String>,
    icon: Option<String>,
    timeout: Timeout,
}

impl Default for BatteryNotif {
    fn default() -> Self {
        Self {
            handle: None,
            urgency: Urgency::Normal,
            summary: "Battery".to_string(),
            body: None,
            icon: None,
            timeout: Timeout::from(5000),
        }
    }
}

impl BatteryNotif {
    fn show(&mut self, percentage: u8) -> Result<(), anyhow::Error> {
        if self.handle.is_none() {
            let mut notif = Notification::new()
                .summary(&self.summary.to_string())
                .urgency(self.urgency)
                .timeout(self.timeout)
                .to_owned();
            if let Some(body) = &self.body {
                notif.body(&body.replace("%p", &percentage.to_string()));
            }
            if let Some(icon) = &self.icon {
                notif.icon(&icon);
            }
            self.handle = Some(notif.show()?);
        }
        Ok(())
    }

    fn close(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.close();
        }
    }
}

pub struct BatteryNotifications {
    appname: String,
    critical: BatteryNotif,
    low: BatteryNotif,
    full: BatteryNotif,
    charging: BatteryNotif,
    discharging: BatteryNotif,
}

impl BatteryNotifications {
    pub fn new() -> Self {
        BatteryNotifications {
            appname: "battery".to_owned(),
            critical: BatteryNotif {
                urgency: Urgency::Critical,
                body: Some("Battery Critical".to_owned()),
                timeout: Timeout::Never,
                ..Default::default()
            },
            low: BatteryNotif {
                body: Some("Battery Low".to_owned()),
                ..Default::default()
            },
            full: BatteryNotif {
                body: Some("Battery Full".to_owned()),
                ..Default::default()
            },
            charging: BatteryNotif {
                body: Some("Plugged".to_owned()),
                ..Default::default()
            },
            discharging: BatteryNotif {
                body: Some("Unplugged".to_owned()),
                ..Default::default()
            },
        }
    }

    pub fn show_critical_notif(&mut self, percentage: u8) -> Result<(), anyhow::Error> {
        self.critical.show(percentage)
    }
    pub fn show_low_notif(&mut self, percentage: u8) -> Result<(), anyhow::Error> {
        self.low.show(percentage)
    }
    pub fn show_full_notif(&mut self, percentage: u8) -> Result<(), anyhow::Error> {
        self.full.show(percentage)
    }
    pub fn show_charging_notif(&mut self) -> Result<(), anyhow::Error> {
        self.charging.show(0)
    }
    pub fn show_discharging_notif(&mut self) -> Result<(), anyhow::Error> {
        self.discharging.show(0)
    }

    pub fn close_critical_notif(&mut self) {
        self.critical.close();
    }
    pub fn close_low_notif(&mut self) {
        self.low.close();
    }
    pub fn close_full_notif(&mut self) {
        self.full.close();
    }
    pub fn close_charging_notif(&mut self) {
        self.charging.close();
    }
    pub fn close_discharging_notif(&mut self) {
        self.discharging.close();
    }
}
