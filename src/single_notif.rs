use std::fmt::Display;

use notify_rust::{Notification, NotificationHandle, Timeout, Urgency};

use crate::config::NotifConfig;

#[derive(Debug)]
pub struct SingleNotif {
    handle: Option<NotificationHandle>,
    urgency: Urgency,
    appname: String,
    summary: String,
    body: Option<String>,
    icon: Option<String>,
    timeout: Timeout,
}

impl SingleNotif {
    pub fn new_from_config(settings: &NotifConfig, appname: String) -> Self {
        SingleNotif {
            handle: None,
            urgency: settings.urgency,
            appname,
            summary: settings.summary.clone(),
            body: settings.body.clone(),
            icon: settings.icon.clone(),
            timeout: settings.timeout,
        }
    }

    pub fn show<S: Display>(&mut self, value: S) -> Result<(), anyhow::Error> {
        if self.handle.is_none() {
            let mut notif = Notification::new()
                .summary(&self.summary)
                .appname(&self.appname)
                .urgency(self.urgency)
                .timeout(self.timeout)
                .to_owned();
            if let Some(body) = &self.body {
                notif.body(&body.replace("%v", &value.to_string()));
            }
            if let Some(icon) = &self.icon {
                notif.icon(icon);
            }
            self.handle = Some(notif.show()?);
        }
        Ok(())
    }

    pub fn close(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.close();
        }
    }
}
