use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub desktop_file: PathBuf,
}

impl Application {
    pub fn icon_path(&self) -> Option<PathBuf> {
        self.icon.as_deref().and_then(resolve_icon)
    }
}

static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();

pub fn resolve_icon(name: &str) -> Option<PathBuf> {
    let cache = ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(name) {
            return cached.clone();
        }
    }

    let found = find_icon_on_disk(name);

    if let Ok(mut guard) = cache.lock() {
        guard.insert(name.to_string(), found.clone());
    }

    found
}

fn find_icon_on_disk(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() && path.exists() {
        return Some(path.to_path_buf());
    }

    let clean = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".svg"))
        .or_else(|| name.strip_suffix(".xpm"))
        .unwrap_or(name);

    // Direct check in /usr/share/pixmaps
    for ext in &[".png", ".svg"] {
        let p = PathBuf::from(format!("/usr/share/pixmaps/{clean}{ext}"));
        if p.exists() {
            return Some(p);
        }
    }

    // Search hicolor and system icon themes
    let home = std::env::var("HOME").unwrap_or_default();
    let bases = [
        format!("{home}/.local/share/icons/hicolor"),
        "/usr/share/icons/hicolor".to_string(),
        "/usr/share/icons/Adwaita".to_string(),
        "/usr/share/icons/breeze".to_string(),
        "/usr/share/icons/Papirus".to_string(),
    ];

    let sizes = [
        "48x48", "scalable", "32x32", "64x64", "128x128", "256x256", "symbolic",
    ];
    for base in &bases {
        for size in &sizes {
            for sub in &["apps", "categories", "devices", "status", "ui"] {
                for ext in &[".png", ".svg"] {
                    let p = PathBuf::from(format!("{base}/{size}/{sub}/{clean}{ext}"));
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone, Default)]
pub struct ApplicationService {
    applications: Vec<Application>,
}

impl ApplicationService {
    /// Discover desktop applications in the standard XDG data directories.
    pub fn new() -> io::Result<Self> {
        let mut service = Self::default();
        service.refresh()?;
        Ok(service)
    }

    /// Rescan application directories.
    pub fn refresh(&mut self) -> io::Result<()> {
        let mut directories = application_directories();
        let mut applications = Vec::new();
        let mut seen_ids = HashSet::new();

        // Earlier directories take precedence over later ones.
        for directory in directories.drain(..) {
            if !directory.is_dir() {
                continue;
            }

            let mut files = Vec::new();
            collect_desktop_files(&directory, &mut files)?;

            // Stable results make launcher ordering predictable.
            files.sort();

            for file in files {
                let Some(id) = desktop_id(&directory, &file) else {
                    continue;
                };

                // Preserve the first occurrence of a desktop-file ID.
                if seen_ids.contains(&id) {
                    continue;
                }

                let Ok(entry) = parse_desktop_file(&file) else {
                    continue;
                };

                if entry.hidden || entry.no_display {
                    seen_ids.insert(id);
                    continue;
                }

                if entry.entry_type.as_deref() != Some("Application") {
                    continue;
                }

                if !entry.try_exec.as_deref().is_none_or(command_exists) {
                    continue;
                }

                let Some(name) = entry.name.filter(|name| !name.is_empty()) else {
                    continue;
                };

                seen_ids.insert(id.clone());

                applications.push(Application {
                    id,
                    name,
                    comment: entry.comment.filter(|c| !c.is_empty()),
                    icon: entry.icon.filter(|icon| !icon.is_empty()),
                    desktop_file: file,
                });
            }
        }

        applications.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });

        self.applications = applications;

        for app in &self.applications {
            if let Some(ref icon) = app.icon {
                resolve_icon(icon);
            }
        }

        Ok(())
    }

    pub fn all(&self) -> &[Application] {
        &self.applications
    }

    /// Case-insensitive search with relevance sorting.
    pub fn search(&self, query: &str) -> Vec<Application> {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return self.applications.clone();
        }

        let mut scored: Vec<(u32, usize, Application)> = Vec::new();

        for (orig_idx, app) in self.applications.iter().enumerate() {
            let name_lower = app.name.to_lowercase();
            let id_lower = app.id.to_lowercase();
            let comment_lower = app.comment.as_deref().unwrap_or("").to_lowercase();

            let score = if name_lower == query {
                Some(0)
            } else if name_lower.starts_with(&query) {
                Some(10)
            } else if name_lower
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word.starts_with(&query))
            {
                Some(20)
            } else if name_lower.contains(&query) {
                Some(30)
            } else if id_lower.starts_with(&query) {
                Some(40)
            } else if id_lower.contains(&query) {
                Some(50)
            } else if comment_lower.contains(&query) {
                Some(60)
            } else if let Some(skipped) = subsequence_skips(&name_lower, &query) {
                Some(70 + skipped.min(29) as u32)
            } else if let Some(distance) = fuzzy_distance(&name_lower, &query) {
                Some(100 + distance as u32)
            } else if let Some(skipped) = subsequence_skips(&id_lower, &query) {
                Some(120 + skipped.min(29) as u32)
            } else {
                None
            };

            if let Some(s) = score {
                scored.push((s, orig_idx, app.clone()));
            }
        }

        scored.sort_by(|(s1, idx1, a1), (s2, idx2, a2)| {
            s1.cmp(s2)
                .then_with(|| a1.name.len().cmp(&a2.name.len()))
                .then_with(|| a1.name.cmp(&a2.name))
                .then_with(|| idx1.cmp(idx2))
        });

        scored.into_iter().map(|(_, _, app)| app).collect()
    }

    /// Launch an application using its desktop entry.
    pub fn launch(app: &Application) -> io::Result<()> {
        // GIO understands desktop entries and their launch semantics.
        match Command::new("gio")
            .arg("launch")
            .arg(&app.desktop_file)
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(gio_error) if gio_error.kind() == io::ErrorKind::NotFound => {
                // Fallback for systems with gtk-launch but no gio executable.
                let status = Command::new("gtk-launch").arg(&app.id).status()?;

                if status.success() {
                    Ok(())
                } else {
                    Err(io::Error::other(format!(
                        "gtk-launch failed for {}",
                        app.name
                    )))
                }
            }
            Err(error) => Err(error),
        }
    }
}

