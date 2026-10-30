use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aether::{
    Align, Center, Color, Column, Cursor, Horizontal, Image, InputArea, Layer, LayerWindow, Margin,
    Parent, Radius, Rectangle, Row, Start, Text, Vertical, Weight, Widget, Zone, children,
};

use aether_services::notifications::Notification;

use crate::panel::svg::recolor::ink_image;
use crate::services::notifications::{
    VisiblePopup, dismiss, set_card_hovered, set_close_hovered, set_hovered, visible,
};

// the popup card size and the space between two of them
fn premium_text(value: impl Into<String>) -> Text {
    Text::new(value).font("Adwaita Sans")
}

const CARD_WIDTH: f32 = 372.0;
const CARD_MIN_HEIGHT: f32 = 94.0;
const CARD_GAP: f32 = 8.0;
const CARD_RADIUS: f32 = 16.0;
const CARD_PADDING: f32 = 13.0;

// the distance kept from the screen edges (positioned right under the top panel)
const EDGE_INSET_RIGHT: f32 = 0.0;
const EDGE_INSET_TOP: f32 = 36.0;

// a surface cannot be zero high, so an empty stack still asks for a sliver
const EMPTY_HEIGHT: f32 = 1.0;

// the avatar in front of the text, and the close button beside it
const AVATAR_SIZE: f32 = 42.0;
const CLOSE_SIZE: f32 = 28.0;

// the space between the avatar, the text and the close button
const COLUMN_GAP: f32 = 12.0;

const CARD_COLOR: Color = Color::rgba(24, 25, 31, 200);
const CARD_HOVER_COLOR: Color = Color::rgba(41, 43, 53, 200);
const CARD_BORDER: Color = Color::rgba(255, 255, 255, 26);
const AVATAR_COLOR: Color = Color::rgba(255, 255, 255, 18);
const TEXT_COLOR: Color = Color::rgb(235, 237, 245);
const MUTED_TEXT_COLOR: Color = Color::rgba(235, 237, 245, 158);
const FAINT_TEXT_COLOR: Color = Color::rgba(235, 237, 245, 97);
const CLOSE_HOVER_COLOR: Color = Color::rgba(41, 43, 53, 230);

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
        .height(stack_height(&popups).max(EMPTY_HEIGHT))
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
    let mut y = 0.0;
    let regions = popups
        .iter()
        .filter_map(|popup| {
            let region = card_region(popup, y);
            y += card_height(&popup.notification) + CARD_GAP;
            region
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
fn stack_height(popups: &[VisiblePopup]) -> f32 {
    if popups.is_empty() {
        return 0.0;
    }

    popups
        .iter()
        .map(|popup| card_height(&popup.notification))
        .sum::<f32>()
        + (popups.len() - 1) as f32 * CARD_GAP
}

fn card_height(notification: &Notification) -> f32 {
    let body = strip_markup(&notification.body);
    let body_lines = wrapped_line_count(&body).clamp(1, 4) as f32;
    (CARD_MIN_HEIGHT + (body_lines - 2.0).max(0.0) * 14.0).max(CARD_MIN_HEIGHT)
}

fn wrapped_line_count(body: &str) -> usize {
    const CHARS_PER_LINE: usize = 42;

    body.lines()
        .map(|line| line.chars().count().div_ceil(CHARS_PER_LINE).max(1))
        .sum::<usize>()
        .max(1)
}

// the spot a card covers once it has slid in part of the way, or none once it is fully out
fn card_region(popup: &VisiblePopup, y: f32) -> Option<InputArea> {
    // at rest the card covers the window; each step right hides that much of it
    let left = popup.offset * CARD_WIDTH;

    if left >= CARD_WIDTH {
        return None;
    }

    Some(InputArea {
        x: left as i32,
        y: y as i32,
        width: (CARD_WIDTH - left) as i32,
        height: card_height(&popup.notification) as i32,
    })
}

// the cards in one column, oldest at the top, each pushed right by its own slide
fn stack(popups: Vec<VisiblePopup>) -> Column {
    let mut y = 0.0;
    let cards: Vec<Box<dyn Widget>> = popups
        .into_iter()
        .map(|popup| {
            let height = card_height(&popup.notification);
            let card = Box::new(card(&popup, height)) as Box<dyn Widget>;
            y += height + CARD_GAP;
            card
        })
        .collect();

    Column::new(cards).width(Parent).gap(CARD_GAP)
}

/*
 * One floating card: the avatar of the sender, what it says and a button to
 * send it away. The card brightens while the pointer is over it.
 */
fn card(popup: &VisiblePopup, height: f32) -> Rectangle {
    let visual = popup.offset.clamp(0.0, 1.0);
    let opacity = card_opacity(visual, popup.leaving);
    let scale = card_scale(visual);
    let slide_x = visual * CARD_WIDTH;
    let id = popup.id;

    // hovering any card holds the whole stack open; clicking one dismisses it
    Rectangle::new()
        .width(CARD_WIDTH)
        .height(height)
        .fill(mix_color(CARD_COLOR, CARD_HOVER_COLOR, popup.hover))
        .border(1.0, CARD_BORDER)
        .radius(CARD_RADIUS)
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
                text_column(&popup.notification, height),
                close_button(id, popup.close_hover),
            ])
            .width(Parent)
            .height(Parent)
            .gap(COLUMN_GAP)
            .align(Align::Center),
        )
}

