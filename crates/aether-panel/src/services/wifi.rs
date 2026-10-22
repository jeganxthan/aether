use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::refresh as wake;
use aether_services::wifi::saved_wifi_ssids;
use aether_services::{ConnectedWifiService, WifiConnection, WifiScanner, WifiToggle};
use tokio::sync::Notify;

use super::runtime::runtime;

/*
 * What the panel knows about the connection: the signal, for the icon, plus the
 * name and link speed, for the small box the icon opens on hover. They are read
 * alongside the signal so all of it comes from one reading of the bus.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiReading {
    pub strength: u8,
    pub secured: bool,
    pub ssid: Option<String>,
    // the link speed the access point negotiated, in Mbps, when it is known
    pub rate_mbps: Option<u32>,
}

/*
 * One network in range, as the menu lists it: the name, the signal for its icon,
 * and whether it is secured and to what the icon should answer. The connected one
 * is marked so the list can pick it out.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub strength: u8,
    pub secured: bool,
    pub connected: bool,
    pub saved: bool,
}

// the latest reading, kept where the sync view can read it without waiting on the network
static LATEST: OnceLock<RwLock<Option<WifiReading>>> = OnceLock::new();

// starts the refresh task exactly once, after LATEST exists
static STARTED: OnceLock<()> = OnceLock::new();

// whether the wifi radio is on, kept where the sync view can read it
static RADIO: OnceLock<RwLock<bool>> = OnceLock::new();

// starts the radio watcher exactly once, after RADIO exists
static RADIO_STARTED: OnceLock<()> = OnceLock::new();

// the networks in range, strongest first, kept where the sync view can read them
static NETWORKS: OnceLock<RwLock<Vec<WifiNetwork>>> = OnceLock::new();

// starts the scanner exactly once, after NETWORKS exists
static SCAN_STARTED: OnceLock<()> = OnceLock::new();
static SCAN_REQUEST: OnceLock<Notify> = OnceLock::new();
static ETHERNET: OnceLock<RwLock<bool>> = OnceLock::new();
static ETHERNET_STARTED: OnceLock<()> = OnceLock::new();

pub fn ethernet_connected() -> bool {
    let state = ETHERNET.get_or_init(|| RwLock::new(false));
    ETHERNET_STARTED.get_or_init(|| {
        runtime().spawn(watch_ethernet());
    });
    state.read().map(|connected| *connected).unwrap_or(false)
}

async fn watch_ethernet() {
    let mut ticker = tokio::time::interval(Duration::from_secs(3));
    loop {
        ticker.tick().await;
        let Ok(connected) = aether_services::wifi::ethernet_connected().await else {
            continue;
        };
        if let Some(state) = ETHERNET.get()
            && let Ok(mut state) = state.write()
            && *state != connected
        {
            *state = connected;
            drop(state);
            wake();
        }
    }
}

/*
 * The network the menu is asking a password for, if any. Clicking a secured
 * network sets it; connecting, or canceling, clears it. Kept where the sync view
 * can read it, and read by the view to decide whether to show the input.
 */
static PENDING: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/*
 * Reading the network talks to NetworkManager and is async, so it happens on a
 * task that refreshes the cached value every few seconds. The view only ever
 * reads the last value, so drawing never waits on the bus.
 */
pub fn wifi() -> Option<WifiReading> {
    let latest = LATEST.get_or_init(|| RwLock::new(None));

    STARTED.get_or_init(|| {
        runtime().spawn(run());
    });

    latest.read().ok().and_then(|reading| reading.clone())
}

// just the network name, for the small box the icon opens on hover
pub fn ssid() -> Option<String> {
    wifi().and_then(|reading| reading.ssid)
}

// the link speed in Mbps, for the small box the icon opens on hover
pub fn speed() -> Option<u32> {
    wifi().and_then(|reading| reading.rate_mbps)
}

/*
 * Whether the wifi radio is on. A failure to read it keeps the last known state,
 * so the switch never flickers off on a single missed read.
 */
pub fn radio() -> bool {
    let radio = RADIO.get_or_init(|| RwLock::new(true));

    RADIO_STARTED.get_or_init(|| {
        runtime().spawn(watch_radio());
    });

    radio.read().map(|on| *on).unwrap_or(true)
}

