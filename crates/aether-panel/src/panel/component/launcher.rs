use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Column, Cursor, Easing, Horizontal, Image, Key, Keyboard,
    Layer, LayerWindow, Padding, Parent, Path, Rectangle, Row, Shadow, Shape, Stack,
    Start, Text, TextInput, Vertical, Weight, Widget, Zone, children,
};

use crate::panel::svg::recolor;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::applications::{Application, ApplicationService};
use crate::theme::theme;

// ------------------------------------------------------------
// Layout — Compact Dynamic Island Launcher
// ------------------------------------------------------------

const BOX_WIDTH: f32 = 440.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;
const TOTAL_WIDTH: f32 = BOX_WIDTH + EAR_RADIUS * 2.0;

const EDGE_INSET_LEFT: f32 = 8.0;

const SEARCH_HEIGHT: f32 = 38.0;
const PADDING_TOP: f32 = 9.0;
const PADDING_BOTTOM: f32 = 9.0;
const PADDING_SIDE: f32 = 12.0;
const COMPACT_HEIGHT: f32 = SEARCH_HEIGHT + PADDING_TOP + PADDING_BOTTOM; // 56.0

const ROW_HEIGHT: f32 = 42.0;
const ROW_GAP: f32 = 3.0;
const MAX_SUGGESTIONS: usize = 6;

const ICON_TILE_SIZE: f32 = 28.0;
const APP_NAME_SIZE: f32 = 12.5;
const APP_DESC_SIZE: f32 = 10.5;

const SEARCH_FIELD: &str = "aether-launcher-search";

// ------------------------------------------------------------
// Animation
// ------------------------------------------------------------

const LAUNCHER_ANIM_DURATION: Duration = Duration::from_millis(80);
static LAUNCHER_HEIGHT_ANIM: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();

fn launcher_animated_height(target: f32) -> f32 {
    let animation = LAUNCHER_HEIGHT_ANIM.get_or_init(|| {
        Mutex::new(
            Animation::new(0.0)
                .duration(LAUNCHER_ANIM_DURATION)
                .easing(Easing::Out),
        )
    });

    let mut anim = animation.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    anim.to(target);
    anim.value()
}

// ------------------------------------------------------------
// State
// ------------------------------------------------------------

static OPEN: AtomicBool = AtomicBool::new(false);
static SELECTED_INDEX: AtomicUsize = AtomicUsize::new(0);
static INDEX_LOADING: AtomicBool = AtomicBool::new(false);
static INDEX_LOADED: AtomicBool = AtomicBool::new(false);
static SEARCH_QUERY: OnceLock<Mutex<String>> = OnceLock::new();
static APPLICATIONS: OnceLock<Mutex<ApplicationService>> = OnceLock::new();

fn search_state() -> &'static Mutex<String> {
    SEARCH_QUERY.get_or_init(|| Mutex::new(String::new()))
}

fn application_service() -> &'static Mutex<ApplicationService> {
    APPLICATIONS.get_or_init(|| Mutex::new(ApplicationService::default()))
}

fn query() -> String {
    search_state()
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default()
}

fn set_query(value: &str) {
    if let Ok(mut current) = search_state().lock() {
        *current = value.to_string();
    }
    SELECTED_INDEX.store(0, Ordering::Relaxed);
    aether::refresh();
}

pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

// ------------------------------------------------------------
// Public Actions & Preloading
// ------------------------------------------------------------

pub fn preload() {
    let service = application_service();
    if !INDEX_LOADED.load(Ordering::Relaxed)
        && INDEX_LOADING.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_ok()
    {
        crate::services::runtime::runtime().spawn(async move {
            let mut loaded_service = ApplicationService::default();
            let _ = loaded_service.refresh();
            if let Ok(mut service) = service.lock() {
                *service = loaded_service;
            }
            INDEX_LOADED.store(true, Ordering::Relaxed);
            INDEX_LOADING.store(false, Ordering::Relaxed);
            aether::refresh();
        });
    }
}

