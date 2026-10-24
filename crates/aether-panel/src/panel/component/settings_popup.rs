use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Column, Cursor, Easing, Horizontal, InputArea, Layer,
    LayerWindow, Margin, Padding, Parent, Path, Radius, Rectangle, Row, Shape, Stack, Text,
    Vertical, Weight, Zone, children,
};

use crate::panel::top::PANEL_HEIGHT;
use crate::services::{runtime::runtime, wifi};
use crate::theme::theme;

const WIDTH: f32 = 192.0;
const HEIGHT: f32 = 344.0;
const COLLAPSED_WIDTH: f32 = 42.0;
const COLLAPSED_HEIGHT: f32 = 40.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;
const ANIMATION_DURATION: Duration = super::popup_motion::EXPANSION_DURATION;
const BUTTON_SIZE: f32 = 44.0;
const BUTTON_ICON_SIZE: f32 = 22.0;
const SLIDER_HEIGHT: f32 = 108.0;
const SLIDER_TOUCH_HEIGHT: f32 = 132.0;
const SLIDER_WIDTH: f32 = 54.0;

static HOVERED: AtomicBool = AtomicBool::new(false);
static HOVER_GENERATION: AtomicU64 = AtomicU64::new(0);
static PROGRESS: OnceLock<std::sync::Mutex<Animation<f32>>> = OnceLock::new();
static BLUETOOTH: OnceLock<RwLock<Option<bool>>> = OnceLock::new();
static BLUETOOTH_WATCHER: OnceLock<()> = OnceLock::new();
static LEVELS: OnceLock<RwLock<Levels>> = OnceLock::new();
static LEVEL_WATCHER: OnceLock<()> = OnceLock::new();

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Levels {
    volume: Option<u8>,
    brightness: Option<u8>,
}

pub fn set_hovered(hovered: bool) {
    super::popup_motion::set_hovered(hovered, &HOVERED, &HOVER_GENERATION);
}

fn progress() -> f32 {
    let animation = PROGRESS.get_or_init(|| {
        std::sync::Mutex::new(
            Animation::new(0.0)
                .duration(ANIMATION_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    animation.to(if HOVERED.load(Ordering::Relaxed) {
        1.0
    } else {
        0.0
    });
    animation.value()
}

fn bluetooth_cache() -> &'static RwLock<Option<bool>> {
    BLUETOOTH.get_or_init(|| RwLock::new(None))
}

fn bluetooth_state() -> Option<bool> {
    BLUETOOTH_WATCHER.get_or_init(|| {
        runtime().spawn(async {
            let mut ticker = tokio::time::interval(Duration::from_secs(3));
            loop {
                ticker.tick().await;
                if !HOVERED.load(Ordering::Relaxed) {
                    continue;
                }
                let state = tokio::task::spawn_blocking(aether_services::bluetooth::powered)
                    .await
                    .ok()
                    .and_then(Result::ok);
                if let Ok(mut cached) = bluetooth_cache().write()
                    && *cached != state
                {
                    *cached = state;
                    aether::refresh();
                }
            }
        });
    });

    bluetooth_cache().read().ok().and_then(|state| *state)
}

fn toggle_bluetooth() {
    runtime().spawn(async move {
        let result = tokio::task::spawn_blocking(aether_services::bluetooth::toggle_power).await;

        match result {
            Ok(Ok(enabled)) => {
                if let Ok(mut state) = bluetooth_cache().write() {
                    *state = Some(enabled);
                }
                aether::refresh();
            }
            Ok(Err(error)) => eprintln!("aether-panel: Bluetooth toggle failed: {error}"),
            Err(error) => eprintln!("aether-panel: Bluetooth task failed: {error}"),
        }
    });
}

fn toggle_wifi() {
    wifi::toggle_radio();
}

fn levels() -> Levels {
    LEVEL_WATCHER.get_or_init(|| {
        runtime().spawn(async {
            let mut ticker = tokio::time::interval(Duration::from_millis(700));
            loop {
                ticker.tick().await;
                if !HOVERED.load(Ordering::Relaxed) {
                    continue;
                }
                let levels = tokio::task::spawn_blocking(|| {
                    use aether_services::osd::{AudioService, BrightnessService};
                    let audio = AudioService::current().ok();
                    Levels {
                        volume: audio.map(|state| state.volume_percent),
                        brightness: BrightnessService::current().ok().map(|state| state.percent),
                    }
                })
                .await;

                let Ok(levels) = levels else {
                    continue;
                };
                if let Ok(mut current) = levels_cache().write()
                    && *current != levels
                {
                    *current = levels;
                    aether::refresh();
                }
            }
        });
    });

    levels_cache()
        .read()
        .map(|levels| *levels)
        .unwrap_or_default()
}

fn levels_cache() -> &'static RwLock<Levels> {
    LEVELS.get_or_init(|| RwLock::new(Levels::default()))
}

fn set_volume(percent: u8) {
    set_level(percent, true);
}

fn set_brightness(percent: u8) {
    set_level(percent, false);
}

fn set_level(percent: u8, volume: bool) {
    let percent = percent.min(100);
    if let Ok(mut levels) = levels_cache().write() {
        if volume {
            levels.volume = Some(percent);
        } else {
            levels.brightness = Some(percent);
        }
    }
    aether::refresh();

    runtime().spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            if volume {
                aether_services::osd::AudioService::set_volume(percent).map(|_| ())
            } else {
                aether_services::osd::BrightnessService::set_percent(percent).map(|_| ())
            }
        })
        .await;

        match result {
            Ok(Ok(())) => aether::refresh(),
            Ok(Err(error)) => eprintln!("aether-panel: failed to set level: {error}"),
            Err(error) => eprintln!("aether-panel: level task failed: {error}"),
        }
    });
}