/*
 * Flips the radio and lets the watcher pick the new state up. The flip runs on
 * its own task, so the click that asks for it never waits on the bus.
 */
pub fn toggle_radio() {
    eprintln!("aether-panel: Wi-Fi radio toggle requested by popup");
    // the watcher owns the cached state, so make sure it is running first
    let _ = radio();

    runtime().spawn(async move {
        let Ok(toggle) = WifiToggle::new().await else {
            eprintln!("aether-panel: failed to create NetworkManager Wi-Fi controller");
            return;
        };

        // a failure leaves the switch where it was, and the next read corrects it
        if let Err(error) = toggle.toggle().await {
            eprintln!("aether-panel: Wi-Fi radio toggle failed: {error}");
            return;
        }
        eprintln!("aether-panel: Wi-Fi radio toggle completed");

        // the watcher polls slowly, so read the new state back right away
        if let Ok(enabled) = toggle.is_enabled().await
            && let Some(radio) = RADIO.get()
        {
            let Ok(mut radio) = radio.write() else {
                return;
            };

            if *radio != enabled {
                *radio = enabled;

                drop(radio);

                wake();
            }
        }
    });
}

/*
 * The network the menu is asking a password for, if any. Clicking a secured
 * network sets it; connecting, or canceling, clears it. Kept where the sync view
 * can read it, so the view decides whether to show the input without waiting.
 */
pub fn pending() -> Option<String> {
    let pending = PENDING.get_or_init(|| RwLock::new(None));

    pending.read().ok().and_then(|pending| pending.clone())
}

// whether the menu is asking a password for a network
pub fn awaiting_password() -> bool {
    pending().is_some()
}

/*
 * Asks for the password of a network, waking the shell so the input appears. A
 * click on the same network again does nothing.
 */
pub fn ask_password(ssid: &str) {
    let pending = PENDING.get_or_init(|| RwLock::new(None));

    let Ok(mut pending) = pending.write() else {
        return;
    };

    if pending.as_deref() == Some(ssid) {
        return;
    }

    *pending = Some(String::from(ssid));

    drop(pending);

    wake();
}

// packs the password input away without connecting
pub fn cancel_password() {
    let Some(pending) = PENDING.get() else {
        return;
    };

    let Ok(mut pending) = pending.write() else {
        return;
    };

    if pending.is_none() {
        return;
    }

    *pending = None;

    drop(pending);

    wake();
}

/*
 * Connects to a network, on its own task so the click never waits on the bus. A
 * named network is joined with the password; a network with no password is
 * joined as an open one, or from its saved profile when there is one.
 */
pub fn connect(ssid: &str, password: Option<String>) {
    let ssid = String::from(ssid);
    eprintln!("aether-panel: connect requested for SSID {ssid:?}");

    // the input is done with, whatever comes of the connect
    cancel_password();

    runtime().spawn(async move {
        let Ok(connection) = WifiConnection::new().await else {
            eprintln!("aether-panel: failed to create NetworkManager connection client");
            return;
        };

        let result = match password {
            Some(password) if !password.is_empty() => {
                connection.join_with_password(&ssid, &password).await
            }
            _ => {
                // a saved profile carries its own password; an open one needs none
                connection.join_saved(&ssid).await
            }
        };
        match result {
            Ok(()) => eprintln!("aether-panel: connect request for {ssid:?} succeeded"),
            Err(error) => eprintln!("aether-panel: connect request for {ssid:?} failed: {error}"),
        }
    });
}

pub fn forget(ssid: &str) {
    let ssid = ssid.to_owned();
    runtime().spawn(async move {
        if aether_services::wifi::forget_saved_wifi(&ssid)
            .await
            .is_ok()
        {
            scan_refresh();
        }
    });
}

fn scan_refresh() {
    if let Some(networks) = NETWORKS.get() {
        if let Ok(mut networks) = networks.write() {
            for network in networks.iter_mut() {
                network.saved = false;
            }
        }
    }
    wake();
}

