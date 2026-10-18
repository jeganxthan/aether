
use aether::{
    Color, Horizontal, Layer, LayerWindow, Parent, Rectangle, Service, Vertical, Zone,
};

use crate::island_state::IslandState;

const ISLAND_WIDTH: f32 = 180.0;
const ISLAND_HEIGHT: f32 = 26.0;

const EXPANDED_WIDTH: f32 = 320.0;
const EXPANDED_HEIGHT: f32 = 100.0;

const CORNER_RADIUS: f32 = 12.0;

pub fn island() -> LayerWindow {
    let expanded = IslandState::read().expanded;

    let (width, height) = if expanded {
        (EXPANDED_WIDTH, EXPANDED_HEIGHT)
    } else {
        (ISLAND_WIDTH, ISLAND_HEIGHT)
    };

    LayerWindow::new()
        .width(width)
        .height(height)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("aether-island")
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .radius_bottom_left(CORNER_RADIUS)
                .radius_bottom_right(CORNER_RADIUS)
                .fill(Color::BLACK)
                .on_hover(|hovered| {
                    let mut state = IslandState::write();
                    state.expanded = hovered;
                }),
        )
}
