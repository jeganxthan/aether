use aether::{
    Align, Center, Color, Cursor, End, Full, Layer, LayerWindow, Padding, Parent, Rectangle, Row,
    Stack, Start, Text, Vertical, Widget, Zone, children,
};

use crate::panel::component::battery_popup;
use crate::panel::component::bluetooth_popup;
use crate::panel::component::launcher;
use crate::panel::component::notification_menu;
use crate::panel::component::player_popup;
use crate::panel::component::settings_popup;
use crate::panel::component::wifi_popup;
use crate::panel::svg::battery_svg::BatteryIcon;
use crate::panel::svg::distro_svg::distro_icon;
use crate::panel::svg::player_svg::player_brand_icon;
use crate::panel::svg::wifi_svg::WifiSignalIcon;
use crate::services::battery::battery;
use crate::services::bluetooth as bt_service;
use crate::services::clock::{clock, date};
use crate::services::display;
use crate::services::player::player;
use crate::services::wifi::{ethernet_connected, wifi};
use crate::theme::theme;

pub const PANEL_HEIGHT: f32 = 26.0;
const ICON_SIZE: f32 = 14.0;
const PLAYER_HIT_WIDTH: f32 = 22.0;
const NOTIFICATION_HIT_WIDTH: f32 = 22.0;
const NOTIFICATION_SETTINGS_GAP: f32 = 4.0;

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
pub const NOTIFICATION_EDGE_RIGHT: f32 = EDGE_INSET + PLAYER_HIT_WIDTH + NOTIFICATION_SETTINGS_GAP;

