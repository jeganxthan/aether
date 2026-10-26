use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::refresh as wake;
use aether_services::bluetooth::{devices, power};
use tokio::sync::Notify;

use super::runtime::runtime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub path: String,
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub rssi: Option<i16>,
}

static POWERED: OnceLock<RwLock<bool>> = OnceLock::new();
static POWER_STARTED: OnceLock<()> = OnceLock::new();
static DEVICES: OnceLock<RwLock<Vec<BluetoothDevice>>> = OnceLock::new();
static DEVICES_STARTED: OnceLock<()> = OnceLock::new();
static REFRESH: OnceLock<Notify> = OnceLock::new();

pub fn powered() -> bool {
    let state = POWERED.get_or_init(|| RwLock::new(false));
    POWER_STARTED.get_or_init(|| {
        runtime().spawn(watch_power());
    });
    state.read().map(|v| *v).unwrap_or(false)
}

pub fn toggle_power() {
    let _ = powered();
    runtime().spawn(async move {
        let new_state = match power::toggle_power().await {
            Ok(new_state) => new_state,
            Err(error) => {
                eprintln!("aether-panel: Bluetooth power toggle failed: {error}");
                return;
            }
        };
        if let Some(state) = POWERED.get()
            && let Ok(mut state) = state.write()
        {
            if *state != new_state {
                *state = new_state;
                drop(state);
                wake();
            }
        }
        refresh_devices();
    });
}

pub fn device_action(path: String, paired: bool, connected: bool) {
    runtime().spawn(async move {
        let result = if connected {
            devices::disconnect_device(&path).await
        } else if paired {
            devices::connect_device(&path).await
        } else {
            devices::pair_device(&path).await
        };

        match result {
            Ok(()) => refresh_devices(),
            Err(error) => {
                eprintln!("aether-panel: Bluetooth device action failed for {path}: {error}");
                refresh_devices();
            }
        }
    });
}

async fn watch_power() {
    let mut ticker = tokio::time::interval(Duration::from_secs(3));
    loop {
        ticker.tick().await;
        let Ok(new_state) = power::is_powered().await else {
            continue;
        };
        let Some(state) = POWERED.get() else {
            continue;
        };
        let Ok(mut state) = state.write() else {
            continue;
        };
        if *state != new_state {
            *state = new_state;
            drop(state);
            wake();
        }
    }
}

pub fn devices() -> Vec<BluetoothDevice> {
    let state = DEVICES.get_or_init(|| RwLock::new(Vec::new()));
    DEVICES_STARTED.get_or_init(|| {
        runtime().spawn(watch_devices());
        REFRESH.get_or_init(Notify::new).notify_one();
    });
    state.read().map(|v| v.clone()).unwrap_or_default()
}

pub fn refresh_devices() {
    REFRESH.get_or_init(Notify::new).notify_one();
}

async fn watch_devices() {
    use aether_services::bluetooth::devices::DiscoverySession;

    let mut ticker = tokio::time::interval(Duration::from_secs(3));
    let mut session: Option<DiscoverySession> = None;
    let refresh = REFRESH.get_or_init(Notify::new);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = refresh.notified() => {}
        }

        let is_powered = power::is_powered().await.unwrap_or(false);

        if !is_powered {
            if let Some(active) = session.take() {
                let _ = active.stop().await;
            }

            update_device_cache(Vec::new());
            continue;
        }

        if session.is_none() {
            session = DiscoverySession::start().await.ok();

            if session.is_none() {
                continue;
            }
        }

        let result = match session.as_ref() {
            Some(active) => active.list_devices().await,
            None => continue,
        };

        match result {
            Ok(list) => {
                let mapped = list
                    .into_iter()
                    .map(|device| BluetoothDevice {
                        path: device.path,
                        address: device.address,
                        name: device.name,
                        paired: device.paired,
                        connected: device.connected,
                        trusted: device.trusted,
                        rssi: device.rssi,
                    })
                    .collect();

                update_device_cache(mapped);
            }
            Err(_) => {
                // Drop the failed D-Bus session and retry next cycle.
                session = None;
            }
        }
    }
}

fn update_device_cache(mapped: Vec<BluetoothDevice>) {
    let Some(state) = DEVICES.get() else {
        return;
    };

    let Ok(mut state) = state.write() else {
        return;
    };

    if *state != mapped {
        *state = mapped;
        drop(state);
        wake();
    }
}
