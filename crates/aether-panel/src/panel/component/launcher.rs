
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use aether::{
    Align, Color, Column, Horizontal, Keyboard, Layer, LayerWindow, Margin, Parent,
    Padding, Rectangle, Row, ScrollArea, Start, Text, TextInput, Vertical, Weight,
    Zone, children,
};

use crate::panel::top::PANEL_HEIGHT;
use crate::services::applications::{Application, ApplicationService};
use crate::theme::theme;

const WINDOW_WIDTH: f32 = 520.0;
const WINDOW_HEIGHT: f32 = 560.0;
const PANEL_RADIUS: f32 = 16.0;
const PANEL_PADDING: f32 = 16.0;
const PANEL_TOP: f32 = 12.0;

const SEARCH_HEIGHT: f32 = 42.0;
const SEARCH_RADIUS: f32 = 10.0;
const SEARCH_TEXT_SIZE: f32 = 13.0;

const GRID_HEIGHT: f32 = 430.0;
const TILE_HEIGHT: f32 = 64.0;
const TILE_GAP: f32 = 8.0;
const TILE_RADIUS: f32 = 10.0;
const ICON_SIZE: f32 = 26.0;
const APP_NAME_SIZE: f32 = 11.0;

const SEARCH_FIELD: &str = "aether-launcher-search";

static OPEN: AtomicBool = AtomicBool::new(false);
static STARTED: OnceLock<()> = OnceLock::new();

static QUERY: OnceLock<Mutex<String>> = OnceLock::new();
static SELECTED: OnceLock<Mutex<usize>> = OnceLock::new();
static APPS: OnceLock<Mutex<Vec<Application>>> = OnceLock::new();

fn query() -> String {
    QUERY
        .get_or_init(|| Mutex::new(String::new()))
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default()
}

fn set_query(value: &str) {
    if let Ok(mut current) = QUERY
        .get_or_init(|| Mutex::new(String::new()))
        .lock()
    {
        *current = value.to_owned();
    }
}

