
use aether::{Horizontal, Layer, LayerWindow, Parent, Rectangle, Vertical, Zone};

use super::island::{ISLAND_HEIGHT, ISLAND_WIDTH};

/*
 * An invisible window the size of the resting island. It keeps the top band
 * reserved, so opening the island floats over the windows instead of moving
 * them, and so they stay clear of it while it is closed.
 */
pub fn reserve() -> LayerWindow {
    LayerWindow::new()
        .width(ISLAND_WIDTH)
        .height(ISLAND_HEIGHT)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("aether-island-reserve")
        .click_through()
        .child(Rectangle::new().width(Parent).height(Parent))
}
