use aether::{
    Align, Center, Color, Cursor, End, Full, Layer, LayerWindow, Padding, Parent, Rectangle, Row,
    Stack, Start, Text, Vertical, Widget, Zone, children,
};

use crate::panel::component::battery_popup;
use crate::panel::component::launcher;
use crate::panel::component::player_popup;
use crate::panel::component::wifi_popup::{hovered, open, set_hovered, toggle_open};
use crate::panel::svg::battery_svg::BatteryIcon;
use crate::panel::svg::distro_svg::distro_icon;
use crate::panel::svg::player_svg::player_brand_icon;
use crate::panel::svg::wifi_svg::WifiSignalIcon;
use crate::services::battery::battery;
use crate::services::clock::{clock, date};
use crate::services::player::player;
use crate::services::wifi::{ethernet_connected, wifi};
use crate::theme::theme;

pub const PANEL_HEIGHT: f32 = 26.0;
const ICON_SIZE: f32 = 14.0;
const PLAYER_HIT_WIDTH: f32 = 22.0;

const BATTERY_ICON_SIZE: f32 = 14.0;

const WIFI_HIT_WIDTH: f32 = 22.0;

// Keep the player, wifi, and battery icon groups visually separated.
const ICON_GAP: f32 = 14.0;

// a tighter gap between the percentage and the icon it belongs to
const PERCENT_GAP: f32 = 6.0;

// the gap between the date and the time in the centered clock
const CLOCK_GAP: f32 = 8.0;

// the size both the date and the time are drawn at
const CLOCK_SIZE: f32 = 12.0;

// keeps the centered clock and edge icons off the screen edge
const EDGE_INSET: f32 = 12.0;

