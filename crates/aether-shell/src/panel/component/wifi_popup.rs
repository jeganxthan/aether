use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Column, Cursor, Easing, Horizontal, InputArea, Key, Keyboard,
    Layer, LayerWindow, Margin, Padding, Parent, Path, Radius, Rectangle, Row, ScrollArea, Shape,
    Size, SpaceBetween, Stack, Start, Text, TextInput, Vertical, Weight, Widget, Zone, children,
};

use crate::panel::svg::wifi_svg::WifiSignalIcon;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::wifi::{
    WifiNetwork, ask_password, awaiting_password, cancel_password, connect, ethernet_connected,
    networks, pending, radio, search_networks, speed, ssid, toggle_radio, wifi,
};
use crate::theme::theme;

/*
 * The wifi popup has two parts:
 *
 *  - a hint that hangs under the wifi icon while the pointer rests on it, sized
 *    to the connected network's name alone;
 *  - a menu that opens on a click, holding the glass panel, and covering the
 *    whole screen so a click anywhere, or Escape, packs it away again.
 *
 * They are separate windows: the hint is small and only as wide as the name, and
 * the menu spans the screen so it can catch a click that lands away from it.
 */

// the height of both parts, and the padding that frames the name
fn premium_text(value: impl Into<String>) -> Text {
    Text::new(value).font("Maple Mono")
}

#[allow(dead_code)]
const HINT_HEIGHT: f32 = 36.0;
#[allow(dead_code)]
const HINT_PADDING: f32 = 12.0;

// the gap between the name and the speed that trails it
#[allow(dead_code)]
const HINT_GAP: f32 = 8.0;

// what the name is drawn at, so the hint can be sized to it
#[allow(dead_code)]
const HINT_TEXT_SIZE: f32 = 12.0;

// the full panel inside the menu
const BOX_WIDTH: f32 = 280.0;
const BOX_RADIUS: f32 = 14.0;
const BOX_PADDING: f32 = 14.0;

// the gap between the switch row and the network list below it
const BOX_GAP: f32 = 12.0;

// the wifi switch inside the menu
const SWITCH_LABEL_SIZE: f32 = 12.0;
const SWITCH_TRACK_WIDTH: f32 = 40.0;
const SWITCH_TRACK_HEIGHT: f32 = 22.0;
const SWITCH_KNOB: f32 = 16.0;

// the network list below the switch
const LIST_MAX_VISIBLE_ROWS: usize = 6;
const LIST_ROW_HEIGHT: f32 = 34.0;
const LIST_ROW_GAP: f32 = 4.0;
const LIST_ICON_SIZE: f32 = 15.0;
const LIST_ICON_GAP: f32 = 10.0;
const LIST_TEXT_SIZE: f32 = 12.0;

// the soft panel a connected row sits on, and the chip that names its state
const LIST_ROW_PADDING: f32 = 8.0;
const LIST_ROW_RADIUS: f32 = 8.0;
const LIST_CHIP_TEXT_SIZE: f32 = 10.0;
const LIST_CHIP_HEIGHT: f32 = 18.0;
const LIST_CHIP_GAP: f32 = 6.0;

// the password input the menu opens below the list when a network is picked
const INPUT_FIELD_HEIGHT: f32 = 36.0;
const INPUT_TEXT_SIZE: f32 = 12.0;
const INPUT_RADIUS: f32 = 8.0;

// the connect and cancel pair below the field
const BUTTON_HEIGHT: f32 = 34.0;
const BUTTON_TEXT_SIZE: f32 = 12.0;
const BUTTON_RADIUS: f32 = 8.0;
const BUTTON_GAP: f32 = 8.0;

// what the input section adds to the box: the field, a gap, and the buttons
const INPUT_SECTION_HEIGHT: f32 = INPUT_FIELD_HEIGHT + BOX_GAP + BUTTON_HEIGHT;

// the width the two buttons share, which is the box inside its padding
const BUTTON_AREA_WIDTH: f32 = BOX_WIDTH - BOX_PADDING * 2.0;

// the list grows with available networks up to a scrollable six-row limit
const SWITCH_ROW_HEIGHT: f32 = SWITCH_TRACK_HEIGHT;

fn visible_network_rows(network_count: usize) -> usize {
    network_count.clamp(1, LIST_MAX_VISIBLE_ROWS)
}

fn network_list_height(network_count: usize) -> f32 {
    let rows = visible_network_rows(network_count) as f32;

    rows * LIST_ROW_HEIGHT + (rows - 1.0) * LIST_ROW_GAP
}

fn menu_height(network_count: usize) -> f32 {
    BOX_PADDING * 2.0 + SWITCH_ROW_HEIGHT + BOX_GAP + network_list_height(network_count)
}

fn network_section_height(saved: bool) -> f32 {
    if saved {
        BUTTON_HEIGHT
    } else {
        INPUT_SECTION_HEIGHT
    }
}

// the gap between a part and the bottom of the top bar
const EDGE_INSET_TOP: f32 = 30.0;

