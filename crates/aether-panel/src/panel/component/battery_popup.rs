use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Easing, Horizontal, InputArea, Layer, LayerWindow, Margin,
    Padding, Parent, Path, Rectangle, Row, Shadow, Shape, Stack, Start, Text, Vertical, Weight,
    Zone, children,
};
use aether_services::battery::info::{BatteryStatus, format_duration};

use crate::panel::svg::battery_svg::BatteryIcon;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::battery::battery_details;
use crate::theme::theme;

const WINDOW_WIDTH: f32 = 330.0;
const WINDOW_HEIGHT: f32 = 52.0;
const EDGE_INSET_RIGHT: f32 = 12.0;

const MIN_BODY_WIDTH: f32 = 42.0;
const MAX_BODY_WIDTH: f32 = 210.0;
const ISLAND_HEIGHT: f32 = 40.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;

const ICON_BADGE_SIZE: f32 = 22.0;
const ICON_SIZE: f32 = 14.0;

const ANIMATION_DURATION: Duration = Duration::from_millis(220);
const HOVER_DELAY: Duration = Duration::from_millis(140);
static PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
static HOVERED: AtomicBool = AtomicBool::new(false);
static HOVER_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn set_hovered(hovered: bool) {
    if hovered {
        HOVER_GENERATION.fetch_add(1, Ordering::Relaxed);
        if !HOVERED.swap(true, Ordering::Relaxed) {
            aether::refresh();
        }
        return;
    }

    let generation = HOVER_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    crate::services::runtime::runtime().spawn(async move {
        tokio::time::sleep(HOVER_DELAY).await;
        if HOVER_GENERATION.load(Ordering::Relaxed) == generation
            && HOVERED.swap(false, Ordering::Relaxed)
        {
            aether::refresh();
        }
    });
}

pub fn hovered() -> bool {
    HOVERED.load(Ordering::Relaxed)
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

pub fn battery_hint() -> LayerWindow {
    let progress = progress();
    let window = LayerWindow::new()
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
        });

    let Some(battery) = battery_details() else {
        return window.click_through().child(blank());
    };

    if progress <= 0.001 && !hovered() {
        return window.click_through().child(blank());
    }

    let theme = theme();
    let current_height = (ISLAND_HEIGHT * progress).max(2.0);
    let current_body_width = MIN_BODY_WIDTH + (MAX_BODY_WIDTH - MIN_BODY_WIDTH) * progress;
    let current_ear_radius = (EAR_RADIUS * progress).min(current_height * 0.44);
    let current_corner_radius = (CORNER_RADIUS * progress).min(current_height * 0.44);
    let current_width = current_body_width + 2.0 * current_ear_radius;

    let charging = battery.status == BatteryStatus::Charging;
    let estimate = match battery.status {
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
    };

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
        Text::new(estimate)
            .font("Maple Mono")
            .color(theme.text_muted)
            .size(11.0)
            .weight(Weight::Medium)
            .elide(),
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
        .opacity(progress),
    ) as Box<dyn Shape>;
    let border = Box::new(
        build_island_border_path(
            current_width,
            current_height,
            current_ear_radius,
            current_corner_radius,
        )
        .stroke(1.0, theme.surface_border)
        .opacity(progress),
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
                    .shadow(
                        Shadow::drop(theme.shadow)
                            .opacity(0.35 * progress)
                            .blur(14.0)
                            .offset(0.0, 4.0),
                    ),
            ),
        canvas,
        Rectangle::new()
            .width(current_width)
            .height(current_height)
            .fill(Color::TRANSPARENT)
            .opacity(content_opacity)
            .padding(Padding {
                top: 0.0,
                right: current_ear_radius + 12.0,
                bottom: 0.0,
                left: current_ear_radius + 12.0,
            })
            .align_child(Align::Center, Align::Center)
            .child(content),
    ]);

    let card = Rectangle::new()
        .width(current_width)
        .height(current_height)
        .fill(Color::TRANSPARENT)
        .on_hover(set_hovered)
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

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
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
