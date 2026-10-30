use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Cursor, Easing, Horizontal, Key, Keyboard, Layer, LayerWindow,
    Padding, Parent, Path, Radius, Rectangle, Row, ScrollArea, Shape, Stack, Text, Vertical,
    Weight, Widget, Zone, children,
};

use crate::panel::top::PANEL_HEIGHT;
use crate::services::{bluetooth::{self, BluetoothDevice}, display};
use crate::theme::theme;

const BOX_WIDTH: f32 = 280.0;
const BOX_RADIUS: f32 = 14.0;
const BOX_PADDING: f32 = 14.0;
const BOX_GAP: f32 = 12.0;

const SWITCH_LABEL_SIZE: f32 = 12.0;
const SWITCH_TRACK_WIDTH: f32 = 40.0;
const SWITCH_TRACK_HEIGHT: f32 = 22.0;
const SWITCH_KNOB: f32 = 16.0;

const LIST_MAX_VISIBLE_ROWS: usize = 6;
const LIST_ROW_HEIGHT: f32 = 34.0;
const LIST_ROW_GAP: f32 = 4.0;
const LIST_TEXT_SIZE: f32 = 12.0;
const LIST_ROW_PADDING: f32 = 8.0;
const LIST_ROW_RADIUS: f32 = 8.0;
const LIST_CHIP_HEIGHT: f32 = 18.0;
const ACTION_TEXT_SIZE: f32 = 9.0;
const ACTION_HORIZONTAL_PADDING: f32 = 8.0;

const EAR_RADIUS: f32 = 13.0;
const ISLAND_EDGE_INSET_RIGHT: f32 = 20.0;

const ISLAND_MAX_HEIGHT: f32 = 36.0;
const MENU_ANIM_DURATION: Duration = Duration::from_millis(280);

static OPEN: AtomicBool = AtomicBool::new(false);
static MENU_PROGRESS: std::sync::OnceLock<std::sync::Mutex<Animation<f32>>> =
    std::sync::OnceLock::new();
static TARGET_MONITOR: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();

fn target_monitor() -> &'static std::sync::Mutex<Option<String>> {
    TARGET_MONITOR.get_or_init(|| std::sync::Mutex::new(None))
}

fn is_target_monitor(monitor: &aether::Monitor) -> bool {
    let target = target_monitor()
        .lock()
        .ok()
        .and_then(|target| target.clone());

    target.as_deref().map_or_else(
        || {
            display::is_primary(
                &monitor.name,
                monitor.x,
                monitor.y,
                &monitor.outputs,
                monitor.origin_output.as_deref(),
            )
        },
        |target| target == monitor.name,
    )
}

fn premium_text(value: impl Into<String>) -> Text {
    Text::new(value).font("Maple Mono")
}

pub fn set_open(open: bool) {
    OPEN.store(open, Ordering::Relaxed);
    aether::refresh();
}

pub fn toggle_open() {
    OPEN.store(!open(), Ordering::Relaxed);
    aether::refresh();
}

pub fn open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

