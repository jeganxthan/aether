use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use crate::models::launcher::AppInfo;
use crate::models::launcher::LauncherResults;


// ---------------------------------------------------------
// Desktop file locations
// ---------------------------------------------------------

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/usr/share/applications"));

    dirs
}

// ---------------------------------------------------------
// Parse .desktop file
// ---------------------------------------------------------

fn parse_desktop_file(path: &Path) -> Option<AppInfo> {
    let content = fs::read_to_string(path).ok()?;

    let mut in_desktop_entry = false;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut comment = None;
    let mut categories = Vec::new();
    let mut hidden = false;
    let mut no_display = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }

        if line.starts_with('[') && line != "[Desktop Entry]" {
            in_desktop_entry = false;
        }

        if !in_desktop_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = value.trim();

        match key {
            "Name" => {
                if name.is_none() {
                    name = Some(value.to_string());
                }
            }

            "Exec" => {
                exec = Some(value.to_string());
            }

            "Icon" => {
                icon = Some(value.to_string());
            }

            "Comment" => {
                comment = Some(value.to_string());
            }

            "Categories" => {
                categories = value
                    .split(';')
                    .filter(|x| !x.is_empty())
                    .map(|x| x.to_string())
                    .collect();
            }

            "Hidden" => {
                hidden = value.eq_ignore_ascii_case("true");
            }

            "NoDisplay" => {
                no_display = value.eq_ignore_ascii_case("true");
            }

            _ => {}
        }
    }

    if hidden || no_display {
        return None;
    }

    let name = name?;
    let exec = exec?;

    let id = path
        .file_name()?
        .to_string_lossy()
        .trim_end_matches(".desktop")
        .to_string();

    Some(AppInfo {
        id,
        name,
        exec,
        icon,
        comment,
        categories,
    })
}

// ---------------------------------------------------------
// Scan applications
// ---------------------------------------------------------

fn scan_applications() -> Vec<AppInfo> {
    let mut applications: HashMap<String, AppInfo> = HashMap::new();

    for directory in application_dirs() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|x| x.to_str()) != Some("desktop") {
                continue;
            }

            if let Some(app) = parse_desktop_file(&path) {
                applications.entry(app.id.clone()).or_insert(app);
            }
        }
    }

    let mut applications: Vec<AppInfo> = applications.into_values().collect();

    applications.sort_by_key(|app| app.name.to_lowercase());

    applications
}

// ---------------------------------------------------------
// Search scoring
// ---------------------------------------------------------

fn score_app(app: &AppInfo, query: &str) -> Option<i32> {
    let query = query.to_lowercase();
    let name = app.name.to_lowercase();

    if query.is_empty() {
        return Some(0);
    }

    // Exact match
    if name == query {
        return Some(1000);
    }

    // Starts with query
    if name.starts_with(&query) {
        return Some(900);
    }

    // Word starts with query
    for word in name.split_whitespace() {
        if word.starts_with(&query) {
            return Some(800);
        }
    }

    // Contains query
    if name.contains(&query) {
        return Some(700);
    }

    // Simple fuzzy matching
    let mut query_chars = query.chars();
    let mut current = query_chars.next();

    let mut matched = 0;

    for ch in name.chars() {
        if Some(ch) == current {
            matched += 1;
            current = query_chars.next();

            if current.is_none() {
                break;
            }
        }
    }

    if current.is_none() {
        return Some(500 + matched);
    }

    None
}

// ---------------------------------------------------------
// Search command
// ---------------------------------------------------------

#[tauri::command]
pub fn launcher_search(query: String) -> Result<LauncherResults, String> {
    let applications = scan_applications();

    let query = query.trim();

    // -----------------------------------------------------
    // "*" = all applications
    // -----------------------------------------------------

    if query == "*" {
        let results = applications.into_iter().take(100).collect();

        return Ok(LauncherResults {
            results,
            autocomplete: None,
        });
    }

    // -----------------------------------------------------
    // Search
    // -----------------------------------------------------

    let mut scored: Vec<(i32, AppInfo)> = applications
        .into_iter()
        .filter_map(|app| {
            score_app(&app, query).map(|score| (score, app))
        })
        .collect();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
    });

    let autocomplete = scored
        .first()
        .map(|(_, app)| app.name.clone());

    let results = scored
        .into_iter()
        .take(8)
        .map(|(_, app)| app)
        .collect();

    Ok(LauncherResults {
        results,
        autocomplete,
    })
}

// ---------------------------------------------------------
// Launch application
// ---------------------------------------------------------

#[tauri::command]
pub fn launcher_launch(app: AppInfo) -> Result<(), String> {
    let exec = clean_exec(&app.exec);

    let mut parts = shell_split(&exec);

    if parts.is_empty() {
        return Err("Invalid application command".to_string());
    }

    let program = parts.remove(0);

    Command::new(program)
        .args(parts)
        .spawn()
        .map_err(|e| format!("Failed to launch {}: {}", app.name, e))?;

    Ok(())
}

