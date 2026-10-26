use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Center, Color, Column, Cursor, Easing, Full, Horizontal, Key,
    Keyboard, Layer, LayerWindow, Padding, Parent, Path, Radius, Rectangle, Row, ScrollArea, Shape,
    Stack, Text, Vertical, Weight, Widget, Zone, children,
};
use aether_services::notifications::Notification;

use crate::panel::notification::icon_path;
use crate::panel::svg::recolor::ink_image;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::notifications;
use crate::theme::theme;

const MENU_WIDTH: f32 = 360.0;
const HINT_WIDTH: f32 = 194.0;
const HINT_HEIGHT: f32 = 36.0;
const MENU_PADDING: f32 = 14.0;
const HEADER_HEIGHT: f32 = 34.0;
const ROW_HEIGHT: f32 = 70.0;
const ROW_GAP: f32 = 7.0;
const CLEAR_HEIGHT: f32 = 32.0;
const MAX_ROWS: usize = 5;
const SWIPE_DISTANCE: f32 = 72.0;
const RIGHT_INSET: f32 = 64.0;
const EAR_RADIUS: f32 = 13.0;

static OPEN: AtomicBool = AtomicBool::new(false);
static HOVERED: AtomicBool = AtomicBool::new(false);
static PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
static DRAG_ORIGINS: OnceLock<Mutex<HashMap<u32, f32>>> = OnceLock::new();

fn premium_text(value: impl Into<String>) -> Text {
    Text::new(value).font("Adwaita Sans")
}

fn island_path(width: f32, height: f32, ear_radius: f32, bottom_radius: f32) -> Path {
    let ear_curve = 0.55228475 * ear_radius;
    let bottom_curve = 0.55228475 * bottom_radius;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(
            ear_curve,
            0.0,
            ear_radius,
            ear_radius - ear_curve,
            ear_radius,
            ear_radius,
        )
        .line_to(ear_radius, height - bottom_radius)
        .cubic_to(
            ear_radius,
            height - bottom_radius + bottom_curve,
            ear_radius + bottom_radius - bottom_curve,
            height,
            ear_radius + bottom_radius,
            height,
        )
        .line_to(width - ear_radius - bottom_radius, height)
        .cubic_to(
            width - ear_radius - bottom_radius + bottom_curve,
            height,
            width - ear_radius,
            height - bottom_radius + bottom_curve,
            width - ear_radius,
            height - bottom_radius,
        )
        .line_to(width - ear_radius, ear_radius)
        .cubic_to(
            width - ear_radius,
            ear_radius - ear_curve,
            width - ear_curve,
            0.0,
            width,
            0.0,
        )
        .line_to(0.0, 0.0)
        .close()
}

fn island_border_path(width: f32, height: f32, ear_radius: f32, bottom_radius: f32) -> Path {
    let ear_curve = 0.55228475 * ear_radius;
    let bottom_curve = 0.55228475 * bottom_radius;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(
            ear_curve,
            0.0,
            ear_radius,
            ear_radius - ear_curve,
            ear_radius,
            ear_radius,
        )
        .line_to(ear_radius, height - bottom_radius)
        .cubic_to(
            ear_radius,
            height - bottom_radius + bottom_curve,
            ear_radius + bottom_radius - bottom_curve,
            height,
            ear_radius + bottom_radius,
            height,
        )
        .line_to(width - ear_radius - bottom_radius, height)
        .cubic_to(
            width - ear_radius - bottom_radius + bottom_curve,
            height,
            width - ear_radius,
            height - bottom_radius + bottom_curve,
            width - ear_radius,
            height - bottom_radius,
        )
        .line_to(width - ear_radius, ear_radius)
        .cubic_to(
            width - ear_radius,
            ear_radius - ear_curve,
            width - ear_curve,
            0.0,
            width,
            0.0,
        )
}

pub fn open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

fn dnd_button() -> Rectangle {
    let enabled = notifications::do_not_disturb();
    let icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/panel/moon.svg");

    Rectangle::new()
        .width(26.0)
        .height(26.0)
        .radius(Radius::Full)
        .fill(if enabled {
            theme().accent_faint
        } else {
            theme().chip
        })
        .cursor(Cursor::Pointer)
        .on_click(|_| notifications::toggle_do_not_disturb())
        .align_child(Align::Center, Align::Center)
        .child(
            Rectangle::new()
                .width(14.0)
                .height(14.0)
                .fill(ink_image(icon, theme().icon)),
        )
}

pub fn hovered() -> bool {
    HOVERED.load(Ordering::Relaxed)
}

pub fn set_hovered(hovered: bool) {
    if HOVERED.swap(hovered, Ordering::Relaxed) != hovered {
        aether::refresh();
    }
}

pub fn toggle_open() {
    OPEN.store(!open(), Ordering::Relaxed);
    aether::refresh();
}

pub fn close() {
    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

fn progress() -> f32 {
    let animation = PROGRESS.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(Duration::from_millis(260))
                .easing(Easing::Out),
        )
    });
    let mut animation = animation.lock().unwrap_or_else(PoisonError::into_inner);
    animation.to(if open() { 1.0 } else { 0.0 });
    animation.value()
}

