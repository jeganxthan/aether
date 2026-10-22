use std::sync::OnceLock;

use tokio::runtime::Runtime;

/*
 * ClockService, BatteryService, ConnectedWifiService and NotificationService
 * all tick on tokio tasks, which need one running runtime for the whole
 * shell. The notification service talks to NetworkManager and the session
 * bus over sockets, so the runtime needs IO as well as timers. It is built
 * once, leaked so it outlives every task, and entered while each service is
 * built so their tasks land on it instead of a missing one.
 */
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .worker_threads(2)
            .build()
            .expect("failed to start the service runtime")
    })
}
