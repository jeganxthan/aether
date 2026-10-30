use std::{fs, io, path::PathBuf};

use super::types::ClipboardEntry;

pub fn data_dir() -> io::Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Cannot determine user data directory",
            )
        })?;

    Ok(base.join("aether/clipboard"))
}

pub fn load_history() -> io::Result<Vec<ClipboardEntry>> {
    let path = data_dir()?.join("history.json");

    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(io::Error::other)
}

pub fn save_history(entries: &[ClipboardEntry]) -> io::Result<()> {
    let path = data_dir()?.join("history.json");

    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Invalid clipboard history path"))?;

    fs::create_dir_all(parent)?;

    let temporary = parent.join("history.json.tmp");
    let contents = serde_json::to_vec_pretty(entries)?;

    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)?;

    Ok(())
}