pub fn panel(monitor: &aether::Monitor) -> LayerWindow {
    let launcher_monitor = monitor.name.clone();
    let hover_monitor = launcher_monitor.clone();
    let click_monitor = launcher_monitor;
    display::watch_primary_display();
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
        let hover_monitor_player = monitor.name.clone();
        let click_monitor_player = monitor.name.clone();
        right_items.push(Box::new(
            Rectangle::new()
                .width(PLAYER_HIT_WIDTH)
                .height(Parent)
                .fill(Color::TRANSPARENT)
                .cursor(Cursor::Pointer)
                .align_child(Align::Center, Align::Center)
                .on_hover(move |hovered| {
                    if hovered {
                        player_popup::open_player_on_hover_for_monitor(hover_monitor_player.clone());
                    } else {
                        player_popup::set_hovered(false);
                    }
                })
                .on_click(move |_| {
                    player_popup::toggle_player_for_monitor(click_monitor_player.clone());
                })
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
    // Wifi icon
    let wifi_active = wifi_popup::hovered() || wifi_popup::open();
    let hover_monitor_wifi = monitor.name.clone();
    let click_monitor_wifi = monitor.name.clone();
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
            .on_hover(move |hovered| {
                if hovered {
                    wifi_popup::open_wifi_on_hover_for_monitor(hover_monitor_wifi.clone());
                } else {
                    wifi_popup::set_hovered(false);
                }
            })
            .on_click(move |_| {
                wifi_popup::toggle_wifi_for_monitor(click_monitor_wifi.clone());
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
        Rectangle::new().width(6.0).height(1.0).fill(theme.bar),
    ));
    let bt_powered = bt_service::powered();
    let bluetooth_icon = if bt_powered {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/panel/bluetooth.svg")
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/panel/bluetooth-off.svg")
    };
    let hover_monitor_bt = monitor.name.clone();
    let click_monitor_bt = monitor.name.clone();
    right_items.push(Box::new(
        Rectangle::new()
            .width(PLAYER_HIT_WIDTH)
            .height(Parent)
            .fill(
                if crate::panel::component::bluetooth_popup::hovered()
                    || crate::panel::component::bluetooth_menu::open()
                {
                    aether::Color::BLACK
                } else {
                    Color::TRANSPARENT
                },
            )
            .cursor(Cursor::Pointer)
            .align_child(Align::Center, Align::Center)
            .on_hover(move |hovered| {
                if hovered {
                    bluetooth_popup::open_bluetooth_on_hover_for_monitor(hover_monitor_bt.clone());
                } else {
                    bluetooth_popup::set_hovered(false);
                }
            })
            .on_click(move |_| {
                bluetooth_popup::toggle_bluetooth_for_monitor(click_monitor_bt.clone());
            })
            .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                crate::panel::svg::recolor::ink_image(bluetooth_icon, theme.icon),
            )),
    ));
    right_items.push(Box::new(
        Rectangle::new().width(6.0).height(1.0).fill(theme.bar),
    ));
    // battery reading and icon
    let battery_item = match battery_icon {
        Some((icon, charging, percentage)) => {
            let hover_monitor_bat = monitor.name.clone();
            let click_monitor_bat = monitor.name.clone();
            Rectangle::new()
                .width(50.0)
                .height(Parent)
                .fill(if battery_popup::hovered() {
                    aether::Color::BLACK
                } else {
                    aether::Color::TRANSPARENT
                })
                .cursor(Cursor::Pointer)
                .align_child(Align::End, Align::Center)
                .on_hover(move |hovered| {
                    if hovered {
                        battery_popup::open_battery_on_hover_for_monitor(hover_monitor_bat.clone());
                    } else {
                        battery_popup::set_hovered(false);
                    }
                })
                .on_click(move |_| {
                    battery_popup::toggle_battery_for_monitor(click_monitor_bat.clone());
                })
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
                )
        }
        None => Rectangle::new()
            .width(50.0)
            .height(Parent)
            .fill(Color::TRANSPARENT),
    };
    right_items.push(Box::new(battery_item));
    right_items.push(Box::new(
        Rectangle::new().width(ICON_GAP).height(1.0).fill(theme.bar),
    ));

    let notification_hovered = crate::panel::component::notification_menu::hovered()
        || crate::panel::component::notification_menu::open();
    let notification_icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        if crate::services::notifications::do_not_disturb() {
            "../../assets/icons/notification/bell-off.svg"
        } else {
            "../../assets/icons/notification/bell.svg"
        },
    );
    let hover_monitor_notif = monitor.name.clone();
    let click_monitor_notif = monitor.name.clone();
    right_items.push(Box::new(
        Rectangle::new()
            .width(NOTIFICATION_HIT_WIDTH)
            .height(Parent)
            .fill(if notification_hovered {
                aether::Color::BLACK
            } else {
                Color::TRANSPARENT
            })
            .cursor(Cursor::Pointer)
            .align_child(Align::Center, Align::Center)
            .on_hover(move |hovered| {
                if hovered {
                    notification_menu::open_notification_on_hover_for_monitor(hover_monitor_notif.clone());
                } else {
                    notification_menu::set_hovered(false);
                }
            })
            .on_click(move |_| {
                notification_menu::toggle_notification_for_monitor(click_monitor_notif.clone());
            })
            .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                crate::panel::svg::recolor::ink_image(notification_icon, theme.icon),
            )),
    ));
    right_items.push(Box::new(
        Rectangle::new()
            .width(NOTIFICATION_SETTINGS_GAP)
            .height(1.0)
            .fill(theme.bar),
    ));

    let settings_icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/panel/settings.svg");
    let hover_monitor_settings = monitor.name.clone();
    let click_monitor_settings = monitor.name.clone();
    right_items.push(Box::new(
        Rectangle::new()
            .width(PLAYER_HIT_WIDTH)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .on_hover(move |hovered| {
                if hovered {
                    settings_popup::open_settings_on_hover_for_monitor(hover_monitor_settings.clone());
                } else {
                    settings_popup::set_hovered(false);
                }
            })
            .on_click(move |_| {
                settings_popup::toggle_settings_for_monitor(click_monitor_settings.clone());
            })
            .align_child(Align::Center, Align::Center)
            .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                crate::panel::svg::recolor::ink_image(settings_icon, theme.icon),
            )),
    ));

    let left_items: Vec<Box<dyn Widget>> = vec![Box::new(
        Rectangle::new()
            .width(26.0)
            .height(Parent)
            .fill(if launcher_active {
                aether::Color::BLACK
            } else {
                Color::TRANSPARENT
            })
            .cursor(Cursor::Pointer)
            .align_child(Align::Center, Align::Center)
            .on_hover(move |hovered| {
                if hovered {
                    launcher::open_launcher_on_hover_for_monitor(hover_monitor.clone());
                } else {
                    launcher::set_hovered(false);
                }
            })
            .on_click(move |_| launcher::toggle_for_monitor(click_monitor.clone()))
            .child(distro_icon()),
    )];
    LayerWindow::new()
        .width(Full)
        .height(PANEL_HEIGHT)
        .anchor_vertical(Vertical::Top)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .visible(display::is_primary(
            &monitor.name,
            monitor.x,
            monitor.y,
            &monitor.outputs,
            monitor.origin_output.as_deref(),
        ))
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
                        Row::new(left_items)
                            .width(Parent)
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
