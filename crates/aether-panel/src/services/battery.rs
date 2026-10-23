use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::refresh as wake;
use aether_services::battery::{BatteryInfo, BatteryService, info::Battery};

use super::runtime::runtime;

// the latest reading, kept where the sync view can read it without touching sysfs
static LATEST: OnceLock<RwLock<Option<BatteryInfo>>> = OnceLock::new();
static DETAILS: OnceLock<RwLock<Option<Battery>>> = OnceLock::new();

// starts the refresh task exactly once, after LATEST exists
static STARTED: OnceLock<()> = OnceLock::new();

/*
 * The view runs on every redraw, so the service is built once and only the
 * cached reading is handed back. A machine without a battery keeps none, and
 * the panel simply shows no icon.
 */
pub fn battery() -> Option<BatteryInfo> {
    let latest = LATEST.get_or_init(|| RwLock::new(None));
    DETAILS.get_or_init(|| RwLock::new(None));

    STARTED.get_or_init(|| {
        runtime().spawn(refresh());
    });

    latest.read().ok().and_then(|battery| *battery)
}

pub fn battery_details() -> Option<Battery> {
    let _ = battery();

    DETAILS
        .get()
        .and_then(|details| details.read().ok().and_then(|details| details.clone()))
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
        let details = aether_services::battery::info::battery();

        let Some(latest) = LATEST.get() else {
            continue;
        };

        let Ok(mut current) = latest.write() else {
            continue;
        };

        let reading_changed = *current != reading;
        if reading_changed {
            *current = reading;
        }

        let details_changed = DETAILS.get().is_some_and(|cache| {
            let Ok(mut current) = cache.write() else {
                return false;
            };

            let changed = *current != details;
            if changed {
                *current = details;
            }
            changed
        });

        drop(current);

        if reading_changed || details_changed {
            wake();
        }
    }
}
