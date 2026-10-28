use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Duration;

use aether::{
    Animation, Center, Color, Easing, Horizontal, InputArea, Layer, LayerWindow, Parent, Rectangle,
    Service, Start, Text, Vertical, Zone,
};

use crate::island_state::IslandState;
use crate::services::clock::clock;

pub(crate) const ISLAND_WIDTH: f32 = 180.0;
pub(crate) const ISLAND_HEIGHT: f32 = 26.0;

const EXPANDED_WIDTH: f32 = 320.0;
const EXPANDED_HEIGHT: f32 = 100.0;

const CORNER_RADIUS: f32 = 12.0;

// how long the island takes to open or close
const OPEN_TIME: Duration = Duration::from_millis(240);

// the two sides move on their own, kept between views since the view runs again every frame
static WIDTH: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();
static HEIGHT: OnceLock<Mutex<Animation<f32>>> = OnceLock::new();

/*
 * The window keeps the expanded size and only the island drawn inside it grows,
 * so opening never resizes the surface or moves anything. The top the resting
 * island sits in is reserved by reserve(), which keeps windows out of it while
 * the island opens over them.
 */
pub fn island() -> LayerWindow {
    let expanded = IslandState::read().expanded;

    let width = size(&WIDTH, ISLAND_WIDTH, EXPANDED_WIDTH, expanded);
    let height = size(&HEIGHT, ISLAND_HEIGHT, EXPANDED_HEIGHT, expanded);

    // the rest of the window is transparent, so only the island takes the pointer
    let input = InputArea {
        x: ((EXPANDED_WIDTH - width) / 2.0).round() as i32,
        y: 0,
        width: width.round() as i32,
        height: height.round() as i32,
    };
    let time = clock();
    LayerWindow::new()
        .width(EXPANDED_WIDTH)
        .height(EXPANDED_HEIGHT)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Top)
        // floats over windows instead of pushing them out of the way
        .space(Zone::Ignore)
        .input_region(vec![input])
        .namespace("aether-island")
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .align_child(Center, Start)
                .child(
                    Rectangle::new()
                        .width(width)
                        .height(height)
                        .radius_bottom_left(CORNER_RADIUS)
                        .radius_bottom_right(CORNER_RADIUS)
                        .fill(Color::BLACK)
                        .align_child(Center, Center)
                        .child(
                            Text::new(time)
                                .font("Maple Mono")
                                .color(Color::WHITE)
                                .size(12.0),
                        )
                        .on_hover(|hovered| {
                            let mut state = IslandState::write();
                            state.expanded = hovered;
                        }),
                ),
        )
}

// one side eased towards the size of its state, which keeps frames coming until it arrives
fn size(
    side: &'static OnceLock<Mutex<Animation<f32>>>,
    rest: f32,
    open: f32,
    expanded: bool,
) -> f32 {
    let animation = side
        .get_or_init(|| Mutex::new(Animation::new(rest).duration(OPEN_TIME).easing(Easing::Out)));

    let mut animation = animation.lock().unwrap_or_else(PoisonError::into_inner);

    animation.to(if expanded { open } else { rest });

    animation.value()
}
