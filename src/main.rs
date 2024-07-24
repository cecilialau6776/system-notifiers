use brightness::Brightness;
use futures::TryStreamExt;
use notify_rust::{Notification, Timeout};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use notify::{
    event::{AccessKind, AccessMode},
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

mod battery_notif;

use battery_notif::{BatteryEvent, BatteryNotifications};
use tokio::{sync::mpsc, time::interval};
use tokio_stream::{
    wrappers::{IntervalStream, ReceiverStream},
    StreamExt,
};

#[derive(Debug)]
enum SysEvent {
    Brightness,
    Battery(BatteryEvent),
}

impl From<acpid_plug::Event> for BatteryEvent {
    fn from(item: acpid_plug::Event) -> Self {
        match item {
            acpid_plug::Event::Plugged => Self::Plugged,
            acpid_plug::Event::Unplugged => Self::Unplugged,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Get AC adapter event stream
    let ac_plug_events = acpid_plug::connect().await?.map(|event| match event {
        Ok(event) => Ok(SysEvent::Battery(event.into())),
        Err(e) => Err(e),
    });

    // Get battery poller stream
    let battery_poller = IntervalStream::new(interval(Duration::from_secs(8)))
        .map(|_| Ok(SysEvent::Battery(BatteryEvent::Poll)));

    // Get file watcher event stream
    let (tx, rx) = mpsc::channel::<Event>(1);
    let watcher_rx = ReceiverStream::new(rx).map(|_| Ok(SysEvent::Brightness));

    let mut watcher = RecommendedWatcher::new(
        move |res: std::result::Result<Event, notify::Error>| {
            futures::executor::block_on(async {
                if let Ok(r) = res {
                    if r.kind == EventKind::Access(AccessKind::Close(AccessMode::Write)) {
                        tx.send(r).await.unwrap();
                    }
                }
            })
        },
        Config::default(),
    )?;

    // Merge streams. Not using StreamMap because fairness isn't required
    let mut merged = ac_plug_events.merge(watcher_rx).merge(battery_poller);

    let path = "/sys/class/backlight/intel_backlight";
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    let bat_notifs = Arc::new(Mutex::new(BatteryNotifications::new()));

    while let Some(ev) = merged.next().await {
        let bat_notifs = bat_notifs.clone();
        tokio::spawn(async move {
            process_event(ev.expect("event error"), bat_notifs).await;
        });
    }

    Ok(())
}

fn brightness_notification<T>(body: &str, timeout: T) -> Result<(), anyhow::Error>
where
    T: Into<Timeout>,
{
    Notification::new()
        .summary("Brightness")
        .body(body)
        .appname("brightness")
        .icon("/home/pixel/dotfiles/icons/brightness.png")
        .timeout(timeout)
        .show()?;
    Ok(())
}

async fn process_event(
    event: SysEvent,
    bat_notifs: Arc<Mutex<BatteryNotifications>>,
) -> Result<(), anyhow::Error> {
    match event {
        SysEvent::Brightness => notify_brightness().await?,
        SysEvent::Battery(e) => process_battery_event(e, bat_notifs).await?,
    }
    Ok(())
}

fn get_battery_soc_and_state() -> Result<(battery::State, u8), anyhow::Error> {
    let manager = battery::Manager::new()?;
    let battery = match manager.batteries()?.next() {
        Some(Ok(battery)) => battery,
        Some(Err(e)) => {
            eprintln!("Unable to access battery information");
            return Err(e.into());
        }
        None => {
            eprintln!("Unable to find any batteries");
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
    };
    Ok((
        battery.state(),
        (battery.state_of_charge().value * 100.0) as u8,
    ))
}

fn notify_battery(bat_notifs: Arc<Mutex<BatteryNotifications>>) -> Result<(), anyhow::Error> {
    let (state, percentage) = get_battery_soc_and_state()?;
    let crit_pct = 5;
    let low_pct = 20;
    let full_pct = 85;
    if state == battery::State::Discharging {
        if percentage < crit_pct {
            bat_notifs.lock().unwrap().show_critical_notif(percentage)?;
        } else if percentage < low_pct {
            bat_notifs.lock().unwrap().show_low_notif(percentage)?;
        } else {
            let mut bat_notifs = bat_notifs.lock().unwrap();
            bat_notifs.close_low_notif();
            bat_notifs.close_critical_notif();
        }
    }
    if state == battery::State::Charging {
        if percentage > full_pct {
            bat_notifs.lock().unwrap().show_full_notif(percentage)?;
        } else {
            bat_notifs.lock().unwrap().close_full_notif();
        }
    }
    Ok(())
}

async fn process_battery_event(
    event: BatteryEvent,
    bat_notifs: Arc<Mutex<BatteryNotifications>>,
) -> Result<(), anyhow::Error> {
    match event {
        BatteryEvent::Poll => notify_battery(bat_notifs),
        BatteryEvent::Plugged => {
            let mut bat_notifs = bat_notifs.lock().unwrap();
            bat_notifs.close_critical_notif();
            bat_notifs.close_low_notif();
            bat_notifs.close_discharging_notif();
            bat_notifs.show_charging_notif()
        }
        BatteryEvent::Unplugged => {
            let mut bat_notifs = bat_notifs.lock().unwrap();
            bat_notifs.close_full_notif();
            bat_notifs.close_charging_notif();
            bat_notifs.show_discharging_notif()
        }
    }
}

async fn notify_brightness() -> Result<(), anyhow::Error> {
    brightness::brightness_devices()
        .try_for_each(|dev| async move {
            let name = dev.device_name().await?;
            let value = dev.get().await?;
            Notification::new()
                .summary("Brightness")
                .body(&value.to_string().to_owned())
                .appname("brightness")
                .icon("/home/pixel/dotfiles/icons/brightness.png")
                .timeout(5000)
                .show()
                .expect("notif excpetion");
            // println!("Brightness of device {} is {}%", name, value);
            Ok(())
        })
        .await?;
    Ok(())
}
