use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aether::{
    Align, Center, Color, Column, Cursor, Horizontal, Image, InputArea, Layer, LayerWindow, Margin,
    Parent, Radius, Rectangle, Row, Shadow, Start, Text, Vertical, Weight, Widget, Zone, children,
};

use aether_services::notifications::Notification;

use crate::panel::svg::recolor::ink_image;
use crate::services::notifications::{
    MAX_VISIBLE, VisiblePopup, dismiss, set_card_hovered, set_close_hovered, set_hovered, visible,
};
use crate::theme::{mix, theme};

// the popup card size and the space between two of them
fn premium_text(value: impl Into<String>) -> Text {
    Text::new(value).font("Cantarell")
}

const CARD_WIDTH: f32 = 372.0;
const CARD_HEIGHT: f32 = 84.0;
const CARD_GAP: f32 = 10.0;
const CARD_RADIUS: f32 = 14.0;
const CARD_PADDING: f32 = 13.0;

// the distance kept from the screen edges (positioned right under the top panel)
const EDGE_INSET_RIGHT: f32 = 14.0;
const EDGE_INSET_TOP: f32 = 36.0;

// a surface cannot be zero high, so an empty stack still asks for a sliver
const EMPTY_HEIGHT: f32 = 1.0;

// the avatar in front of the text, and the close button beside it
const AVATAR_SIZE: f32 = 40.0;
const CLOSE_SIZE: f32 = 26.0;

// the space between the avatar, the text and the close button
const COLUMN_GAP: f32 = 12.0;

/*
 * The popups glide in smoothly from the right edge with a cubic ease-out curve,
 * opacity fade, and micro-scale pop. Dismissed popups accelerate off to the right.
 * The window stays one card wide and anchored to the right, so cards sliding out
 * are naturally clipped at the surface boundary.
 */
pub fn notifications() -> LayerWindow {
    let popups = visible();

    let window = LayerWindow::new()
        .width(CARD_WIDTH)
        .height(max_stack_height())
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-notification")
        .margin(Margin {
            top: EDGE_INSET_TOP as i32,
            right: EDGE_INSET_RIGHT as i32,
            bottom: 0,
            left: 0,
        });

    // nothing on screen means nothing to draw or click, so the window is click through
    if popups.is_empty() {
        return window.click_through().child(blank());
    }

    // only the cards themselves take the pointer, the gaps stay click through
    let regions = popups
        .iter()
        .enumerate()
        .filter_map(|(index, popup)| {
            let visual = ease_slide(popup.offset, popup.leaving);
            card_region(index, visual)
        })
        .collect();

    window.input_region(regions).child(stack(popups))
}

// an empty, transparent child, used while the window has nothing to show
fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}

// the height the window always asks for, matching the tallest stack it can show
fn max_stack_height() -> f32 {
    stack_height(MAX_VISIBLE).max(EMPTY_HEIGHT)
}

// the full height the window needs for a stack of this many cards
fn stack_height(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }

    count as f32 * CARD_HEIGHT + (count - 1) as f32 * CARD_GAP
}

// the spot a card covers once it has slid in part of the way, or none once it is fully out
fn card_region(index: usize, offset: f32) -> Option<InputArea> {
    // at rest the card covers the window; each step right hides that much of it
    let left = offset * CARD_WIDTH;

    if left >= CARD_WIDTH {
        return None;
    }

    Some(InputArea {
        x: left as i32,
        y: (index as f32 * (CARD_HEIGHT + CARD_GAP)) as i32,
        width: (CARD_WIDTH - left) as i32,
        height: CARD_HEIGHT as i32,
    })
}

// the cards in one column, oldest at the top, each pushed right by its own slide
fn stack(popups: Vec<VisiblePopup>) -> Column {
    let cards: Vec<Box<dyn Widget>> = popups
        .into_iter()
        .map(|popup| Box::new(card(&popup)) as Box<dyn Widget>)
        .collect();

    Column::new(cards).width(Parent).gap(CARD_GAP)
}

/*
 * One floating card: the avatar of the sender, what it says and a button to
 * send it away. The card brightens while the pointer is over it.
 */
