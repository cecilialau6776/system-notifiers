use brightness::Brightness;
use futures::TryStreamExt;
use libpulse_binding::{
    callbacks::ListResult,
    context::{self, subscribe::InterestMaskSet},
    mainloop::threaded,
    operation,
};
use single_notif::SingleNotif;
use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use notify::{
    event::{AccessKind, AccessMode},
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use tokio::{sync::mpsc, time::interval};
use tokio_stream::{
    wrappers::{IntervalStream, ReceiverStream, UnboundedReceiverStream},
    StreamExt,
};

mod audio_notif;
mod battery_notif;
mod config;
mod single_notif;

use audio_notif::{AudioEvent, AudioNotifications};
use battery_notif::{BatteryEvent, BatteryNotifications};

#[derive(Debug)]
enum SysEvent {
    Brightness,
    Battery(BatteryEvent),
    Audio(AudioEvent),
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let settings: config::Config = confy::load("system-notifiers", None)?;
    println!(
        "Using configuration at {:?}",
        confy::get_configuration_file_path("system-notifiers", None)
            .expect("failed to get configuration")
    );
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

    // Pulseaudio stuff
    let mainloop = Rc::new(RefCell::new(
        threaded::Mainloop::new().expect("Failed to create pulseaudio mainloop"),
    ));
    let context = Rc::new(RefCell::new(
        context::Context::new(mainloop.borrow().deref(), "system-notifiers")
            .expect("failed to create pulseaudio context"),
    ));

    {
        let ml_ref = Rc::clone(&mainloop);
        let context_ref = Rc::clone(&context);
        context
            .borrow_mut()
            .set_state_callback(Some(Box::new(move || {
                let state = unsafe { (*context_ref.as_ptr()).get_state() };
                match state {
                    context::State::Ready | context::State::Failed | context::State::Terminated => unsafe {
                        (*ml_ref.as_ptr()).signal(false);
                    },
                    _ => {}
                }
            })));
    }

    context
        .borrow_mut()
        .connect(None, context::FlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

    mainloop.borrow_mut().lock();
    mainloop.borrow_mut().start()?;

    // Wait for stream to be ready
    loop {
        match context.borrow().get_state() {
            context::State::Ready => {
                break;
            }
            context::State::Failed | context::State::Terminated => {
                eprintln!("Stream state failed/terminated, quitting...");
                mainloop.borrow_mut().unlock();
                mainloop.borrow_mut().stop();
                return Ok(());
            }
            _ => {
                mainloop.borrow_mut().wait();
            }
        }
    }
    context.borrow_mut().set_state_callback(None);

    let introspector = context.borrow_mut().introspect();

    let (sink_send, mut sink_recv) = tokio::sync::mpsc::unbounded_channel();
    let send = sink_send.clone();
    let sink = "@DEFAULT_SINK@".to_owned();

    let ml_ref = Rc::clone(&mainloop);
    let o = introspector.get_sink_info_by_name(sink.as_str(), move |r| {
        if let ListResult::Item(s) = r {
            let volume = s.volume.get()[0];
            let mute = s.mute;
            send.send(Ok(SysEvent::Audio(AudioEvent { volume, mute })))
                .unwrap();
            unsafe {
                (*ml_ref.as_ptr()).signal(false);
            }
        }
    });

    while o.get_state() != operation::State::Done {
        mainloop.borrow_mut().wait();
    }

    context
        .borrow_mut()
        .subscribe(InterestMaskSet::SINK, |_| {});

    let cb: Option<Box<dyn FnMut(_, _, _)>> = Some(Box::new(move |_, _, _| {
        let send = sink_send.clone();
        introspector.get_sink_info_by_name(sink.as_str(), move |r| {
            if let ListResult::Item(s) = r {
                let volume = s.volume.get()[0];
                let mute = s.mute;
                send.send(Ok(SysEvent::Audio(AudioEvent { volume, mute })))
                    .expect("failed to send AudioEvent");
            }
        });
    }));

    context.borrow_mut().set_subscribe_callback(cb);

    mainloop.borrow_mut().unlock();

    let initial = sink_recv
        .recv()
        .await
        .ok_or("failed to get first Pulseaudio event")
        .unwrap();
    let pa_stream = tokio_stream::once(initial).chain(UnboundedReceiverStream::new(sink_recv));
    // Merge streams. Not using StreamMap because fairness isn't required
    let mut merged = ac_plug_events
        .merge(watcher_rx)
        .merge(battery_poller)
        .merge(pa_stream);

    let path = "/sys/class/backlight/intel_backlight";
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    let bat_notifs = Arc::new(Mutex::new(BatteryNotifications::new(settings.battery)?));
    let audio_notifs = Arc::new(Mutex::new(AudioNotifications::new(settings.audio)));
    let brightness_notif = Arc::new(Mutex::new(SingleNotif::new_from_config(
        &settings.brightness.notification,
        settings.brightness.appname.clone(),
    )));

    while let Some(ev) = merged.next().await {
        let bat_notifs = bat_notifs.clone();
        let audio_notifs = audio_notifs.clone();
        let brightness_notif = brightness_notif.clone();
        tokio::spawn(async move {
            match process_event(
                ev.expect("event error"),
                bat_notifs,
                audio_notifs,
                brightness_notif,
            )
            .await
            {
                Ok(_) => {}
                Err(_) => eprintln!(),
            };
        });
    }

    Ok(())
}

async fn process_event(
    event: SysEvent,
    bat_notifs: Arc<Mutex<BatteryNotifications>>,
    vol_notifs: Arc<Mutex<AudioNotifications>>,
    brightness_notif: Arc<Mutex<SingleNotif>>,
) -> Result<(), anyhow::Error> {
    match event {
        SysEvent::Brightness => notify_brightness(&brightness_notif).await?,
        SysEvent::Battery(e) => process_battery_event(e, bat_notifs).await?,
        SysEvent::Audio(e) => process_audio_event(e, vol_notifs).await?,
    }
    Ok(())
}

async fn process_audio_event(
    event: AudioEvent,
    audio_notifs: Arc<Mutex<AudioNotifications>>,
) -> Result<(), anyhow::Error> {
    let mut audio_notifs = audio_notifs.lock().unwrap();
    audio_notifs.update_status(event)?;
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
    bat_notifs.lock().unwrap().update_soc(state, percentage)?;
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

async fn notify_brightness(
    brightness_notif: &Arc<Mutex<SingleNotif>>,
) -> Result<(), anyhow::Error> {
    brightness::brightness_devices()
        .try_for_each(|dev| async move {
            let value = dev.get().await?;
            let mut notif = brightness_notif.lock().unwrap();
            notif.close();
            notif
                .show(value)
                .expect("Failed to send brightness notification");
            Ok(())
        })
        .await?;
    Ok(())
}
