mod island_state;
mod panel;

use aether::App;
use panel::island::island;
// use panel::island::panel;

fn main() {
    App::new()
        .window(island)
        // .window(panel)
        .without_ipc()
        .run();
}
