use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Column, Cursor, Easing, Horizontal, Key, Keyboard, Layer,
    LayerWindow, Padding, Parent, Path, Rectangle, Row, Shape, SpaceBetween, Stack, Text, Vertical,
    Weight, Zone, children,
};

use aether_services::battery::info::{BatteryStatus, format_duration};
use aether_services::power::{PowerProfile, PowerProfileService};

use super::battery_popup::{COLLAPSED_BODY_WIDTH, current_island_metrics};
use crate::panel::svg::battery_svg::BatteryIcon;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::battery::battery_details;
use crate::theme::theme;

const BOX_WIDTH: f32 = 330.0;
const BOX_PADDING: f32 = 18.0;
const BOX_RADIUS: f32 = 14.0;
const EAR_RADIUS: f32 = 13.0;
const EDGE_RIGHT: f32 = 12.0;

const HEADER_HEIGHT: f32 = 22.0;
const SUMMARY_HEIGHT: f32 = 76.0;
const PROGRESS_HEIGHT: f32 = 5.0;
const DETAIL_HEIGHT: f32 = 24.0;
const PROFILE_HEIGHT: f32 = 82.0;
const PROFILE_FEEDBACK_HEIGHT: f32 = 16.0;
const GAP: f32 = 14.0;

const CONTENT_HEIGHT: f32 = HEADER_HEIGHT
    + GAP
    + SUMMARY_HEIGHT
    + GAP
    + PROGRESS_HEIGHT
    + GAP
    + DETAIL_HEIGHT * 2.0
    + GAP
    + HEADER_HEIGHT
    + GAP
    + PROFILE_HEIGHT
    + GAP
    + PROFILE_FEEDBACK_HEIGHT;

const BOX_HEIGHT: f32 = CONTENT_HEIGHT + BOX_PADDING * 2.0;

static OPEN: AtomicBool = AtomicBool::new(false);
static BUSY: AtomicBool = AtomicBool::new(false);
static ANIMATION: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
static PROFILE: OnceLock<Mutex<Option<PowerProfile>>> = OnceLock::new();
static ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn text(value: impl Into<String>) -> Text {
    Text::new(value).font("Maple Mono")
}

fn profile_state() -> &'static Mutex<Option<PowerProfile>> {
    PROFILE.get_or_init(|| Mutex::new(None))
}

fn error_state() -> &'static Mutex<Option<String>> {
    ERROR.get_or_init(|| Mutex::new(None))
}

pub fn open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

pub fn toggle_open() {
    if open() {
        close();
        return;
    }

    OPEN.store(true, Ordering::Relaxed);
    refresh_profile();
    aether::refresh();
}

pub fn close() {
    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

fn animation_progress() -> f32 {
    let animation = ANIMATION.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(Duration::from_millis(260))
                .easing(Easing::Out),
        )
    });

    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    animation.to(if open() { 1.0 } else { 0.0 });
    animation.value()
}

pub(super) fn transition_progress() -> f32 {
    animation_progress()
}

fn refresh_profile() {
    crate::services::runtime::runtime().spawn(async {
        let result =
            tokio::task::spawn_blocking(|| PowerProfileService::new().active_profile()).await;

        match result {
            Ok(Ok(profile)) => {
                if let Ok(mut state) = profile_state().lock() {
                    *state = Some(profile);
                }
                if let Ok(mut error) = error_state().lock() {
                    *error = None;
                }
            }
            Ok(Err(error)) => {
                if let Ok(mut message) = error_state().lock() {
                    *message = Some(error.to_string());
                }
            }
            Err(error) => {
                if let Ok(mut message) = error_state().lock() {
                    *message = Some(error.to_string());
                }
            }
        }

        aether::refresh();
    });
}

