use libpulse_binding::volume::Volume;

use crate::{config::AudioConfig, single_notif::SingleNotif};

#[derive(Debug)]
pub struct AudioEvent {
    pub volume: Volume,
    pub mute: bool,
}

pub struct AudioNotifications {
    last_state: Option<AudioEvent>,
    volume: SingleNotif,
    mute: SingleNotif,
}

impl AudioNotifications {
    pub fn new(config: AudioConfig) -> Self {
        AudioNotifications {
            last_state: None,
            volume: SingleNotif::new_from_config(&config.volume, config.appname.clone()),
            mute: SingleNotif::new_from_config(&config.mute, config.appname.clone()),
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
        self.volume.show(volume)
    }
}
