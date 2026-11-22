use aether_services::{ThemeKind, ThemeService};
use gpui_kit::component::input::InputState;
use gpui_kit::*;

use crate::{components::sidebar::SettingsSidebar, pages};

#[derive(Clone, Copy, PartialEq)]
pub enum SettingsPage {
    Appearance,
    Display,
    Wifi,
    Ethernet,
}

pub struct SettingsApp {
    sidebar_collapsed: bool,
    search_input: Entity<InputState>,
    niri_blur: bool,
    current_page: SettingsPage,
    theme: ThemeKind,
    _theme_subscription: Subscription,
}

impl SettingsApp {
    pub fn new(search_input: Entity<InputState>, niri_blur: bool, cx: &mut Context<Self>) -> Self {
        let theme = ThemeService::current(cx);
        let theme_subscription = cx.observe_global::<ThemeService>(|this, cx| {
            this.theme = ThemeService::current(cx);
            cx.notify();
        });

        Self {
            sidebar_collapsed: false,
            search_input,
            niri_blur,
            current_page: SettingsPage::Appearance,
            theme,
            _theme_subscription: theme_subscription,
        }
    }
}

impl Render for SettingsApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.sidebar_collapsed;
        let niri_blur = self.niri_blur;
        let theme = self.theme;
        let palette = theme.palette();
        let app = cx.entity();

        let toggle_app = app.clone();
        let navigate_app = app.clone();
        let content = match self.current_page {
            SettingsPage::Appearance => pages::appearance::render(theme, move |next, _, cx| {
                ThemeService::set(cx, next);
            }),
            SettingsPage::Display => pages::display::render(),
            SettingsPage::Wifi => pages::wifi::render(),
            SettingsPage::Ethernet => pages::ethernet::render(),
        };

        let sidebar = SettingsSidebar::render(
            collapsed,
            self.search_input.clone(),
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

        root.child(sidebar).child(
            div()
                .flex_1()
                .size_full()
                .bg(rgb(palette.content_background))
                .text_color(rgb(palette.foreground))
                .child(content),
        )
    }
}
