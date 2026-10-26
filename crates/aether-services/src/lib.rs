pub mod battery;
pub mod bluetooth;
pub mod clipboard;
pub mod device_monitor;
pub mod notifications;
pub mod osd;
pub mod power;
pub mod screen_utils;
pub mod wifi;

pub use screen_utils::{full_screenshot, ocr_selection, pick_color, selective_screenshot};

pub use device_monitor::start_device_monitor;
mod clock;

pub use clock::ClockService;
pub use wifi::{
    ConnectedWifiService, ConnectedWifiSignal, WifiConnection, WifiScanner, WifiToggle,
};
pub mod player;
pub use player::{PlayerBrand, PlayerMetadata, PlayerService, PlayerState};
