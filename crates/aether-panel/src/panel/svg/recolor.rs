use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use aether::{Color, Image};

/*
 * The bundled icons are drawn in the color a dark shell needs: the lucide ones
 * with stroke="currentColor", which the rasterizer reads as black, and the
 * symbolic ones with white fills. Neither reads on a light surface, so each
 * icon is copied once with that ink written out in the color the theme asks
 * for, into a cache folder, and the copy is what gets loaded. The originals are
 * left untouched, and the copies are keyed by color so the two themes can live
 * side by side.
 */

// the folder the recolored copies live in for the length of the run
static CACHE: OnceLock<PathBuf> = OnceLock::new();

// the placeholder and the two whites the symbolic icons are drawn in
const STROKE: &str = "currentColor";
const WHITE_HEX: &str = "#FFFFFF";
const WHITE_LOWER_HEX: &str = "#ffffff";
const WHITE_WORD: &str = "white";

// the folder the recolored copies live in for the length of the run
fn cache_dir() -> &'static Path {
    CACHE.get_or_init(|| {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        let dir = base.join("aether-panel").join("icons");

        let _ = std::fs::create_dir_all(&dir);

        dir
    })
}

// the hex an icon is copied with, from a theme's ink color
fn ink_hex(ink: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", ink.red(), ink.green(), ink.blue())
}

/*
 * A copy of the icon at `source`, drawn in `ink`, written once. A source that
 * cannot be read, or an SVG with no ink to replace, falls back to the original
 * path so the icon still shows (in its own color) instead of vanishing.
 */
pub fn ink(source: &Path, ink: Color) -> PathBuf {
    let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
        return source.to_path_buf();
    };

    let hex = ink_hex(ink);

    // the color goes in the name so the two themes each get their own copy
    let target = cache_dir().join(format!("{}.{}.svg", ink_stem(name), hex.trim_start_matches('#')));

    if target.exists() {
        return target;
    }

    let Ok(bytes) = std::fs::read(source) else {
        return source.to_path_buf();
    };

    let svg = String::from_utf8_lossy(&bytes);

    let recolored = recolor(&svg, &hex);

    // nothing to recolor, so the original is already as good as it gets
    let Some(recolored) = recolored else {
        return source.to_path_buf();
    };

    // a copy that cannot be written still shows the original
    if std::fs::write(&target, recolored).is_err() {
        return source.to_path_buf();
    }

    target
}

// the recolored icon, ready to hand to a view
pub fn ink_image(source: impl AsRef<Path>, color: Color) -> Image {
    
    Image::contain(ink(source.as_ref(), color))
    
}

/*
 * Swaps the shell ink for `hex`. Both the currentColor placeholder and the
 * white the symbolic icons use move to the theme color, while a real color like
 * the battery's green charging or red low stays put, so a themed icon keeps its
 * meaning. Returns none when there was no ink to replace.
 */
fn recolor(svg: &str, hex: &str) -> Option<String> {
    let mut found = false;

    let mut out = svg.to_owned();

    for from in [STROKE, WHITE_HEX, WHITE_LOWER_HEX, WHITE_WORD] {
        if out.contains(from) {
            found = true;

            out = out.replace(from, hex);
        }
    }

    found.then_some(out)
}

// the file name without its extension, so the color can be appended
fn ink_stem(name: &str) -> &str {
    name.strip_suffix(".svg").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume_icon() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/osd/volume_full.svg")
    }

    fn wifi_icon() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/wifi/network-wireless-secure-signal-excellent-symbolic.svg")
    }

    #[test]
    fn the_copy_swaps_the_stroke_for_the_ink() {
        let copy = ink(&volume_icon(), Color::WHITE);

        let text = std::fs::read_to_string(&copy).expect("the copy should be readable");

        assert!(text.contains("#ffffff"), "the copy is not white: {copy:?}");
        assert!(!text.contains(STROKE), "the copy still has currentColor");
    }

    #[test]
    fn the_two_themes_get_their_own_copy() {
        let light = ink(&volume_icon(), Color::rgb(32, 34, 40));
        let dark = ink(&volume_icon(), Color::WHITE);

        assert_ne!(light, dark, "the copies share one path");

        // both are ready to load, side by side
        assert!(light.exists() && dark.exists());
    }

    #[test]
    fn a_white_symbolic_icon_takes_the_ink_too() {
        let copy = ink(&wifi_icon(), Color::rgb(32, 34, 40));

        let text = std::fs::read_to_string(&copy).expect("the copy should be readable");

        assert!(text.contains("#202228"), "the white fill was not recolored");
        assert!(!text.contains(WHITE_WORD), "a white fill survived");
    }

    #[test]
    fn a_missing_file_falls_back_to_its_own_path() {
        let missing = PathBuf::from("/nonexistent/icon.svg");

        assert_eq!(ink(&missing, Color::WHITE), missing);
    }

    #[test]
    fn a_file_without_ink_is_left_alone() {
        // kept outside the cache folder so the copy branch is the one under test
        let dir = std::env::temp_dir().join("aether-panel-recolor-test");
        std::fs::create_dir_all(&dir).unwrap();

        let plain = dir.join("plain.svg");
        std::fs::write(&plain, "<svg fill=\"#000\"/>").unwrap();

        assert_eq!(ink(&plain, Color::WHITE), plain);

        let _ = std::fs::remove_file(&plain);
    }

    #[test]
    fn a_real_color_is_kept() {
        // the battery green must survive so charging still reads
        let svg = "<svg fill=\"#30D158\" stroke=\"currentColor\"/>";

        let out = recolor(svg, "#202228").expect("there is ink to replace");

        assert!(out.contains("#30D158"), "the green was lost");
        assert!(out.contains("#202228"), "the ink was not applied");
        assert!(!out.contains(STROKE));
    }

    #[test]
    fn the_hex_is_built_from_the_color() {
        assert_eq!(ink_hex(Color::WHITE), "#ffffff");
        assert_eq!(ink_hex(Color::rgb(32, 34, 40)), "#202228");
    }

    #[test]
    fn a_stem_drops_its_extension() {
        assert_eq!(ink_stem("sun.svg"), "sun");
        assert_eq!(ink_stem("plain"), "plain");
    }
}