// the distance from the right screen edge, matched to where the wifi icon sits
const EDGE_INSET_RIGHT: f32 = 14.0;

// true while the pointer is over the wifi icon or the hint it shows
static HOVERED: AtomicBool = AtomicBool::new(false);
static HOVER_GENERATION: AtomicU64 = AtomicU64::new(0);

// true once the icon has been clicked, which opens the menu and keeps it open
static OPEN: AtomicBool = AtomicBool::new(false);

// true while the list has been scrolled by the input opening, so it is undone once
static SCROLLED: AtomicBool = AtomicBool::new(false);

// both flags settle false, so a stale value never opens anything on start
static STARTED: OnceLock<()> = OnceLock::new();

// what the password field holds, so Connect can read it without reaching into the widget
static PASSWORD: OnceLock<std::sync::Mutex<String>> = OnceLock::new();

// the name the password field keeps its text under, so it survives a redraw
const PASSWORD_FIELD: &str = "wifi-menu-password";

// the padding that frames the password field
const INPUT_FIELD_PADDING: Padding = Padding {
    top: 0.0,
    right: 10.0,
    bottom: 0.0,
    left: 10.0,
};

// sets whether the pointer is over the icon or the hint, waking the shell to redraw
pub fn set_hovered(hovered: bool) {
    STARTED.get_or_init(|| ());
    super::popup_motion::set_hovered(hovered, &HOVERED, &HOVER_GENERATION);
}

// whether the pointer is over the icon or the hint
pub fn hovered() -> bool {
    STARTED.get_or_init(|| ());

    HOVERED.load(Ordering::Relaxed)
}

// opens the menu, or packs it away when it is already open
pub fn toggle_open() {
    STARTED.get_or_init(|| ());

    OPEN.store(!open(), Ordering::Relaxed);

    aether::refresh();
}