fn select_profile(requested: PowerProfile) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }

    if let Ok(mut error) = error_state().lock() {
        *error = None;
    }

    aether::refresh();

    crate::services::runtime::runtime().spawn(async move {
        let result =
            tokio::task::spawn_blocking(move || PowerProfileService::new().set_profile(requested))
                .await;

        match result {
            Ok(Ok(change)) => {
                if let Ok(mut state) = profile_state().lock() {
                    *state = Some(change.active);
                }

                if let Ok(mut error) = error_state().lock() {
                    *error = None;
                }

                if change.previous != Some(change.active) {
                    // Integrate with your existing Aether notification
                    // publisher here, after the real profile change.
                    //
                    // "Power mode changed"
                    // change.active.label()
                }
            }
            other => {
                let message = match other {
                    Ok(Err(error)) => error.to_string(),
                    Err(error) => error.to_string(),
                    _ => unreachable!(),
                };

                if let Ok(mut error) = error_state().lock() {
                    *error = Some(message);
                }

                // Re-read actual state after failure.
                if let Ok(profile) = PowerProfileService::new().active_profile() {
                    if let Ok(mut state) = profile_state().lock() {
                        *state = Some(profile);
                    }
                }
            }
        }

        BUSY.store(false, Ordering::SeqCst);
        aether::refresh();
    });
}

fn island_path(w: f32, h: f32, ear: f32, bottom: f32) -> Path {
    let ke = 0.55228475 * ear;
    let kb = 0.55228475 * bottom;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(ke, 0.0, ear, ear - ke, ear, ear)
        .line_to(ear, h - bottom)
        .cubic_to(ear, h - bottom + kb, ear + bottom - kb, h, ear + bottom, h)
        .line_to(w - ear - bottom, h)
        .cubic_to(
            w - ear - bottom + kb,
            h,
            w - ear,
            h - bottom + kb,
            w - ear,
            h - bottom,
        )
        .line_to(w - ear, ear)
        .cubic_to(w - ear, ear - ke, w - ke, 0.0, w, 0.0)
        .line_to(0.0, 0.0)
        .close()
}

fn border_path(w: f32, h: f32, ear: f32, bottom: f32) -> Path {
    let ke = 0.55228475 * ear;
    let kb = 0.55228475 * bottom;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(ke, 0.0, ear, ear - ke, ear, ear)
        .line_to(ear, h - bottom)
        .cubic_to(ear, h - bottom + kb, ear + bottom - kb, h, ear + bottom, h)
        .line_to(w - ear - bottom, h)
        .cubic_to(
            w - ear - bottom + kb,
            h,
            w - ear,
            h - bottom + kb,
            w - ear,
            h - bottom,
        )
        .line_to(w - ear, ear)
        .cubic_to(w - ear, ear - ke, w - ke, 0.0, w, 0.0)
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}

