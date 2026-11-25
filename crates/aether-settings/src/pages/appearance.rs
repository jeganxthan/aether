use std::rc::Rc;

use aether_themes::ThemePalette;
use gpui_kit::{
    base::Selectable,
    component::{Theme, ThemeConfig, button::Button},
    *,
};

pub fn render(
    current: SharedString,
    themes: Vec<Rc<ThemeConfig>>,
    on_change: impl Fn(Rc<ThemeConfig>, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let on_change: Rc<dyn Fn(Rc<ThemeConfig>, &mut Window, &mut App)> = Rc::new(on_change);
    let theme_cards = themes
        .into_iter()
        .map(|theme| theme_option(theme, &current, Rc::clone(&on_change)));

    div()
        .flex()
        .size_full()
        .flex_col()
        .gap_4()
        .p_6()
        .child(div().text_xl().child("Appearance"))
        .child(div().text_sm().child(format!("Current theme: {current}")))
        .child(
            div()
                .id("appearance-theme-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(div().flex().flex_wrap().gap_3().children(theme_cards)),
        )
        .into_any_element()
}

fn theme_option(
    theme: Rc<ThemeConfig>,
    current: &SharedString,
    on_change: Rc<dyn Fn(Rc<ThemeConfig>, &mut Window, &mut App)>,
) -> AnyElement {
    let mut preview_theme = Theme::default();
    preview_theme.apply_config(&theme);
    let palette = ThemePalette::from_theme(&preview_theme);
    let name = theme.name.clone();
    let selected = name == *current;

    div()
        .w(px(190.0))
        .flex_shrink_0()
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(if selected {
            palette.accent
        } else {
            palette.border
        }))
        .bg(rgb(palette.surface_background))
        .child(
            div()
                .h(px(64.0))
                .p_2()
                .flex()
                .flex_col()
                .justify_between()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.background))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.foreground))
                        .child(name.clone()),
                )
                .child(
                    div()
                        .w(px(42.0))
                        .h(px(5.0))
                        .rounded_full()
                        .bg(rgb(palette.accent)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.muted_foreground))
                        .child(if theme.mode.is_dark() {
                            "Dark"
                        } else {
                            "Light"
                        }),
                )
                .child(
                    Button::new(format!("theme-{}", name.replace(' ', "-")))
                        .label(if selected { "Selected" } else { "Apply" })
                        .selected(selected)
                        .on_click(move |_, window, cx| {
                            on_change(Rc::clone(&theme), window, cx);
                        }),
                ),
        )
        .into_any_element()
}
