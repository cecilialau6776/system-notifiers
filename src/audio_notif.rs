use libpulse_binding::volume::Volume;
use notify_rust::{Notification, NotificationHandle, Timeout, Urgency};

#[derive(Debug)]
pub struct AudioEvent {
    pub volume: Volume,
    pub mute: bool,
}

struct AudioNotif {
    handle: Option<NotificationHandle>,
    urgency: Urgency,
    summary: String,
    icon: Option<String>,
    timeout: Timeout,
}

impl Default for AudioNotif {
    fn default() -> Self {
        Self {
            handle: None,
            urgency: Urgency::Normal,
            summary: "Volume".to_string(),
            icon: None,
            timeout: Timeout::from(5000),
        }
    }
}

impl AudioNotif {
    fn show(&mut self, body: &str) -> Result<(), anyhow::Error> {
        if self.handle.is_none() {
            let mut notif = Notification::new()
                .summary(&self.summary.to_string())
                .urgency(self.urgency)
                .timeout(self.timeout)
                .body(body)
                .to_owned();
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

pub struct AudioNotifications {
    appname: String,
    last_state: Option<AudioEvent>,
    volume: AudioNotif,
    mute: AudioNotif,
}

impl AudioNotifications {
    pub fn new() -> Self {
        AudioNotifications {
            appname: "volume".to_owned(),
            last_state: None,
            volume: AudioNotif {
                ..Default::default()
            },
            mute: AudioNotif {
                ..Default::default()
            },
        }
    }

    pub fn update_status(&mut self, event: AudioEvent) -> Result<(), anyhow::Error> {
        if let Some(last_event) = &self.last_state {
            if event.mute != last_event.mute {
                self.show_mute_notif(event.mute)?;
            } else if event.volume != last_event.volume {
                self.show_volume_notif(event.volume)?;
            }
        } else {
            self.show_mute_notif(event.mute)?;
            self.show_volume_notif(event.volume)?;
        }
        self.last_state = Some(event);
        Ok(())
    }

    fn show_mute_notif(&mut self, muted: bool) -> Result<(), anyhow::Error> {
        self.mute.close();
        self.mute.show(if muted { "Muted" } else { "Unmuted" })
    }

    fn show_volume_notif(&mut self, volume: Volume) -> Result<(), anyhow::Error> {
        self.volume.close();
        self.volume.show(&volume.to_string())
    }
}
