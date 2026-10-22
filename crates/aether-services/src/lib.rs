pub mod wifi;
pub mod battery;
pub mod notifications;
pub mod osd;
pub mod device_monitor;

pub mod screen_utils;

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