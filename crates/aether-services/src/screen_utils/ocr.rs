use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::{capture, clipboard, icons, notify, region};

/*
 * Void Linux installs the engine as `tesseract-ocr`, while most other distros
 * call it `tesseract`. Look for both so the shortcut works on either.
 */
const BINARIES: [&str; 2] = ["tesseract", "tesseract-ocr"];

// the language the OCR reads, when the user does not ask for another one
const LANGUAGE: &str = "eng";

fn tesseract() -> Result<PathBuf, String> {
    // an explicit path wins, for a copy that is not on the PATH
    if let Some(path) = std::env::var_os("AETHER_TESSERACT") {
        let path = PathBuf::from(path);

        if path.is_file() {
            return Ok(path);
        }
    }

    // also check common locations directly
    for candidate in [
        "/usr/bin/tesseract",
        "/usr/bin/tesseract-ocr",
        "/bin/tesseract",
        "/bin/tesseract-ocr",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    let path = std::env::var_os("PATH").ok_or("PATH is not set")?;

    for directory in std::env::split_paths(&path) {
        for name in BINARIES {
            let candidate = directory.join(name);

            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err("Tesseract is not installed (install tesseract-ocr and tesseract-ocr-eng)".into())
}

// reads the error Tesseract wrote, so a missing language pack is not a mystery
fn failure(stderr: &[u8]) -> String {
    let message = String::from_utf8_lossy(stderr);
    let message = message.trim();

    if message.is_empty() {
        "Tesseract could not recognize the image".into()
    } else {
        format!("Tesseract could not recognize the image: {message}")
    }
}

fn notify_failure(error: &str) {
    notify::notify("OCR", &icons::icon("clipboard"), "OCR failed", error);
}

/// Recognize text in a selected screen region and copy it.
pub fn ocr_selection() -> Result<String, String> {
    let selected = match region::select_region() {
        Ok(selected) => selected,
        Err(e) => {
            notify_failure(&e);
            return Err(e);
        }
    };

    let png = match capture::capture_region(&selected) {
        Ok(png) => png,
        Err(e) => {
            notify_failure(&e);
            return Err(e);
        }
    };

    let binary = match tesseract() {
        Ok(binary) => binary,
        Err(e) => {
            notify_failure(&e);
            return Err(e);
        }
    };

    let language = std::env::var("AETHER_TESSERACT_LANG").unwrap_or_else(|_| LANGUAGE.to_string());

    let mut child = match Command::new(&binary)
        .args(["stdin", "stdout", "-l", &language])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let msg = if e.kind() == std::io::ErrorKind::NotFound {
                "Tesseract is not installed (install tesseract-ocr and tesseract-ocr-eng)"
                    .to_string()
            } else {
                format!("Failed to start Tesseract: {e}")
            };
            notify_failure(&msg);
            return Err(msg);
        }
    };

    {
        let mut stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let msg = "OCR input unavailable".to_string();
                notify_failure(&msg);
                return Err(msg);
            }
        };

        if let Err(e) = stdin.write_all(&png) {
            let msg = format!("Failed to pass image to OCR: {e}");
            notify_failure(&msg);
            return Err(msg);
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            let msg = format!("OCR process failed: {e}");
            notify_failure(&msg);
            return Err(msg);
        }
    };

    if !output.status.success() {
        let msg = failure(&output.stderr);
        notify_failure(&msg);
        return Err(msg);
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if text.is_empty() {
        let msg = "No text detected".to_string();
        notify_failure(&msg);
        return Err(msg);
    }

    if let Err(e) = clipboard::copy_text(&text) {
        let msg = format!("Failed to copy text to clipboard: {e}");
        notify_failure(&msg);
        return Err(msg);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_language_pack_is_reported() {
        let error =
            failure(b"Error, cannot initialize Tesseract.\nFailed loading language 'eng'\n");

        assert!(error.contains("could not recognize"));
    }

    #[test]
    fn an_empty_error_still_reads_as_a_failure() {
        assert_eq!(failure(b""), "Tesseract could not recognize the image");
    }
}