pub fn open_launcher() {
    preload();
    if let Ok(mut value) = search_state().lock() {
        value.clear();
    }
    TextInput::set_text(SEARCH_FIELD, "");
    SELECTED_INDEX.store(0, Ordering::Relaxed);

    OPEN.store(true, Ordering::Relaxed);
    aether::refresh();
}

pub fn toggle() {
    if is_open() {
        close();
    } else {
        open_launcher();
    }
}

pub fn close() {
    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

// ------------------------------------------------------------
// Dynamic Island Geometry
// ------------------------------------------------------------

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

// ------------------------------------------------------------
// Launcher Overlay Window
// ------------------------------------------------------------

fn get_filtered_apps() -> Vec<Application> {
    let q = query();
    let trimmed = q.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    application_service()
        .lock()
        .map(|service| service.search(trimmed))
        .unwrap_or_default()
}

fn launch_selected() {
    let apps = get_filtered_apps();
    let cur = SELECTED_INDEX.load(Ordering::Relaxed);
    if let Some(app) = apps.get(cur) {
        let _ = ApplicationService::launch(app);
        close();
    }
}

pub fn launcher() -> LayerWindow {
    let is_open = is_open();
    let q = query();
    let has_query = !q.trim().is_empty();
    let suggestions = if has_query { get_filtered_apps() } else { Vec::new() };

    let target_height = if !is_open {
        0.0
    } else if !has_query {
        COMPACT_HEIGHT
    } else {
        let count = suggestions.len().min(MAX_SUGGESTIONS);
        let content_rows = if count == 0 { 1 } else { count };
        let list_h = content_rows as f32 * ROW_HEIGHT + (content_rows.saturating_sub(1)) as f32 * ROW_GAP;
        COMPACT_HEIGHT + 7.0 + list_h
    };

    let current_h = launcher_animated_height(target_height);

    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-launcher")
        .keyboard(if is_open { Keyboard::Exclusive } else { Keyboard::None })
        .on_key(|key| match key {
            Key::Escape => close(),
            Key::Down => {
                let count = get_filtered_apps().len().min(MAX_SUGGESTIONS);
                if count > 0 {
                    let cur = SELECTED_INDEX.load(Ordering::Relaxed);
                    let next = (cur + 1).min(count - 1);
                    SELECTED_INDEX.store(next, Ordering::Relaxed);
                    aether::refresh();
                }
            }
            Key::Up => {
                let cur = SELECTED_INDEX.load(Ordering::Relaxed);
                if cur > 0 {
                    let next = cur - 1;
                    SELECTED_INDEX.store(next, Ordering::Relaxed);
                    aether::refresh();
                }
            }
            Key::Enter => launch_selected(),
            _ => {}
        });

    if current_h <= 1.0 && !is_open {
        return window.click_through().child(blank());
    }

    let theme = theme();
    let current_r_e = EAR_RADIUS.min(current_h * 0.44);
    let current_r_b = CORNER_RADIUS.min(current_h * 0.44);

    let background = Box::new(
        build_island_path(TOTAL_WIDTH, current_h, current_r_e, current_r_b)
            .fill(theme.bar)
            .opacity((current_h / COMPACT_HEIGHT).clamp(0.0, 1.0)),
    ) as Box<dyn Shape>;

    let border = Box::new(
        build_island_border_path(TOTAL_WIDTH, current_h, current_r_e, current_r_b)
            .stroke(1.0, theme.surface_border)
            .opacity((current_h / COMPACT_HEIGHT).clamp(0.0, 1.0)),
    ) as Box<dyn Shape>;

    let canvas = Canvas::new()
        .width(TOTAL_WIDTH)
        .height(current_h)
        .shapes(vec![background, border]);

    let open_ratio = (current_h / COMPACT_HEIGHT).clamp(0.0, 1.0);
    let content_opacity = (open_ratio * 2.0).min(1.0);

    let mut body_children: Vec<Box<dyn Widget>> = Vec::new();
    body_children.push(Box::new(search_field()));

    if has_query {
        body_children.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(1.0)
                .fill(theme.chip_border),
        ));
        body_children.push(Box::new(suggestions_view(&suggestions)));
    }

    let body = Column::new(body_children)
        .width(BOX_WIDTH - PADDING_SIDE * 2.0)
        .gap(6.0)
        .align(Align::Start);

    let card = Rectangle::new()
        .width(TOTAL_WIDTH)
        .height(current_h)
        .fill(Color::TRANSPARENT)
        .clip()
        .on_click(|_| {})
        .child(Stack::new(children![
            canvas,
            Rectangle::new()
                .width(TOTAL_WIDTH)
                .height(target_height.max(COMPACT_HEIGHT))
                .fill(Color::TRANSPARENT)
                .opacity(content_opacity)
                .padding(Padding {
                    top: PADDING_TOP,
                    right: EAR_RADIUS + PADDING_SIDE,
                    bottom: PADDING_BOTTOM,
                    left: EAR_RADIUS + PADDING_SIDE,
                })
                .align_child(Align::Start, Align::Start)
                .child(body),
        ]));

    let island_stack = Stack::new(children![
        // Drop shadow sits behind card, unclipped so blur can spread smoothly
        Rectangle::new()
            .width(TOTAL_WIDTH)
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
                    .width((TOTAL_WIDTH - current_r_e * 2.0).max(0.0))
                    .height(current_h)
                    .radius_bottom_left(current_r_b)
                    .radius_bottom_right(current_r_b)
                    .fill(Color::TRANSPARENT)
                    .shadow(
                        Shadow::drop(theme.shadow)
                            .opacity(0.35 * open_ratio)
                            .blur(20.0)
                            .offset(0.0, 8.0),
                    ),
            ),
        card,
    ]);

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
                        right: 0.0,
                        bottom: 0.0,
                        left: EDGE_INSET_LEFT,
                    })
                    .align_child(Align::Start, Align::Start)
                    .child(island_stack),
            ),
    )
}

