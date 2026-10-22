use std::{
    io::Write,
    process::{Command, Stdio},
};

fn copy(data: &[u8], content_type: Option<&str>) -> Result<(), String> {
    let mut command = Command::new("wl-copy");

    if let Some(content_type) = content_type {
        command.args(["--type", content_type]);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start wl-copy: {e}"))?;

    child
        .stdin
        .take()
        .ok_or("Clipboard stdin unavailable")?
        .write_all(data)
        .map_err(|e| format!("Clipboard write failed: {e}"))?;

    let status = child
        .wait()
        .map_err(|e| format!("Clipboard process failed: {e}"))?;

    if !status.success() {
        return Err("Clipboard copy failed".into());
    }

    Ok(())
}

pub fn copy_image(png: &[u8]) -> Result<(), String> {
    copy(png, Some("image/png"))
}

pub fn copy_text(text: &str) -> Result<(), String> {
    copy(text.as_bytes(), None)
}