pub fn close() {
    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

pub(super) fn menu_progress() -> f32 {
    let animation = MENU_PROGRESS.get_or_init(|| {
        std::sync::Mutex::new(
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

pub fn set_target_monitor(monitor: Option<String>) {
    if let Ok(mut target) = target_monitor().lock() {
        *target = monitor;
    }
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

fn visible_network_rows(count: usize) -> usize {
    count.clamp(1, LIST_MAX_VISIBLE_ROWS)
}

fn list_height(count: usize) -> f32 {
    let rows = visible_network_rows(count) as f32;
    rows * LIST_ROW_HEIGHT + (rows - 1.0) * LIST_ROW_GAP
}

fn menu_height(count: usize) -> f32 {
    BOX_PADDING * 2.0 + SWITCH_TRACK_HEIGHT + BOX_GAP + list_height(count)
}

pub fn bluetooth_menu(monitor: &aether::Monitor) -> LayerWindow {
    if !is_target_monitor(monitor) {
        return LayerWindow::new()
            .width(aether::Full)
            .height(aether::Full)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Overlay)
            .space(Zone::Ignore)
            .namespace("aether-bluetooth-menu")
            .keyboard(Keyboard::OnDemand)
            .on_key(|key| {
                if key == Key::Escape {
                    close();
                }
            })
            .click_through()
            .child(blank());
    }

    let progress = menu_progress();

    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-bluetooth-menu")
        .visible(is_target_monitor(monitor))
        .keyboard(Keyboard::OnDemand)
        .on_key(|key| {
            if key == Key::Escape {
                close();
            }
        });

    if progress <= 0.001 && !open() {
        return window.click_through().child(blank());
    }

    let devices = bluetooth::devices();
    let powered = bluetooth::powered();
    let has_connected = powered && devices.iter().any(|device| device.connected);
    let theme = theme();
    let r_e = EAR_RADIUS;
    let total_w = BOX_WIDTH + 2.0 * r_e;
    let total_h = menu_height(devices.len());

    let hint_progress = super::bluetooth_popup::island_progress(
        super::bluetooth_popup::hovered() || open() || progress > 0.001,
    );
    let hint_body_w = (super::bluetooth_popup::hint_body_width(has_connected)
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

    let content_opacity = ((progress - 0.25) / 0.75).clamp(0.0, 1.0);

    let menu_card = Rectangle::new()
        .width(current_w)
        .height(current_h)
        .fill(Color::TRANSPARENT)
        .clip()
        .on_click(|_| {})
        .child(Stack::new(children![
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
                        .shadow(crate::panel::component::island_shadow(0.35 * progress)),
                ),
            canvas,
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
                .child(box_body(devices, powered)),
        ]));

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
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

fn box_body(devices: Vec<BluetoothDevice>, powered: bool) -> aether::Column {
    aether::Column::new(children![
        Row::new(children![
            premium_text("Bluetooth")
                .color(theme().text)
                .size(SWITCH_LABEL_SIZE)
                .weight(Weight::SemiBold),
            bluetooth_switch(powered),
        ])
        .width(Parent)
        .height(SWITCH_TRACK_HEIGHT)
        .align(Align::Center)
        .justify(aether::SpaceBetween),
        device_list(devices, powered),
    ])
    .width(Parent)
    .height(Parent)
    .gap(BOX_GAP)
    .align(Align::Start)
}

fn bluetooth_switch(powered: bool) -> Rectangle {
    let theme = theme();
    let track = if powered { theme.accent } else { theme.track };
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
        .align_child(
            if powered { Align::End } else { Align::Start },
            Align::Center,
        )
        .child(knob)
        .cursor(Cursor::Pointer)
        .on_click(|_| bluetooth::toggle_power())
}

fn device_list(devices: Vec<BluetoothDevice>, powered: bool) -> ScrollArea {
    let height = list_height(devices.len());
    let content = if !powered {
        aether::Column::new(children![
            Row::new(children![
                premium_text("Bluetooth off")
                    .color(theme().text_muted)
                    .size(LIST_TEXT_SIZE),
            ])
            .width(Parent)
            .height(LIST_ROW_HEIGHT)
            .align(Align::Center),
        ])
        .width(Parent)
        .height(LIST_ROW_HEIGHT)
    } else if devices.is_empty() {
        aether::Column::new(children![
            Row::new(children![
                premium_text("No devices found")
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
        let mut h = 0.0f32;
        for device in devices {
            items.push(device_row(device));
            h += LIST_ROW_HEIGHT + LIST_ROW_GAP;
        }
        if h > 0.0 {
            h -= LIST_ROW_GAP;
        }
        aether::Column::new(items)
            .width(Parent)
            .height(h)
            .gap(LIST_ROW_GAP)
    };

    ScrollArea::new("bluetooth-menu-list", content)
        .width(Parent)
        .height(height)
}

fn device_row(device: BluetoothDevice) -> Box<dyn Widget> {
    let theme = theme();
    let action = if device.connected {
        "Disconnect"
    } else if device.paired {
        "Connect"
    } else {
        "Pair"
    };
    let path = device.path;
    let paired = device.paired;
    let connected = device.connected;
    let ink = if device.connected {
        theme.accent
    } else {
        theme.text
    };

    let content = Row::new(children![
        premium_text(device.name)
            .color(ink)
            .size(LIST_TEXT_SIZE)
            .weight(if device.connected {
                Weight::SemiBold
            } else {
                Weight::Regular
            })
            .elide(),
        action_chip(action),
    ])
    .width(Parent)
    .height(LIST_ROW_HEIGHT)
    .align(Align::Center)
    .justify(aether::SpaceBetween);

    Box::new(
        Rectangle::new()
            .width(Parent)
            .height(LIST_ROW_HEIGHT)
            .fill(if device.connected {
                theme.accent_faint
            } else {
                Color::TRANSPARENT
            })
            .radius(LIST_ROW_RADIUS)
            .padding(LIST_ROW_PADDING)
            .align_child(Align::Start, Align::Center)
            .cursor(Cursor::Pointer)
            .on_click(move |_| {
                bluetooth::device_action(path.clone(), paired, connected);
            })
            .child(content),
    )
}

fn action_chip(label: &str) -> Rectangle {
    let theme = theme();
    let text = premium_text(label)
        .color(theme.accent)
        .size(ACTION_TEXT_SIZE)
        .weight(Weight::Medium);

    Rectangle::new()
        .width(measured_width(&text) + ACTION_HORIZONTAL_PADDING * 2.0)
        .height(LIST_CHIP_HEIGHT)
        .fill(theme.accent_faint)
        .radius(LIST_CHIP_HEIGHT / 2.0)
        .align_child(Align::Center, Align::Center)
        .child(text)
}

fn measured_width(text: &Text) -> f32 {
    match text.width() {
        aether::Size::Fixed(width) => width,
        aether::Size::Parent => 80.0,
    }
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
