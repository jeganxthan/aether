use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use aether::{
    Align, Animation, Canvas, Color, Column, Cursor, Easing, Horizontal, InputArea, Key, Keyboard,
    Layer, LayerWindow, Padding, Parent, Path, Rectangle, Row, Shape, Stack, Start, Text, TextInput,
    Vertical, Weight, Widget, Zone, children,
};
use aether_services::clipboard::{self, ClipboardEntry};

use crate::panel::svg::recolor;
use crate::panel::top::PANEL_HEIGHT;
use crate::services::runtime::runtime;
use crate::theme::theme;

const BOX_WIDTH: f32 = 440.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;
const TOTAL_WIDTH: f32 = BOX_WIDTH + EAR_RADIUS * 2.0;
const EDGE_INSET_LEFT: f32 = 44.0;
const SEARCH_HEIGHT: f32 = 38.0;
const PADDING_TOP: f32 = 9.0;
const PADDING_BOTTOM: f32 = 9.0;
const PADDING_SIDE: f32 = 12.0;
const COMPACT_HEIGHT: f32 = SEARCH_HEIGHT + PADDING_TOP + PADDING_BOTTOM;
const ROW_HEIGHT: f32 = 42.0;
const ROW_GAP: f32 = 3.0;
const MAX_VISIBLE: usize = 6;
const SEARCH_FIELD: &str = "aether-clipboard-search";
const ANIMATION_DURATION: Duration = Duration::from_millis(80);

static OPEN: AtomicBool = AtomicBool::new(false);
static SELECTED_INDEX: AtomicUsize = AtomicUsize::new(0);
static ENTRIES: OnceLock<RwLock<Vec<ClipboardEntry>>> = OnceLock::new();
static QUERY: OnceLock<RwLock<String>> = OnceLock::new();
static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);
static HEIGHT_ANIMATION: OnceLock<std::sync::Mutex<Animation<f32>>> = OnceLock::new();

fn animated_height(target: f32) -> f32 {
    let animation = HEIGHT_ANIMATION.get_or_init(|| {
        std::sync::Mutex::new(
            Animation::new(0.0)
                .duration(ANIMATION_DURATION)
                .easing(Easing::Out),
        )
    });
    let mut animation = animation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    animation.to(target);
    animation.value()
}

fn entries() -> &'static RwLock<Vec<ClipboardEntry>> {
    ENTRIES.get_or_init(|| RwLock::new(Vec::new()))
}

fn query() -> String {
    QUERY
        .get_or_init(|| RwLock::new(String::new()))
        .read()
        .map(|query| query.clone())
        .unwrap_or_default()
}

fn set_query(query: &str) {
    if let Ok(mut current) = QUERY.get_or_init(|| RwLock::new(String::new())).write() {
        *current = query.to_owned();
    }
    SELECTED_INDEX.store(0, Ordering::Relaxed);
    aether::refresh();
}

pub fn start() {
    if WATCHER_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }

    match clipboard::load_history() {
        Ok(history) => {
            if let Ok(mut current) = entries().write() {
                *current = history;
            }
        }
        Err(error) => eprintln!("aether-panel: could not load clipboard history: {error}"),
    }

    runtime().spawn(async {
        let mut ticker = tokio::time::interval(Duration::from_millis(700));
        let mut last_text: Option<String> = None;
        let mut last_error: Option<String> = None;
        loop {
            ticker.tick().await;
            let result = tokio::task::spawn_blocking(clipboard::read_text).await;
            let text = match result {
                Ok(Ok(text)) => {
                    last_error = None;
                    text
                }
                Ok(Err(error)) => {
                    let message = error.to_string();
                    if last_error.as_deref() != Some(&message) {
                        eprintln!("aether-panel: could not read clipboard: {error}");
                        last_error = Some(message);
                    }
                    continue;
                }
                Err(error) => {
                    let message = error.to_string();
                    if last_error.as_deref() != Some(&message) {
                        eprintln!("aether-panel: clipboard read task failed: {error}");
                        last_error = Some(message);
                    }
                    continue;
                }
            };

            if text == last_text {
                continue;
            }
            last_text = text.clone();

            let Some(text) = text else {
                continue;
            };

            let mut history = match entries().write() {
                Ok(history) => history,
                Err(error) => {
                    eprintln!("aether-panel: clipboard history lock failed: {error}");
                    continue;
                }
            };
            if clipboard::record_text(&mut history, text, clipboard::history_config().max_entries) {
                if let Err(error) = clipboard::save_history(&history) {
                    eprintln!("aether-panel: could not save clipboard history: {error}");
                }
                aether::refresh();
            }
        }
    });
}

pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

pub fn open() {
    start();
    if let Ok(mut query) = QUERY.get_or_init(|| RwLock::new(String::new())).write() {
        query.clear();
    }
    TextInput::set_text(SEARCH_FIELD, "");
    SELECTED_INDEX.store(0, Ordering::Relaxed);
    OPEN.store(true, Ordering::Relaxed);
    aether::refresh();
}

pub fn close() {
    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

pub fn toggle() {
    if is_open() {
        close();
    } else {
        open();
    }
}

fn filtered_entries() -> Vec<ClipboardEntry> {
    let needle = query().to_lowercase();
    entries()
        .read()
        .map(|history| {
            let mut matching: Vec<_> = history
                .iter()
                .filter(|entry| needle.is_empty() || entry.text.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            matching.sort_by_key(|entry| !entry.pinned);
            matching
        })
        .unwrap_or_default()
}

fn selected_entry() -> Option<ClipboardEntry> {
    let matching = filtered_entries();
    if matching.is_empty() {
        return None;
    }
    let selected = SELECTED_INDEX
        .load(Ordering::Relaxed)
        .min(matching.len() - 1);
    matching.get(selected).cloned()
}

fn copy_entry(entry: ClipboardEntry) {
    runtime().spawn(async move {
        match tokio::task::spawn_blocking(move || clipboard::copy_text(&entry.text)).await {
            Ok(Ok(())) => close(),
            Ok(Err(error)) => eprintln!("aether-panel: could not copy clipboard entry: {error}"),
            Err(error) => eprintln!("aether-panel: clipboard copy task failed: {error}"),
        }
    });
}

pub fn pin_selected() {
    let Some(entry) = selected_entry() else {
        eprintln!("aether-panel: there is no clipboard entry to pin");
        return;
    };

    let mut history = match entries().write() {
        Ok(history) => history,
        Err(error) => {
            eprintln!("aether-panel: clipboard history lock failed: {error}");
            return;
        }
    };
    match clipboard::pin_entry(&mut history, &entry.id) {
        Ok(_) => {
            if let Err(error) = clipboard::save_history(&history) {
                eprintln!("aether-panel: could not save clipboard pin: {error}");
            }
            aether::refresh();
        }
        Err(error) => eprintln!("aether-panel: could not pin clipboard entry: {error}"),
    }
}

fn move_selection(delta: isize) {
    let count = filtered_entries().len().min(MAX_VISIBLE);
    if count == 0 {
        return;
    }
    let current = SELECTED_INDEX.load(Ordering::Relaxed).min(count - 1) as isize;
    let next = (current + delta).clamp(0, count as isize - 1) as usize;
    SELECTED_INDEX.store(next, Ordering::Relaxed);
    aether::refresh();
}

pub fn view() -> LayerWindow {
    let is_open = is_open();
    let matching = filtered_entries();
    let visible_count = matching.len().min(MAX_VISIBLE);
    let content_rows = visible_count.max(1);
    let target_height = if is_open {
        COMPACT_HEIGHT
            + 7.0
            + content_rows as f32 * ROW_HEIGHT
            + content_rows.saturating_sub(1) as f32 * ROW_GAP
    } else {
        0.0
    };
    let height = animated_height(target_height);

    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-clipboard")
        .keyboard(if is_open {
            Keyboard::Exclusive
        } else {
            Keyboard::None
        })
        .on_key(move |key| match key {
            Key::Escape => close(),
            Key::Down => move_selection(1),
            Key::Up => move_selection(-1),
            Key::Enter => {
                if let Some(entry) = selected_entry() {
                    copy_entry(entry);
                }
            }
            _ => {}
        });

    if height <= 1.0 && !is_open {
        return window.click_through().child(blank());
    }

    let theme = theme();
    let open_ratio = (height / target_height.max(1.0)).clamp(0.0, 1.0);
    let background = Box::new(
        island_path(TOTAL_WIDTH, height)
            .fill(theme.bar)
            .opacity(open_ratio),
    ) as Box<dyn Shape>;
    let border = Box::new(
        island_path(TOTAL_WIDTH, height)
            .stroke(1.0, theme.surface_border)
            .opacity(open_ratio),
    ) as Box<dyn Shape>;
    let canvas = Canvas::new()
        .width(TOTAL_WIDTH)
        .height(height)
        .shapes(vec![background, border]);

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();
    if matching.is_empty() {
        rows.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(ROW_HEIGHT)
                .fill(Color::TRANSPARENT)
                .align_child(Align::Center, Align::Center)
                .child(
                    Text::new(if query().is_empty() {
                        "Clipboard is empty"
                    } else {
                        "No matching clipboard entries"
                    })
                    .font("Maple Mono")
                    .size(12.5)
                    .color(theme.text_muted),
                ),
        ));
    } else {
        for (index, entry) in matching.iter().take(MAX_VISIBLE).cloned().enumerate() {
            rows.push(Box::new(entry_row(
                entry,
                index == SELECTED_INDEX.load(Ordering::Relaxed),
            )));
        }
    }

    let content = Column::new(children![
        search_field(),
        Rectangle::new()
            .width(Parent)
            .height(1.0)
            .fill(theme.chip_border),
        Column::new(rows)
            .width(Parent)
            .gap(ROW_GAP)
            .align(Align::Start),
    ])
    .width(BOX_WIDTH - PADDING_SIDE * 2.0)
    .gap(6.0)
    .align(Align::Start);

    let island = Stack::new(children![
        Rectangle::new()
            .width(TOTAL_WIDTH)
            .height(height)
            .fill(Color::TRANSPARENT)
            .opacity((open_ratio * 2.0).min(1.0))
            .padding(Padding {
                top: 0.0,
                right: EAR_RADIUS,
                bottom: 0.0,
                left: EAR_RADIUS,
            })
            .align_child(Align::Center, Align::Start)
            .child(
                Rectangle::new()
                    .width(BOX_WIDTH)
                    .height(height)
                    .radius_bottom_left(CORNER_RADIUS)
                    .radius_bottom_right(CORNER_RADIUS)
                    .fill(Color::TRANSPARENT)
                    .shadow(super::island_shadow(0.35)),
            ),
        canvas,
        Rectangle::new()
            .width(TOTAL_WIDTH)
            .height(height)
            .fill(Color::TRANSPARENT)
            .padding(Padding {
                top: PADDING_TOP,
                right: EAR_RADIUS + PADDING_SIDE,
                bottom: PADDING_BOTTOM,
                left: EAR_RADIUS + PADDING_SIDE,
            })
            .align_child(Align::Start, Align::Start)
            .child(content),
    ]);

    let input = InputArea {
        x: EDGE_INSET_LEFT as i32,
        y: PANEL_HEIGHT as i32,
        width: TOTAL_WIDTH as i32,
        height: height.ceil() as i32,
    };

    window.input_region(vec![input]).child(
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
                    .child(island),
            ),
    )
}

