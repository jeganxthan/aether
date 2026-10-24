pub mod wifi;
pub mod battery;
pub mod notifications;
pub mod osd;
pub mod device_monitor;
pub mod power;
pub mod screen_utils;
pub mod clipboard;
pub mod bluetooth;

pub use screen_utils::{
    full_screenshot,
    ocr_selection,
    pick_color,
    selective_screenshot,
};

pub use device_monitor::start_device_monitor;
mod clock;

pub use wifi::{WifiToggle, WifiScanner, WifiConnection, ConnectedWifiService, ConnectedWifiSignal};
pub use clock::ClockService;
pub mod player;
pub use player::{PlayerService, PlayerState, PlayerBrand, PlayerMetadata};
