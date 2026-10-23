use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::refresh as wake;
use aether_services::player::{PlayerService, PlayerState};

use super::runtime::runtime;

static LATEST: OnceLock<RwLock<Option<PlayerState>>> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();
static SEEK_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn player() -> Option<PlayerState> {
    let latest = LATEST.get_or_init(|| RwLock::new(None));

    STARTED.get_or_init(|| {
        runtime().spawn(refresh_loop());
    });

    latest.read().ok().and_then(|p| p.clone())
}

pub fn play_pause() {
    let Some(current) = player() else {
        return;
    };
    runtime().spawn(async move {
        if let Ok(service) = PlayerService::new().await {
            let _ = service.play_pause(&current.bus_name).await;
            trigger_refresh().await;
        }
    });
}

pub fn next() {
    let Some(current) = player() else {
        return;
    };
    runtime().spawn(async move {
        if let Ok(service) = PlayerService::new().await {
            let _ = service.next(&current.bus_name).await;
            trigger_refresh().await;
        }
    });
}

pub fn previous() {
    let Some(current) = player() else {
        return;
    };
    runtime().spawn(async move {
        if let Ok(service) = PlayerService::new().await {
            let _ = service.previous(&current.bus_name).await;
            trigger_refresh().await;
        }
    });
}

pub fn seek_to(position_secs: u64) {
    let Some(current) = player() else {
        return;
    };
    let generation = SEEK_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    runtime().spawn(async move {
        tokio::time::sleep(Duration::from_millis(70)).await;
        if SEEK_GENERATION.load(Ordering::Relaxed) != generation {
            return;
        }
        if let Ok(service) = PlayerService::new().await {
            let current_position = service.current_player().await
                .filter(|state| state.bus_name == current.bus_name)
                .map(|state| state.position_secs)
                .unwrap_or(current.position_secs);
            let difference = position_secs as i128 - current_position as i128;
            let offset_us = difference.saturating_mul(1_000_000).clamp(i64::MIN as i128, i64::MAX as i128) as i64;
            let _ = service.seek(&current.bus_name, offset_us).await;
            trigger_refresh().await;
        }
    });
}

async fn trigger_refresh() {
    tokio::time::sleep(Duration::from_millis(50)).await;
    if let Ok(service) = PlayerService::new().await {
        let new_state = service.current_player().await;
        if let Some(latest) = LATEST.get() {
            if let Ok(mut write) = latest.write() {
                *write = new_state;
            }
        }
        wake();
    }
}

async fn refresh_loop() {
    let Ok(service) = PlayerService::new().await else {
        return;
    };

    let mut ticker = tokio::time::interval(Duration::from_millis(500));

    loop {
        ticker.tick().await;

        let reading = service.current_player().await;

        let Some(latest) = LATEST.get() else {
            continue;
        };

        let mut changed = false;
        if let Ok(mut write) = latest.write() {
            if *write != reading {
                *write = reading;
                changed = true;
            }
        }

        if changed {
            wake();
        }
    }
}
