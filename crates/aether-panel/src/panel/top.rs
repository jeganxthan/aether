use aether::{
    Align, Color, Full, Layer, LayerWindow, Parent, Rectangle, Text, Vertical, Zone,
};

use crate::services::clock::clock;

const PANEL_HEIGHT: f32 = 26.0;

pub fn panel() -> LayerWindow {
    let time = clock().current();

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
                .fill(Color::BLACK)
                .align_child(Align::Center, Align::Center)
                .child(
                    Text::new(time)
                        .color(Color::WHITE)
                        .size(12.0),
                ),
        )
}