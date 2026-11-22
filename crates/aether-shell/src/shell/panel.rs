
use gpui::{Context, IntoElement, Render, Window, div, prelude::*};

use crate::theme::panel_background;

pub struct ShellPanel;

impl ShellPanel {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for ShellPanel {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .bg(panel_background())
    }
}
