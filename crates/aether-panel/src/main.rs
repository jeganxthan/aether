mod island_state;
mod panel;
mod services;

use aether::App;
use panel::top::panel;
// use panel::reserve::reserve;
// use panel::island::island;

fn main() {
    App::new()
        // .window(reserve)
        .window(panel)
        // .window(island)
        .without_ipc()
        .run();
}