fn mix_color(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    let blend = |start: u8, end: u8| {
        (f32::from(start) + (f32::from(end) - f32::from(start)) * amount).round() as u8
    };

    Color::rgba(
        blend(from.red(), to.red()),
        blend(from.green(), to.green()),
        blend(from.blue(), to.blue()),
        blend(from.alpha(), to.alpha()),
    )
}

// path to the bundled SVG notification icon
fn fallback_icon_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/notification/bell.svg")
}

pub(crate) fn icon_path(notification: &Notification) -> PathBuf {
    if let Some(path) = resolve_icon(&notification.app_icon) {
        return path;
    }

    let category = format!(
        "{} {} {}",
        notification.category, notification.app_name, notification.summary
    )
    .to_ascii_lowercase();
    let icon = if contains_any(&category, &["network", "connectivity", "wifi", "ethernet"]) {
        "network"
    } else if contains_any(&category, &["battery", "power", "charging"]) {
        "battery"
    } else if contains_any(
        &category,
        &[
            "email",
            "mail",
            "message",
            "im.received",
            "im.sent",
            "presence",
            "chat",
        ],
    ) {
        "message"
    } else if contains_any(&category, &["transfer", "download", "upload", "file"]) {
        "download"
    } else if contains_any(&category, &["calendar", "reminder", "appointment"]) {
        "calendar"
    } else if contains_any(&category, &["media", "audio", "music", "playback"]) {
        "media"
    } else if contains_any(
        &category,
        &[
            "warning", "error", "security", "system", "device", "hardware",
        ],
    ) {
        "warning"
    } else {
        return fallback_icon_path();
    };

    let path = match icon {
        "network" => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/wifi/network-wireless-signal-good-symbolic.svg"),
        "battery" => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/battery/aether-applet-battery-level-65-symbolic.svg"),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../assets/icons/notification/notification-{icon}.svg"
        )),
    };
    if path.is_file() {
        path
    } else {
        fallback_icon_path()
    }
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
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
    let path = icon_path(notification);
    // a picture fills the whole box, an icon sits in it
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        Rectangle::new()
            .width(AVATAR_SIZE)
            .height(AVATAR_SIZE)
            .radius(Radius::Fixed(12.0))
            .fill(Image::cover(&path))
            .clip();
    }

    return Rectangle::new()
        .width(AVATAR_SIZE)
        .height(AVATAR_SIZE)
        .radius(Radius::Fixed(12.0))
        .fill(AVATAR_COLOR)
        .padding(7.0)
        .align_child(Center, Center)
        .child(
            Rectangle::new()
                .width(26.0)
                .height(26.0)
                .fill(ink_image(path, TEXT_COLOR)),
        );
}

// the square button that sends the notification away
fn close_button(id: u32, hover: f32) -> Rectangle {
    Rectangle::new()
        .width(CLOSE_SIZE)
        .height(CLOSE_SIZE)
        .radius(Radius::Full)
        .fill(Color::rgba(
            CLOSE_HOVER_COLOR.red(),
            CLOSE_HOVER_COLOR.green(),
            CLOSE_HOVER_COLOR.blue(),
            (f32::from(CLOSE_HOVER_COLOR.alpha()) * hover.clamp(0.0, 1.0)).round() as u8,
        ))
        .cursor(Cursor::Pointer)
        .align_child(Center, Center)
        .on_hover(move |inside| set_close_hovered(id, inside))
        .on_click(move |_| dismiss(id))
        .child(premium_text("✕").color(FAINT_TEXT_COLOR).size(11.0).tight())
}