pub fn notification_menu() -> LayerWindow {
    let notifications = notifications::active_notifications();
    let count = notifications.len();
    let progress = progress();
    let expanded = open() || progress > 0.01;
    let hovered = HOVERED.load(Ordering::Relaxed);
    let hint_visible = hovered && !expanded;
    let content_height = menu_content_height(count);
    let current_width = HINT_WIDTH + (MENU_WIDTH - HINT_WIDTH) * progress;
    let current_height = HINT_HEIGHT + (content_height - HINT_HEIGHT) * progress;
    let content_opacity = ((progress - 0.2) / 0.8).clamp(0.0, 1.0);

    let window = LayerWindow::new()
        .width(Full)
        .height(Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-notification-menu")
        .keyboard(Keyboard::OnDemand)
        .on_key(|key| {
            if key == Key::Escape {
                close();
            }
        });

    if !expanded && !hint_visible {
        return window.click_through().child(blank());
    }

    let header = if expanded {
        menu_header(count)
    } else {
        hint_header(count)
    };
    let foreground = if expanded {
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .child(menu_content(notifications, count))
    } else {
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .align_child(Align::Center, Align::Center)
            .child(header)
    };

    let radius = (EAR_RADIUS * progress.max(if hint_visible { 1.0 } else { 0.0 }))
        .min(current_height * 0.44);
    let fill_shape =
        Box::new(island_path(current_width, current_height, radius, radius).fill(theme().surface))
            as Box<dyn Shape>;
    let border_shape = Box::new(
        island_border_path(current_width, current_height, radius, radius)
            .stroke(1.0, theme().surface_border),
    ) as Box<dyn Shape>;
    let canvas = Canvas::new()
        .width(current_width)
        .height(current_height)
        .shapes(vec![fill_shape, border_shape]);

    let card = Rectangle::new()
        .width(if expanded { current_width } else { HINT_WIDTH })
        .height(if expanded {
            current_height
        } else {
            HINT_HEIGHT
        })
        .shadow(crate::panel::component::island_shadow(0.35))
        .clip()
        .on_hover(set_hovered)
        .on_click(|_| {
            if !open() {
                toggle_open();
            }
        })
        .child(Stack::new(children![
            canvas,
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Color::TRANSPARENT)
                .padding(if expanded {
                    Padding {
                        top: MENU_PADDING,
                        right: MENU_PADDING,
                        bottom: MENU_PADDING,
                        left: MENU_PADDING,
                    }
                } else {
                    Padding {
                        top: 0.0,
                        right: 14.0,
                        bottom: 0.0,
                        left: 14.0,
                    }
                })
                .opacity(if expanded { content_opacity } else { 1.0 })
                .align_child(Align::Center, Align::Center)
                .child(foreground),
        ]));

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .on_click(|_| close())
            .padding(Padding {
                top: PANEL_HEIGHT,
                right: RIGHT_INSET,
                bottom: 0.0,
                left: 0.0,
            })
            .align_child(Align::End, Align::Start)
            .child(card),
    )
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}

fn menu_content_height(count: usize) -> f32 {
    let visible_rows = count.clamp(1, MAX_ROWS) as f32;
    MENU_PADDING * 2.0
        + HEADER_HEIGHT
        + 10.0
        + visible_rows * ROW_HEIGHT
        + (visible_rows - 1.0) * ROW_GAP
        + 10.0
        + CLEAR_HEIGHT
}

fn menu_header(count: usize) -> Row {
    Row::new(children![
        premium_text("Notifications")
            .color(theme().text)
            .size(15.0)
            .weight(Weight::SemiBold),
        Rectangle::new()
            .width(Parent)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        dnd_button(),
        count_chip(count),
    ])
    .width(Parent)
    .height(HEADER_HEIGHT)
    .align(Align::Center)
    .justify(aether::SpaceBetween)
}

fn hint_header(count: usize) -> Row {
    Row::new(children![
        Rectangle::new()
            .width(16.0)
            .height(16.0)
            .fill(ink_image(notification_bell(), theme().icon)),
        premium_text("Notifications")
            .color(theme().text)
            .size(12.0)
            .weight(Weight::Medium),
        count_chip(count),
    ])
    .width(Parent)
    .height(Parent)
    .gap(8.0)
    .align(Align::Center)
}

fn count_chip(count: usize) -> Rectangle {
    Rectangle::new()
        .width(if count > 99 { 34.0 } else { 27.0 })
        .height(21.0)
        .radius(Radius::Full)
        .fill(theme().accent_faint)
        .align_child(Align::Center, Align::Center)
        .child(
            premium_text(if count > 99 {
                "99+".to_owned()
            } else {
                count.to_string()
            })
            .color(theme().text)
            .size(10.0)
            .weight(Weight::SemiBold),
        )
}