// ------------------------------------------------------------
// Search Field & Suggestions
// ------------------------------------------------------------

fn search_icon_image() -> Image {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/launcher/search.svg");
    recolor::ink_image(path, theme().text_muted)
}

fn search_field() -> Rectangle {
    let theme = theme();

    Rectangle::new()
        .width(Parent)
        .height(SEARCH_HEIGHT)
        .fill(Color::rgba(26, 26, 26, 255))
        .border(1.0, theme.chip_border)
        .radius(9.0)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        })
        .align_child(Align::Start, Align::Center)
        .child(
            Row::new(children![
                Rectangle::new()
                    .width(15.0)
                    .height(15.0)
                    .fill(search_icon_image()),
                Rectangle::new()
                    .width(10.0)
                    .height(1.0)
                    .fill(Color::TRANSPARENT),
                TextInput::new(SEARCH_FIELD)
                    .width(Parent)
                    .size(APP_NAME_SIZE)
                    .color(theme.text)
                    .placeholder("Search applications...")
                    .focused()
                    .on_change(|value| set_query(&value))
                    .on_submit(|_| launch_selected()),
            ])
            .width(Parent)
            .height(SEARCH_HEIGHT)
            .align(Align::Center),
        )
}

fn suggestions_view(apps: &[Application]) -> Column {
    let theme = theme();
    let selected = SELECTED_INDEX.load(Ordering::Relaxed);

    if apps.is_empty() {
        return Column::new(children![
            Rectangle::new()
                .width(Parent)
                .height(ROW_HEIGHT)
                .fill(Color::TRANSPARENT)
                .align_child(Align::Center, Align::Center)
                .child(
                    Text::new("No matching applications")
                        .font("Maple Mono")
                        .color(theme.text_muted)
                        .size(APP_NAME_SIZE),
                ),
        ])
        .width(Parent)
        .align(Align::Start);
    }

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();
    for (idx, app) in apps.iter().take(MAX_SUGGESTIONS).enumerate() {
        rows.push(Box::new(suggestion_row(app.clone(), idx == selected)));
    }

    Column::new(rows)
        .width(Parent)
        .gap(ROW_GAP)
        .align(Align::Start)
}