fn icon_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons")
        .join(name)
}

fn button(icon: &str, enabled: bool, available: bool, action: fn()) -> Rectangle {
    let theme = theme();
    Rectangle::new()
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .fill(if enabled && available {
            theme.accent_faint
        } else {
            theme.chip
        })
        .border(
            1.0,
            if enabled && available {
                theme.accent
            } else {
                theme.chip_border
            },
        )
        .radius(Radius::Full)
        .cursor(if available {
            Cursor::Pointer
        } else {
            Cursor::Default
        })
        .align_child(Align::Center, Align::Center)
        .on_click(move |_| {
            if available {
                action();
            }
        })
        .child(
            Rectangle::new()
                .width(BUTTON_ICON_SIZE)
                .height(BUTTON_ICON_SIZE)
                .fill(crate::panel::svg::recolor::ink_image(
                    icon_path(icon),
                    if available {
                        theme.text
                    } else {
                        theme.text_faint
                    },
                )),
        )
}

fn toggle_control(icon: &str, enabled: bool, available: bool, action: fn()) -> Rectangle {
    button(icon, enabled, available, action)
}

fn status_badge(icon: &str, available: bool) -> Rectangle {
    let theme = theme();
    Rectangle::new()
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .fill(theme.chip)
        .border(1.0, theme.chip_border)
        .radius(Radius::Full)
        .align_child(Align::Center, Align::Center)
        .child(
            Rectangle::new()
                .width(BUTTON_ICON_SIZE)
                .height(BUTTON_ICON_SIZE)
                .fill(crate::panel::svg::recolor::ink_image(
                    icon_path(icon),
                    if available {
                        theme.text
                    } else {
                        theme.text_faint
                    },
                )),
        )
}

