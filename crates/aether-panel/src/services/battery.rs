use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::refresh as wake;
use aether_services::battery::{BatteryInfo, BatteryService};

use super::runtime::runtime;

// the latest reading, kept where the sync view can read it without touching sysfs
static LATEST: OnceLock<RwLock<Option<BatteryInfo>>> = OnceLock::new();

// starts the refresh task exactly once, after LATEST exists
static STARTED: OnceLock<()> = OnceLock::new();

/*
 * The view runs on every redraw, so the service is built once and only the
 * cached reading is handed back. A machine without a battery keeps none, and
 * the panel simply shows no icon.
 */
pub fn battery() -> Option<BatteryInfo> {
    let latest = LATEST.get_or_init(|| RwLock::new(None));

    STARTED.get_or_init(|| {
        runtime().spawn(refresh());
    });

    latest.read().ok().and_then(|battery| *battery)
}

async fn refresh() {
    // no battery on this machine leaves the cache empty for good
    let Ok(service) = BatteryService::new() else {
        return;
    };

    let mut ticker = tokio::time::interval(Duration::from_secs(5));

    loop {
        ticker.tick().await;

        let reading = service.current().ok();

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
