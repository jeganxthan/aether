use aether_themes::ThemePalette;
use gpui_kit::*;

use gpui_kit::component::{
    IconName,
    input::{Input, InputState},
};

pub struct SearchInput;

impl SearchInput {
    pub fn render(
        input: Entity<InputState>,
        width: Option<f32>,
        palette: ThemePalette,
    ) -> impl IntoElement {
        Self::render_internal(input, width, palette, false)
    }

    pub fn render_translucent(
        input: Entity<InputState>,
        width: Option<f32>,
        palette: ThemePalette,
    ) -> impl IntoElement {
        Self::render_internal(input, width, palette, true)
    }

    fn render_internal(
        input: Entity<InputState>,
        width: Option<f32>,
        palette: ThemePalette,
        translucent: bool,
    ) -> impl IntoElement {
        let (border, bg) = if translucent {
            (
                rgba(palette.sidebar_blur_border),
                rgba(palette.sidebar_blur_background),
            )
        } else {
            (
                rgb(palette.input_border),
                rgb(palette.input_background),
            )
        };

        let mut container = div()
            .h(px(40.0))
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(bg)
            .hover(move |style| {
                style
                    .border_color(rgb(palette.accent))
                    .bg(bg)
            });

        if let Some(width) = width {
            container = container.w(px(width));
        } else {
            container = container.w_full();
        }

        container.child(
            Input::new(&input)
                .prefix(IconName::Search)
                .h_full()
                .w_full()
                .appearance(false)
                .bordered(false),
        )
    }
}