fn search_field() -> Rectangle {
    let theme = theme();
    let icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/launcher/search.svg");

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
                    .fill(recolor::ink_image(icon, theme.text_muted)),
                Rectangle::new()
                    .width(10.0)
                    .height(1.0)
                    .fill(Color::TRANSPARENT),
                TextInput::new(SEARCH_FIELD)
                    .width(Parent)
                    .size(12.5)
                    .color(theme.text)
                    .placeholder("Search clipboard...")
                    .focused()
                    .on_change(|value| set_query(&value))
                    .on_submit(|_| {
                        if let Some(entry) = selected_entry() {
                            copy_entry(entry);
                        }
                    }),
            ])
            .width(Parent)
            .height(SEARCH_HEIGHT)
            .align(Align::Center),
        )
}

fn entry_row(entry: ClipboardEntry, selected: bool) -> Rectangle {
    let theme = theme();
    let preview = entry.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let preview: String = preview.chars().take(160).collect();
    let is_pinned = entry.pinned;
    let copy_on_click = entry.clone();

    Rectangle::new()
        .width(Parent)
        .height(ROW_HEIGHT)
        .fill(if selected {
            theme.accent_faint
        } else {
            Color::TRANSPARENT
        })
        .border(
            1.0,
            if selected {
                theme.accent
            } else {
                Color::TRANSPARENT
            },
        )
        .radius(7.0)
        .cursor(Cursor::Pointer)
        .on_click(move |_| copy_entry(copy_on_click.clone()))
        .padding(Padding {
            top: 0.0,
            right: 9.0,
            bottom: 0.0,
            left: 10.0,
        })
        .align_child(Align::Start, Align::Center)
        .child(
            Row::new(children![
                Column::new(children![
                    Text::new(preview)
                        .font("Maple Mono")
                        .size(12.5)
                        .weight(if selected {
                            Weight::SemiBold
                        } else {
                            Weight::Medium
                        })
                        .color(theme.text)
                        .elide(),
                    Text::new(if is_pinned {
                        "Pinned"
                    } else {
                        "Clipboard item"
                    })
                    .font("Maple Mono")
                    .size(10.5)
                    .color(theme.text_faint)
                    .elide(),
                ])
                .width(Parent)
                .gap(1.0)
                .align(Align::Start),
                Text::new(if is_pinned { "PIN" } else { "" })
                    .font("Maple Mono")
                    .size(9.0)
                    .weight(Weight::SemiBold)
                    .color(theme.text_muted),
            ])
            .width(Parent)
            .height(ROW_HEIGHT)
            .align(Align::Center)
            .justify(Start),
        )
}

