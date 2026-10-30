use super::battery_menu;
use aether::{
    Align, Animation, Canvas, Color, Easing, Horizontal, InputArea, Layer, LayerWindow, Margin,
    Padding, Parent, Path, Rectangle, Row, Shape, Size, Stack, Start, Text, Vertical, Weight,
    Widget, Zone, children,
};
use aether_services::battery::info::{BatteryStatus, format_duration};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::panel::svg::battery_svg::BatteryIcon;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::{battery::battery_details, display};
use crate::theme::theme;

const WINDOW_WIDTH: f32 = 330.0;
const WINDOW_HEIGHT: f32 = 52.0;
const EDGE_INSET_RIGHT: f32 = 12.0;

pub(super) const COLLAPSED_BODY_WIDTH: f32 = 42.0;
pub(super) const ISLAND_HEIGHT: f32 = 40.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;

const ICON_BADGE_SIZE: f32 = 22.0;
const ICON_SIZE: f32 = 14.0;

const ANIMATION_DURATION: Duration = super::popup_motion::EXPANSION_DURATION;
static PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
static HOVERED: AtomicBool = AtomicBool::new(false);
static HOVER_GENERATION: AtomicU64 = AtomicU64::new(0);
static TARGET_MONITOR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn target_monitor() -> &'static Mutex<Option<String>> {
    TARGET_MONITOR.get_or_init(|| Mutex::new(None))
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

pub fn set_hovered(hovered: bool) {
    super::popup_motion::set_hovered(hovered, &HOVERED, &HOVER_GENERATION);
}

pub fn hovered() -> bool {
    HOVERED.load(Ordering::Relaxed)
}

pub fn open_battery_on_hover_for_monitor(monitor: String) {
    open_for_monitor(true, Some(monitor));
}

pub fn toggle_battery_for_monitor(monitor: String) {
    let same_monitor = target_monitor()
        .lock()
        .map(|target| target.as_deref() == Some(monitor.as_str()))
        .unwrap_or(false);

    if hovered() && same_monitor {
        close();
    } else {
        open_for_monitor(false, Some(monitor));
    }
}

fn open_for_monitor(_close_on_unhover: bool, monitor: Option<String>) {
    HOVER_GENERATION.fetch_add(1, Ordering::Relaxed);
    HOVERED.store(true, Ordering::Relaxed);
    if let Ok(mut target) = target_monitor().lock() {
        *target = monitor;
    }
    aether::refresh();
}

fn close() {
    HOVER_GENERATION.fetch_add(1, Ordering::Relaxed);
    HOVERED.store(false, Ordering::Relaxed);
    if let Ok(mut target) = target_monitor().lock() {
        *target = None;
    }
    aether::refresh();
}