fn card(popup: &VisiblePopup) -> Rectangle {
    let visual = ease_slide(popup.offset, popup.leaving);
    let opacity = card_opacity(visual, popup.leaving);
    let scale = card_scale(visual);
    let slide_x = visual * CARD_WIDTH;
    let id = popup.id;
    let theme = theme();

    // hovering any card holds the whole stack open; clicking one dismisses it
    Rectangle::new()
        .width(CARD_WIDTH)
        .height(CARD_HEIGHT)
        .fill(mix(theme.surface, theme.surface_hi, popup.hover))
        .border(1.0, theme.surface_border)
        .radius(CARD_RADIUS)
        .shadow(
            Shadow::drop(theme.shadow)
                .opacity(0.38)
                .blur(18.0)
                .offset(0.0, 6.0),
        )
        // smooth physics-based motion: eased slide, opacity fade, and micro-scale pop
        .opacity(opacity)
        .scale(scale)
        .translate(slide_x, 0.0)
        .padding(CARD_PADDING)
        .cursor(Cursor::Pointer)
        .clip()
        .on_hover(move |inside| {
            set_hovered(inside);
            set_card_hovered(id, inside);
        })
        .on_click(move |_| dismiss(id))
        .child(
            Row::new(children![
                avatar(&popup.notification),
                text_column(&popup.notification),
                close_button(id, popup.close_hover),
            ])
            .width(Parent)
            .height(Parent)
            .gap(COLUMN_GAP)
            .align(Align::Center),
        )
}

// path to the bundled SVG notification icon
fn fallback_icon_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/notification/aether-applet-notification-symbolic.svg")
}

