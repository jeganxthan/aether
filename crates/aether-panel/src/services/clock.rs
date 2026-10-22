use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::refresh as wake;
use aether_services::ClockService;

use super::runtime::runtime;

// the latest time and date strings, kept where the sync view can read them
static LATEST: OnceLock<RwLock<String>> = OnceLock::new();
static TODAY: OnceLock<RwLock<String>> = OnceLock::new();

// starts the refresh task exactly once, after the caches exist
static STARTED: OnceLock<()> = OnceLock::new();

pub fn clock() -> String {
    let latest = LATEST.get_or_init(|| RwLock::new(String::new()));

    STARTED.get_or_init(|| {
        let _guard = runtime().enter();

        let service = ClockService::new();

        runtime().spawn(async move {
            // the clock ticks every second; refreshing on the minute alone is enough
            let mut ticker = tokio::time::interval(Duration::from_secs(1));

            loop {
                ticker.tick().await;

                let now = service.current();
                let today = service.today();

                let Some(latest) = LATEST.get() else {
                    continue;
                };

                let Ok(mut latest) = latest.write() else {
                    continue;
                };

                // only a real change wakes the panel to redraw
                if *latest != now {
                    *latest = now;

                    drop(latest);

                    wake();
                }

                // the date only changes at midnight, so it needs its own check
                if let Some(cache) = TODAY.get()
                    && let Ok(mut today_cache) = cache.write()
                    && *today_cache != today
                {
                    *today_cache = today;

                    drop(today_cache);

                    wake();
                }
            }
        });
    });

    latest
        .read()
        .map(|time| time.clone())
        .unwrap_or_default()
}

// the month and day, like OCT 18, read by the bar next to the time
pub fn date() -> String {
    let today = TODAY.get_or_init(|| RwLock::new(String::new()));

    // the first call primes the cache, so the same task is started once here too
    let _ = clock();

    today
        .read()
        .map(|date| date.clone())
        .unwrap_or_default()
}