fn progress() -> f32 {
    let animation = PROGRESS.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(ANIMATION_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    animation.to(if hovered() { 1.0 } else { 0.0 });
    animation.value()
}

pub fn battery_hint(monitor: &aether::Monitor) -> LayerWindow {
    if !is_target_monitor(monitor) {
        return LayerWindow::new()
            .width(WINDOW_WIDTH)
            .height(WINDOW_HEIGHT)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Top)
            .space(Zone::Ignore)
            .namespace("aether-battery")
            .margin(Margin {
                top: PANEL_HEIGHT as i32,
                right: EDGE_INSET_RIGHT as i32,
                bottom: 0,
                left: 0,
            })
            .click_through()
            .child(blank());
    }

    let progress = progress();
    let menu_open = battery_menu::open();
    let window = LayerWindow::new()
        .width(WINDOW_WIDTH)
        .height(WINDOW_HEIGHT)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Top)
        .space(Zone::Ignore)
        .namespace("aether-battery")
        .visible(is_target_monitor(monitor))
        .margin(Margin {
            top: PANEL_HEIGHT as i32,
            right: EDGE_INSET_RIGHT as i32,
            bottom: 0,
            left: 0,
        });

    let Some(battery) = battery_details() else {
        return window.click_through().child(blank());
    };

    if (progress <= 0.001 && !hovered() && !menu_open)
        || (menu_open && (!hovered() || progress <= 0.001))
    {
        return window.click_through().child(blank());
    }

    let theme = theme();
    let handoff = if menu_open {
        battery_menu::transition_progress()
    } else {
        0.0
    };
    let charging = battery.status == BatteryStatus::Charging;
    let estimate = battery_estimate(&battery);

    let estimate_text = Text::new(estimate)
        .font("Maple Mono")
        .color(theme.text_muted)
        .size(11.0)
        .weight(Weight::Medium);
    let (
        current_width,
        current_height,
        current_body_width,
        current_ear_radius,
        current_corner_radius,
    ) = island_metrics(&battery, progress);

    let badge = Rectangle::new()
        .width(ICON_BADGE_SIZE)
        .height(ICON_BADGE_SIZE)
        .radius(ICON_BADGE_SIZE / 2.0)
        .fill(theme.chip)
        .border(1.0, theme.chip_border)
        .align_child(Align::Center, Align::Center)
        .child(
            Rectangle::new()
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .fill(BatteryIcon::from_percentage(battery.percentage).image(charging, false)),
        );

    let content = Row::new(children![
        badge,
        Rectangle::new()
            .width(8.0)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        estimate_text,
    ])
    .width(Parent)
    .height(current_height)
    .align(Align::Center)
    .justify(Start);

    let fill = Box::new(
        build_island_path(
            current_width,
            current_height,
            current_ear_radius,
            current_corner_radius,
        )
        .fill(theme.bar)
        .opacity(progress * (1.0 - handoff)),
    ) as Box<dyn Shape>;
    let border = Box::new(
        build_island_border_path(
            current_width,
            current_height,
            current_ear_radius,
            current_corner_radius,
        )
        .stroke(1.0, theme.surface_border)
        .opacity(progress * (1.0 - handoff)),
    ) as Box<dyn Shape>;
    let canvas = Canvas::new()
        .width(current_width)
        .height(current_height)
        .shapes(vec![fill, border]);

    let content_opacity = ((progress - 0.2) / 0.8).clamp(0.0, 1.0);
    let island = Stack::new(children![
        Rectangle::new()
            .width(current_width)
            .height(current_height)
            .fill(Color::TRANSPARENT)
            .padding(Padding {
                top: 0.0,
                right: current_ear_radius,
                bottom: 0.0,
                left: current_ear_radius,
            })
            .align_child(Align::Center, Align::Start)
            .child(
                Rectangle::new()
                    .width(current_body_width)
                    .height(current_height)
                    .radius_bottom_left(current_corner_radius)
                    .radius_bottom_right(current_corner_radius)
                    .fill(Color::TRANSPARENT)
                    .shadow(super::island_shadow(0.35 * progress * (1.0 - handoff)),),
            ),
        canvas,
        Rectangle::new()
            .width(current_width)
            .height(current_height)
            .fill(Color::TRANSPARENT)
            .opacity(content_opacity * (1.0 - handoff))
            .padding(Padding {
                top: 0.0,
                right: current_ear_radius + 12.0,
                bottom: 0.0,
                left: current_ear_radius + 12.0,
            })
            .align_child(Align::Center, Align::Center)
            .child(content),
    ]);

    let monitor_name = monitor.name.clone();
    let card = Rectangle::new()
        .width(current_width)
        .height(current_height)
        .fill(Color::TRANSPARENT)
        .cursor(aether::Cursor::Pointer)
        .on_hover(set_hovered)
        .on_click(move |_| {
            super::battery_menu::set_target_monitor(Some(monitor_name.clone()));
            battery_menu::toggle_open();
        })
        .child(island);

    let input = InputArea {
        x: (WINDOW_WIDTH - current_width).max(0.0) as i32,
        y: 0,
        width: current_width as i32,
        height: current_height as i32,
    };

    let window = if progress > 0.05 {
        window.input_region(vec![input])
    } else {
        window.click_through()
    };

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            // Align the island's right edge with the battery icon in the panel.
            .align_child(Align::End, Align::Start)
            .child(card),
    )
}

pub(super) fn battery_estimate(battery: &aether_services::battery::info::Battery) -> String {
    match battery.status {
        BatteryStatus::Discharging => battery
            .remaining_time
            .map(|time| format!("{} remaining", format_duration(time)))
            .unwrap_or_else(|| "On battery".to_string()),
        BatteryStatus::Charging => battery
            .time_to_full
            .map(|time| format!("{} to full", format_duration(time)))
            .unwrap_or_else(|| "Charging".to_string()),
        BatteryStatus::Full => "Fully charged".to_string(),
        BatteryStatus::NotCharging => "Not charging".to_string(),
        BatteryStatus::Unknown => "Battery".to_string(),
    }
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}

