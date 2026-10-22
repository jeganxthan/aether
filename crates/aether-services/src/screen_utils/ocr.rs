use std::io::Write;
use std::process::{Command, Stdio};

use super::{capture, clipboard, icons, notify, region};

/// Recognize text in a selected screen region and copy it.
pub fn ocr_selection() -> Result<String, String> {
    let selected = region::select_region()?;
    let png = capture::capture_region(&selected)?;

    let mut child = Command::new("tesseract")
        .args(["stdin", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "tesseract is not installed".to_string()
            } else {
                format!("Failed to start Tesseract: {e}")
            }
        })?;

    child
        .stdin
        .take()
        .ok_or("OCR input unavailable")?
        .write_all(&png)
        .map_err(|e| format!("Failed to pass image to OCR: {e}"))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("OCR process failed: {e}"))?;

    if !output.status.success() {
        return Err("Tesseract could not recognize the image".into());
    }

    let text = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_owned();

    if text.is_empty() {
        return Err("No text detected".into());
    }

    clipboard::copy_text(&text)?;

    // the popup shows the first lines of what was read, the rest is on the clipboard
    let shown: String = text.chars().take(240).collect();

    notify::notify(
        "OCR",
        &icons::icon("clipboard"),
        "Text copied",
        &format!("{shown} · copied to the clipboard"),
    );

    Ok(text)
}