fn slider(label: &'static str, percent: Option<u8>, volume: bool) -> Column {
    let theme = theme();
    let value = percent.unwrap_or(0).min(100);
    let filled_height = SLIDER_HEIGHT * value as f32 / 100.0;
    let icon_asset = if volume {
        if value == 0 {
            "osd/mute.svg"
        } else {
            "osd/volume_full.svg"
        }
    } else if value < 50 {
        "osd/sun-dim.svg"
    } else {
        "osd/sun.svg"
    };
    let setter = if volume { set_volume } else { set_brightness };

    Column::new(children![
        Rectangle::new()
            .width(24.0)
            .height(24.0)
            .fill(Color::TRANSPARENT)
            .align_child(Align::Center, Align::Center)
            .child(Rectangle::new().width(17.0).height(17.0).fill(
                crate::panel::svg::recolor::ink_image(icon_path(icon_asset), theme.text,)
            ),),
        Rectangle::new()
            .width(Parent)
            .height(SLIDER_TOUCH_HEIGHT)
            .fill(Color::TRANSPARENT)
            .cursor(Cursor::Pointer)
            .on_drag(move |point| {
                let track_top = (SLIDER_TOUCH_HEIGHT - SLIDER_HEIGHT) / 2.0;
                let percent = ((1.0 - (point.y - track_top) / SLIDER_HEIGHT).clamp(0.0, 1.0)
                    * 100.0)
                    .round() as u8;
                setter(percent);
            })
            .align_child(Align::Center, Align::Center)
            .child(Stack::new(vec![
                Box::new(
                    Rectangle::new()
                        .width(10.0)
                        .height(SLIDER_HEIGHT)
                        .fill(theme.track)
                        .radius(Radius::Full),
                ) as Box<dyn aether::Widget>,
                Box::new(
                    Rectangle::new()
                        .width(10.0)
                        .height(SLIDER_HEIGHT)
                        .fill(Color::TRANSPARENT)
                        .align_child(Align::Center, Align::End)
                        .child(
                            Rectangle::new()
                                .width(10.0)
                                .height(filled_height.max(2.0))
                                .fill(if percent.is_some() {
                                    theme.accent
                                } else {
                                    theme.chip_border
                                })
                                .radius(Radius::Full),
                        ),
                ) as Box<dyn aether::Widget>,
            ])),
        Text::new(percent.map_or("--%".into(), |value| format!("{value}%")))
            .font("Maple Mono")
            .size(11.0)
            .weight(Weight::SemiBold)
            .color(theme.text),
        Text::new(label)
            .font("Maple Mono")
            .size(9.0)
            .color(theme.text_muted),
    ])
    .width(SLIDER_WIDTH)
    .gap(2.0)
    .align(Align::Center)
}

fn controls() -> Column {
    let bluetooth_enabled = bluetooth_state();
    let levels = levels();

    Column::new(children![
        Row::new(children![
            toggle_control(
                if wifi::radio() {
                    "panel/wifi.svg"
                } else {
                    "panel/wifi-off.svg"
                },
                wifi::radio(),
                true,
                toggle_wifi,
            ),
            toggle_control(
                if bluetooth_enabled == Some(true) {
                    "panel/bluetooth.svg"
                } else {
                    "panel/bluetooth-off.svg"
                },
                bluetooth_enabled == Some(true),
                true,
                toggle_bluetooth,
            ),
        ])
        .width(Parent)
        .gap(16.0)
        .align(Align::Center)
        .justify(aether::Center),
        Row::new(children![
            status_badge("panel/moon.svg", true),
            status_badge("panel/plane.svg", true),
        ])
        .width(Parent)
        .gap(16.0)
        .align(Align::Center)
        .justify(aether::Center),
        Rectangle::new()
            .width(Parent)
            .height(1.0)
            .fill(theme().chip_border),
        Row::new(children![
            slider("Volume", levels.volume, true),
            slider("Brightness", levels.brightness, false),
        ])
        .width(Parent)
        .gap(14.0)
        .align(Align::Center)
        .justify(aether::Center),
    ])
    .width(Parent)
    .gap(7.0)
    .align(Align::Center)
}

