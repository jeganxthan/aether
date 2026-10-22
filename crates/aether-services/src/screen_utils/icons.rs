use std::path::PathBuf;

/*
 * The icons the shell ships with. The installer copies them next to the other
 * data, so a tool that lives outside the source tree can still point at one.
 */

// the folder the icons are copied into
fn icon_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("aether/icons");
    }

    let home = std::env::var_os("HOME").unwrap_or_default();

    PathBuf::from(home).join(".local/share/aether/icons")
}

/*
 * The path of one of the shipped icons, or none when it is not installed.
 * An empty path is what the notification server reads as "no icon".
 */
pub fn icon(name: &str) -> String {
    let path = icon_dir().join(format!("{name}.svg"));

    if path.exists() {
        return path.to_string_lossy().into_owned();
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_icon_is_an_empty_path() {
        assert_eq!(icon("this-icon-is-not-installed"), "");
    }
}
