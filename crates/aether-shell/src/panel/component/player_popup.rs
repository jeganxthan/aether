use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use aether::{
    Align, Animation, Canvas, Color, Column, Cursor, Easing, Horizontal, Image, InputArea, Key,
    Keyboard, Layer, LayerWindow, Margin, Padding, Parent, Path, Rectangle, Row, Shape,
    SpaceBetween, Stack, Start, Text, Vertical, Weight, children,
};

use crate::panel::svg::player_svg::{control_icon, player_brand_icon};
use crate::panel::top::PANEL_HEIGHT;
use crate::services::{display, player::{next, play_pause, player, previous, seek_to}};
use crate::theme::theme;

const ISLAND_WINDOW_WIDTH: f32 = 260.0;
const ISLAND_WINDOW_HEIGHT: f32 = 48.0;
const ISLAND_MAX_HEIGHT: f32 = 40.0;

const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;
const ISLAND_BODY_WIDTH: f32 = 200.0;
const MENU_PROGRESS_WIDTH: f32 = 170.0;

// Margin from screen right edge — centers under player icon in top bar
pub const PLAYER_EDGE_INSET_RIGHT: f32 = 80.0;

const ISLAND_ANIM_DURATION: Duration = Duration::from_millis(220);
static PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();

static HOVERED: AtomicBool = AtomicBool::new(false);
static HOVER_GENERATION: AtomicU64 = AtomicU64::new(0);
static HOVER_START: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static MENU_PROGRESS: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
const MENU_ANIM_DURATION: Duration = Duration::from_millis(280);
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
    animation.to(if MENU_OPEN.load(Ordering::Relaxed) {
        1.0
    } else {
        0.0
    });
    animation.value()
}

fn hover_start_lock() -> &'static Mutex<Option<Instant>> {
    HOVER_START.get_or_init(|| Mutex::new(None))
}

pub fn hovered() -> bool {
    HOVERED.load(Ordering::Relaxed)
}

pub fn toggle_menu() {
    MENU_OPEN.fetch_xor(true, Ordering::Relaxed);
    aether::refresh();
}

