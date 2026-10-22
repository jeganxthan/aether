use super::{capture, clipboard, icons, notify, region, swatch};

/// Pick the color under the pointer and copy its hex value.
pub fn pick_color() -> Result<String, String> {
    let (x, y) = region::select_point()?;
    let png = capture::capture_point(x, y)?;

    let image = image::load_from_memory(&png)
        .map_err(|e| format!("Failed to decode screenshot: {e}"))?;

    let rgba = image.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    let color = format!("#{:02X}{:02X}{:02X}", pixel[0], pixel[1], pixel[2]);

    clipboard::copy_text(&color)?;

    // the popup shows the colour itself, so it is worth the small square
    let swatch = swatch::write(&color)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| icons::icon("color-picker"));

    notify::notify(
        "Color Picker",
        &swatch,
        "Color picked",
        &format!("{color} · copied to the clipboard"),
    );

    Ok(color)
}
