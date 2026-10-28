use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Cursor, Easing, Horizontal, InputArea, Layer, LayerWindow,
    Margin, Padding, Parent, Path, Rectangle, Row, Shape, Stack, Start, Text, Vertical, Weight,
    Zone, children,
};

use crate::panel::top::PANEL_HEIGHT;
use crate::theme::theme;

const ISLAND_MAX_HEIGHT: f32 = 36.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;
const INNER_PADDING: f32 = 14.0;

const ISLAND_WINDOW_WIDTH: f32 = 280.0;
const ISLAND_WINDOW_HEIGHT: f32 = 52.0;
// Align under bluetooth icon in right cluster (matches wifi/battery style)
const ISLAND_EDGE_INSET_RIGHT: f32 = 20.0;

const ICON_BADGE_SIZE: f32 = 20.0;
const ICON_DRAW_SIZE: f32 = 12.0;
const BADGE_GAP: f32 = 8.0;

const ISLAND_ANIM_DURATION: Duration = super::popup_motion::EXPANSION_DURATION;

static PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
static HOVERED: AtomicBool = AtomicBool::new(false);
static HOVER_GENERATION: AtomicU64 = AtomicU64::new(0);

fn premium_text(value: impl Into<String>) -> Text {
    Text::new(value).font("Maple Mono")
}

pub fn set_hovered(hovered: bool) {
    super::popup_motion::set_hovered(hovered, &HOVERED, &HOVER_GENERATION);
}

pub fn hovered() -> bool {
    HOVERED.load(Ordering::Relaxed)
}

pub fn dismiss() {
    HOVER_GENERATION.fetch_add(1, Ordering::Relaxed);
    if HOVERED.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

pub(super) fn island_progress(expanded: bool) -> f32 {
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

pub(super) fn hint_body_width(has_connected: bool) -> f32 {
    let status_width = if has_connected { 70.0 } else { 90.0 };
    let content_width = ICON_BADGE_SIZE + BADGE_GAP + status_width;
    (content_width + INNER_PADDING * 2.0).clamp(110.0, 170.0)
}

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

pub fn bluetooth_hint() -> LayerWindow {
    let menu_progress = super::bluetooth_menu::menu_progress();
    let expanded = hovered() || super::bluetooth_menu::open() || menu_progress > 0.001;
    let progress = island_progress(expanded);

    if progress <= 0.001 && !expanded {
        return LayerWindow::new()
            .width(ISLAND_WINDOW_WIDTH)
            .height(ISLAND_WINDOW_HEIGHT)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Top)
            .space(Zone::Ignore)
            .namespace("aether-bluetooth")
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
    let powered = crate::services::bluetooth::powered();
    let devices = crate::services::bluetooth::devices();
    let has_connected = powered && devices.iter().any(|d| d.connected);
    let status = if !powered {
        "Bluetooth off"
    } else if has_connected {
        "Connected"
    } else {
        "Not connected"
    };
    let status_color = if has_connected {
        Color::rgba(74, 222, 128, 230)
    } else if powered {
        theme.text_faint
    } else {
        theme.text_faint
    };

    let icon_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/panel/bluetooth.svg");
    let icon_image = crate::panel::svg::recolor::ink_image(icon_path, theme.icon);

    let status_widget = premium_text(status)
        .color(status_color)
        .size(11.0)
        .weight(Weight::Medium);

    let target_body_w = hint_body_width(has_connected);

    let current_h = (ISLAND_MAX_HEIGHT * progress).round().max(2.0);
    let current_body_w = (target_body_w * (0.75 + 0.25 * progress)).round();

    let r_e = (EAR_RADIUS * progress).min(current_h * 0.44);
    let r_b = (CORNER_RADIUS * progress).min(current_h * 0.44);

    let current_total_w = current_body_w + 2.0 * r_e;

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
        status_widget,
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
                    .shadow(super::island_shadow(0.35 * progress)),
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
                    .child(content_row),
            ),
    ]);

    let island_card = Rectangle::new()
        .width(current_total_w)
        .height(current_h)
        .fill(Color::TRANSPARENT)
        .cursor(Cursor::Pointer)
        .on_hover(set_hovered)
        .on_click(|_| {
            dismiss();
            super::bluetooth_menu::set_open(true);
        })
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
        .namespace("aether-bluetooth")
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

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