// searches for a valid icon from an absolute path or common system icon themes
fn resolve_icon(app_icon: &str) -> Option<PathBuf> {
    if app_icon.is_empty() {
        return None;
    }

    let direct = std::path::Path::new(app_icon);
    if direct.is_absolute() && direct.exists() {
        return Some(direct.to_path_buf());
    }

    // Common system icon search locations for named application icons
    let extensions = ["svg", "png"];
    let search_dirs = [
        "/usr/share/icons/hicolor/scalable/apps",
        "/usr/share/icons/hicolor/48x48/apps",
        "/usr/share/icons/hicolor/64x64/apps",
        "/usr/share/icons/hicolor/128x128/apps",
        "/usr/share/icons/hicolor/256x256/apps",
        "/usr/share/icons/hicolor/symbolic/apps",
        "/usr/share/icons/hicolor/symbolic/status",
        "/usr/share/pixmaps",
    ];

    for dir in &search_dirs {
        for ext in &extensions {
            let candidate = PathBuf::from(dir).join(format!("{app_icon}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/*
 * The picture of the sender: its own icon when it sent one, a small badge
 * with the notification bell when it did not.
 */
fn avatar(notification: &Notification) -> Rectangle {
    if let Some(path) = resolve_icon(&notification.app_icon) {
        // a picture fills the whole box, an icon sits in it
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            return Rectangle::new()
                .width(AVATAR_SIZE)
                .height(AVATAR_SIZE)
                .radius(Radius::Fixed(10.0))
                .fill(Image::cover(path))
                .clip();
        }

        let theme = theme();

        return Rectangle::new()
            .width(AVATAR_SIZE)
            .height(AVATAR_SIZE)
            .radius(Radius::Fixed(10.0))
            .fill(theme.chip)
            .border(1.0, theme.chip_border)
            .padding(7.0)
            .align_child(Center, Center)
            .child(
                Rectangle::new()
                    .width(26.0)
                    .height(26.0)
                    .fill(Image::contain(path)),
            );
    }

    // fallback: a sleek rounded glass badge with the crisp symbolic notification bell
    let fallback = fallback_icon_path();
    let theme = theme();

    Rectangle::new()
        .width(AVATAR_SIZE)
        .height(AVATAR_SIZE)
        .radius(Radius::Fixed(10.0))
        .fill(theme.chip)
        .border(1.0, theme.chip_border)
        .align_child(Center, Center)
        .child(
            Rectangle::new()
                .width(20.0)
                .height(20.0)
                .fill(ink_image(fallback, theme.icon)),
        )
}

// the round button that sends the notification away
fn close_button(id: u32, hover: f32) -> Rectangle {
    let theme = theme();

    Rectangle::new()
        .width(CLOSE_SIZE)
        .height(CLOSE_SIZE)
        .radius(Radius::Fixed(9.0))
        .fill(mix(theme.chip, theme.surface_hi, hover))
        .border(1.0, theme.chip_border)
        .cursor(Cursor::Pointer)
        .align_child(Center, Center)
        .on_hover(move |inside| set_close_hovered(id, inside))
        .on_click(move |_| dismiss(id))
        .child(premium_text("✕").color(theme.text_faint).size(11.0).tight())
}

// the name of the sender and how long ago it spoke, the headline and the text
fn text_column(notification: &Notification) -> Column {
    let theme = theme();

    let name = if notification.app_name.trim().is_empty() {
        "Notification"
    } else {
        notification.app_name.trim()
    };

    let head = Row::new(children![
        premium_text(name.to_uppercase())
            .color(theme.text_faint)
            .size(10.0)
            .weight(Weight::SemiBold)
            .elide(),
        // the time sits at the far end of the row
        Rectangle::new()
            .width(Parent)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        premium_text(ago(notification.created_at))
            .color(theme.text_faint)
            .size(10.5),
    ])
    .width(Parent)
    .align(Align::Center);

    let summary = premium_text(strip_markup(&notification.summary))
        .color(theme.text)
        .size(12.5)
        .weight(Weight::Bold)
        .elide();

    let body = premium_text(strip_markup(&notification.body))
        .color(theme.text_muted)
        .size(11.5)
        .wrap()
        .max_lines(2);

    let lines: Vec<Box<dyn Widget>> = vec![
        Box::new(Rectangle::new().width(Parent).height(13.0).child(head)),
        Box::new(Rectangle::new().width(Parent).height(17.0).child(summary)),
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(28.0)
                .clip()
                .child(body),
        ),
    ];

    Column::new(lines)
        .width(Parent)
        .height(Parent)
        .align(Align::Start)
        .justify(Start)
        .gap(1.0)
}

/*
 * How long ago a notification arrived: "now", "5m", "3h", "2d". The clock is
 * read from the system, so a notification that came in while the shell was
 * busy still shows the right age.
 */
fn ago(created_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());

    let seconds = now.saturating_sub(created_at);

    match seconds {
        0..=59 => String::from("now"),
        60..=3599 => format!("{}m", seconds / 60),
        3600..=86399 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

// notification bodies may hold <b> and <a href> markup and entities like &amp; and &#39;
pub fn strip_markup(text: &str) -> String {
    let mut tagless = String::new();
    let mut inside = false;

    for letter in text.chars() {
        match letter {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => tagless.push(letter),
            _ => {}
        }
    }

    unescape(&tagless)
}

// one pass, so "&amp;lt;" becomes "&lt;" (what the sender meant) and not "<"
fn unescape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find('&') {
        result.push_str(&rest[..start]);

        let after = &rest[start + 1..];

        let decoded = after.find(';').filter(|end| *end <= 8).and_then(|end| {
            let name = &after[..end];

            let letter = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => name
                    .strip_prefix('#')
                    .and_then(|number| match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => number.parse().ok(),
                    })
                    .and_then(char::from_u32),
            }?;

            Some((letter, end))
        });

        match decoded {
            Some((letter, end)) => {
                result.push(letter);

                rest = &after[end + 1..];
            }
            None => {
                result.push('&');

                rest = after;
            }
        }
    }

    result.push_str(rest);

    result
}

/// Eased visual offset from raw linear offset (1.0 = offscreen right, 0.0 = at rest).
/// - Entering: Cubic Ease-Out (rapid entrance, buttery-smooth deceleration to rest).
/// - Leaving: Ease-In (gentle departure, accelerating off-screen).
pub fn ease_slide(offset: f32, leaving: bool) -> f32 {
    let offset = offset.clamp(0.0, 1.0);
    if leaving {
        offset * offset
    } else {
        offset * offset * offset
    }
}

/// Dynamic opacity based on visual offset:
/// - Entering: Fades in rapidly as it slides onto screen.
/// - Leaving: Softly fades out to 0 as it leaves.
pub fn card_opacity(visual_offset: f32, leaving: bool) -> f32 {
    let visual = visual_offset.clamp(0.0, 1.0);
    if leaving {
        (1.0 - visual).clamp(0.0, 1.0)
    } else {
        (1.0 - visual * visual).clamp(0.0, 1.0)
    }
}

/// Subtle micro-scale pop (from 0.95 at edge to 1.0 at rest).
pub fn card_scale(visual_offset: f32) -> f32 {
    let visual = visual_offset.clamp(0.0, 1.0);
    0.95 + 0.05 * (1.0 - visual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stack_grows_by_a_card_and_a_gap() {
        assert_eq!(stack_height(0), 0.0);
        assert_eq!(stack_height(1), CARD_HEIGHT);
        assert_eq!(stack_height(2), CARD_HEIGHT * 2.0 + CARD_GAP);
    }

    #[test]
    fn the_window_always_fits_the_tallest_stack() {
        assert_eq!(max_stack_height(), stack_height(MAX_VISIBLE));
        assert!(max_stack_height() > EMPTY_HEIGHT);
    }

    #[test]
    fn a_card_at_rest_covers_the_whole_width() {
        let region = card_region(0, 0.0).expect("a resting card takes input");

        assert_eq!(region.x, 0);
        assert_eq!(region.width, CARD_WIDTH as i32);
        assert_eq!(region.height, CARD_HEIGHT as i32);
    }

    #[test]
    fn a_sliding_card_shrinks_and_a_gone_card_takes_nothing() {
        let half = card_region(0, 0.5).expect("a half shown card still takes input");
        assert_eq!(half.x, (CARD_WIDTH / 2.0) as i32);
        assert_eq!(half.width, (CARD_WIDTH / 2.0) as i32);

        assert!(card_region(0, 1.0).is_none());
    }

    #[test]
    fn each_card_in_a_stack_sits_below_the_one_before() {
        let first = card_region(0, 0.0).unwrap();
        let second = card_region(1, 0.0).unwrap();

        assert_eq!(second.y, first.y + (CARD_HEIGHT + CARD_GAP) as i32);
    }

    #[test]
    fn ease_slide_entering_decelerates_smoothly() {
        assert_eq!(ease_slide(1.0, false), 1.0);
        assert_eq!(ease_slide(0.0, false), 0.0);
        // at 20% remaining distance, cubic out has traveled over 99% of the way
        assert!(ease_slide(0.2, false) < 0.01);
    }

    #[test]
    fn ease_slide_leaving_accelerates_away() {
        assert_eq!(ease_slide(0.0, true), 0.0);
        assert_eq!(ease_slide(1.0, true), 1.0);
        // gentle departure at the start
        assert!((ease_slide(0.2, true) - 0.04).abs() < 1e-5);
    }

    #[test]
    fn opacity_and_scale_transitions() {
        assert_eq!(card_opacity(0.0, false), 1.0);
        assert_eq!(card_opacity(1.0, false), 0.0);
        assert_eq!(card_opacity(0.0, true), 1.0);
        assert_eq!(card_opacity(1.0, true), 0.0);

        assert_eq!(card_scale(0.0), 1.0);
        assert_eq!(card_scale(1.0), 0.95);
    }

    #[test]
    fn fallback_icon_asset_exists() {
        assert!(
            fallback_icon_path().exists(),
            "fallback icon missing: {}",
            fallback_icon_path().display()
        );
    }

    #[test]
    fn markup_is_taken_out_of_the_text() {
        assert_eq!(strip_markup("Battery at 20%"), "Battery at 20%");
        assert_eq!(
            strip_markup("<b>Bold</b> and <a href=\"https://example.com\">a link</a>"),
            "Bold and a link"
        );
    }

    #[test]
    fn entities_are_decoded_once() {
        assert_eq!(
            strip_markup("Tom &amp; Jerry &lt;3 &quot;quotes&quot; &apos;x&apos;"),
            "Tom & Jerry <3 \"quotes\" 'x'"
        );
        assert_eq!(
            strip_markup("it&#39;s &#x41;&#X42; &#128512;"),
            "it's AB \u{1f600}"
        );
        assert_eq!(strip_markup("&amp;lt;"), "&lt;");
        assert_eq!(strip_markup("&amp;amp;"), "&amp;");
        assert_eq!(strip_markup("a &lt;b&gt; c"), "a <b> c");
    }

    #[test]
    fn stray_ampersands_stay() {
        assert_eq!(strip_markup("R&D"), "R&D");
        assert_eq!(
            strip_markup("&nosuch; &#xZZ; &#99999999999;"),
            "&nosuch; &#xZZ; &#99999999999;"
        );
        assert_eq!(strip_markup("trailing &"), "trailing &");
    }

    #[test]
    fn the_age_of_a_notification_reads_as_a_word() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());

        assert_eq!(ago(now), "now");
        assert_eq!(ago(now - 60), "1m");
        assert_eq!(ago(now - 3600), "1h");
        assert_eq!(ago(now - 86400 * 2), "2d");
    }
}