pub fn close_menu() {
    if MENU_OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

pub fn open_player_on_hover_for_monitor(monitor: String) {
    open_for_monitor(true, Some(monitor));
}

pub fn toggle_player_for_monitor(monitor: String) {
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

pub fn set_target_monitor(monitor: Option<String>) {
    if let Ok(mut target) = target_monitor().lock() {
        *target = monitor;
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

pub fn player_menu(monitor: &aether::Monitor) -> LayerWindow {
    if !is_target_monitor(monitor) {
        return LayerWindow::new()
            .width(aether::Full)
            .height(aether::Full)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Overlay)
            .space(aether::Zone::Ignore)
            .namespace("aether-player-menu")
            .keyboard(Keyboard::OnDemand)
            .on_key(|key| {
                if key == Key::Escape {
                    close_menu();
                }
            })
            .click_through()
            .child(blank());
    }

    const MENU_WIDTH: f32 = 340.0;
    const MENU_HEIGHT: f32 = 140.0;
    let progress = menu_progress();
    let current_width = ISLAND_BODY_WIDTH
        + EAR_RADIUS * 2.0
        + (MENU_WIDTH - ISLAND_BODY_WIDTH - EAR_RADIUS * 2.0) * progress;
    let current_height = ISLAND_MAX_HEIGHT + (MENU_HEIGHT - ISLAND_MAX_HEIGHT) * progress;
    let current_corner_radius = CORNER_RADIUS.min(current_height * 0.44);
    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .space(aether::Zone::Ignore)
        .namespace("aether-player-menu")
        .keyboard(Keyboard::OnDemand)
        .on_key(|key| {
            if key == Key::Escape {
                close_menu();
            }
        });

    if progress <= 0.001 && !MENU_OPEN.load(Ordering::Relaxed) {
        return window.click_through().child(blank());
    }
    let Some(state) = player() else {
        return window.click_through().child(blank());
    };

    let theme = theme();
    let duration = state.metadata.length_secs.unwrap_or(0);
    let position = state.position_secs.min(duration);
    let fraction = if duration > 0 {
        position as f32 / duration as f32
    } else {
        0.0
    };
    let mut cover = Rectangle::new()
        .width(60.0)
        .height(60.0)
        .radius(8.0)
        .fill(Color::rgba(35, 35, 35, 255))
        .clip();
    if let Some(path) = state.local_art_path() {
        cover = cover.fill(Image::cover(path));
    }

    let title = if state.metadata.title.is_empty() {
        "Playing Media".to_string()
    } else {
        state.metadata.title.clone()
    };
    let subtitle = [
        state.metadata.artist.as_str(),
        state.metadata.album.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ");
    let controls = transport_buttons(state.is_playing);
    let progress_bar = Rectangle::new()
        .width(MENU_PROGRESS_WIDTH)
        .height(6.0)
        .radius(3.0)
        .fill(Color::rgba(75, 75, 75, 255))
        .cursor(Cursor::Pointer)
        .on_drag(move |point| {
            if duration > 0 {
                seek_to(
                    ((point.x / MENU_PROGRESS_WIDTH).clamp(0.0, 1.0) * duration as f32).round()
                        as u64,
                );
            }
        })
        .child(
            Rectangle::new()
                .width((MENU_PROGRESS_WIDTH * fraction).max(3.0))
                .height(6.0)
                .radius(3.0)
                .fill(Color::rgba(30, 215, 96, 255)),
        );

    let menu_background = Canvas::new()
        .width(current_width)
        .height(current_height)
        .shapes(vec![
            Box::new(
                build_island_path(
                    current_width,
                    current_height,
                    EAR_RADIUS,
                    current_corner_radius,
                )
                .fill(theme.bar)
                .opacity(progress),
            ) as Box<dyn Shape>,
            Box::new(
                build_island_border_path(
                    current_width,
                    current_height,
                    EAR_RADIUS,
                    current_corner_radius,
                )
                .stroke(1.0, theme.surface_border)
                .opacity(progress),
            ) as Box<dyn Shape>,
        ]);
    let card = Rectangle::new()
        .width(current_width)
        .height(current_height)
        .fill(Color::TRANSPARENT)
        .clip()
        .child(Stack::new(children![
            Rectangle::new()
                .width(current_width)
                .height(current_height)
                .fill(Color::TRANSPARENT)
                .padding(Padding {
                    top: 0.0,
                    right: EAR_RADIUS,
                    bottom: 0.0,
                    left: EAR_RADIUS
                })
                .align_child(Align::Center, Align::Start)
                .child(
                    Rectangle::new()
                        .width(current_width - 2.0 * EAR_RADIUS)
                        .height(current_height)
                        .fill(Color::TRANSPARENT)
                        .shadow(super::island_shadow(0.35 * progress))
                ),
            menu_background,
            Rectangle::new()
                .width(current_width)
                .height(current_height)
                .fill(Color::TRANSPARENT)
                .opacity(((progress - 0.18) / 0.82).clamp(0.0, 1.0))
                .padding(Padding {
                    top: 14.0,
                    right: EAR_RADIUS + 14.0,
                    bottom: 8.0,
                    left: EAR_RADIUS + 14.0
                })
                .child(
                    Column::new(children![
                        Row::new(children![
                            cover,
                            Column::new(children![
                                Text::new(title)
                                    .font("Maple Mono")
                                    .color(theme.text)
                                    .size(13.0)
                                    .weight(Weight::SemiBold)
                                    .elide(),
                                Text::new(if subtitle.is_empty() {
                                    format!("{:?}", state.brand)
                                } else {
                                    subtitle
                                })
                                .font("Maple Mono")
                                .color(theme.text_muted)
                                .size(10.0)
                                .elide(),
                            ])
                            .width(Parent)
                            .gap(7.0)
                            .align(Align::Start)
                        ])
                        .gap(16.0)
                        .align(Align::Center),
                        Stack::new(children![
                            Row::new(children![
                                Text::new(format_time(position))
                                    .font("Maple Mono")
                                    .color(theme.text_muted)
                                    .size(10.0),
                                Text::new(format_time(duration))
                                    .font("Maple Mono")
                                    .color(theme.text_muted)
                                    .size(10.0),
                            ])
                            .width(Parent)
                            .justify(SpaceBetween)
                            .align(Align::Center),
                            Rectangle::new()
                                .width(Parent)
                                .height(6.0)
                                .fill(Color::TRANSPARENT)
                                .align_child(Align::Center, Align::Center)
                                .child(progress_bar),
                        ])
                        .width(Parent)
                        .height(12.0),
                        Row::new(children![
                            Rectangle::new()
                                .width(Parent)
                                .height(1.0)
                                .fill(Color::TRANSPARENT),
                            controls,
                            Rectangle::new()
                                .width(Parent)
                                .height(1.0)
                                .fill(Color::TRANSPARENT)
                        ])
                        .align(Align::Center),
                    ])
                    .width(Parent)
                    .height(Parent)
                    .gap(10.0)
                    .align(Align::Start)
                )
        ]));

    let monitor_name = monitor.name.clone();
    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .on_click(|_| close_menu())
            .child(
                Rectangle::new()
                    .width(Parent)
                    .height(Parent)
                    .fill(Color::TRANSPARENT)
                    .padding(Padding {
                        top: PANEL_HEIGHT,
                        right: PLAYER_EDGE_INSET_RIGHT,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .align_child(Align::End, Align::Start)
                    .child(card.on_click(move |_| {
                        super::player_popup::set_target_monitor(Some(monitor_name.clone()));
                        toggle_menu();
                    })),
            ),
    )
}

fn transport_button(icon: &str, is_playing: bool) -> Rectangle {
    let (size, icon_size, fill) = match icon {
        "prev" | "next" => (20.0, 13.0, Color::TRANSPARENT),
        "play-pause" => (22.0, 11.0, Color::rgba(30, 215, 96, 255)),
        _ => unreachable!("player controls use known icon names"),
    };
    let action = match icon {
        "prev" => previous as fn(),
        "next" => next as fn(),
        "play-pause" => play_pause as fn(),
        _ => unreachable!("player controls use known icon names"),
    };
    let icon = match icon {
        "play-pause" if is_playing => "pause",
        "play-pause" => "play",
        icon => icon,
    };
    Rectangle::new()
        .width(size)
        .height(size)
        .radius(size / 2.0)
        .fill(fill)
        .cursor(Cursor::Pointer)
        .align_child(Align::Center, Align::Center)
        .on_hover(set_hovered)
        .on_click(move |_| action())
        .child(
            Rectangle::new()
                .width(icon_size)
                .height(icon_size)
                .fill(control_icon(icon)),
        )
}

fn transport_buttons(is_playing: bool) -> Row {
    Row::new(children![
        transport_button("prev", is_playing),
        transport_button("play-pause", is_playing),
        transport_button("next", is_playing),
    ])
    .gap(5.0)
    .align(Align::Center)
}

fn format_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn set_hovered(is_hov: bool) {
    let generation = HOVER_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;

    if is_hov {
        HOVERED.store(true, Ordering::Relaxed);
        if let Ok(mut start) = hover_start_lock().lock() {
            if start.is_none() {
                *start = Some(Instant::now());
            }
        }
        aether::refresh();
        return;
    }

    #[cfg(test)]
    {
        let _ = generation;
        HOVERED.store(false, Ordering::Relaxed);
        if let Ok(mut start) = hover_start_lock().lock() {
            *start = None;
        }
        aether::refresh();
    }

    #[cfg(not(test))]
    {
        crate::services::runtime::runtime().spawn(async move {
            tokio::time::sleep(Duration::from_millis(250)).await;

            if HOVER_GENERATION.load(Ordering::Relaxed) == generation {
                HOVERED.store(false, Ordering::Relaxed);
                if let Ok(mut start) = hover_start_lock().lock() {
                    *start = None;
                }
                aether::refresh();
            }
        });
    }
}

fn island_progress(expanded: bool) -> f32 {
    let animation = PROGRESS.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(ISLAND_ANIM_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut anim = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    anim.to(if expanded { 1.0 } else { 0.0 });
    anim.value()
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

pub fn player_popup(monitor: &aether::Monitor) -> LayerWindow {
    if !is_target_monitor(monitor) {
        return LayerWindow::new()
            .width(ISLAND_WINDOW_WIDTH)
            .height(ISLAND_WINDOW_HEIGHT)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Top)
            .space(aether::Zone::Ignore)
            .namespace("aether-player-popup")
            .visible(is_target_monitor(monitor))
            .margin(Margin {
                top: PANEL_HEIGHT as i32,
                right: PLAYER_EDGE_INSET_RIGHT as i32,
                bottom: 0,
                left: 0,
            })
            .click_through()
            .child(blank());
    }

    let active_player = player();
    let has_player = active_player.is_some();
    let is_hovered = (hovered() || MENU_OPEN.load(Ordering::Relaxed)) && has_player;
    let progress = island_progress(is_hovered);

    if progress <= 0.001 && !is_hovered {
        return LayerWindow::new()
            .width(ISLAND_WINDOW_WIDTH)
            .height(ISLAND_WINDOW_HEIGHT)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Top)
            .space(aether::Zone::Ignore)
            .namespace("aether-player-popup")
            .visible(is_target_monitor(monitor))
            .margin(Margin {
                top: PANEL_HEIGHT as i32,
                right: PLAYER_EDGE_INSET_RIGHT as i32,
                bottom: 0,
                left: 0,
            })
            .click_through()
            .child(blank());
    }

    let Some(current_state) = active_player else {
        return LayerWindow::new()
            .width(ISLAND_WINDOW_WIDTH)
            .height(ISLAND_WINDOW_HEIGHT)
            .anchor_vertical(Vertical::Top)
            .anchor_horizontal(Horizontal::Right)
            .layer(Layer::Top)
            .space(aether::Zone::Ignore)
            .namespace("aether-player-popup")
            .visible(is_target_monitor(monitor))
            .click_through()
            .child(blank());
    };

    let theme = theme();
    let body_width = ISLAND_BODY_WIDTH;
    let total_w = body_width + 2.0 * EAR_RADIUS;
    let total_h = ISLAND_MAX_HEIGHT;

    let current_h = (total_h * progress).round().max(2.0);
    let current_r_e = EAR_RADIUS * progress;
    let current_r_b = (CORNER_RADIUS * progress).min(current_h * 0.44);

    let fill_shape = Box::new(
        build_island_path(total_w, current_h, current_r_e, current_r_b)
            .fill(theme.bar)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let border_shape = Box::new(
        build_island_border_path(total_w, current_h, current_r_e, current_r_b)
            .stroke(1.0, theme.surface_border)
            .opacity(progress),
    ) as Box<dyn Shape>;

    let canvas = Canvas::new()
        .width(total_w)
        .height(current_h)
        .shapes(vec![fill_shape, border_shape]);

    let content_opacity = ((progress - 0.20) / 0.80).clamp(0.0, 1.0);
    // Timing check for controls vs running title
    let hover_duration = hover_start_lock()
        .lock()
        .ok()
        .and_then(|guard| guard.map(|s| s.elapsed()))
        .unwrap_or(Duration::ZERO);

    let show_running_title = hover_duration >= Duration::from_secs(3);

    // Continuous frames while animating or running marquee text
    if is_hovered {
        crate::services::runtime::runtime().spawn(async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            if hovered() {
                aether::refresh();
            }
        });
    }

    // Left album art thumbnail
    let album_art_tile = {
        let art_size = 30.0;
        let mut tile = Rectangle::new()
            .width(art_size)
            .height(art_size)
            .radius(5.0)
            .cursor(Cursor::Pointer)
            .on_hover(set_hovered)
            .on_click(|_| {
                if let Ok(mut start) = hover_start_lock().lock() {
                    *start = Some(Instant::now());
                }
                play_pause();
            })
            .clip();

        if let Some(path) = current_state.local_art_path() {
            tile = tile.fill(Image::cover(path));
        } else {
            tile = tile
                .fill(Color::rgba(40, 40, 40, 255))
                .align_child(Align::Center, Align::Center)
                .child(
                    Rectangle::new()
                        .width(14.0)
                        .height(14.0)
                        .fill(player_brand_icon(current_state.brand)),
                );
        }
        tile
    };

    let inner_content: Row = if show_running_title {
        // Horizontal running title (Marquee)
        let display_title = current_state.display_title();
        let elapsed_secs = (hover_duration.as_millis() as f32 - 3000.0).max(0.0) / 1000.0;
        let scroll_distance = (elapsed_secs * 42.0) % 320.0;
        let marquee_width = body_width - 34.0;

        Row::new(children![
            album_art_tile,
            Rectangle::new()
                .width(marquee_width)
                .height(current_h)
                .fill(Color::TRANSPARENT)
                .clip()
                .cursor(Cursor::Pointer)
                .on_hover(set_hovered)
                .on_click(|_| {
                    // Clicking running title resets timer to show controls again for 3s
                    if let Ok(mut start) = hover_start_lock().lock() {
                        *start = Some(Instant::now());
                    }
                    aether::refresh();
                })
                .align_child(Align::Start, Align::Center)
                .child(
                    Rectangle::new()
                        .width(420.0)
                        .height(current_h)
                        .fill(Color::TRANSPARENT)
                        .translate(-scroll_distance, 0.0)
                        .align_child(Align::Start, Align::Center)
                        .child(
                            Text::new(format!("{}         {}", display_title, display_title))
                                .font("Maple Mono")
                                .color(theme.text)
                                .size(11.5)
                                .weight(Weight::Medium)
                        )
                ),
        ])
        .width(Parent)
        .height(current_h)
        .gap(6.0)
        .align(Align::Center)
        .justify(Start)
    } else {
        // Controls mode: Prev, Play/Pause, Next + Time
        let is_playing = current_state.is_playing;
        let time_str = current_state.time_display();

        let time_badge = Text::new(time_str)
            .font("Maple Mono")
            .color(theme.text_muted)
            .size(11.0)
            .weight(Weight::Medium);

        Row::new(children![
            album_art_tile,
            transport_buttons(is_playing),
            Rectangle::new()
                .width(Parent)
                .height(1.0)
                .fill(Color::TRANSPARENT),
            time_badge,
        ])
        .width(Parent)
        .height(current_h)
        .gap(6.0)
        .align(Align::Center)
        .justify(Start)
    };

    let island = Rectangle::new()
        .width(total_w)
        .height(current_h)
        .fill(Color::TRANSPARENT)
        .cursor(Cursor::Pointer)
        .on_hover(set_hovered)
        .child(Stack::new(children![
            // Soft, refined ambient shadow strictly beneath the body (no leakage into bar)
            Rectangle::new()
                .width(total_w)
                .height(current_h)
                .fill(Color::TRANSPARENT)
                .padding(Padding {
                    top: 2.0,
                    right: current_r_e,
                    bottom: 0.0,
                    left: current_r_e,
                })
                .align_child(Align::Center, Align::Start)
                .child(
                    Rectangle::new()
                        .width(body_width)
                        .height((current_h - 2.0).max(1.0))
                        .radius_bottom_left(current_r_b)
                        .radius_bottom_right(current_r_b)
                        .fill(theme.bar)
                        .shadow(super::island_shadow(0.35 * progress),),
                ),
            canvas,
            // Content padded within the ears
            Rectangle::new()
                .width(total_w)
                .height(current_h)
                .fill(Color::TRANSPARENT)
                .opacity(content_opacity)
                .padding(Padding {
                    top: 3.0,
                    right: EAR_RADIUS + 7.0,
                    bottom: 3.0,
                    left: EAR_RADIUS + 7.0,
                })
                .align_child(Align::Center, Align::Center)
                .child(inner_content),
        ]));

    let island_x = (ISLAND_WINDOW_WIDTH - total_w).max(0.0).round() as i32;
    let island_y = 0;
    let island_w = total_w.round() as i32;
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
        .space(aether::Zone::Ignore)
        .namespace("aether-player-popup")
        .margin(Margin {
            top: PANEL_HEIGHT as i32,
            right: PLAYER_EDGE_INSET_RIGHT as i32,
            bottom: 0,
            left: 0,
        });

    if progress > 0.05 {
        window = window.input_region(vec![input]);
    } else {
        window = window.click_through();
    }

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .on_hover(set_hovered)
            .align_child(Align::End, Align::Start)
            .child(island),
    )
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
