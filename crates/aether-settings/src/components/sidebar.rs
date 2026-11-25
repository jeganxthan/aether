use gpui_kit::*;

use crate::settings_assets::{SIDEBAR_ETHERNET, SIDEBAR_WIFI, svg_data};
use crate::{app::SettingsPage, components::search_input::SearchInput};
use aether_themes::ThemePalette;
use gpui_kit::component::{
    Icon, IconName,
    input::InputState,
    sidebar::{
        Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
        SidebarToggleButton,
    },
};
use std::sync::Arc;

pub struct SettingsSidebar;

impl SettingsSidebar {
    pub fn render(
        collapsed: bool,
        search_input: Entity<InputState>,
        niri_blur: bool,
        palette: ThemePalette,
        current_page: SettingsPage,
        on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,

        on_navigate: impl Fn(SettingsPage, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let on_navigate = Arc::new(on_navigate);
        let sidebar_bg = if niri_blur {
            rgba(palette.sidebar_blur_background)
        } else {
            rgb(palette.sidebar_background)
        };
        let sidebar_border = if niri_blur {
            rgba(palette.sidebar_blur_border)
        } else {
            rgb(palette.sidebar_border)
        };

        let sidebar = Sidebar::new("settings-sidebar")
            .w(px(240.0))
            .bg(sidebar_bg)
            .border_color(sidebar_border)
            .border_r_1()
            .collapsible(true)
            .collapsed(collapsed)
            .header(
                SidebarHeader::new().child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(if collapsed {
                            div().w_full().flex().justify_center().child(
                                SidebarToggleButton::new()
                                    .collapsed(collapsed)
                                    .on_click(on_toggle),
                            )
                        } else {
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().flex_1().child(if niri_blur {
                                    SearchInput::render_translucent(
                                        search_input,
                                        Some(180.0), // Width
                                        palette,
                                    )
                                    .into_any_element()
                                } else {
                                    SearchInput::render(
                                        search_input,
                                        Some(180.0), // Width
                                        palette,
                                    )
                                    .into_any_element()
                                }))
                                .child(
                                    SidebarToggleButton::new()
                                        .collapsed(collapsed)
                                        .on_click(on_toggle),
                                )
                        }),
                ),
            )
            .child(
                SidebarGroup::new("General").child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("Appearance")
                                .icon(Icon::new(IconName::Palette).text_color(rgb(
                                    if current_page == SettingsPage::Appearance {
                                        palette.sidebar_active_foreground
                                    } else {
                                        palette.sidebar_foreground
                                    },
                                )))
                                .active(current_page == SettingsPage::Appearance)
                                .on_click({
                                    let on_navigate = Arc::clone(&on_navigate);

                                    move |_, window, cx| {
                                        on_navigate(SettingsPage::Appearance, window, cx);
                                    }
                                }),
                        )
                        .child(
                            SidebarMenuItem::new("Display")
                                .icon(Icon::new(IconName::Frame).text_color(rgb(
                                    if current_page == SettingsPage::Display {
                                        palette.sidebar_active_foreground
                                    } else {
                                        palette.sidebar_foreground
                                    },
                                )))
                                .active(current_page == SettingsPage::Display)
                                .on_click({
                                    let on_navigate = Arc::clone(&on_navigate);
                                    move |_, window, cx| {
                                        on_navigate(SettingsPage::Display, window, cx);
                                    }
                                }),
                        )
                        .child(
                            SidebarMenuItem::new("Network")
                                .icon(
                                    Icon::new(IconName::Network)
                                        .text_color(rgb(palette.sidebar_foreground)),
                                )
                                .children([
                                    SidebarMenuItem::new("Wi-Fi")
                                        .icon(
                                            Icon::default()
                                                .data(svg_data(SIDEBAR_WIFI))
                                                .w(px(16.0))
                                                .h(px(16.0))
                                                .text_color(rgb(
                                                    if current_page == SettingsPage::Wifi {
                                                        palette.sidebar_active_foreground
                                                    } else {
                                                        palette.sidebar_foreground
                                                    },
                                                )),
                                        )
                                        .active(current_page == SettingsPage::Wifi)
                                        .on_click({
                                            let on_navigate = Arc::clone(&on_navigate);
                                            move |_, window, cx| {
                                                on_navigate(SettingsPage::Wifi, window, cx);
                                            }
                                        }),
                                    SidebarMenuItem::new("Ethernet")
                                        .icon(
                                            Icon::default()
                                                .data(svg_data(SIDEBAR_ETHERNET))
                                                .w(px(16.0))
                                                .h(px(16.0))
                                                .text_color(rgb(
                                                    if current_page == SettingsPage::Ethernet {
                                                        palette.sidebar_active_foreground
                                                    } else {
                                                        palette.sidebar_foreground
                                                    },
                                                )),
                                        )
                                        .active(current_page == SettingsPage::Ethernet)
                                        .on_click({
                                            let on_navigate = Arc::clone(&on_navigate);
                                            move |_, window, cx| {
                                                on_navigate(SettingsPage::Ethernet, window, cx);
                                            }
                                        }),
                                ]),
                        ),
                ),
            )
            .footer(
                SidebarFooter::new().child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(IconName::CircleUser))
                        .child(if collapsed {
                            div()
                        } else {
                            div().child("User Profile")
                        }),
                ),
            );

        if niri_blur {
            div()
                .h_full()
                .flex_shrink_0()
                .bg(rgba(0x00000000))
                .child(sidebar)
                .into_any_element()
        } else {
            sidebar.into_any_element()
        }
    }
}
