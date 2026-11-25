use std::error::Error;

use crate::{
    components::sidebar::SettingsSidebar,
    pages,
    services::wifi::{
        WifiClient,
        client::{ConnectedNetwork, WifiNetwork},
    },
};
use aether_themes::ThemeService;
use gpui_kit::assets::IconName as AssetIconName;
use gpui_kit::base::{
    Dialog, DialogBackdrop, DialogClose, DialogDescription, DialogHandle, DialogPopup, DialogTitle,
};
use gpui_kit::component::{Icon, Theme, button::Button, input::InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

type ServiceResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Copy, PartialEq)]
pub enum SettingsPage {
    Appearance,
    Display,
    Wifi,
    Ethernet,
}

pub struct SettingsAppConfig {
    pub search_input: Entity<InputState>,
    pub wifi_search_input: Entity<InputState>,
    pub niri_blur: bool,
}

pub struct WifiConfig {
    wifi_client: Option<WifiClient>,
    wifi_enabled: Option<bool>,
    wifi_pending: bool,
    wifi_error: Option<String>,

    wifi_networks: Vec<WifiNetwork>,
    connected_networks: Option<ConnectedNetwork>,
    selected_network: Option<ConnectedNetwork>,
    show_password: bool,

    networks_pending: bool,
    networks_error: Option<String>,
    wifi_toggle_generation: u64,
}

pub struct SettingsApp {
    sidebar_collapsed: bool,
    config: SettingsAppConfig,
    current_page: SettingsPage,
    theme: SharedString,
    wifi: WifiConfig,
    network_dialog: DialogHandle,
    _theme_subscription: Subscription,
}

impl SettingsApp {
    pub fn new(config: SettingsAppConfig, cx: &mut Context<Self>) -> Self {
        if config.niri_blur {
            set_translucent_sidebar(cx);
        }

        let theme = ThemeService::current_name(cx);
        let network_dialog = DialogHandle::new(false);
        let theme_subscription = cx.observe_global::<Theme>(|this, cx| {
            this.theme = ThemeService::current_name(cx);
            cx.notify();
        });

        cx.spawn(async move |entity, cx| {
            let result: ServiceResult<(
                WifiClient,
                bool,
                ServiceResult<(Vec<WifiNetwork>, Option<ConnectedNetwork>)>,
            )> = async {
                let client = WifiClient::connect().await?;
                let enabled = client.is_enabled().await?;

                let networks_result = if enabled {
                    async {
                        let networks = client.scan_and_list_networks().await?;
                        let connected = client.connected_wifi().await?.into_iter().next();

                        Ok((networks, connected))
                    }
                    .await
                } else {
                    Ok((Vec::new(), None))
                };

                Ok((client, enabled, networks_result))
            }
            .await;

            if entity
                .update(cx, |this, cx| {
                    this.wifi.wifi_pending = false;
                    this.wifi.networks_pending = false;

                    match result {
                        Ok((client, enabled, networks_result)) => {
                            this.wifi.wifi_client = Some(client);
                            this.wifi.wifi_enabled = Some(enabled);

                            match networks_result {
                                Ok((networks, connected)) => {
                                    this.wifi.wifi_networks = networks;
                                    this.wifi.connected_networks = connected;
                                    this.wifi.networks_error = None;
                                }
                                Err(error) => {
                                    this.wifi.wifi_networks.clear();
                                    this.wifi.connected_networks.take();
                                    this.wifi.networks_error = Some(error.to_string());
                                }
                            }
                        }
                        Err(error) => {
                            this.wifi.wifi_error = Some(error.to_string());
                            this.wifi.networks_error = Some(error.to_string());
                        }
                    }

                    cx.notify();
                })
                .is_err()
            {
                eprintln!("Settings window closed before Wi-Fi status was loaded");
            }
        })
        .detach();

        Self {
            sidebar_collapsed: false,
            config,
            current_page: SettingsPage::Appearance,
            theme,

            wifi: WifiConfig {
                wifi_client: None,
                wifi_enabled: None,
                wifi_pending: true,
                wifi_error: None,
                wifi_networks: Vec::new(),
                connected_networks: None,
                selected_network: None,
                show_password: false,
                networks_pending: true,
                networks_error: None,
                wifi_toggle_generation: 0,
            },
            network_dialog,
            _theme_subscription: theme_subscription,
        }
    }
}

