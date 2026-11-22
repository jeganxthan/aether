use std::rc::Rc;

use gpui_kit::{base::Selectable, component::button::Button, *};

pub fn render(
    current: aether_services::ThemeKind,
    on_change: impl Fn(aether_services::ThemeKind, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let on_change: Rc<dyn Fn(aether_services::ThemeKind, &mut Window, &mut App)> =
        Rc::new(on_change);
    let black = theme_option(
        aether_services::ThemeKind::Black,
        current,
        Rc::clone(&on_change),
    );
    let white = theme_option(aether_services::ThemeKind::White, current, on_change);

    div()
        .flex()
        .size_full()
        .flex_col()
        .gap_4()
        .p_6()
        .child(div().text_xl().child("Appearance"))
        .child(div().text_color(rgb(0x888888)).child("Choose a theme"))
        .child(div().flex().gap_4().child(black).child(white))
        .into_any_element()
}

fn theme_option(
    theme: aether_services::ThemeKind,
    current: aether_services::ThemeKind,
    on_change: Rc<dyn Fn(aether_services::ThemeKind, &mut Window, &mut App)>,
) -> AnyElement {
    let palette = theme.palette();
    let selected = theme == current;

    div()
        .w(px(180.0))
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(if selected {
            palette.accent
        } else {
            palette.border
        }))
        .bg(rgb(palette.background))
        .child(
            div()
                .h(px(72.0))
                .rounded_sm()
                .bg(rgb(palette.content_background))
                .border_1()
                .border_color(rgb(palette.border)),
        )
        .child(
            Button::new(format!("{}-theme", palette.name.to_lowercase()))
                .label(palette.name)
                .selected(selected)
                .on_click(move |_, window, cx| on_change(theme, window, cx)),
        )
        .into_any_element()
}
