mod storage;
mod types;

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::{self, Write},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub use storage::{data_dir, load_history, save_history};
pub use types::{ClipboardConfig, ClipboardEntry};

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const TEXT_MIME_TYPES: [&str; 5] = [
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
    "TEXT",
];
const SENSITIVE_MIME_TYPES: [&str; 4] = [
    "application/x-kde-passwordManagerHint",
    "application/vnd.portal.Secret",
    "application/x-secret",
    "application/x-password",
];

pub fn read_text() -> io::Result<Option<String>> {
    let types = Command::new("wl-paste").args(["--list-types"]).output()?;
    if !types.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&types.stderr).trim().to_owned(),
        ));
    }

    let offered_text = String::from_utf8_lossy(&types.stdout).into_owned();
    let offered: Vec<_> = offered_text
        .lines()
        .map(str::trim)
        .filter(|mime| !mime.is_empty())
        .collect();
    if offered
        .iter()
        .any(|mime| SENSITIVE_MIME_TYPES.contains(mime))
    {
        return Ok(None);
    }

    let Some(mime) = TEXT_MIME_TYPES.iter().find(|mime| offered.contains(mime)) else {
        return Ok(None);
    };

    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", mime])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    if output.stdout.len() > MAX_TEXT_BYTES {
        return Ok(None);
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

pub fn read_image() -> io::Result<Option<Vec<u8>>> {
    let types = Command::new("wl-paste").args(["--list-types"]).output()?;
    if !types.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&types.stderr).trim().to_owned(),
        ));
    }

    let offered = String::from_utf8_lossy(&types.stdout);
    let Some(mime) = offered
        .lines()
        .map(str::trim)
        .find(|mime| *mime == "image/png")
    else {
        return Ok(None);
    };

    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", mime])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    if output.stdout.is_empty() || output.stdout.len() > 16 * MAX_TEXT_BYTES {
        return Ok(None);
    }

    Ok(Some(output.stdout))
}

pub fn copy_text(text: &str) -> io::Result<()> {
    let mut child = Command::new("wl-copy").stdin(Stdio::piped()).spawn()?;
    let result = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("wl-copy stdin unavailable"))?
        .write_all(text.as_bytes());
    let status = child.wait()?;
    result?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("wl-copy exited with {status}")))
    }
}

pub fn copy_image(path: &str) -> io::Result<()> {
    let file = std::fs::File::open(path)?;
    let mut child = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::from(file))
        .spawn()?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("wl-copy exited with {status}")))
    }
}

pub fn record_text(history: &mut Vec<ClipboardEntry>, text: String, max_entries: usize) -> bool {
    if text.is_empty() || text.len() > MAX_TEXT_BYTES {
        return false;
    }

    if let Some(index) = history.iter().position(|entry| entry.text == text) {
        let mut entry = history.remove(index);
        entry.copied_at = now_unix_seconds();
        history.insert(0, entry);
    } else {
        history.insert(
            0,
            ClipboardEntry {
                id: entry_id(&text),
                text,
                image_path: None,
                copied_at: now_unix_seconds(),
                pinned: false,
            },
        );
    }

    trim_history(history, max_entries);
    true
}

pub fn record_image(
    history: &mut Vec<ClipboardEntry>,
    image: &[u8],
    max_entries: usize,
) -> io::Result<bool> {
    if image.is_empty() {
        return Ok(false);
    }

    let id = entry_id_bytes(image);
    if let Some(index) = history.iter().position(|entry| entry.id == id) {
        let mut entry = history.remove(index);
        entry.copied_at = now_unix_seconds();
        history.insert(0, entry);
        return Ok(true);
    }

    let path = data_dir()?.join(format!("{id}.png"));
    fs::create_dir_all(path.parent().expect("image path has a parent"))?;
    fs::write(&path, image)?;
    history.insert(
        0,
        ClipboardEntry {
            id,
            text: "Image".into(),
            image_path: Some(path.to_string_lossy().into_owned()),
            copied_at: now_unix_seconds(),
            pinned: false,
        },
    );
    trim_history(history, max_entries);
    Ok(true)
}

pub fn pin_entry(history: &mut [ClipboardEntry], id: &str) -> io::Result<bool> {
    let entry = history
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Clipboard entry not found"))?;
    entry.pinned = !entry.pinned;
    Ok(entry.pinned)
}

pub fn history_config() -> ClipboardConfig {
    ClipboardConfig::default()
}

fn trim_history(history: &mut Vec<ClipboardEntry>, max_entries: usize) {
    while history.len() > max_entries {
        let Some(index) = history.iter().rposition(|entry| !entry.pinned) else {
            break;
        };
        history.remove(index);
    }
}

fn entry_id(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:x}-{:x}", now_unix_seconds(), hasher.finish())
}

fn entry_id_bytes(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("image-{:x}", hasher.finish())
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs().min(i64::MAX as u64) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, text: &str, pinned: bool) -> ClipboardEntry {
        ClipboardEntry {
            id: id.into(),
            text: text.into(),
            image_path: None,
            copied_at: 0,
            pinned,
        }
    }

    #[test]
    fn recording_moves_duplicate_to_front_without_losing_pin() {
        let mut history = vec![entry("one", "first", true), entry("two", "second", false)];

        assert!(record_text(&mut history, "first".into(), 10));

        assert_eq!(history[0].id, "one");
        assert!(history[0].pinned);
        assert_eq!(history[1].text, "second");
    }

    #[test]
    fn history_cap_evicts_oldest_unpinned_entries_only() {
        let mut history = vec![entry("pinned", "keep", true), entry("old", "remove", false)];

        assert!(record_text(&mut history, "new".into(), 2));

        assert_eq!(history.len(), 2);
        assert!(history.iter().any(|entry| entry.id == "pinned"));
        assert!(history.iter().any(|entry| entry.text == "new"));
    }

    #[test]
    fn pinning_requires_a_matching_entry() {
        let mut history = vec![entry("id", "text", false)];
        assert!(pin_entry(&mut history, "id").unwrap());
        assert!(history[0].pinned);
        assert_eq!(
            pin_entry(&mut history, "missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }
}