pub fn settings_hint() -> LayerWindow {
    let progress = progress();
    let visible = HOVERED.load(Ordering::Relaxed);
    let window = LayerWindow::new()
        .width(WIDTH)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Top)
        .space(Zone::Ignore)
        .namespace("aether-settings-hint")
        .margin(Margin {
            top: PANEL_HEIGHT as i32,
            right: 0,
            bottom: 0,
            left: 0,
        });

    if progress <= 0.001 && !visible {
        return window.click_through().child(blank());
    }

    let theme = theme();
    let (current_width, current_height, ear_radius, corner_radius) = island_metrics(progress);
    let fill = Box::new(
        island_path(current_width, current_height, ear_radius, corner_radius)
            .fill(theme.bar)
            .opacity(progress),
    ) as Box<dyn Shape>;
    let border = Box::new(
        island_path(current_width, current_height, ear_radius, corner_radius)
            .stroke(1.0, theme.surface_border)
            .opacity(progress),
    ) as Box<dyn Shape>;
    let canvas = Canvas::new()
        .width(current_width)
        .height(current_height)
        .shapes(vec![fill, border]);
    let shadow = Rectangle::new()
        .width(current_width - ear_radius)
        .height(current_height)
        .fill(Color::TRANSPARENT)
        .radius_bottom_left(corner_radius)
        .shadow(
            aether::Shadow::drop(theme.shadow)
                .opacity(0.26 * progress)
                .blur(18.0)
                .offset(0.0, 4.0),
        );
    let content_opacity = ((progress - 0.25) / 0.75).clamp(0.0, 1.0);

    window
        .input_region(vec![InputArea {
            x: (WIDTH - current_width).round() as i32,
            y: 0,
            width: current_width.ceil() as i32,
            height: current_height.ceil() as i32,
        }])
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Color::TRANSPARENT)
                .on_hover(set_hovered)
                .child(Stack::new(vec![
                    Box::new(
                        Rectangle::new()
                            .width(WIDTH)
                            .height(HEIGHT)
                            .fill(Color::TRANSPARENT)
                            .padding(Padding {
                                top: 0.0,
                                right: 0.0,
                                bottom: 0.0,
                                left: ear_radius,
                            })
                            .align_child(Align::End, Align::Start)
                            .child(shadow),
                    ) as Box<dyn aether::Widget>,
                    Box::new(
                        Rectangle::new()
                            .width(WIDTH)
                            .height(HEIGHT)
                            .fill(Color::TRANSPARENT)
                            .align_child(Align::End, Align::Start)
                            .child(canvas),
                    ) as Box<dyn aether::Widget>,
                    Box::new(
                        Rectangle::new()
                            .width(WIDTH)
                            .height(HEIGHT)
                            .fill(Color::TRANSPARENT)
                            .opacity(content_opacity)
                            .padding(Padding {
                                top: 10.0,
                                right: 12.0,
                                bottom: 10.0,
                                left: EAR_RADIUS + 12.0,
                            })
                            .align_child(Align::End, Align::Start)
                            .child(controls()),
                    ) as Box<dyn aether::Widget>,
                ])),
        )
}

fn island_metrics(progress: f32) -> (f32, f32, f32, f32) {
    let progress = progress.clamp(0.0, 1.0);
    let width = COLLAPSED_WIDTH + (WIDTH - COLLAPSED_WIDTH) * progress;
    let height = COLLAPSED_HEIGHT + (HEIGHT - COLLAPSED_HEIGHT) * progress;
    let ear_radius = (EAR_RADIUS * progress).min(height * 0.44);
    let corner_radius = (CORNER_RADIUS * progress).min(height * 0.44);

    (width, height, ear_radius, corner_radius)
}

fn island_path(width: f32, height: f32, ear_radius: f32, corner_radius: f32) -> Path {
    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(
            0.552_284_75 * ear_radius,
            0.0,
            ear_radius,
            ear_radius - 0.552_284_75 * ear_radius,
            ear_radius,
            ear_radius,
        )
        .line_to(ear_radius, height - corner_radius)
        .cubic_to(
            ear_radius,
            height - corner_radius + 0.552_284_75 * corner_radius,
            ear_radius + corner_radius - 0.552_284_75 * corner_radius,
            height,
            ear_radius + corner_radius,
            height,
        )
        .line_to(width, height)
        .line_to(width, 0.0)
        .close()
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_popup_animates_from_the_compact_bar_shape() {
        let (start_width, start_height, start_ear, start_corner) = island_metrics(0.0);
        let (middle_width, middle_height, _, _) = island_metrics(0.5);
        let (end_width, end_height, end_ear, end_corner) = island_metrics(1.0);

        assert_eq!(
            (start_width, start_height),
            (COLLAPSED_WIDTH, COLLAPSED_HEIGHT)
        );
        assert_eq!((start_ear, start_corner), (0.0, 0.0));
        assert!(middle_width > start_width && middle_width < end_width);
        assert!(middle_height > start_height && middle_height < end_height);
        assert_eq!((end_width, end_height), (WIDTH, HEIGHT));
        assert_eq!((end_ear, end_corner), (EAR_RADIUS, CORNER_RADIUS));
    }
}
