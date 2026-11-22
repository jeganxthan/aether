use gpui_kit::*;

pub fn render() -> AnyElement {
    div()
        .flex()
        .size_full()
        .items_center()
        .justify_center()
        .child("Display Settings")
        .into_any_element()
}
