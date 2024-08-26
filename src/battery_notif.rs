use crate::{config::BatteryConfig, single_notif::SingleNotif};

#[derive(Debug)]
pub enum BatteryEvent {
    Poll,
    Plugged,
    Unplugged,
}

impl From<acpid_plug::Event> for BatteryEvent {
    fn from(item: acpid_plug::Event) -> Self {
        match item {
            acpid_plug::Event::Plugged => Self::Plugged,
            acpid_plug::Event::Unplugged => Self::Unplugged,
        }
    }
}

#[derive(Debug)]
pub struct BatteryNotifications {
    critical: SingleNotif,
    low: SingleNotif,
    full: SingleNotif,
    charging: SingleNotif,
    discharging: SingleNotif,
    crit_percentage: u8,
    low_percentage: u8,
    full_percentage: u8,
}

impl BatteryNotifications {
    pub fn new(config: BatteryConfig) -> Result<Self, anyhow::Error> {
        Ok(BatteryNotifications {
            critical: SingleNotif::new_from_config(&config.critical, config.appname.clone()),
            low: SingleNotif::new_from_config(&config.low, config.appname.clone()),
            full: SingleNotif::new_from_config(&config.full, config.appname.clone()),
            charging: SingleNotif::new_from_config(&config.charging, config.appname.clone()),
            discharging: SingleNotif::new_from_config(&config.discharging, config.appname.clone()),
            crit_percentage: config.crit_percentage,
            low_percentage: config.low_percentage,
            full_percentage: config.full_percentage,
        })
    }

    pub fn update_soc(
        &mut self,
        state: battery::State,
        percentage: u8,
    ) -> Result<(), anyhow::Error> {
        if state == battery::State::Discharging {
            if percentage < self.crit_percentage {
                self.show_critical_notif(percentage.into())?;
            } else if percentage < self.low_percentage {
                self.show_low_notif(percentage.into())?;
            } else {
                self.close_low_notif();
                self.close_critical_notif();
            }
        }
        if state == battery::State::Charging {
            self.close_critical_notif();
            if percentage > self.full_percentage {
                self.show_full_notif(percentage.into())?;
            } else {
                self.close_full_notif();
            }
        }
        Ok(())
    }

    fn show_critical_notif(&mut self, percentage: i64) -> Result<(), anyhow::Error> {
        self.critical.show(percentage)
    }
    fn show_low_notif(&mut self, percentage: i64) -> Result<(), anyhow::Error> {
        self.low.show(percentage)
    }
    fn show_full_notif(&mut self, percentage: i64) -> Result<(), anyhow::Error> {
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