fn menu_content(items: Vec<Notification>, count: usize) -> Column {
    let item_count = items.len();
    let list_height = item_count.clamp(1, MAX_ROWS) as f32 * ROW_HEIGHT
        + item_count.clamp(1, MAX_ROWS).saturating_sub(1) as f32 * ROW_GAP;
    let content = if items.is_empty() {
        Column::new(children![
            premium_text("You’re all caught up")
                .color(theme().text_muted)
                .size(12.0),
        ])
        .width(Parent)
        .height(list_height)
        .align(Align::Center)
        .justify(Center)
    } else {
        let rows: Vec<Box<dyn Widget>> = items
            .into_iter()
            .rev()
            .map(|item| Box::new(notification_row(item)) as Box<dyn Widget>)
            .collect();
        let content_height =
            item_count as f32 * ROW_HEIGHT + item_count.saturating_sub(1) as f32 * ROW_GAP;
        let rows = Column::new(rows)
            .width(Parent)
            .height(content_height)
            .gap(ROW_GAP);
        Column::new(children![
            ScrollArea::new("aether-notification-center-list", rows)
                .width(Parent)
                .height(list_height),
        ])
        .width(Parent)
        .height(list_height)
    };

    let clear = Rectangle::new()
        .width(Parent)
        .height(CLEAR_HEIGHT)
        .radius(Radius::Full)
        .fill(if count > 0 {
            theme().accent_faint
        } else {
            theme().chip
        })
        .cursor(if count > 0 {
            Cursor::Pointer
        } else {
            Cursor::Default
        })
        .on_click(|_| notifications::clear_all())
        .align_child(Align::Center, Align::Center)
        .child(
            premium_text("Clear all")
                .color(if count > 0 {
                    theme().text
                } else {
                    theme().text_faint
                })
                .size(11.0)
                .weight(Weight::Medium),
        );

    Column::new(children![menu_header(count), content, clear,])
        .width(Parent)
        .height(Parent)
        .gap(10.0)
        .align(Align::Start)
}

fn notification_row(notification: Notification) -> Rectangle {
    let id = notification.id;
    let icon = icon_path(&notification);
    let title = if notification.summary.trim().is_empty() {
        notification.app_name.clone()
    } else {
        notification.summary.clone()
    };
    let source = if notification.app_name.trim().is_empty() {
        "Notification".to_owned()
    } else {
        notification.app_name.clone()
    };
    let row = Row::new(children![
        Rectangle::new()
            .width(38.0)
            .height(38.0)
            .radius(11.0)
            .fill(theme().chip)
            .align_child(Align::Center, Align::Center)
            .child(
                Rectangle::new()
                    .width(21.0)
                    .height(21.0)
                    .fill(ink_image(icon, theme().icon)),
            ),
        Column::new(children![
            Row::new(children![
                premium_text(source)
                    .color(theme().text_faint)
                    .size(9.5)
                    .weight(Weight::SemiBold)
                    .elide(),
                Rectangle::new()
                    .width(Parent)
                    .height(1.0)
                    .fill(Color::TRANSPARENT),
                premium_text(ago(notification.created_at))
                    .color(theme().text_faint)
                    .size(9.5),
            ])
            .width(Parent)
            .align(Align::Center),
            premium_text(strip_markup(&title))
                .color(theme().text)
                .size(12.0)
                .weight(Weight::SemiBold)
                .elide(),
            premium_text(strip_markup(&notification.body))
                .color(theme().text_muted)
                .size(10.5)
                .wrap()
                .max_lines(2),
        ])
        .width(Parent)
        .height(Parent)
        .gap(2.0)
        .align(Align::Start)
        .justify(Center),
    ])
    .width(Parent)
    .height(Parent)
    .gap(10.0)
    .align(Align::Center);

    Rectangle::new()
        .width(Parent)
        .height(ROW_HEIGHT)
        .radius(12.0)
        .fill(theme().chip)
        .padding(8.0)
        .cursor(Cursor::Grab)
        .on_click(move |_| clear_drag(id))
        .on_drag(move |point| swipe_notification(id, point.x))
        .on_hover(move |inside| {
            if inside {
                clear_drag(id);
            }
        })
        .child(row)
}

fn swipe_notification(id: u32, x: f32) {
    let origins = DRAG_ORIGINS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut origins = origins.lock().unwrap_or_else(PoisonError::into_inner);
    let origin = origins.entry(id).or_insert(x);
    if (x - *origin).abs() >= SWIPE_DISTANCE {
        origins.remove(&id);
        drop(origins);
        notifications::close_notification(id);
        aether::refresh();
    }
}

fn clear_drag(id: u32) {
    if let Some(origins) = DRAG_ORIGINS.get() {
        origins
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }
}

fn notification_bell() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/notification/bell.svg")
}

fn ago(created_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let seconds = now.saturating_sub(created_at);
    match seconds {
        0..=59 => "now".to_owned(),
        60..=3599 => format!("{}m", seconds / 60),
        3600..=86399 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

fn strip_markup(text: &str) -> String {
    let mut plain = String::new();
    let mut inside_tag = false;
    for character in text.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => plain.push(character),
            _ => {}
        }
    }
    plain
}