pub fn panel() -> LayerWindow {
    let theme = theme();

    let time = clock();
    let today = date();

    // the last reading of each service; the view never waits on the bus for them
    let signal = wifi();
    let ethernet = ethernet_connected();
    let battery = battery();
    let active_player = player();

    // no connection shows the crossed-out icon, a connection shows its strength
    let wifi_icon = match &signal {
        Some(signal) => WifiSignalIcon::from_strength(signal.strength),
        None => WifiSignalIcon::None,
    };
    let wifi_secured = signal.as_ref().is_some_and(|signal| signal.secured);

    // the charging icon only while the battery reports charging; the percentage rides along
    let battery_icon = battery.map(|battery| {
        (
            BatteryIcon::from_percentage(battery.percentage),
            battery.is_charging(),
            battery.percentage,
        )
    });

    let launcher_active = launcher::is_open();

    let mut right_items: Vec<Box<dyn Widget>> = Vec::new();

    // Keep the active player's brand icon visible in the panel.
    if let Some(p) = active_player {
        right_items.push(Box::new(
            Rectangle::new()
                .width(PLAYER_HIT_WIDTH)
                .height(Parent)
                .fill(Color::TRANSPARENT)
                .cursor(Cursor::Pointer)
                .align_child(Align::Center, Align::Center)
                .on_hover(player_popup::set_hovered)
                .on_click(|_| player_popup::toggle_menu())
                .child(
                    Rectangle::new()
                        .width(ICON_SIZE)
                        .height(ICON_SIZE)
                        .fill(player_brand_icon(p.brand)),
                ),
        ));
        right_items.push(Box::new(
            Rectangle::new().width(ICON_GAP).height(1.0).fill(theme.bar),
        ));
    }
    let clipboard_icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/panel/clipboard.svg");
    right_items.push(Box::new(
        Rectangle::new()
            .width(PLAYER_HIT_WIDTH)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .cursor(Cursor::Pointer)
            .align_child(Align::Center, Align::Center)
            .on_click(|_| crate::panel::component::clipboard_popup::toggle())
            .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                crate::panel::svg::recolor::ink_image(clipboard_icon, theme.icon),
            )),
    ));
    right_items.push(Box::new(
        Rectangle::new().width(6.0).height(1.0).fill(theme.bar),
    ));
    // Wifi icon
    let wifi_active = hovered() || open();
    right_items.push(Box::new(
        Rectangle::new()
            .width(WIFI_HIT_WIDTH)
            .height(Parent)
            .fill(if wifi_active {
                aether::Color::BLACK
            } else {
                aether::Color::TRANSPARENT
            })
            .cursor(Cursor::Pointer)
            .align_child(Align::Center, Align::Center)
            .on_hover(set_hovered)
            .on_click(|_| {
                toggle_open();
                aether::refresh();
            })
            .child(
                Rectangle::new()
                    .width(ICON_SIZE)
                    .height(ICON_SIZE)
                    .fill(if ethernet {
                        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("../../assets/icons/wifi/network-wired-symbolic.svg");

                        crate::panel::svg::recolor::ink_image(path, theme.icon)
                    } else {
                        wifi_icon.image(wifi_secured)
                    }),
            ),
    ));

    // small gap between wifi and bluetooth icons
    right_items.push(Box::new(
        Rectangle::new()
            .width(6.0)
            .height(1.0)
            .fill(theme.bar),
    ));
    let bluetooth = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/panel/bluetooth.svg");
    right_items.push(Box::new(
        Rectangle::new()
            .width(PLAYER_HIT_WIDTH)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .cursor(Cursor::Pointer)
            .align_child(Align::Center, Align::Center)
            .on_click(|_| crate::panel::component::clipboard_popup::toggle())
            .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                crate::panel::svg::recolor::ink_image(bluetooth, theme.icon),
            )),
    ));
    right_items.push(Box::new(
        Rectangle::new().width(6.0).height(1.0).fill(theme.bar),
    ));
    // battery reading and icon
    let battery_item = match battery_icon {
        Some((icon, charging, percentage)) => Rectangle::new()
            .width(50.0)
            .height(Parent)
            .fill(if battery_popup::hovered() {
                aether::Color::BLACK
            } else {
                aether::Color::TRANSPARENT
            })
            .cursor(Cursor::Pointer)
            .align_child(Align::End, Align::Center)
            .on_hover(battery_popup::set_hovered)
            .on_click(|_| crate::panel::component::battery_menu::toggle_open())
            .child(
                Row::new(children![
                    Text::new(format!("{percentage}%"))
                        .font("Maple Mono")
                        .color(theme.bar_text)
                        .size(12.0),
                    Rectangle::new()
                        .width(PERCENT_GAP)
                        .height(1.0)
                        .fill(Color::TRANSPARENT),
                    Rectangle::new()
                        .width(BATTERY_ICON_SIZE)
                        .height(BATTERY_ICON_SIZE)
                        .fill(icon.image(charging, false)),
                ])
                .width(Parent)
                .height(Parent)
                .align(Align::Center)
                .justify(End),
            ),
        None => Rectangle::new()
            .width(50.0)
            .height(Parent)
            .fill(Color::TRANSPARENT),
    };
    right_items.push(Box::new(battery_item));
    right_items.push(Box::new(
        Rectangle::new().width(ICON_GAP).height(1.0).fill(theme.bar),
    ));

    let settings_icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/panel/settings.svg");
    right_items.push(Box::new(
        Rectangle::new()
            .width(PLAYER_HIT_WIDTH)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .on_hover(crate::panel::component::settings_popup::set_hovered)
            .align_child(Align::Center, Align::Center)
            .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                crate::panel::svg::recolor::ink_image(settings_icon, theme.icon),
            )),
    ));

    LayerWindow::new()
        .width(Full)
        .height(PANEL_HEIGHT)
        .anchor_vertical(Vertical::Top)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("aether-panel")
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(theme.bar)
                .padding(Padding {
                    top: 0.0,
                    right: EDGE_INSET,
                    bottom: 0.0,
                    left: EDGE_INSET,
                })
                .child(
                    Stack::new(children![
                        Row::new(children![
                            Rectangle::new()
                                .width(26.0)
                                .height(Parent)
                                .fill(if launcher_active {
                                    aether::Color::BLACK
                                } else {
                                    aether::Color::TRANSPARENT
                                })
                                .cursor(Cursor::Pointer)
                                .align_child(Align::Center, Align::Center)
                                .on_hover(|hovered| {
                                    if hovered {
                                        launcher::open_launcher_on_hover();
                                    }
                                })
                                .on_click(|_| launcher::toggle())
                                .child(distro_icon()),
                        ])
                        .width(26.0)
                        .height(Parent)
                        .align(Align::Center)
                        .justify(Start),
                        Row::new(children![
                            Text::new(today)
                                .font("Maple Mono")
                                .color(theme.bar_text)
                                .size(CLOCK_SIZE),
                            Rectangle::new()
                                .width(CLOCK_GAP)
                                .height(1.0)
                                .fill(theme.bar),
                            Text::new(time)
                                .font("Maple Mono")
                                .color(theme.bar_text)
                                .size(CLOCK_SIZE),
                        ])
                        .width(Parent)
                        .height(Parent)
                        .align(Align::Center)
                        .justify(Center),
                        Row::new(right_items)
                            .width(Parent)
                            .height(Parent)
                            .align(Align::Center)
                            .justify(End),
                    ])
                    .width(Parent)
                    .height(Parent),
                ),
        )
}
