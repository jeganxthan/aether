use std::sync::OnceLock;

use aether_services::ClockService;

static CLOCK: OnceLock<ClockService> = OnceLock::new();

pub fn clock() -> &'static ClockService {
    CLOCK.get_or_init(start_clock)
}

fn start_clock() -> ClockService {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(1)
        .build()
        .expect("failed to start the clock runtime");

    let clock = {
        let _guard = runtime.enter();
        ClockService::new()
    };

    // Keep the runtime alive for the lifetime of the application.
    Box::leak(Box::new(runtime));

    clock
}