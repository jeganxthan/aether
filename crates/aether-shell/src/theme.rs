use std::path::PathBuf;
use std::sync::{OnceLock, PoisonError, RwLock};

use aether::Color;

/*
 * Every color the shell draws comes from one place, so a whole look can be
 * swapped without touching a view. A theme is chosen once at startup from a
 * small config file, and the AETHER_THEME environment variable wins over it.
 * The chosen theme lives in a global the views read while they build, and can
 * be switched again at run time with set_mode, which is what redraws the shell
 * in the other look.
 */

// the two looks the shell ships with
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    // dark ink on a light pane
    Light,

    // light ink on a dark pane, the shell's default
    Dark,
}

impl Mode {
    // the name written in the config file and the environment
    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    // reads a mode name, or none when it is not one of the two
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    // the other one, for a toggle
    pub fn flipped(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        // the shell has always been dark, so that stays the fallback
        Self::Dark
    }
}

/*
 * The colors one look needs. Views never reach for a literal color; they ask
 * the theme, so both looks cover every surface the shell draws.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // the top bar and the island
    pub bar: Color,
    pub bar_text: Color,

    // a floating card, the notifications and the on screen display
    pub surface: Color,
    pub surface_border: Color,
    pub shadow: Color,

    // the same card with the pointer over it, a little brighter
    pub surface_hi: Color,

    // text on a card
    pub text: Color,
    pub text_muted: Color,
    pub text_faint: Color,

    // small chips like the app icon badge and the close button
    pub chip: Color,
    pub chip_border: Color,

    // the on screen display's progress bar
    pub track: Color,
    pub accent: Color,

    // the faint wash of the accent, for a chip or a selected row
    pub accent_faint: Color,

    // the color the line art icons are drawn in, so they read on the surface
    pub icon: Color,
}

impl Theme {
    // light ink on a dark pane, the look the shell has always had
    pub const DARK: Self = Self {
        bar: Color::rgb(0, 0, 0),
        bar_text: Color::WHITE,

        surface: Color::rgba(22, 23, 29, 246),
        surface_border: Color::rgba(255, 255, 255, 22),
        shadow: Color::BLACK,

        surface_hi: Color::rgba(41, 43, 53, 246),

        text: Color::WHITE,
        text_muted: Color::rgba(255, 255, 255, 170),
        text_faint: Color::rgba(255, 255, 255, 110),

        chip: Color::rgba(255, 255, 255, 14),
        chip_border: Color::rgba(255, 255, 255, 18),

        track: Color::rgba(255, 255, 255, 28),
        accent: Color::rgba(140, 180, 255, 235),
        accent_faint: Color::rgba(140, 180, 255, 34),

        icon: Color::WHITE,
    };

    // dark ink on a light pane
    pub const LIGHT: Self = Self {
        bar: Color::rgb(248, 249, 251),
        bar_text: Color::rgb(28, 30, 36),

        surface: Color::rgba(252, 252, 254, 246),
        surface_border: Color::rgba(0, 0, 0, 24),
        shadow: Color::rgb(40, 44, 52),

        surface_hi: Color::rgba(236, 238, 244, 246),

        text: Color::rgb(24, 26, 32),
        text_muted: Color::rgba(24, 26, 32, 170),
        text_faint: Color::rgba(24, 26, 32, 120),

        chip: Color::rgba(0, 0, 0, 14),
        chip_border: Color::rgba(0, 0, 0, 22),

        track: Color::rgba(0, 0, 0, 28),
        accent: Color::rgba(64, 120, 240, 235),
        accent_faint: Color::rgba(64, 120, 240, 30),

        icon: Color::rgb(32, 34, 40),
    };

    // the set of colors that pairs with a mode
    pub fn of(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self::LIGHT,
            Mode::Dark => Self::DARK,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::of(Mode::default())
    }
}

// the chosen look, read by every view while it builds
static CURRENT: OnceLock<RwLock<Theme>> = OnceLock::new();

fn current() -> &'static RwLock<Theme> {
    CURRENT.get_or_init(|| RwLock::new(load()))
}

// the theme the views draw with right now
pub fn theme() -> Theme {
    *current().read().unwrap_or_else(PoisonError::into_inner)
}

/*
 * Blends two colors, t from 0 (the first color) to 1 (the second one). A
 * view uses it to fade a surface towards another one, so a card can brighten
 * under the pointer without a color of its own.
 */
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);

    let lerp = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;

    Color::rgba(
        lerp(a.red(), b.red()),
        lerp(a.green(), b.green()),
        lerp(a.blue(), b.blue()),
        lerp(a.alpha(), b.alpha()),
    )
}