fn suggestion_row(app: Application, is_selected: bool) -> Rectangle {
    let theme = theme();
    let name = app.name.clone();
    let desc = app
        .comment
        .clone()
        .unwrap_or_else(|| app.id.replace(".desktop", ""));

    let icon_tile = if let Some(path) = app.icon_path() {
        Rectangle::new()
            .width(ICON_TILE_SIZE)
            .height(ICON_TILE_SIZE)
            .radius(6.0)
            .fill(Color::TRANSPARENT)
            .align_child(Align::Center, Align::Center)
            .child(
                Rectangle::new()
                    .width(ICON_TILE_SIZE - 2.0)
                    .height(ICON_TILE_SIZE - 2.0)
                    .fill(Image::contain(path)),
            )
    } else {
        let first_letter = app
            .name
            .chars()
            .next()
            .unwrap_or('•')
            .to_uppercase()
            .to_string();

        Rectangle::new()
            .width(ICON_TILE_SIZE)
            .height(ICON_TILE_SIZE)
            .radius(6.0)
            .fill(theme.chip)
            .border(1.0, theme.chip_border)
            .align_child(Align::Center, Align::Center)
            .child(
                Text::new(first_letter)
                    .font("Maple Mono")
                    .color(theme.text)
                    .size(12.0)
                    .weight(Weight::SemiBold),
            )
    };

    let text_column = Column::new(children![
        Text::new(name)
            .font("Maple Mono")
            .color(theme.text)
            .size(APP_NAME_SIZE)
            .weight(if is_selected { Weight::SemiBold } else { Weight::Medium })
            .elide(),
        Text::new(desc)
            .font("Maple Mono")
            .color(theme.text_faint)
            .size(APP_DESC_SIZE)
            .elide(),
    ])
    .width(Parent)
    .gap(1.0)
    .align(Align::Start);

    let mut row_children: Vec<Box<dyn Widget>> = Vec::new();
    row_children.push(Box::new(icon_tile));
    row_children.push(Box::new(
        Rectangle::new().width(10.0).height(1.0).fill(Color::TRANSPARENT),
    ));
    row_children.push(Box::new(text_column));

    if is_selected {
        row_children.push(Box::new(
            Rectangle::new()
                .width(22.0)
                .height(16.0)
                .radius(4.0)
                .fill(theme.chip)
                .border(1.0, theme.chip_border)
                .align_child(Align::Center, Align::Center)
                .child(
                    Text::new("↵")
                        .font("Maple Mono")
                        .color(theme.text_faint)
                        .size(10.0)
                        .weight(Weight::Medium),
                ),
        ));
    }

    let bg_color = if is_selected {
        theme.accent_faint
    } else {
        Color::TRANSPARENT
    };

    Rectangle::new()
        .width(Parent)
        .height(ROW_HEIGHT)
        .fill(bg_color)
        .radius(7.0)
        .border(
            1.0,
            if is_selected {
                theme.accent
            } else {
                Color::TRANSPARENT
            },
        )
        .cursor(Cursor::Pointer)
        .on_click(move |_| {
            let _ = ApplicationService::launch(&app);
            close();
        })
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: 8.0,
        })
        .align_child(Align::Start, Align::Center)
        .child(
            Row::new(row_children)
                .width(Parent)
                .height(ROW_HEIGHT)
                .align(Align::Center)
                .justify(Start),
        )
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