impl Render for SettingsApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.sidebar_collapsed;
        let niri_blur = self.config.niri_blur;
        let theme = self.theme.clone();
        let palette = ThemeService::palette(cx);
        let available_themes = ThemeService::available(cx);

        let app = cx.entity();

        let toggle_app = app.clone();
        let navigate_app = app.clone();
        let wifi_app = app.clone();
        let refresh_app = app.clone();
        let network_settings_app = app.clone();
        let content = match self.current_page {
            SettingsPage::Appearance => {
                let translucent_sidebar = self.config.niri_blur;
                pages::appearance::render(theme, available_themes, move |next, _, cx| {
                    ThemeService::apply_config(cx, next);
                    if translucent_sidebar {
                        set_translucent_sidebar(cx);
                    }
                })
            }

            SettingsPage::Display => pages::display::render(),

            SettingsPage::Wifi => pages::wifi::render(
                self.config.wifi_search_input.clone(),
                palette,
                self.wifi.wifi_enabled,
                self.wifi.wifi_pending,
                self.wifi.wifi_error.clone(),
                self.wifi.wifi_networks.clone(),
                self.wifi.connected_networks.clone(),
                self.wifi.networks_pending,
                self.wifi.networks_error.clone(),
                // Wi-Fi enable / disable
                move |enabled, cx| {
                    wifi_app.update(cx, |this, cx| {
                        if this.wifi.wifi_pending || this.wifi.wifi_enabled == Some(enabled) {
                            return;
                        }

                        let Some(client) = this.wifi.wifi_client.clone() else {
                            return;
                        };

                        let previous_enabled = this.wifi.wifi_enabled;
                        this.wifi.wifi_enabled = Some(enabled);
                        this.wifi.wifi_pending = true;
                        this.wifi.networks_pending = enabled;
                        this.wifi.wifi_error = None;
                        this.wifi.networks_error = None;
                        this.wifi.wifi_toggle_generation =
                            this.wifi.wifi_toggle_generation.wrapping_add(1);
                        let generation = this.wifi.wifi_toggle_generation;

                        cx.notify();

                        cx.spawn(async move |this, cx| {
                            let result = client.set_enabled(enabled).await;

                            let update_result = this.update(cx, |this, cx| {
                                if this.wifi.wifi_toggle_generation != generation {
                                    return false;
                                }

                                this.wifi.wifi_pending = false;
                                match &result {
                                    Ok(()) => {
                                        this.wifi.wifi_enabled = Some(enabled);
                                        this.wifi.wifi_error = None;
                                        if !enabled {
                                            this.wifi.wifi_networks.clear();
                                            this.wifi.connected_networks = None;
                                            this.wifi.networks_pending = false;
                                            this.wifi.networks_error = None;
                                        }
                                    }
                                    Err(error) => {
                                        this.wifi.wifi_enabled = previous_enabled;
                                        this.wifi.networks_pending = false;
                                        this.wifi.wifi_error = Some(error.to_string());
                                    }
                                }
                                cx.notify();
                                true
                            });

                            match update_result {
                                Ok(true) => {}
                                Ok(false) => return,
                                Err(_) => {
                                    eprintln!(
                                        "Settings window closed before Wi-Fi state was updated"
                                    );
                                    return;
                                }
                            }

                            if result.is_err() || !enabled {
                                return;
                            }

                            futures_timer::Delay::new(std::time::Duration::from_millis(750)).await;
                            let networks_result = async {
                                let networks = client.scan_and_list_networks().await?;
                                let connected = client.connected_wifi().await?.into_iter().next();
                                Ok::<_, Box<dyn Error + Send + Sync>>((networks, connected))
                            }
                            .await;

                            if this
                                .update(cx, |this, cx| {
                                    if this.wifi.wifi_toggle_generation != generation {
                                        return;
                                    }
                                    this.wifi.networks_pending = false;
                                    match networks_result {
                                        Ok((networks, connected)) => {
                                            this.wifi.wifi_networks = networks;
                                            this.wifi.connected_networks = connected;
                                            this.wifi.networks_error = None;
                                        }
                                        Err(error) => {
                                            this.wifi.networks_error = Some(error.to_string());
                                        }
                                    }
                                    cx.notify();
                                })
                                .is_err()
                            {
                                eprintln!("Settings window closed during Wi-Fi scan");
                            }
                        })
                        .detach();
                    });
                },
                // Refresh available networks
                move |cx| {
                    refresh_app.update(cx, |this, cx| {
                        if this.wifi.wifi_pending
                            || this.wifi.networks_pending
                            || this.wifi.wifi_enabled != Some(true)
                        {
                            return;
                        }

                        let Some(client) = this.wifi.wifi_client.clone() else {
                            return;
                        };

                        this.wifi.networks_pending = true;
                        this.wifi.networks_error = None;
                        let generation = this.wifi.wifi_toggle_generation;

                        cx.notify();

                        cx.spawn(async move |this, cx| {
                            let result = async {
                                let networks = client.scan_and_list_networks().await?;
                                let connected = client.connected_wifi().await?.into_iter().next();

                                Ok::<_, Box<dyn Error + Send + Sync>>((networks, connected))
                            }
                            .await;

                            if this
                                .update(cx, |this, cx| {
                                    if this.wifi.wifi_toggle_generation != generation {
                                        return;
                                    }
                                    this.wifi.networks_pending = false;

                                    match result {
                                        Ok((networks, connected)) => {
                                            this.wifi.wifi_networks = networks;
                                            this.wifi.connected_networks = connected;
                                            this.wifi.networks_error = None;
                                        }
                                        Err(error) => {
                                            this.wifi.networks_error = Some(error.to_string());
                                        }
                                    }

                                    cx.notify();
                                })
                                .is_err()
                            {
                                eprintln!("Settings window closed during Wi-Fi scan");
                            }
                        })
                        .detach();
                    });
                },
                // Open connected network settings
                move |network, window, cx| {
                    network_settings_app.update(cx, |this, cx| {
                        this.wifi.selected_network = Some(network);
                        this.wifi.show_password = false;
                        this.network_dialog.open(window, cx);
                        cx.notify();
                    });
                },
            ),

            SettingsPage::Ethernet => pages::ethernet::render(),
        };
        let selected_network = self.wifi.selected_network.clone();
        let show_password = self.wifi.show_password;

        let password_app = app.clone();
        let dialog_state_app = app.clone();

        let qr_element = selected_network
            .as_ref()
            .and_then(|network| qr_code_element(&network.qr_payload));
        let qr_is_unavailable = qr_element.is_none();

        let dialog = Dialog::new(cx)
            .handle(self.network_dialog.clone())
            .on_open_change(move |open, _, _, cx| {
                if !open {
                    dialog_state_app.update(cx, |this, cx| {
                        this.wifi.selected_network = None;
                        this.wifi.show_password = false;
                        cx.notify();
                    });
                }
            })
            .backdrop(DialogBackdrop::new().bg(rgba(0x000000d9)))
            .popup(
                DialogPopup::new()
                    .w(px(560.))
                    .p(px(24.))
                    .bg(rgb(palette.content_background))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                DialogTitle::new()
                                                    .text_size(px(18.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child("Wi-Fi network details"),
                                            )
                                            .child(
                                                DialogDescription::new()
                                                    .text_size(px(12.))
                                                    .text_color(rgb(palette.foreground))
                                                    .child(
                                                        "View your connected network information and QR code.",
                                                    ),
                                            ),
                                    )
                                    .child(DialogClose::new().trigger(|button| {
                                        button.child(Icon::new(AssetIconName::X))
                                    })),
                            )
                            .when_some(selected_network, |popup, network| {
                                let password = if show_password {
                                    network.password.clone()
                                } else {
                                    "••••••••".to_string()
                                };

                                popup
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w_full()
                                            .child(
                                                div()
                                                    .p(px(12.))
                                                    .bg(rgb(0xffffff))
                                                    .rounded_md()
                                                    .when_some(qr_element, |this, qr| {
                                                        this.child(qr)
                                                    })
                                                    .when(qr_is_unavailable, |this| {
                                                        this.child(
                                                            div()
                                                                .size(px(180.))
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .child("QR unavailable"),
                                                        )
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_3()
                                            .child(network_detail_row(
                                                "SSID",
                                                network.ssid.clone(),
                                                palette.muted_foreground,
                                            ))
                                            .child(network_detail_row(
                                                "Interface",
                                                network.interface.clone(),
                                                palette.muted_foreground,
                                            ))
                                            .child(network_detail_row(
                                                "BSSID",
                                                network.bssid.clone(),
                                                palette.muted_foreground,
                                            ))
                                            .child(network_detail_row(
                                                "Signal strength",
                                                format!("{}%", network.strength),
                                                palette.muted_foreground,
                                            ))
                                            .child(network_detail_row(
                                                "IPv4",
                                                network.ip4_address.clone(),
                                                palette.muted_foreground,
                                            ))
                                            .child(network_detail_row(
                                                "IPv6",
                                                network.ip6_address.clone(),
                                                palette.muted_foreground,
                                            ))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .text_size(px(13.))
                                                            .text_color(rgb(palette.foreground))
                                                            .child("Password"),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .text_size(px(13.))
                                                                    .child(password),
                                                            )
                                                            .child(
                                                                Button::new("toggle-wifi-password")
                                                                    .icon(Icon::new(if show_password {
                                                                        AssetIconName::EyeOff
                                                                    } else {
                                                                        AssetIconName::Eye
                                                                    }))
                                                                    .accessibility_label(
                                                                        if show_password {
                                                                            "Hide password"
                                                                        } else {
                                                                            "Show password"
                                                                        },
                                                                    )
                                                                    .on_click(move |_, _, cx| {
                                                                        password_app.update(
                                                                            cx,
                                                                            |this, cx| {
                                                                                this.wifi.show_password =
                                                                                    !this.wifi.show_password;
                                                                                cx.notify();
                                                                            },
                                                                        );
                                                                    }),
                                                            ),
                                                    ),
                                            ),
                                    )
                            }),
                    ),
            );
        let sidebar = SettingsSidebar::render(
            collapsed,
            self.config.search_input.clone(),
            niri_blur,
            palette,
            self.current_page,
            // Sidebar toggle
            move |_, _, cx| {
                toggle_app.update(cx, |this, cx| {
                    this.sidebar_collapsed = !this.sidebar_collapsed;
                    cx.notify();
                });
            },
            // Page navigation
            move |page, _, cx| {
                navigate_app.update(cx, |this, cx| {
                    this.current_page = page;
                    cx.notify();
                });
            },
        );

        let root = div().flex().size_full();

        let root = if niri_blur {
            root
        } else {
            root.bg(rgb(palette.background))
        };

        root.child(sidebar)
            .child(
                div()
                    .flex_1()
                    .size_full()
                    .bg(rgb(palette.content_background))
                    .text_color(rgb(palette.foreground))
                    .child(content),
            )
            .child(dialog)
    }
}