// ---------------------------------------------------------
// Resolve an application icon to (mime, bytes)
// ---------------------------------------------------------

fn icon_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/icons"));
        dirs.push(home.join(".icons"));
    }

    match std::env::var_os("XDG_DATA_DIRS") {
        Some(data_dirs) => {
            for dir in std::env::split_paths(&data_dirs) {
                dirs.push(dir.join("icons"));
            }
        }
        None => {
            dirs.push(PathBuf::from("/usr/local/share/icons"));
            dirs.push(PathBuf::from("/usr/share/icons"));
        }
    }

    dirs.push(PathBuf::from("/usr/share/pixmaps"));

    dirs
}

const ICON_THEMES: &[&str] = &[
    "hicolor", "Adwaita", "gnome", "Papirus", "breeze", "Yaru",
];

const ICON_SIZES: &[&str] = &[
    "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
    "scalable",
];

const ICON_EXTS: &[&str] = &["png", "svg", "xpm"];

fn mime_for_icon(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("xpm") => "image/x-xpixmap",
        _ => "image/png",
    }
}

fn resolve_icon(name: &str) -> Option<PathBuf> {
    let name = name.trim();

    if name.is_empty() {
        return None;
    }

    // Absolute path straight to a file.
    let direct = Path::new(name);
    if direct.is_absolute() && direct.is_file() {
        return Some(direct.to_path_buf());
    }

    // A name that already carries its extension, or a flat pixmap.
    for dir in icon_search_dirs() {
        let exact = dir.join(name);

        if exact.is_file() {
            return Some(exact);
        }

        // Desktop entries usually omit the extension, so also try the name
        // with each known extension in the flat directories (pixmaps etc.).
        for ext in ICON_EXTS {
            let candidate = dir.join(format!("{name}.{ext}"));

            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // Themed icon: <icons>/<theme>/<size>/apps/<name>.<ext>
    for dir in icon_search_dirs() {
        for theme in ICON_THEMES {
            for size in ICON_SIZES {
                for ext in ICON_EXTS {
                    let candidate = dir
                        .join(theme)
                        .join(size)
                        .join("apps")
                        .join(format!("{name}.{ext}"));

                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    None
}

#[tauri::command]
pub fn launcher_icon(icon: String) -> Result<(String, Vec<u8>), String> {
    let path = resolve_icon(&icon)
        .ok_or_else(|| format!("icon not found: {icon}"))?;

    let bytes = fs::read(&path)
        .map_err(|err| format!("failed to read icon: {err}"))?;

    Ok((mime_for_icon(&path).to_string(), bytes))
}

// ---------------------------------------------------------
// Remove desktop Exec field codes
// ---------------------------------------------------------

fn clean_exec(exec: &str) -> String {
    exec.replace("%U", "")
        .replace("%u", "")
        .replace("%F", "")
        .replace("%f", "")
        .replace("%D", "")
        .replace("%d", "")
        .replace("%N", "")
        .replace("%n", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------
// Small shell-like argument parser
// ---------------------------------------------------------

fn shell_split(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();

    let mut single_quote = false;
    let mut double_quote = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !single_quote => {
                escaped = true;
            }

            '\'' if !double_quote => {
                single_quote = !single_quote;
            }

            '"' if !single_quote => {
                double_quote = !double_quote;
            }

            ' ' | '\t' if !single_quote && !double_quote => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }

            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

// ---------------------------------------------------------
// Get all applications
// ---------------------------------------------------------

#[tauri::command]
pub fn launcher_apps() -> Result<Vec<AppInfo>, String> {
    Ok(scan_applications())
}

#[cfg(test)]
mod tests {
    use super::resolve_icon;
    use std::path::Path;

    #[test]
    fn empty_or_unknown_icon_is_none() {
        assert!(resolve_icon("").is_none());
        assert!(resolve_icon("   ").is_none());
        assert!(resolve_icon("definitely-not-a-real-icon-abcxyz").is_none());
    }

    #[test]
    fn resolves_icon_by_absolute_path() {
        let path = std::env::temp_dir().join("aether-test-icon.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();

        let resolved = resolve_icon(path.to_str().unwrap());

        assert_eq!(resolved.as_deref(), Some(path.as_path()));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolves_flat_and_themed_system_icons() {
        // Flat pixmap, desktop entry omits the extension: Icon=vscode.
        if Path::new("/usr/share/pixmaps/vscode.png").is_file() {
            assert!(
                resolve_icon("vscode").is_some(),
                "flat pixmap lookup failed"
            );
        }

        // Themed hicolor icon: <icons>/hicolor/<size>/apps/<name>.png
        if Path::new("/usr/share/icons/hicolor/256x256/apps/brave-browser.png").is_file()
        {
            assert!(
                resolve_icon("brave-browser").is_some(),
                "themed icon lookup failed"
            );
        }
    }
}