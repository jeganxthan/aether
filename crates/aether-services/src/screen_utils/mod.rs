mod capture;
mod clipboard;
mod color_picker;
mod icons;
mod notify;
mod ocr;
mod region;
mod swatch;

pub use capture::{capture_point, capture_region, full_screenshot, selective_screenshot};
pub use color_picker::pick_color;
pub use notify::notify;
pub use ocr::ocr_selection;
pub use region::{select_point, select_region};