// packs the menu away, for a click away from it or an Escape
pub fn close() {
    STARTED.get_or_init(|| ());

    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

// whether the menu is open
pub fn open() -> bool {
    STARTED.get_or_init(|| ());

    OPEN.load(Ordering::Relaxed)
}

// what the password field holds right now
fn password_text() -> String {
    PASSWORD
        .get_or_init(|| std::sync::Mutex::new(String::new()))
        .lock()
        .map(|password| password.clone())
        .unwrap_or_default()
}

// keeps the password as it is typed, so Connect can read the latest text
fn remember_password(password: &str) {
    if let Ok(mut kept) = PASSWORD
        .get_or_init(|| std::sync::Mutex::new(String::new()))
        .lock()
    {
        kept.clear();
        kept.push_str(password);
    }
}

// empties the field, for a cancel or once a connect has been asked for
fn clear_password() {
    remember_password("");

    TextInput::set_text(PASSWORD_FIELD, "");

    SCROLLED.store(false, Ordering::Relaxed);
}

// dynamic island parameters
const ISLAND_MAX_HEIGHT: f32 = 36.0;
const ISLAND_MIN_BODY_WIDTH: f32 = 140.0;
const ISLAND_MAX_BODY_WIDTH: f32 = 280.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;
const INNER_PADDING: f32 = 14.0;

const ISLAND_WINDOW_WIDTH: f32 = 360.0;
const ISLAND_WINDOW_HEIGHT: f32 = 52.0;
const ISLAND_EDGE_INSET_RIGHT: f32 = 8.0;

const ISLAND_ANIM_DURATION: Duration = super::popup_motion::EXPANSION_DURATION;

const ICON_BADGE_SIZE: f32 = 20.0;
const ICON_DRAW_SIZE: f32 = 12.0;
const BADGE_GAP: f32 = 8.0;
const ELEMENT_GAP: f32 = 6.0;

// the dynamic island expansion progress animation
static PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();

// the menu open/close animation, separate from the hover hint
const MENU_ANIM_DURATION: Duration = Duration::from_millis(280);
static MENU_PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();

// the extra height and form fade when a network is selected
const SECTION_ANIM_DURATION: Duration = Duration::from_millis(240);
static SECTION_PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();

fn island_progress(expanded: bool) -> f32 {
    let animation = PROGRESS.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(ISLAND_ANIM_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    animation.to(if expanded { 1.0 } else { 0.0 });
    animation.value()
}

fn menu_progress() -> f32 {
    let animation = MENU_PROGRESS.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(MENU_ANIM_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    animation.to(if open() { 1.0 } else { 0.0 });
    animation.value()
}

fn section_progress(expanded: bool) -> f32 {
    let animation = SECTION_PROGRESS.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(SECTION_ANIM_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    animation.to(if expanded { 1.0 } else { 0.0 });
    animation.value()
}

/*
 * Builds the exact Dynamic Island closed outline path.
 * It connects seamlessly to the bottom edge of the top panel (y = 0):
 *  - starting at (0, 0) on the panel edge with a horizontal tangent;
 *  - curving smoothly down and inward via a concave ear into the left side;
 *  - dropping down vertically;
 *  - rounding the bottom-left corner with a convex curve;
 *  - running straight across the bottom;
 *  - rounding the bottom-right corner with a convex curve;
 *  - rising vertically up the right side;
 *  - curving outward and up via a concave ear to become horizontally tangent
 *    at (w, 0) on the panel edge;
 *  - running horizontally along y = 0 from w to 0 to seal against the panel.
 */
fn build_island_path(w: f32, h: f32, r_e: f32, r_b: f32) -> Path {
    let k_e = 0.55228475 * r_e;
    let k_b = 0.55228475 * r_b;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(k_e, 0.0, r_e, r_e - k_e, r_e, r_e)
        .line_to(r_e, h - r_b)
        .cubic_to(r_e, h - r_b + k_b, r_e + r_b - k_b, h, r_e + r_b, h)
        .line_to(w - r_e - r_b, h)
        .cubic_to(
            w - r_e - r_b + k_b,
            h,
            w - r_e,
            h - r_b + k_b,
            w - r_e,
            h - r_b,
        )
        .line_to(w - r_e, r_e)
        .cubic_to(w - r_e, r_e - k_e, w - k_e, 0.0, w, 0.0)
        .line_to(0.0, 0.0)
        .close()
}

/*
 * Builds the open border stroke path. Traces the ears and bottom perimeter,
 * leaving the top edge at y = 0 unstroked so it stays seamlessly fused to the panel.
 */
fn build_island_border_path(w: f32, h: f32, r_e: f32, r_b: f32) -> Path {
    let k_e = 0.55228475 * r_e;
    let k_b = 0.55228475 * r_b;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(k_e, 0.0, r_e, r_e - k_e, r_e, r_e)
        .line_to(r_e, h - r_b)
        .cubic_to(r_e, h - r_b + k_b, r_e + r_b - k_b, h, r_e + r_b, h)
        .line_to(w - r_e - r_b, h)
        .cubic_to(
            w - r_e - r_b + k_b,
            h,
            w - r_e,
            h - r_b + k_b,
            w - r_e,
            h - r_b,
        )
        .line_to(w - r_e, r_e)
        .cubic_to(w - r_e, r_e - k_e, w - k_e, 0.0, w, 0.0)
}

/*
 * The wifi hint window, shaped and animated like Apple's Dynamic Island / notch.
 * It connects seamlessly to the bottom edge of the top panel directly beneath
 * the wifi icon, smoothly expanding downward on hover to show the connected
 * network name, signal icon, and connection status or link speed.
 */
pub fn wifi_hint() -> LayerWindow {
    // stay open while the menu is moving, so the menu can grow out of the
    // hovered island and shrink back into it instead of replacing it abruptly
    let menu = menu_progress();
    let expanded = hovered() || open() || menu > 0.001;
    let progress = island_progress(expanded);

    if progress <= 0.001 && !expanded {
        return LayerWindow::new()
            .width(ISLAND_WINDOW_WIDTH)
            .height(ISLAND_WINDOW_HEIGHT)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Top)
            .space(Zone::Ignore)
            .namespace("aether-wifi")
            .margin(Margin {
                top: PANEL_HEIGHT as i32,
                right: ISLAND_EDGE_INSET_RIGHT as i32,
                bottom: 0,
                left: 0,
            })
            .click_through()
            .child(blank());
    }

    let theme = theme();
    let name = network_name();
    let signal = wifi();
    let ethernet = ethernet_connected();

    let wifi_icon = match &signal {
        Some(signal) => WifiSignalIcon::from_strength(signal.strength),
        None => WifiSignalIcon::None,
    };
    let wifi_secured = signal.as_ref().is_some_and(|signal| signal.secured);

    let icon_image = if ethernet {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/wifi/network-wired-symbolic.svg");
        crate::panel::svg::recolor::ink_image(path, theme.icon)
    } else {
        wifi_icon.image(wifi_secured)
    };

    let (secondary_text, secondary_color) = connection_status();

    let target_body_w = hint_body_width(&name, &secondary_text);

    let current_h = (ISLAND_MAX_HEIGHT * progress).round().max(2.0);
    let current_body_w = (target_body_w * (0.75 + 0.25 * progress)).round();

    let r_e = (EAR_RADIUS * progress).min(current_h * 0.44);
    let r_b = (CORNER_RADIUS * progress).min(current_h * 0.44);

    let current_total_w = current_body_w + 2.0 * r_e;

    let name_widget = premium_text(name)
        .color(theme.text)
        .size(12.0)
        .weight(Weight::SemiBold);

    let dot_widget = premium_text("•").color(theme.text_faint).size(9.0);

    let secondary_widget = premium_text(secondary_text)
        .color(secondary_color)
        .size(11.0)
        .weight(Weight::Regular);

    let icon_badge = Rectangle::new()
        .width(ICON_BADGE_SIZE)
        .height(ICON_BADGE_SIZE)
        .radius(ICON_BADGE_SIZE / 2.0)
        .fill(theme.chip)
        .border(1.0, theme.chip_border)
        .align_child(Align::Center, Align::Center)
        .child(
            Rectangle::new()
                .width(ICON_DRAW_SIZE)
                .height(ICON_DRAW_SIZE)
                .fill(icon_image),
        );

    let content_row = Row::new(children![
        icon_badge,
        Rectangle::new()
            .width(BADGE_GAP)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        name_widget,
        Rectangle::new()
            .width(ELEMENT_GAP)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        dot_widget,
        Rectangle::new()
            .width(ELEMENT_GAP)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        secondary_widget,
    ])
    .height(Parent)
    .align(Align::Center)
    .justify(Start);

    let content_opacity = ((progress - 0.2) / 0.8).clamp(0.0, 1.0);

    let fill_shape = Box::new(
        build_island_path(current_total_w, current_h, r_e, r_b)
            .fill(theme.bar)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let border_shape = Box::new(
        build_island_border_path(current_total_w, current_h, r_e, r_b)
            .stroke(1.0, theme.surface_border)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let canvas = Canvas::new()
        .width(current_total_w)
        .height(current_h)
        .shapes(vec![fill_shape, border_shape]);

    let island_stack = Stack::new(children![
        Rectangle::new()
            .width(current_total_w)
            .height(current_h)
            .fill(Color::TRANSPARENT)
            .padding(Padding {
                top: 0.0,
                right: r_e,
                bottom: 0.0,
                left: r_e,
            })
            .align_child(Align::Center, Align::Start)
            .child(
                Rectangle::new()
                    .width(current_body_w)
                    .height(current_h)
                    .radius_bottom_left(r_b)
                    .radius_bottom_right(r_b)
                    .fill(Color::TRANSPARENT)
                    .shadow(super::island_shadow(0.35 * progress),)
            ),
        canvas,
        Rectangle::new()
            .width(current_total_w)
            .height(current_h)
            .fill(Color::TRANSPARENT)
            .padding(Padding {
                top: 0.0,
                right: r_e,
                bottom: 0.0,
                left: r_e,
            })
            .align_child(Align::Center, Align::Center)
            .child(
                Rectangle::new()
                    .width(current_body_w)
                    .height(current_h)
                    .fill(Color::TRANSPARENT)
                    .opacity(content_opacity)
                    .align_child(Align::Center, Align::Center)
                    .child(content_row)
            ),
    ]);

    let island_card = Rectangle::new()
        .width(current_total_w)
        .height(current_h)
        .fill(Color::TRANSPARENT)
        .cursor(Cursor::Pointer)
        .on_hover(set_hovered)
        .on_click(|_| toggle_open())
        .child(island_stack);

    let island_x = (ISLAND_WINDOW_WIDTH - current_total_w).max(0.0).round() as i32;
    let island_y = 0;
    let island_w = current_total_w.round() as i32;
    let island_h = current_h.round() as i32;

    let input = InputArea {
        x: island_x,
        y: island_y,
        width: island_w,
        height: island_h,
    };

    let mut window = LayerWindow::new()
        .width(ISLAND_WINDOW_WIDTH)
        .height(ISLAND_WINDOW_HEIGHT)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Top)
        .space(Zone::Ignore)
        .namespace("aether-wifi")
        .margin(Margin {
            top: PANEL_HEIGHT as i32,
            right: ISLAND_EDGE_INSET_RIGHT as i32,
            bottom: 0,
            left: 0,
        });

    if progress > 0.05 {
        window = window.input_region(vec![input]);
    } else {
        window = window.click_through();
    }

    let root = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
        .align_child(Align::End, Align::Start)
        .child(island_card);

    window.child(root)
}

// the speed, connection state, or disconnected note the hint trails after the name
fn connection_status() -> (String, Color) {
    if let Some(speed) = speed_text() {
        return (speed, theme().text_muted);
    }

    if ethernet_connected() || wifi().is_some() {
        return ("Connected".to_string(), Color::rgba(74, 222, 128, 230));
    }

    ("Disconnected".to_string(), theme().text_faint)
}

/*
 * The body width of the fully expanded hint. The menu reads it too, so opening
 * can start from the same shape the pointer is already hovering.
 */
fn hint_body_width(name: &str, secondary: &str) -> f32 {
    let name = premium_text(name).size(12.0).weight(Weight::SemiBold);
    let dot = premium_text("•").size(9.0);
    let secondary = premium_text(secondary).size(11.0).weight(Weight::Regular);

    let content_w = ICON_BADGE_SIZE
        + BADGE_GAP
        + measured_width(&name)
        + ELEMENT_GAP
        + measured_width(&dot)
        + ELEMENT_GAP
        + measured_width(&secondary);

    (content_w + INNER_PADDING * 2.0).clamp(ISLAND_MIN_BODY_WIDTH, ISLAND_MAX_BODY_WIDTH)
}

/*
 * The width the pill needs: the name, the gap, and the speed when there is one,
 * all inside the padding. A missing speed adds nothing, so the pill stays the
 * size of the name alone.
 */
#[cfg(test)]
fn hint_width(name: &Text, speed: Option<&str>) -> f32 {
    let mut width = measured_width(name);

    if let Some(speed) = speed {
        let text = premium_text(speed.to_owned())
            .color(theme().text_muted)
            .size(HINT_TEXT_SIZE)
            .weight(Weight::Regular);

        width += HINT_GAP + measured_width(&text);
    }

    width + HINT_PADDING * 2.0
}

// the link speed, drawn as "866 Mbps", or none when it is unknown
fn speed_text() -> Option<String> {
    speed().map(|mbps| format!("{mbps} Mbps"))
}

/*
 * The menu window. It spans the whole screen so any click that lands outside the
 * panel reaches it and packs it away, and it holds the panel itself in the top
 * right, smoothly attached to the panel like a dynamic island with inverse curved ears.
 * The menu animates open and closed with a smooth height and opacity transition.
 */
pub fn wifi_menu() -> LayerWindow {
    let progress = menu_progress();

    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        // floats over everything instead of pushing anything out of the way
        .space(Zone::Ignore)
        .namespace("aether-wifi-menu")
        // Escape reaches it while it is open, and no click passes through
        .keyboard(Keyboard::OnDemand)
        .on_key(|key| {
            if key != Key::Escape {
                return;
            }

            // Escape steps back one layer: it drops the password input first and
            // only closes the menu once nothing is left to step back from
            if awaiting_password() {
                clear_password();
                cancel_password();

                return;
            }

            close();
        });

    // fully collapsed and not opening: hide the window entirely
    if progress <= 0.001 && !open() {
        return window.click_through().child(blank());
    }

    let networks = networks();
    let pending_ssid = pending();
    let section_height = pending_ssid
        .as_ref()
        .and_then(|ssid| networks.iter().find(|network| &network.ssid == ssid))
        .map(|network| network_section_height(network.saved))
        .unwrap_or(0.0);
    let section_progress = section_progress(section_height > 0.0);
    let section_extra = (section_height + LIST_ROW_GAP) * section_progress;
    let list_height = network_list_height(networks.len()) + section_extra;
    let theme = theme();
    let r_e = EAR_RADIUS;
    let total_w = BOX_WIDTH + 2.0 * r_e;
    let total_h = menu_height(networks.len()) + section_extra;

    // the menu grows out of the hint the pointer is on, so a click morphs the
    // island into the panel instead of replacing it with a separate animation
    let hint_progress = island_progress(hovered() || open() || progress > 0.001);
    let hint_body_w = (hint_body_width(&network_name(), &connection_status().0)
        * (0.75 + 0.25 * hint_progress))
        .round();
    let hint_h = (ISLAND_MAX_HEIGHT * hint_progress).round().max(2.0);
    let hint_r_e = (EAR_RADIUS * hint_progress).min(hint_h * 0.44);
    let hint_w = hint_body_w + 2.0 * hint_r_e;

    let current_w = (hint_w + (total_w - hint_w) * progress).round().max(2.0);
    let current_h = (hint_h + (total_h - hint_h) * progress).round().max(2.0);
    let current_r_e = (hint_r_e + (EAR_RADIUS - hint_r_e) * progress).min(current_h * 0.44);
    let current_r_b = BOX_RADIUS.min(current_h * 0.44);
    let current_body_w = (current_w - 2.0 * current_r_e).max(0.0);

    let fill_shape = Box::new(
        build_island_path(current_w, current_h, current_r_e, current_r_b)
            .fill(theme.bar)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let border_shape = Box::new(
        build_island_border_path(current_w, current_h, current_r_e, current_r_b)
            .stroke(1.0, theme.surface_border)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let canvas = Canvas::new()
        .width(current_w)
        .height(current_h)
        .shapes(vec![fill_shape, border_shape]);

    // content fades in after the shape is partially open
    let content_opacity = ((progress - 0.25) / 0.75).clamp(0.0, 1.0);

    let menu_card = Rectangle::new()
        .width(current_w)
        .height(current_h)
        .fill(Color::TRANSPARENT)
        .clip()
        .on_click(|_| {})
        .child(Stack::new(children![
            // Ambient drop shadow beneath the body
            Rectangle::new()
                .width(current_w)
                .height(current_h)
                .fill(Color::TRANSPARENT)
                .padding(Padding {
                    top: 0.0,
                    right: current_r_e,
                    bottom: 0.0,
                    left: current_r_e,
                })
                .align_child(Align::Center, Align::Start)
                .child(
                    Rectangle::new()
                        .width(current_body_w)
                        .height(current_h)
                        .radius_bottom_left(current_r_b)
                        .radius_bottom_right(current_r_b)
                        .fill(Color::TRANSPARENT)
                        .shadow(super::island_shadow(0.35 * progress),)
                ),
            // The curved background seamless with the top bar
            canvas,
            // Menu body padded within the ears, fades in smoothly
            Rectangle::new()
                .width(current_w)
                .height(current_h)
                .fill(Color::TRANSPARENT)
                .opacity(content_opacity)
                .padding(Padding {
                    top: BOX_PADDING,
                    right: current_r_e + BOX_PADDING,
                    bottom: BOX_PADDING,
                    left: current_r_e + BOX_PADDING,
                })
                .align_child(Align::Start, Align::Start)
                .child(box_body(networks, list_height, section_progress)),
        ]));

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            // the whole screen takes the click, so a click away from the panel closes it
            .on_click(|_| close())
            .child(
                Rectangle::new()
                    .width(Parent)
                    .height(Parent)
                    .fill(Color::TRANSPARENT)
                    .padding(aether::Padding {
                        top: PANEL_HEIGHT,
                        right: ISLAND_EDGE_INSET_RIGHT,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .align_child(Align::End, Align::Start)
                    .child(menu_card),
            ),
    )
}

// the width a piece of text needs, or a small fallback when it cannot be measured
fn measured_width(text: &Text) -> f32 {
    match text.width() {
        Size::Fixed(width) => width,
        Size::Parent => 120.0,
    }
}

/*
 * What the menu holds: the wifi switch row on top and the list of networks below.
 * The list grows to fit its rows up to the six-row limit, then scrolls.
 */
fn box_body(networks: Vec<WifiNetwork>, list_height: f32, section_progress: f32) -> Column {
    Column::new(children![
        Row::new(children![
            premium_text("Wi-Fi")
                .color(theme().text)
                .size(SWITCH_LABEL_SIZE)
                .weight(Weight::SemiBold),
            Row::new(children![reload_button(), wifi_switch()])
                .height(SWITCH_ROW_HEIGHT)
                .gap(8.0)
                .align(Align::Center),
        ])
        .width(Parent)
        .height(SWITCH_ROW_HEIGHT)
        .align(Align::Center)
        .justify(SpaceBetween),
        network_list(networks, list_height, section_progress),
    ])
    .width(Parent)
    .height(Parent)
    .gap(BOX_GAP)
    .align(Align::Start)
}

fn reload_button() -> Rectangle {
    Rectangle::new()
        .width(SWITCH_TRACK_HEIGHT)
        .height(SWITCH_TRACK_HEIGHT)
        .fill(theme().chip)
        .radius(SWITCH_TRACK_HEIGHT / 2.0)
        .cursor(Cursor::Pointer)
        .on_click(|_| search_networks())
        .align_child(Align::Center, Align::Center)
        .child(
            premium_text("↻")
                .color(theme().text_muted)
                .size(16.0)
                .weight(Weight::Medium),
        )
}

/*
 * The password input for a network and the pair that decides it: the field, then
 * a row with Cancel on the left and Connect on the right. Enter in the field
 * connects too, so the keyboard alone is enough.
 */
fn password_section(network: &str) -> Column {
    let submit_ssid = String::from(network);

    let field = TextInput::new(PASSWORD_FIELD)
        .width(Parent)
        .size(INPUT_TEXT_SIZE)
        .color(theme().text)
        .placeholder("Password")
        .password()
        // takes the keyboard as soon as it appears, so no second click is needed
        .focused()
        // kept where Connect can read it without reaching into the widget
        .on_change(|password| remember_password(&password))
        .on_submit(move |password| {
            let password = password_of(password);
            clear_password();
            connect(&submit_ssid, password);
        });

    Column::new(children![
        Rectangle::new()
            .width(Parent)
            .height(INPUT_FIELD_HEIGHT)
            .fill(theme().chip)
            .border(1.0, theme().chip_border)
            .radius(INPUT_RADIUS)
            .padding(INPUT_FIELD_PADDING)
            .align_child(Align::Start, Align::Center)
            .child(field),
        button_row(network),
    ])
    .width(Parent)
    .height(INPUT_SECTION_HEIGHT)
    .gap(BOX_GAP)
    .align(Align::Start)
}

// the Cancel and Connect pair, each half the width of the row
fn button_row(network: &str) -> Row {
    let connect_ssid = String::from(network);

    // the field and its padding set how much room the two buttons share
    let half = (BUTTON_AREA_WIDTH - BUTTON_GAP) / 2.0;

    Row::new(children![
        button("Cancel", half, theme().chip, theme().text_muted, |_| {
            clear_password();
            cancel_password();
        },),
        gap(BUTTON_GAP),
        button(
            "Connect",
            half,
            theme().accent,
            theme().surface,
            move |_| {
                // connect with whatever the field holds; an empty one falls back to a saved profile
                let password = password_of(password_text());
                clear_password();
                connect(&connect_ssid, password);
            },
        ),
    ])
    .width(Parent)
    .height(BUTTON_HEIGHT)
    .align(Align::Center)
    .justify(SpaceBetween)
}

// one button: a filled pill with a centred label, taking a click
fn button(
    label: &str,
    width: f32,
    fill: Color,
    ink: Color,
    on_click: impl Fn(aether::Button) + 'static,
) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(BUTTON_HEIGHT)
        .fill(fill)
        .radius(BUTTON_RADIUS)
        .align_child(Align::Center, Align::Center)
        .cursor(Cursor::Pointer)
        .on_click(on_click)
        .child(
            premium_text(String::from(label))
                .color(ink)
                .size(BUTTON_TEXT_SIZE)
                .weight(Weight::SemiBold),
        )
}

// an empty password means "use the saved profile", so it becomes nothing
fn password_of(password: String) -> Option<String> {
    if password.is_empty() {
        return None;
    }

    Some(password)
}

/*
 * The networks in range, strongest first, each a row with a wifi range icon and
 * the name. The connected one is picked out with a brighter name. When a secured
 * network is picked, the password input opens directly below it. The list is a
 * scroll area whose height follows the visible row count up to the six-row cap.
 */
fn network_list(list: Vec<WifiNetwork>, list_height: f32, section_progress: f32) -> ScrollArea {
    let pending_ssid = pending();

    // with nothing in range, a plain note stands in for the rows
    let content = if list.is_empty() {
        Column::new(children![
            Row::new(children![
                premium_text("No networks found")
                    .color(theme().text_muted)
                    .size(LIST_TEXT_SIZE),
            ])
            .width(Parent)
            .height(LIST_ROW_HEIGHT)
            .align(Align::Center),
        ])
        .width(Parent)
        .height(LIST_ROW_HEIGHT)
    } else {
        let mut items: Vec<Box<dyn Widget>> = Vec::new();
        let mut height = 0.0;

        for network in list {
            let ssid = network.ssid.clone();
            let is_pending = pending_ssid.as_deref() == Some(ssid.as_str());
            let saved = network.saved;

            // where this row starts, so its input can be brought into sight
            let row_top = height;

            items.push(network_row(network));

            height += LIST_ROW_HEIGHT;

            // the password input opens directly below the network it belongs to,
            // so the choice and its password sit together
            if is_pending {
                let section_height = network_section_height(saved);
                let section = if saved {
                    saved_actions(&ssid)
                } else {
                    password_section(&ssid)
                };
                items.push(Box::new(
                    Rectangle::new()
                        .width(Parent)
                        .height(section_height)
                        .fill(Color::TRANSPARENT)
                        .opacity(section_progress)
                        .child(section),
                ));

                // the whole section is what has to be seen: the row, its gap, and
                // the input, which is taller than the visible list when it is long
                reveal(row_top, section_height, list_height);

                height += section_height + LIST_ROW_GAP;
            }

            height += LIST_ROW_GAP;
        }

        // the last gap trails the list, so take it back off
        let height = height - LIST_ROW_GAP;

        Column::new(items)
            .width(Parent)
            .height(height)
            .gap(LIST_ROW_GAP)
    };

    ScrollArea::new("wifi-menu-list", content)
        .width(Parent)
        .height(list_height)
}

/*
 * One network: the range icon, the name, and, when it is the connected one, a
 * soft accent panel behind the row and a small "Connected" chip at the far end.
 * The icon is picked by the signal and answers the lock, so a secured network
 * carries the locked variant.
 */
fn network_row(network: WifiNetwork) -> Box<dyn Widget> {
    let theme = theme();

    let connected = network.connected;

    // a click joins the network: a secured one opens the password input first,
    // an open one is joined straight away

    // kept for the click below, which runs after the name has been drawn
    let name = network.ssid.clone();

    // the connected row takes the accent ink, the rest the muted one
    let ink = if connected {
        theme.accent
    } else {
        theme.text_muted
    };

    let icon = WifiSignalIcon::from_strength(network.strength).image_ink(network.secured, ink);

    let mut row = vec![
        Box::new(
            Rectangle::new()
                .width(LIST_ICON_SIZE)
                .height(LIST_ICON_SIZE)
                .fill(icon),
        ) as Box<dyn Widget>,
        Box::new(gap(LIST_ICON_GAP)),
        Box::new(
            premium_text(network.ssid)
                .color(ink)
                .size(LIST_TEXT_SIZE)
                .weight(if connected {
                    Weight::SemiBold
                } else {
                    Weight::Regular
                })
                .elide(),
        ),
    ];

    // the chip names the state at the far end, so the row reads at a glance
    if connected {
        row.push(Box::new(gap(LIST_CHIP_GAP)));
        row.push(Box::new(connected_chip()));
    }

    let content = Row::new(row)
        .width(Parent)
        .height(LIST_ROW_HEIGHT)
        .align(Align::Center);

    // the connected row spreads its parts apart, so the chip sits at the end
    let content = if connected {
        content.justify(SpaceBetween)
    } else {
        content.justify(Start)
    };

    // only the connected row sits on a panel, so it stands apart from the rest
    if connected {
        return Box::new(
            Rectangle::new()
                .width(Parent)
                .height(LIST_ROW_HEIGHT)
                .fill(theme.accent_faint)
                .radius(LIST_ROW_RADIUS)
                .padding(LIST_ROW_PADDING)
                .align_child(Align::Start, Align::Center)
                .child(content),
        );
    }

    // a click joins the network: a secured one opens the password input first,
    // an open one is joined straight away, and either takes a pointer
    // a click joins the network: a secured one opens the password input first,
    // an open one is joined straight away, and either takes a pointer
    let click = move |_| {
        // Saved profiles offer explicit Connect and Forget actions. A new
        // network always opens the credential form, including open networks;
        // submitting an empty password joins an open network.
        ask_password(&name);
    };

    Box::new(
        Rectangle::new()
            .width(Parent)
            .height(LIST_ROW_HEIGHT)
            .fill(Color::TRANSPARENT)
            .radius(LIST_ROW_RADIUS)
            .padding(LIST_ROW_PADDING)
            .align_child(Align::Start, Align::Center)
            .cursor(Cursor::Pointer)
            .on_click(click)
            .child(content),
    )
}

fn saved_actions(network: &str) -> Column {
    let connect_ssid = network.to_owned();
    let forget_ssid = network.to_owned();
    let half = (BUTTON_AREA_WIDTH - BUTTON_GAP) / 2.0;
    Column::new(children![
        Row::new(children![
            button(
                "Connect",
                half,
                theme().accent,
                theme().surface,
                move |_| connect(&connect_ssid, None)
            ),
            gap(BUTTON_GAP),
            button(
                "Forget",
                half,
                theme().chip,
                theme().text_muted,
                move |_| crate::services::wifi::forget(&forget_ssid)
            ),
        ])
        .width(Parent)
        .height(BUTTON_HEIGHT)
        .align(Align::Center)
        .justify(SpaceBetween),
    ])
    .width(Parent)
    .height(BUTTON_HEIGHT)
}

// a blank spacer of a given width
fn gap(width: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(1.0)
        .fill(Color::TRANSPARENT)
}

/*
 * Brings a row and the input below it into sight, so the input never opens out of
 * view. It runs only the first time a network is picked, so a later scroll by the
 * pointer is not undone on the next redraw.
 */
fn reveal(row_top: f32, section_height: f32, list_height: f32) {
    // always try to scroll when a section appears so it stays visible
    let bottom = row_top + LIST_ROW_HEIGHT + LIST_ROW_GAP + section_height;

    if bottom <= list_height {
        // section still fits, but scroll just enough to keep it visible
        ScrollArea::scroll_to("wifi-menu-list", 0.0);
        SCROLLED.store(false, Ordering::Relaxed);

        return;
    }

    // bring the bottom of the input section into view with a bit of padding
    let offset = (bottom - list_height + 4.0).max(0.0);

    ScrollArea::scroll_to("wifi-menu-list", offset);

    SCROLLED.store(true, Ordering::Relaxed);
}

// the small pill that names the connected state
fn connected_chip() -> Rectangle {
    let theme = theme();

    Rectangle::new()
        .width(measured_width(&chip_label()) + LIST_CHIP_GAP * 2.0)
        .height(LIST_CHIP_HEIGHT)
        .fill(theme.accent_faint)
        .radius(LIST_CHIP_HEIGHT / 2.0)
        .align_child(Align::Center, Align::Center)
        .child(chip_label())
}

// the chip's text, measured so the pill hugs it
fn chip_label() -> Text {
    premium_text("Connected")
        .color(theme().accent)
        .size(LIST_CHIP_TEXT_SIZE)
        .weight(Weight::SemiBold)
}

/*
 * The sliding pill switch. The track takes the accent when the radio is on, and
 * a knob rides at the right end when on and the left end when off; a click flips
 * the radio through the service.
 */
fn wifi_switch() -> Rectangle {
    let theme = theme();

    let on = radio();

    let track = if on { theme.accent } else { theme.track };

    // the knob sits inside the track by a small inset on every side
    let inset = (SWITCH_TRACK_HEIGHT - SWITCH_KNOB) / 2.0;

    let knob = Rectangle::new()
        .width(SWITCH_KNOB)
        .height(SWITCH_KNOB)
        .fill(theme.surface)
        .radius(Radius::Full);

    Rectangle::new()
        .width(SWITCH_TRACK_WIDTH)
        .height(SWITCH_TRACK_HEIGHT)
        .fill(track)
        .radius(SWITCH_TRACK_HEIGHT / 2.0)
        .padding(aether::Padding {
            top: inset,
            right: inset,
            bottom: inset,
            left: inset,
        })
        // the knob rides at the end the radio is on
        .align_child(if on { Align::End } else { Align::Start }, Align::Center)
        .child(knob)
        .cursor(Cursor::Pointer)
        .on_click(|_| toggle_radio())
}

// the connected network's name, or a plain note when there is no connection
fn network_name() -> String {
    ssid().unwrap_or_else(|| String::from("Not connected"))
}

// an empty, transparent child, used while a window rests
fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