fn estimate_body_width(text: &Text) -> f32 {
    ICON_BADGE_SIZE + 8.0 + intrinsic_text_width(text) + 24.0
}

pub(super) fn current_island_metrics(
    battery: &aether_services::battery::info::Battery,
) -> (f32, f32, f32, f32) {
    let progress = progress();
    let (width, height, _, ear, corner) = island_metrics(battery, progress);
    (width, height, ear, corner)
}

fn island_metrics(
    battery: &aether_services::battery::info::Battery,
    progress: f32,
) -> (f32, f32, f32, f32, f32) {
    let height = (ISLAND_HEIGHT * progress).max(2.0);
    let text = Text::new(battery_estimate(battery))
        .font("Maple Mono")
        .size(11.0)
        .weight(Weight::Medium);
    let target_body_width = estimate_body_width(&text);
    let body_width = COLLAPSED_BODY_WIDTH + (target_body_width - COLLAPSED_BODY_WIDTH) * progress;
    let ear = (EAR_RADIUS * progress).min(height * 0.44);
    let corner = (CORNER_RADIUS * progress).min(height * 0.44);

    (body_width + 2.0 * ear, height, body_width, ear, corner)
}

fn intrinsic_text_width(text: &Text) -> f32 {
    let Size::Fixed(text_width) = text.width() else {
        unreachable!("battery estimate text must have intrinsic width");
    };

    text_width
}

fn build_island_path(width: f32, height: f32, ear_radius: f32, bottom_radius: f32) -> Path {
    let ke = 0.55228475 * ear_radius;
    let kb = 0.55228475 * bottom_radius;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(ke, 0.0, ear_radius, ear_radius - ke, ear_radius, ear_radius)
        .line_to(ear_radius, height - bottom_radius)
        .cubic_to(
            ear_radius,
            height - bottom_radius + kb,
            ear_radius + bottom_radius - kb,
            height,
            ear_radius + bottom_radius,
            height,
        )
        .line_to(width - ear_radius - bottom_radius, height)
        .cubic_to(
            width - ear_radius - bottom_radius + kb,
            height,
            width - ear_radius,
            height - bottom_radius + kb,
            width - ear_radius,
            height - bottom_radius,
        )
        .line_to(width - ear_radius, ear_radius)
        .cubic_to(
            width - ear_radius,
            ear_radius - ke,
            width - ke,
            0.0,
            width,
            0.0,
        )
        .line_to(0.0, 0.0)
        .close()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_width_tracks_the_rendered_text_width() {
        let short = Text::new("Charging")
            .font("Maple Mono")
            .size(11.0)
            .weight(Weight::Medium);
        let long = Text::new("3 hours 25 minutes to full")
            .font("Maple Mono")
            .size(11.0)
            .weight(Weight::Medium);

        assert!(estimate_body_width(&long) > estimate_body_width(&short));
        assert_eq!(
            estimate_body_width(&long) - estimate_body_width(&short),
            intrinsic_text_width(&long) - intrinsic_text_width(&short)
        );
    }
}

fn build_island_border_path(width: f32, height: f32, ear_radius: f32, bottom_radius: f32) -> Path {
    let ke = 0.55228475 * ear_radius;
    let kb = 0.55228475 * bottom_radius;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(ke, 0.0, ear_radius, ear_radius - ke, ear_radius, ear_radius)
        .line_to(ear_radius, height - bottom_radius)
        .cubic_to(
            ear_radius,
            height - bottom_radius + kb,
            ear_radius + bottom_radius - kb,
            height,
            ear_radius + bottom_radius,
            height,
        )
        .line_to(width - ear_radius - bottom_radius, height)
        .cubic_to(
            width - ear_radius - bottom_radius + kb,
            height,
            width - ear_radius,
            height - bottom_radius + kb,
            width - ear_radius,
            height - bottom_radius,
        )
        .line_to(width - ear_radius, ear_radius)
        .cubic_to(
            width - ear_radius,
            ear_radius - ke,
            width - ke,
            0.0,
            width,
            0.0,
        )
}
