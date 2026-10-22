
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub desktop_file: PathBuf,
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
        Ok(())
    }

    pub fn all(&self) -> &[Application] {
        &self.applications
    }

    /// Case-insensitive search by application name.
    pub fn search(&self, query: &str) -> Vec<Application> {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return self.applications.clone();
        }

        self.applications
            .iter()
            .filter(|app| app.name.to_lowercase().contains(&query))
            .cloned()
            .collect()
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
                let status = Command::new("gtk-launch")
                    .arg(&app.id)
                    .status()?;

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

#[derive(Default)]
struct DesktopEntry {
    name: Option<String>,
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

fn collect_desktop_files(
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> io::Result<()> {
    for item in fs::read_dir(directory)? {
        let item = item?;
        let path = item.path();
        let file_type = item.file_type()?;

        if file_type.is_dir() {
            collect_desktop_files(&path, output)?;
        } else if file_type.is_file()
            && path.extension().is_some_and(|ext| ext == "desktop")
        {
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
            "Icon" => entry.icon = Some(value.to_string()),
            "Type" => entry.entry_type = Some(value.to_string()),
            "TryExec" => entry.try_exec = Some(value.to_string()),
            "Hidden" => entry.hidden = value.eq_ignore_ascii_case("true"),
            "NoDisplay" => {
                entry.no_display = value.eq_ignore_ascii_case("true")
            }
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

    env::split_paths(&path_env)
        .any(|directory| directory.join(command).is_file())
}