pub fn battery_menu() -> LayerWindow {
    let progress = animation_progress();

    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-battery-menu")
        .keyboard(Keyboard::OnDemand)
        .on_key(|key| {
            if key == Key::Escape {
                close();
            }
        });

    if progress <= 0.001 && !open() {
        return window.click_through().child(blank());
    }

    let t = theme();
    let (hint_width, hint_height, hint_ear, hint_corner) = battery_details()
        .as_ref()
        .map(current_island_metrics)
        .unwrap_or((COLLAPSED_BODY_WIDTH, 2.0, 0.0, 0.0));
    let full_width = BOX_WIDTH + EAR_RADIUS * 2.0;

    // Expand from the exact dimensions of the battery hover island.
    let current_width = hint_width + (full_width - hint_width) * progress;
    let current_height = hint_height + (BOX_HEIGHT - hint_height) * progress;
    let ear = hint_ear + (EAR_RADIUS - hint_ear) * progress;
    let corner = hint_corner + (BOX_RADIUS - hint_corner) * progress;

    let fill = Box::new(
        island_path(current_width, current_height, ear, corner)
            .fill(t.bar)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let border = Box::new(
        border_path(current_width, current_height, ear, corner)
            .stroke(1.0, t.surface_border)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let canvas = Canvas::new()
        .width(current_width)
        .height(current_height)
        .shapes(vec![fill, border]);

    let opacity = ((progress - 0.20) / 0.80).clamp(0.0, 1.0);

    let card = Rectangle::new()
        .width(current_width)
        .height(current_height)
        .fill(Color::TRANSPARENT)
        .clip()
        .on_click(|_| {})
        .child(Stack::new(children![
            Rectangle::new()
                .width(current_width)
                .height(current_height)
                .fill(Color::TRANSPARENT)
                .padding(Padding {
                    top: 0.0,
                    right: ear,
                    bottom: 0.0,
                    left: ear,
                })
                .align_child(Align::Center, Align::Start)
                .child(
                    Rectangle::new()
                        .width(BOX_WIDTH)
                        .height(current_height)
                        .fill(Color::TRANSPARENT)
                        .radius_bottom_left(corner)
                        .radius_bottom_right(corner)
                        .shadow(super::island_shadow(0.35 * progress),),
                ),
            canvas,
            Rectangle::new()
                .width(current_width)
                .height(current_height)
                .fill(Color::TRANSPARENT)
                .opacity(opacity)
                .padding(Padding {
                    top: BOX_PADDING,
                    right: ear + BOX_PADDING,
                    bottom: BOX_PADDING,
                    left: ear + BOX_PADDING,
                })
                .align_child(Align::Start, Align::Start)
                .child(menu_body()),
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
                    .padding(Padding {
                        top: PANEL_HEIGHT,
                        right: EDGE_RIGHT,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .align_child(Align::End, Align::Start)
                    .child(card),
            ),
    )
}

fn menu_body() -> Column {
    let t = theme();

    Column::new(children![
        text("Battery")
            .size(14.0)
            .weight(Weight::SemiBold)
            .color(t.text),
        battery_summary(),
        battery_progress(),
        battery_details_rows(),
        text("Power profile")
            .size(13.0)
            .weight(Weight::SemiBold)
            .color(t.text),
        profile_buttons(),
        profile_feedback(),
    ])
    .width(Parent)
    .height(CONTENT_HEIGHT)
    .gap(GAP)
    .align(Align::Start)
}

fn battery_summary() -> Row {
    let t = theme();

    let battery = battery_details();

    let (percentage, status, estimate, charging) = if let Some(battery) = battery {
        let estimate = match battery.status {
            BatteryStatus::Discharging => battery
                .remaining_time
                .map(|d| format!("{} remaining", format_duration(d)))
                .unwrap_or_else(|| "On battery".into()),
            BatteryStatus::Charging => battery
                .time_to_full
                .map(|d| format!("{} to full", format_duration(d)))
                .unwrap_or_else(|| "Charging".into()),
            BatteryStatus::Full => "Fully charged".into(),
            BatteryStatus::NotCharging => "Not charging".into(),
            BatteryStatus::Unknown => "Unknown".into(),
        };

        (
            battery.percentage,
            format!("{:?}", battery.status),
            estimate,
            battery.status == BatteryStatus::Charging,
        )
    } else {
        (0, "Unavailable".into(), "No battery data".into(), false)
    };

    let icon = BatteryIcon::from_percentage(percentage).image(charging, false);

    Row::new(children![
        Rectangle::new()
            .width(62.0)
            .height(62.0)
            .fill(t.chip)
            .border(1.0, t.chip_border)
            .radius(12.0)
            .align_child(Align::Center, Align::Center)
            .child(Rectangle::new().width(32.0).height(32.0).fill(icon),),
        Column::new(children![
            text(format!("{percentage}%"))
                .size(27.0)
                .weight(Weight::SemiBold)
                .color(t.text),
            text(status).size(11.0).color(t.text_muted),
            text(estimate).size(10.0).color(t.text_faint),
        ])
        .gap(3.0),
    ])
    .width(Parent)
    .height(SUMMARY_HEIGHT)
    .gap(14.0)
    .align(Align::Center)
}

fn battery_progress() -> Rectangle {
    let t = theme();
    let percentage = battery_details()
        .map(|battery| battery.percentage)
        .unwrap_or(0);

    let track_width = BOX_WIDTH - BOX_PADDING * 2.0;

    Rectangle::new()
        .width(track_width)
        .height(PROGRESS_HEIGHT)
        .radius(PROGRESS_HEIGHT / 2.0)
        .fill(t.track)
        .align_child(Align::Start, Align::Center)
        .child(
            Rectangle::new()
                .width(track_width * (percentage as f32 / 100.0).clamp(0.0, 1.0))
                .height(PROGRESS_HEIGHT)
                .radius(PROGRESS_HEIGHT / 2.0)
                .fill(t.accent),
        )
}

fn detail_row(label: &str, value: String) -> Row {
    let t = theme();

    Row::new(children![
        text(label).size(11.0).color(t.text_muted),
        text(value).size(11.0).weight(Weight::Medium).color(t.text),
    ])
    .width(Parent)
    .height(DETAIL_HEIGHT)
    .align(Align::Center)
    .justify(SpaceBetween)
}

fn battery_details_rows() -> Column {
    let battery = battery_details();

    let status = battery
        .as_ref()
        .map(|b| format!("{:?}", b.status))
        .unwrap_or_else(|| "Unavailable".into());

    let percentage = battery
        .as_ref()
        .map(|b| format!("{}%", b.percentage))
        .unwrap_or_else(|| "—".into());

    Column::new(children![
        detail_row("Status", status),
        detail_row("Charge level", percentage),
    ])
    .width(Parent)
    .height(DETAIL_HEIGHT * 2.0)
}

fn profile_buttons() -> Row {
    let active = profile_state().lock().ok().and_then(|state| *state);

    let busy = BUSY.load(Ordering::Relaxed);

    let card_width = (BOX_WIDTH - BOX_PADDING * 2.0 - 16.0) / 3.0;

    Row::new(children![
        profile_card(
            PowerProfile::Saver,
            "Saver",
            "leaf.svg",
            active == Some(PowerProfile::Saver),
            busy,
            card_width,
        ),
        profile_card(
            PowerProfile::Balanced,
            "Balanced",
            "scale.svg",
            active == Some(PowerProfile::Balanced),
            busy,
            card_width,
        ),
        profile_card(
            PowerProfile::Performance,
            "Performance",
            "rocket.svg",
            active == Some(PowerProfile::Performance),
            busy,
            card_width,
        ),
    ])
    .width(Parent)
    .height(PROFILE_HEIGHT)
    .gap(8.0)
    .align(Align::Center)
}

fn profile_feedback() -> Text {
    let error = error_state()
        .lock()
        .ok()
        .and_then(|message| message.clone());
    let message = error.clone().unwrap_or_else(|| {
        if BUSY.load(Ordering::Relaxed) {
            "Applying power profile…".to_string()
        } else {
            String::new()
        }
    });

    text(message).size(10.0).elide().color(if error.is_some() {
        Color::rgba(248, 113, 113, 255)
    } else {
        theme().text_muted
    })
}

fn profile_card(
    profile: PowerProfile,
    label: &str,
    icon_filename: &str,
    selected: bool,
    busy: bool,
    width: f32,
) -> Rectangle {
    let t = theme();

    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/profile")
        .join(icon_filename);

    let ink = if selected { t.accent } else { t.text_muted };

    let icon = crate::panel::svg::recolor::ink_image(path, ink);

    Rectangle::new()
        .width(width)
        .height(PROFILE_HEIGHT)
        .fill(if selected { t.accent_faint } else { t.chip })
        .border(1.0, if selected { t.accent } else { t.chip_border })
        .radius(10.0)
        .cursor(Cursor::Pointer)
        .on_click(move |_| {
            if !busy {
                select_profile(profile);
            }
        })
        .align_child(Align::Center, Align::Center)
        .child(
            Column::new(children![
                Rectangle::new().width(22.0).height(22.0).fill(icon),
                text(label)
                    .size(10.0)
                    .weight(Weight::Medium)
                    .color(if selected { t.accent } else { t.text }),
            ])
            .gap(9.0)
            .align(Align::Center),
        )
}
