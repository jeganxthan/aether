use std::path::PathBuf;

use aether::{
    Align, Color, Column, Full, Image, Keyboard, LayerWindow, Padding, Parent, Rectangle, Service,
    Stack, Text, TextInput, Weight, children,
};

use crate::services::clock;

const BACKGROUND: Color = Color::rgb(20, 22, 29);
const SHADE: Color = Color::rgba(0, 0, 0, 60);
const ACCENT: Color = Color::rgb(151, 190, 255);
const TEXT: Color = Color::rgb(245, 246, 250);
const MUTED: Color = Color::rgba(245, 246, 250, 176);
const FAINT: Color = Color::rgba(245, 246, 250, 112);
const FIELD: Color = Color::rgba(0, 0, 0, 110);
const FIELD_BORDER: Color = Color::rgba(245, 246, 250, 72);
const ERROR: Color = Color::rgb(255, 112, 126);

const PASSWORD_FIELD: &str = "aether-lock-password";

pub fn view(_: &aether::Monitor) -> LayerWindow {
    let lock = aether::Lock::read();
    let time = clock::clock();
    let date = clock::date();
    let user = std::env::var("USER").unwrap_or_else(|_| String::from("user"));
    let initial = user
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let (status, status_color) = if lock.checking() {
        ("Checking…", MUTED)
    } else if lock.failed() {
        ("Wrong password, try again", ERROR)
    } else {
        ("Enter your password to unlock", MUTED)
    };

    let background = lock_wallpaper().map_or_else(
        || {
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(BACKGROUND)
        },
        |path| {
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Image::cover(path).thumbnail(640, 360).blurred(32))
        },
    );

    let avatar = Rectangle::new()
        .width(76.0)
        .height(76.0)
        .fill(Color::rgba(151, 190, 255, 50))
        .border(1.5, Color::rgba(151, 190, 255, 153))
        .align_child(Align::Center, Align::Center)
        .child(
            Text::new(initial)
                .color(ACCENT)
                .size(32.0)
                .weight(Weight::Bold),
        );

    let field = Rectangle::new()
        .width(340.0)
        .height(46.0)
        .fill(FIELD)
        .border(1.0, if lock.failed() { ERROR } else { FIELD_BORDER })
        .padding(Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 14.0,
        })
        .align_child(Align::Center, Align::Center)
        .child(
            TextInput::new(PASSWORD_FIELD)
                .placeholder("Password")
                .password()
                .focused()
                .size(16.0)
                .color(TEXT)
                .width(312.0)
                .on_submit(|password| {
                    aether::Lock::unlock(&password);
                    TextInput::set_text(PASSWORD_FIELD, "");
                }),
        );

    let content = Column::new(children![
        Text::new(time)
            .color(TEXT)
            .size(128.0)
            .weight(Weight::Light),
        Text::new(date).color(MUTED).size(22.0),
        Rectangle::new().width(1.0).height(48.0),
        avatar,
        Text::new(user)
            .color(TEXT)
            .size(17.0)
            .weight(Weight::SemiBold),
        Rectangle::new().width(1.0).height(6.0),
        field,
        Text::new(status).color(status_color).size(12.5),
    ])
    .gap(12.0)
    .align(Align::Center);

    let footer = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 28.0,
            left: 0.0,
        })
        .align_child(Align::Center, aether::End)
        .child(
            Text::new("LOCKED")
                .color(FAINT)
                .size(12.0)
                .weight(Weight::Medium),
        );

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .keyboard(Keyboard::OnDemand)
        .namespace("aether-lockscreen")
        .child(Stack::new(children![
            background,
            Rectangle::new().width(Parent).height(Parent).fill(SHADE),
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .align_child(Align::Center, Align::Center)
                .child(content),
            footer,
        ]))
}

fn lock_wallpaper() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("AETHER_LOCK_WALLPAPER")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Some(path);
    }

    wallpaper_from_settings().or_else(wallpaper_from_swaybg)
}

fn wallpaper_from_settings() -> Option<PathBuf> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    let path =
        std::fs::read_to_string(config_dir.join("com.aether-settings/wallpaper.txt")).ok()?;
    let path = PathBuf::from(path.trim());
    path.is_file().then_some(path)
}

fn wallpaper_from_swaybg() -> Option<PathBuf> {
    let processes = std::fs::read_dir("/proc").ok()?;
    for process in processes.flatten() {
        let process_dir = process.path();
        let Ok(executable) = std::fs::read_link(process_dir.join("exe")) else {
            continue;
        };
        if executable.file_name().is_none_or(|name| name != "swaybg") {
            continue;
        }

        let Ok(command_line) = std::fs::read(process_dir.join("cmdline")) else {
            continue;
        };
        let arguments: Vec<&str> = command_line
            .split(|byte| *byte == 0)
            .filter_map(|argument| std::str::from_utf8(argument).ok())
            .collect();
        let mut index = 0;
        while index < arguments.len() {
            if arguments[index] == "-i" || arguments[index] == "--image" {
                if let Some(path) = arguments.get(index + 1).map(PathBuf::from)
                    && path.is_file()
                {
                    return Some(path);
                }
            } else if let Some(image) = arguments[index].strip_prefix("--image=") {
                let path = PathBuf::from(image);
                if path.is_file() {
                    return Some(path);
                }
            }
            index += 1;
        }
    }
    None
}
