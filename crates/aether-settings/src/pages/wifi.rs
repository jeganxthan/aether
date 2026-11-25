use crate::{
    components::search_input::SearchInput,
    services::wifi::client::{ConnectedNetwork, WifiNetwork},
    settings_assets::{SIDEBAR_WIFI, WIFI_FULL, WIFI_HALF, WIFI_LOW, WIFI_QUARTER, svg_data},
};
use aether_themes::ThemePalette;
use gpui_kit::component::button::Button;
use gpui_kit::component::{Icon, IconName};
use gpui_kit::{
    base::{Disableable, input::InputState},
    component::{skeleton::Skeleton, spinner::Spinner, switch::Switch},
    prelude::FluentBuilder,
    *,
};
fn signal_icon(strength: u8) -> &'static str {
    match strength {
        75..=100 => WIFI_FULL,
        50..=74 => WIFI_HALF,
        25..=49 => WIFI_LOW,
        _ => WIFI_QUARTER,
    }
}

fn signal_color(palette: ThemePalette) -> u32 {
    palette.foreground
}

pub fn render(
    search_input: Entity<InputState>,
    palette: ThemePalette,
    wifi_enabled: Option<bool>,
    wifi_pending: bool,
    wifi_error: Option<String>,
    wifi_networks: Vec<WifiNetwork>,
    connected_network: Option<ConnectedNetwork>,
    networks_pending: bool,
    networks_error: Option<String>,
    on_enabled_change: impl Fn(bool, &mut App) + 'static,
    on_refresh: impl Fn(&mut App) + 'static,
    on_network_settings: impl Fn(ConnectedNetwork, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .p_4()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xl()
                        .text_color(rgb(palette.foreground))
                        .child("Wi-Fi"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette.muted_foreground))
                        .child("Manage wireless connections and nearby networks."),
                ),
        )
        // Search input
        .child(div().w_full().max_w(px(2020.0)).child(SearchInput::render(
            search_input,
            None,
            palette,
        )))
        // Wi-Fi settings row
        .child(
            div()
                .w_full()
                .max_w(px(2020.0))
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_3()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.surface_background))
                .hover(|style| style.bg(rgb(palette.hover_background)))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .flex_1()
                        .flex_shrink_1()
                        .min_w_0()
                        .child(
                            Icon::default()
                                .data(svg_data(SIDEBAR_WIFI))
                                .w(px(20.0))
                                .h(px(20.0))
                                .text_color(rgb(palette.foreground)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_1()
                                .child(div().text_color(rgb(palette.foreground)).child("Wi-Fi"))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(palette.muted_foreground))
                                        .child("Manage wireless connections"),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .flex_shrink_0()
                        .when(wifi_enabled != Some(true) || wifi_pending, |controls| {
                            controls.child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_full()
                                    .bg(rgb(palette.selected_background))
                                    .text_xs()
                                    .text_color(rgb(palette.muted_foreground))
                                    .child(if wifi_pending {
                                        "Updating"
                                    } else if wifi_enabled == Some(false) {
                                        "Off"
                                    } else {
                                        "Unavailable"
                                    }),
                            )
                        })
                        .child(
                            Switch::new("wifi-switch")
                                .checked(wifi_enabled.unwrap_or(false))
                                .disabled(wifi_pending || wifi_enabled.is_none())
                                .color(rgb(palette.accent))
                                .accessibility_label("Wi-Fi")
                                .on_change(move |enabled, _, cx| {
                                    on_enabled_change(*enabled, cx);
                                }),
                        ),
                ),
        )
        // Connected Network
        .child(
            div()
                .w_full()
                .max_w(px(2020.0))
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .gap_3()
                // Connected network
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div().flex().items_center().justify_between().child(
                                div()
                                    .text_color(rgb(palette.foreground))
                                    .child("Connected network"),
                            ),
                        )
                        .when_some(connected_network.as_ref(), |element, network| {
                            let selected_network = network.clone();

                            element.child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .p_3()
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.surface_background))
                                    .hover(|style| style.bg(rgb(palette.hover_background)))
                                    // Network information
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_3()
                                            .flex_1()
                                            .min_w_0()
                                            .child(
                                                Icon::default()
                                                    .data(svg_data(signal_icon(network.strength)))
                                                    .w(px(24.0))
                                                    .h(px(24.0))
                                                    .text_color(rgb(signal_color(palette))),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_color(rgb(palette.foreground))
                                                            .child(network.ssid.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(
                                                                palette.muted_foreground
                                                            ))
                                                            .child(format!(
                                                                "{} · {}% signal",
                                                                network.interface, network.strength
                                                            )),
                                                    ),
                                            ),
                                    )
                                    // Settings button
                                    .child(
                                        Button::new("connected-network-settings")
                                            .icon(Icon::new(IconName::Settings))
                                            .on_click(move |_, window, cx| {
                                                on_network_settings(
                                                    selected_network.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                    ),
                            )
                        })
                        .when(connected_network.is_none(), |element| {
                            element.child(
                                div()
                                    .w_full()
                                    .p_3()
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.surface_background))
                                    .text_sm()
                                    .text_color(rgb(palette.muted_foreground))
                                    .child("No connected Wi-Fi network"),
                            )
                        }),
                )
                // Available networks
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_color(rgb(palette.foreground))
                                .child("Available networks"),
                        )
                        .child(if networks_pending {
                            Spinner::new().into_any_element()
                        } else {
                            Button::new("refresh-wifi")
                                .icon(Icon::new(IconName::RefreshCw))
                                .disabled(wifi_enabled != Some(true))
                                .on_click(move |_, _, cx| {
                                    on_refresh(cx);
                                })
                                .into_any_element()
                        }),
                )
                // Loading skeletons
                .when(networks_pending, |element| {
                    element.children((0..3).map(|_| {
                        div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.surface_background))
                            .child(
                                Skeleton::new()
                                    .size_8()
                                    .rounded_md()
                                    .bg(rgb(palette.hover_background)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap_2()
                                    .child(
                                        Skeleton::new()
                                            .w(px(150.0))
                                            .h_4()
                                            .rounded_md()
                                            .bg(rgb(palette.hover_background)),
                                    )
                                    .child(
                                        Skeleton::new()
                                            .w(px(90.0))
                                            .h_3()
                                            .rounded_md()
                                            .bg(rgb(palette.hover_background)),
                                    ),
                            )
                    }))
                })
                // Network list
                .when(!networks_pending, |element| {
                    element.child(
                        div()
                            .id("wifi-network-list")
                            .w_full()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .children(
                                wifi_networks
                                    .iter()
                                    .filter(|network| {
                                        !connected_network
                                            .as_ref()
                                            .is_some_and(|connected| connected.ssid == network.ssid)
                                    })
                                    .map(|network| {
                                        div()
                                            .w_full()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .gap_3()
                                            .p_3()
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(rgb(palette.border))
                                            .bg(rgb(palette.surface_background))
                                            .hover(|style| style.bg(rgb(palette.hover_background)))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap_3()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .child(
                                                        Icon::default()
                                                            .data(svg_data(signal_icon(
                                                                network.strength,
                                                            )))
                                                            .w(px(20.0))
                                                            .h(px(20.0))
                                                            .text_color(rgb(signal_color(palette))),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_color(rgb(
                                                                        palette.foreground
                                                                    ))
                                                                    .child(network.ssid.clone()),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(rgb(
                                                                        palette.muted_foreground
                                                                    ))
                                                                    .child(format!(
                                                                        "{}% signal",
                                                                        network.strength
                                                                    )),
                                                            ),
                                                    ),
                                            )
                                            .when(network.is_saved, |element| {
                                                element.child(
                                                    div()
                                                        .px_2()
                                                        .py_1()
                                                        .rounded_full()
                                                        .bg(rgb(palette.selected_background))
                                                        .text_xs()
                                                        .text_color(rgb(palette.foreground))
                                                        .child("Saved"),
                                                )
                                            })
                                    }),
                            ),
                    )
                })
                .when(wifi_enabled == Some(false), |element| {
                    element.child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.surface_background))
                            .text_sm()
                            .text_color(rgb(palette.muted_foreground))
                            .child("Turn on Wi-Fi to see nearby networks."),
                    )
                })
                // Empty state
                .when(
                    !networks_pending
                        && wifi_enabled == Some(true)
                        && wifi_networks.is_empty()
                        && networks_error.is_none(),
                    |element| {
                        element.child(
                            div()
                                .p_4()
                                .rounded_lg()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.surface_background))
                                .text_sm()
                                .text_color(rgb(palette.muted_foreground))
                                .child("No Wi-Fi networks found"),
                        )
                    },
                )
                // Scan error
                .when_some(networks_error, |element, error| {
                    element.child(
                        div()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.danger))
                            .bg(rgb(palette.surface_background))
                            .text_sm()
                            .text_color(rgb(palette.danger))
                            .child(format!("Network scan error: {error}")),
                    )
                }),
        )
        .when_some(wifi_error, |element, error| {
            element.child(
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.danger))
                    .bg(rgb(palette.surface_background))
                    .text_sm()
                    .text_color(rgb(palette.danger))
                    .child(format!("Wi-Fi service error: {error}")),
            )
        })
        .into_any_element()
}
