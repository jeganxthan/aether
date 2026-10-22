use aether::{
    Align, Center, Cursor, End, Full, Layer, LayerWindow, Padding, Parent, Rectangle, Row, Stack,
    Start, Text, Vertical, Zone, children,
};

use crate::panel::component::wifi_popup::{set_hovered, toggle_open};
use crate::panel::svg::battery_svg::BatteryIcon;
use crate::panel::svg::distro_svg::distro_icon;
use crate::panel::svg::wifi_svg::WifiSignalIcon;
use crate::services::battery::battery;
use crate::services::clock::{clock, date};
use crate::services::wifi::{ethernet_connected, wifi};
use crate::theme::theme;

pub const PANEL_HEIGHT: f32 = 26.0;
const ICON_SIZE: f32 = 14.0;

const BATTERY_ICON_SIZE: f32 = 14.0;

const WIFI_HIT_WIDTH: f32 = 22.0;

// a roomier gap between the wifi and battery icons, adjusted for the wider hit area
const ICON_GAP: f32 = 10.0;

// a tighter gap between the percentage and the icon it belongs to
const PERCENT_GAP: f32 = 4.0;

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
                // both rows fill the bar, so the clock centers and the icons end without top rows
                .child(
                    Stack::new(children![
                        Row::new(children![
                            Rectangle::new()
                                .width(24.0)
                                .height(Parent)
                                .fill(aether::Color::TRANSPARENT)
                                .cursor(Cursor::Pointer)
                                .align_child(Align::Center, Align::Center)
                                .on_click(|_| crate::panel::component::launcher::toggle())
                                .child(distro_icon()),
                        ])
                        .width(24.0)
                        .height(Parent)
                        .align(Align::Center)
                        .justify(Start),
                        Row::new(children![
                            // the date first, then the time, like OCT 20  14:32, both the same size
                            Text::new(today).color(theme.bar_text).size(CLOCK_SIZE),
                            Rectangle::new()
                                .width(CLOCK_GAP)
                                .height(1.0)
                                .fill(theme.bar),
                            Text::new(time).color(theme.bar_text).size(CLOCK_SIZE),
                        ])
                        .width(Parent)
                        .height(Parent)
                        .align(Align::Center)
                        .justify(Center),
                        Row::new(children![
                            Rectangle::new()
                                .width(WIFI_HIT_WIDTH)
                                .height(Parent)
                                .fill(aether::Color::TRANSPARENT)
                                .cursor(Cursor::Pointer)
                                .align_child(Align::Center, Align::Center)
                                // hovering the wifi icon shows the dynamic island, a click opens the full box
                                .on_hover(set_hovered)
                                .on_click(|_| toggle_open())
                                .child(Rectangle::new().width(ICON_SIZE).height(ICON_SIZE).fill(
                                    if ethernet {
                                        let path = std::path::PathBuf::from(env!(
                                            "CARGO_MANIFEST_DIR"
                                        ))
                                        .join("../../assets/icons/wifi/network-wired-symbolic.svg");
                                        crate::panel::svg::recolor::ink_image(path, theme.icon)
                                    } else {
                                        wifi_icon.image(wifi_secured)
                                    }
                                )),
                            // small gap between the wifi and battery icons
                            Rectangle::new().width(ICON_GAP).height(1.0).fill(theme.bar),
                            battery_icon.map_or_else(
                                // no battery: an empty slot keeps the row laid out the same
                                || {
                                    Row::new(children![
                                        Rectangle::new()
                                            .width(BATTERY_ICON_SIZE)
                                            .height(BATTERY_ICON_SIZE)
                                            .fill(theme.bar),
                                    ])
                                    .height(Parent)
                                    .align(Align::Center)
                                },
                                |(icon, charging, percentage)| {
                                    Row::new(children![
                                        // the number sits just left of its icon
                                        Text::new(format!("{percentage}%"))
                                            .color(theme.bar_text)
                                            .size(12.0),
                                        Rectangle::new()
                                            .width(PERCENT_GAP)
                                            .height(1.0)
                                            .fill(theme.bar),
                                        // limiting is not reported yet, so it stays off
                                        Rectangle::new()
                                            .width(BATTERY_ICON_SIZE)
                                            .height(BATTERY_ICON_SIZE)
                                            .fill(icon.image(charging, false)),
                                    ])
                                    .height(Parent)
                                    .align(Align::Center)
                                },
                            ),
                        ])
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