// the mode behind the current theme
pub fn mode() -> Mode {
    let theme = theme();

    if theme == Theme::LIGHT {
        Mode::Light
    } else {
        Mode::Dark
    }
}

// switches the look and wakes the shell so every window redraws in it
pub fn set_mode(mode: Mode) -> bool {
    let mut slot = current().write().unwrap_or_else(PoisonError::into_inner);

    let next = Theme::of(mode);

    if *slot == next {
        return false;
    }

    *slot = next;

    drop(slot);

    // the views read the theme while they build, so a change is a redraw everywhere
    aether::refresh();

    true
}

// toggles between the two looks, for a keybind
pub fn toggle() -> Mode {
    let next = mode().flipped();

    set_mode(next);

    next
}

/*
 * Picks the mode to start in. The environment wins so a launcher or a test can
 * force one, then the config file, and dark when neither says anything usable.
 */
fn load() -> Theme {
    let mode = std::env::var("AETHER_THEME")
        .ok()
        .and_then(|name| Mode::parse(&name))
        .or_else(|| config_mode())
        .unwrap_or_default();

    Theme::of(mode)
}

// the mode written in the config file, or none when there is no usable file
fn config_mode() -> Option<Mode> {
    let path = config_path()?;

    let text = std::fs::read_to_string(path).ok()?;

    parse_config(&text)
}

/*
 * The config file is a simple list of `key = value` lines. Only `theme` is read
 * today, and a line that is blank, a comment, or a key the shell does not know
 * is skipped so an unrelated setting never breaks the look.
 */
pub fn parse_config(text: &str) -> Option<Mode> {
    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim().eq_ignore_ascii_case("theme") {
            return Mode::parse(value);
        }
    }

    None
}

// where the shell looks for its config, following the XDG layout
pub fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(base.join("aether").join("theme.conf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mode_reads_from_its_name() {
        assert_eq!(Mode::parse("light"), Some(Mode::Light));
        assert_eq!(Mode::parse("DARK"), Some(Mode::Dark));
        assert_eq!(Mode::parse("  dark  "), Some(Mode::Dark));

        assert_eq!(Mode::parse("blue"), None);
        assert_eq!(Mode::parse(""), None);
    }

    #[test]
    fn a_mode_flips_to_the_other() {
        assert_eq!(Mode::Light.flipped(), Mode::Dark);
        assert_eq!(Mode::Dark.flipped(), Mode::Light);
        assert_eq!(Mode::Dark.flipped().flipped(), Mode::Dark);
    }

    #[test]
    fn dark_is_the_default() {
        assert_eq!(Mode::default(), Mode::Dark);
        assert_eq!(Theme::default(), Theme::DARK);
    }

    #[test]
    fn the_two_themes_are_different_surfaces() {
        assert_ne!(Theme::LIGHT.surface, Theme::DARK.surface);
        assert_ne!(Theme::LIGHT.icon, Theme::DARK.icon);
        assert_eq!(Theme::of(Mode::Light), Theme::LIGHT);
        assert_eq!(Theme::of(Mode::Dark), Theme::DARK);
    }

    #[test]
    fn the_config_reads_its_theme_line() {
        assert_eq!(parse_config("theme = light"), Some(Mode::Light));
        assert_eq!(parse_config("theme=dark"), Some(Mode::Dark));
        assert_eq!(parse_config("THEME = Light"), Some(Mode::Light));

        // comments, blanks and unknown keys are skipped
        let text = "# a comment\n\nfont = Mono\n; another\ntheme = light\n";
        assert_eq!(parse_config(text), Some(Mode::Light));
    }

    #[test]
    fn a_config_without_a_theme_line_says_nothing() {
        assert_eq!(parse_config(""), None);
        assert_eq!(parse_config("font = Mono"), None);
        assert_eq!(parse_config("theme = blue"), None);
    }

    #[test]
    fn the_config_lands_under_the_xdg_dir() {
        let path = config_path().expect("a home is set in the tests");

        assert!(path.ends_with("aether/theme.conf"), "{}", path.display());
    }
}