fn island_path(width: f32, height: f32) -> Path {
    let ear_curve = 0.55228475 * EAR_RADIUS;
    let corner_curve = 0.55228475 * CORNER_RADIUS;
    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(
            ear_curve,
            0.0,
            EAR_RADIUS,
            EAR_RADIUS - ear_curve,
            EAR_RADIUS,
            EAR_RADIUS,
        )
        .line_to(EAR_RADIUS, height - CORNER_RADIUS)
        .cubic_to(
            EAR_RADIUS,
            height - CORNER_RADIUS + corner_curve,
            EAR_RADIUS + CORNER_RADIUS - corner_curve,
            height,
            EAR_RADIUS + CORNER_RADIUS,
            height,
        )
        .line_to(width - EAR_RADIUS - CORNER_RADIUS, height)
        .cubic_to(
            width - EAR_RADIUS - CORNER_RADIUS + corner_curve,
            height,
            width - EAR_RADIUS,
            height - CORNER_RADIUS + corner_curve,
            width - EAR_RADIUS,
            height - CORNER_RADIUS,
        )
        .line_to(width - EAR_RADIUS, EAR_RADIUS)
        .cubic_to(
            width - EAR_RADIUS,
            EAR_RADIUS - ear_curve,
            width - ear_curve,
            0.0,
            width,
            0.0,
        )
        .close()
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