fn selected() -> usize {
    *SELECTED
        .get_or_init(|| Mutex::new(0))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn set_selected(index: usize) {
    if let Ok(mut current) = SELECTED
        .get_or_init(|| Mutex::new(0))
        .lock()
    {
        *current = index;
    }
}

fn applications() -> Vec<Application> {
    APPS.get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map(|apps| apps.clone())
        .unwrap_or_default()
}

fn filtered_applications() -> Vec<Application> {
    let search = query().to_lowercase();
    let apps = applications();

    if search.is_empty() {
        return apps;
    }

    apps.into_iter()
        .filter(|app| app.name.to_lowercase().contains(&search))
        .collect()
}

pub fn open() -> bool {
    STARTED.get_or_init(|| ());
    OPEN.load(Ordering::Relaxed)
}

pub fn show() {
    STARTED.get_or_init(|| ());

    match ApplicationService::new() {
        Ok(service) => {
            if let Ok(mut apps) = APPS
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
            {
                *apps = service.all().to_vec();
            }
        }
        Err(error) => {
            eprintln!("Aether launcher: failed to load applications: {error}");
        }
    }

    set_query("");
    set_selected(0);

    TextInput::set_text(SEARCH_FIELD, "");
    OPEN.store(true, Ordering::Relaxed);

    aether::refresh();
}

pub fn close() {
    STARTED.get_or_init(|| ());

    if OPEN.swap(false, Ordering::Relaxed) {
        aether::refresh();
    }
}

pub fn toggle() {
    if open() {
        close();
    } else {
        show();
    }
}

fn launch_selected() {
    let apps = filtered_applications();
    let index = selected();

    if let Some(app) = apps.get(index).cloned() {
        close();

        if let Err(error) = ApplicationService::launch(&app) {
            eprintln!(
                "Aether launcher: failed to launch {}: {error}",
                app.name
            );
        }
    }
}

fn move_selection(delta: isize) {
    let apps = filtered_applications();

    if apps.is_empty() {
        set_selected(0);
        aether::refresh();
        return;
    }

    let current = selected().min(apps.len() - 1);
    let next = (current as isize + delta).rem_euclid(apps.len() as isize);

    set_selected(next as usize);
    aether::refresh();
}

pub fn launcher() -> LayerWindow {
    let window = LayerWindow::new()
        .width(aether::Full)
        .height(aether::Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-launcher")
        .margin(Margin {
            top: PANEL_HEIGHT as i32 + PANEL_TOP as i32,
            right: 0,
            bottom: 0,
            left: 12,
        })
        .keyboard(Keyboard::OnDemand)
        .on_key(|key| {
            if !open() {
                return;
            }

            match key {
                aether::Key::Escape => close(),
                aether::Key::Down => move_selection(1),
                aether::Key::Up => move_selection(-1),
                aether::Key::Enter => launch_selected(),
                _ => {}
            }
        });

    if !open() {
        return window.click_through().child(blank());
    }

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .on_click(|_| close())
            .child(
                Rectangle::new()
                    .width(WINDOW_WIDTH)
                    .height(WINDOW_HEIGHT)
                    .fill(theme().surface)
                    .border(1.0, theme().surface_border)
                    .radius(PANEL_RADIUS)
                    .shadow(
                        aether::Shadow::drop(theme().shadow)
                            .opacity(0.45)
                            .blur(24.0)
                            .offset(0.0, 10.0),
                    )
                    .padding(PANEL_PADDING)
                    .on_click(|_| {})
                    .child(launcher_body()),
            ),
    )
}

fn launcher_body() -> Column {
    let theme = theme();

    let search = TextInput::new(SEARCH_FIELD)
        .width(Parent)
        .size(SEARCH_TEXT_SIZE)
        .color(theme.text)
        .placeholder("Search applications...")
        .focused()
        .on_change(|value| {
            set_query(&value);
            set_selected(0);
            aether::refresh();
        })
        .on_submit(|_| launch_selected());

    Column::new(children![
        Row::new(children![
            Text::new("Applications")
                .color(theme.text)
                .size(17.0)
                .weight(Weight::SemiBold),
            Rectangle::new()
                .width(Parent)
                .height(1.0)
                .fill(Color::TRANSPARENT),
            Text::new("ESC")
                .color(theme.text_muted)
                .size(10.0),
        ])
        .width(Parent)
        .height(28.0)
        .align(Align::Center)
        .justify(Start),
        Rectangle::new()
            .width(Parent)
            .height(SEARCH_HEIGHT)
            .fill(theme.chip)
            .border(1.0, theme.chip_border)
            .radius(SEARCH_RADIUS)
            .padding(Padding {
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            })
            .align_child(Align::Start, Align::Center)
            .child(search),
        application_grid(),
        Text::new("↑ ↓ Navigate    ↵ Open    Esc Close")
            .color(theme.text_faint)
            .size(10.0),
    ])
    .width(Parent)
    .height(Parent)
    .gap(12.0)
    .align(Align::Start)
}

fn application_grid() -> ScrollArea {
    let apps = filtered_applications();
    let selected_index = selected();

    let tiles: Vec<Box<dyn aether::Widget>> = apps
        .into_iter()
        .enumerate()
        .map(|(index, app)| {
            Box::new(application_tile(app, index == selected_index))
                as Box<dyn aether::Widget>
        })
        .collect();

    let content = Column::new(tiles)
        .width(Parent)
        .height(GRID_HEIGHT)
        .gap(TILE_GAP);

    ScrollArea::new("aether-launcher-apps", content)
        .width(Parent)
        .height(GRID_HEIGHT)
}

fn application_tile(app: Application, is_selected: bool) -> Rectangle {
    let theme = theme();

    let background = if is_selected {
        theme.chip
    } else {
        Color::TRANSPARENT
    };

    let app_name = app.name.clone();

    Rectangle::new()
        .width(Parent)
        .height(TILE_HEIGHT)
        .fill(background)
        .border(
            if is_selected { 1.0 } else { 0.0 },
            theme.chip_border,
        )
        .radius(TILE_RADIUS)
        .padding(10.0)
        .cursor(aether::Cursor::Pointer)
        .on_click(move |_| {
            if let Err(error) = ApplicationService::launch(&app) {
                eprintln!(
                    "Aether launcher: failed to launch {}: {error}",
                    app.name
                );
            }

            close();
        })
        .child(
            Row::new(children![
                Rectangle::new()
                    .width(ICON_SIZE)
                    .height(ICON_SIZE)
                    .fill(theme.icon),
                Text::new(app_name)
                    .color(theme.text)
                    .size(APP_NAME_SIZE)
                    .weight(Weight::Medium)
                    .elide(),
            ])
            .width(Parent)
            .height(Parent)
            .gap(12.0)
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