fn subsequence_skips(candidate: &str, query: &str) -> Option<usize> {
    let mut chars = candidate.char_indices();
    let mut last_position = None;
    let mut skipped = 0;

    for query_char in query.chars() {
        let (position, _) = chars.find(|(_, candidate_char)| *candidate_char == query_char)?;
        if let Some(previous) = last_position {
            skipped += candidate[previous..position]
                .chars()
                .count()
                .saturating_sub(1);
        }
        last_position = Some(position);
    }

    Some(skipped)
}

fn fuzzy_distance(candidate: &str, query: &str) -> Option<usize> {
    let candidate: Vec<char> = candidate.chars().collect();
    let query: Vec<char> = query.chars().collect();
    if query.len() < 4 || candidate.len().abs_diff(query.len()) > 2 {
        return None;
    }

    let mut previous: Vec<usize> = (0..=candidate.len()).collect();
    let mut current = vec![0; candidate.len() + 1];

    for (row, query_char) in query.iter().enumerate() {
        current[0] = row + 1;
        for (column, candidate_char) in candidate.iter().enumerate() {
            current[column + 1] = (previous[column + 1] + 1)
                .min(current[column] + 1)
                .min(previous[column] + usize::from(query_char != candidate_char));
        }
        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[candidate.len()];
    (distance <= 2).then_some(distance)
}

#[derive(Default)]
struct DesktopEntry {
    name: Option<String>,
    comment: Option<String>,
    icon: Option<String>,
    entry_type: Option<String>,
    try_exec: Option<String>,
    hidden: bool,
    no_display: bool,
}

fn application_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();

    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        directories.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = env::var_os("HOME") {
        directories.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());

    for directory in env::split_paths(&data_dirs) {
        directories.push(directory.join("applications"));
    }

    directories
}

fn collect_desktop_files(directory: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    for item in fs::read_dir(directory)? {
        let item = item?;
        let path = item.path();
        let file_type = item.file_type()?;

        if file_type.is_dir() {
            collect_desktop_files(&path, output)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "desktop") {
            output.push(path);
        }
    }

    Ok(())
}

fn desktop_id(root: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(root).ok()?;
    let relative = relative.to_str()?;

    // XDG desktop-file IDs replace directory separators with hyphens.
    Some(relative.replace('/', "-"))
}

fn parse_desktop_file(path: &Path) -> io::Result<DesktopEntry> {
    let contents = fs::read_to_string(path)?;
    let mut entry = DesktopEntry::default();
    let mut in_desktop_entry = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "Name" => entry.name = Some(value.to_string()),
            "Comment" => entry.comment = Some(value.to_string()),
            "Icon" => entry.icon = Some(value.to_string()),
            "Type" => entry.entry_type = Some(value.to_string()),
            "TryExec" => entry.try_exec = Some(value.to_string()),
            "Hidden" => entry.hidden = value.eq_ignore_ascii_case("true"),
            "NoDisplay" => entry.no_display = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    Ok(entry)
}

fn command_exists(command: &str) -> bool {
    let path = Path::new(command);

    // TryExec may be an absolute or relative executable path.
    if path.components().count() > 1 {
        return path.is_file();
    }

    let Some(path_env) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path_env).any(|directory| directory.join(command).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, id: &str) -> Application {
        Application {
            id: id.to_owned(),
            name: name.to_owned(),
            comment: None,
            icon: None,
            desktop_file: PathBuf::new(),
        }
    }

    #[test]
    fn launcher_search_matches_initials_and_small_typos() {
        let service = ApplicationService {
            applications: vec![
                app("Brave", "brave-browser.desktop"),
                app("Google Chrome", "google-chrome.desktop"),
                app("Calculator", "calculator.desktop"),
            ],
        };

        assert_eq!(service.search("Brv")[0].name, "Brave");
        assert_eq!(service.search("gogle")[0].name, "Google Chrome");
    }
}
