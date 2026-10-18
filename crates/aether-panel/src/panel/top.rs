use aether::{Color, Full, Layer, LayerWindow, Parent, Rectangle, Vertical, Zone};

const PANEL_HEIGHT: f32 = 26.0;

pub fn panel() -> LayerWindow {
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
                .fill(Color::BLACK),
        )
}