// the name of the sender and how long ago it spoke, the headline and the text
fn text_column(notification: &Notification, card_height: f32) -> Column {
    let name = if notification.app_name.trim().is_empty() {
        "Notification"
    } else {
        notification.app_name.trim()
    };

    let head = Row::new(children![
        premium_text(name.to_uppercase())
            .color(FAINT_TEXT_COLOR)
            .size(10.0)
            .weight(Weight::SemiBold)
            .elide(),
        // the time sits at the far end of the row
        Rectangle::new()
            .width(Parent)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        premium_text(ago(notification.created_at))
            .color(FAINT_TEXT_COLOR)
            .size(10.5),
    ])
    .width(Parent)
    .align(Align::Center);

    let summary = premium_text(strip_markup(&notification.summary))
        .color(TEXT_COLOR)
        .size(12.5)
        .weight(Weight::Bold)
        .elide();

    let body_height = (card_height - 66.0).max(28.0);
    let body = premium_text(strip_markup(&notification.body))
        .color(MUTED_TEXT_COLOR)
        .size(11.5)
        .wrap()
        .max_lines((body_height / 14.0).floor() as usize);

    let lines: Vec<Box<dyn Widget>> = vec![
        Box::new(Rectangle::new().width(Parent).height(13.0).child(head)),
        Box::new(Rectangle::new().width(Parent).height(17.0).child(summary)),
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(body_height)
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

    fn notification(body: &str) -> Notification {
        Notification::new(
            1,
            "App".into(),
            String::new(),
            String::new(),
            "Summary".into(),
            body.into(),
            Vec::new(),
            5000,
        )
    }

    fn popup(body: &str, offset: f32) -> VisiblePopup {
        VisiblePopup {
            id: 1,
            notification: notification(body),
            offset,
            leaving: false,
            hover: 0.0,
            close_hover: 0.0,
        }
    }

    #[test]
    fn a_stack_uses_each_card_content_height() {
        let short = popup("short", 0.0);
        let long = popup(&"long ".repeat(100), 0.0);
        let expected =
            card_height(&short.notification) + card_height(&long.notification) + CARD_GAP;

        assert_eq!(stack_height(&[]), 0.0);
        assert!(card_height(&long.notification) > CARD_MIN_HEIGHT);
        assert_eq!(stack_height(&[short, long]), expected);
    }

    #[test]
    fn notification_body_height_is_capped() {
        assert_eq!(card_height(&notification("")), CARD_MIN_HEIGHT);
        assert!(card_height(&notification(&"line ".repeat(100))) <= CARD_MIN_HEIGHT + 28.0);
    }

    #[test]
    fn a_card_at_rest_covers_the_whole_width() {
        let popup = popup("short", 0.0);
        let region = card_region(&popup, 0.0).expect("a resting card takes input");

        assert_eq!(region.x, 0);
        assert_eq!(region.width, CARD_WIDTH as i32);
        assert_eq!(region.height, card_height(&popup.notification) as i32);
    }

    #[test]
    fn a_sliding_card_shrinks_and_a_gone_card_takes_nothing() {
        let half_popup = popup("short", 0.5);
        let half = card_region(&half_popup, 0.0).expect("a half shown card still takes input");
        assert_eq!(half.x, (CARD_WIDTH / 2.0) as i32);
        assert_eq!(half.width, (CARD_WIDTH / 2.0) as i32);

        let gone = popup("short", 1.0);
        assert!(card_region(&gone, 0.0).is_none());
    }

    #[test]
    fn each_card_in_a_stack_sits_below_the_one_before() {
        let first_popup = popup("short", 0.0);
        let second_popup = popup("short", 0.0);
        let first = card_region(&first_popup, 0.0).unwrap();
        let second_y = card_height(&first_popup.notification) + CARD_GAP;
        let second = card_region(&second_popup, second_y).unwrap();

        assert_eq!(second.y, first.y + second_y as i32);
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
    fn categories_select_topic_icons() {
        let topics = [
            (
                "network.disconnected",
                "network-wireless-signal-good-symbolic.svg",
            ),
            ("battery.low", "aether-applet-battery-level-65-symbolic.svg"),
            ("im.received", "notification-message.svg"),
            ("transfer.complete", "notification-download.svg"),
            ("calendar.reminder", "notification-calendar.svg"),
            ("media.player", "notification-media.svg"),
            ("security.warning", "notification-warning.svg"),
        ];

        for (category, icon) in topics {
            let notification = Notification::new(
                1,
                String::new(),
                String::new(),
                category.to_owned(),
                String::new(),
                String::new(),
                Vec::new(),
                0,
            );
            assert_eq!(
                icon_path(&notification).file_name().unwrap(),
                icon,
                "wrong icon for category {category}"
            );
        }
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