fn set_translucent_sidebar(cx: &mut App) {
    let palette = ThemeService::palette(cx);
    Theme::update(cx, |theme| {
        theme.tokens.sidebar = Hsla::from(rgba(palette.sidebar_blur_background)).into();
    });
}

fn qr_code_element(payload: &str) -> Option<AnyElement> {
    let code = qrcode::QrCode::new(payload.as_bytes()).ok()?;
    let width = code.width();
    let quiet_zone = 4;
    let total_width = width + quiet_zone * 2;
    let module_size = px(180.0 / total_width as f32);
    let colors = code.to_colors();

    Some(
        div()
            .flex()
            .flex_col()
            .w(px(180.))
            .h(px(180.))
            .bg(rgb(0xffffff))
            .children((0..total_width).map(|y| {
                div()
                    .flex()
                    .flex_row()
                    .h(module_size)
                    .children((0..total_width).map(|x| {
                        let color = if x < quiet_zone
                            || y < quiet_zone
                            || x >= quiet_zone + width
                            || y >= quiet_zone + width
                        {
                            0xffffff
                        } else {
                            match colors[(y - quiet_zone) * width + (x - quiet_zone)] {
                                qrcode::types::Color::Dark => 0x000000,
                                qrcode::types::Color::Light => 0xffffff,
                            }
                        };

                        div().size(module_size).flex_shrink_0().bg(rgb(color))
                    }))
            }))
            .into_any_element(),
    )
}

fn network_detail_row(
    label: &'static str,
    value: String,
    muted_foreground: u32,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(muted_foreground))
                .child(label),
        )
        .child(div().text_size(px(13.)).child(value))
}