async fn watch_radio() {
    let Ok(toggle) = WifiToggle::new().await else {
        return;
    };

    let mut ticker = tokio::time::interval(Duration::from_secs(2));

    loop {
        ticker.tick().await;

        // a failed read keeps the last known state rather than flipping the switch
        let Ok(enabled) = toggle.is_enabled().await else {
            continue;
        };

        let Some(radio) = RADIO.get() else {
            continue;
        };

        let Ok(mut radio) = radio.write() else {
            continue;
        };

        if *radio != enabled {
            *radio = enabled;

            drop(radio);

            wake();
        }
    }
}

/*
 * The networks in range, strongest first. A scan takes a few seconds, so it runs
 * on its own slow task and only the cached list is handed to the view. With the
 * radio off there is nothing to scan, so the list empties instead.
 */
pub fn networks() -> Vec<WifiNetwork> {
    let networks = NETWORKS.get_or_init(|| RwLock::new(Vec::new()));

    SCAN_STARTED.get_or_init(|| {
        runtime().spawn(scan());
    });

    networks.read().map(|list| list.clone()).unwrap_or_default()
}

/// Ask NetworkManager to refresh the nearby network list.
pub fn search_networks() {
    eprintln!("aether-panel: nearby Wi-Fi scan requested from popup reload button");
    SCAN_REQUEST.get_or_init(Notify::new).notify_one();
}

async fn scan() {
    let scanner = match WifiScanner::new().await {
        Ok(scanner) => scanner,
        Err(_) => return,
    };

    // Do not start an active scan as a side effect of launching the panel.
    // Some Wi-Fi drivers briefly drop their link during a scan. The popup's
    // reload control explicitly requests one when the user wants nearby devices.
    loop {
        SCAN_REQUEST.get_or_init(Notify::new).notified().await;

        // with the radio off there is nothing in range, so the list goes empty
        if !radio() {
            store(Vec::new());

            continue;
        }

        // Search only when asked. Frequent active scans can interrupt Wi-Fi on
        // some adapters and appear as repeated disconnect/reconnect events.
        let found = match scanner.scan_and_list().await {
            Ok(found) => found,
            Err(error) => {
                eprintln!("aether-panel: Wi-Fi scan failed: {error}");
                continue;
            }
        };

        // the connected name marks the one entry the list highlights
        let connected = ssid();

        let saved = saved_wifi_ssids().await;
        let saved_names: std::collections::HashSet<String> =
            saved.unwrap_or_default().into_iter().collect();

        let list: Vec<WifiNetwork> = found
            .into_iter()
            .filter(|network| !network.ssid.is_empty())
            .map(|network| WifiNetwork {
                ssid: network.ssid.clone(),
                strength: network.strength.unwrap_or(0),
                secured: network.secured,
                connected: connected.as_deref() == Some(network.ssid.as_str()),
                saved: saved_names.contains(&network.ssid),
            })
            .collect();

        store(list);
    }
}

// keeps the list, waking the shell only when it really changed
fn store(list: Vec<WifiNetwork>) {
    let Some(networks) = NETWORKS.get() else {
        return;
    };

    let Ok(mut networks) = networks.write() else {
        return;
    };

    if *networks != list {
        *networks = list;

        drop(networks);

        wake();
    }
}

async fn run() {
    let Ok(service) = ConnectedWifiService::new().await else {
        return;
    };

    let mut ticker = tokio::time::interval(Duration::from_secs(5));

    loop {
        ticker.tick().await;

        // a failure keeps the last good reading instead of blanking the icon
        let Ok(signal) = service.signal().await else {
            continue;
        };

        // the name and link speed come from the same connected network, and are absent when there is none
        let (ssid, rate_mbps) = match service.get().await {
            Ok(Some(details)) => (Some(details.network.ssid), details.network.rate_mbps),
            _ => (None, None),
        };

        let reading = signal.map(|signal| WifiReading {
            strength: signal.strength,
            secured: signal.secured,
            ssid,
            rate_mbps,
        });

        let Some(latest) = LATEST.get() else {
            continue;
        };

        let Ok(mut latest) = latest.write() else {
            continue;
        };

        // only a real change wakes the panel to redraw
        if *latest != reading {
            *latest = reading;

            drop(latest);

            wake();
        }
    }
}
