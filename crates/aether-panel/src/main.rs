mod island_state;
mod panel;
mod services;
mod theme;

use aether::App;
use panel::notification::notifications;
use panel::osd::osd;
use panel::top::panel;
// use panel::reserve::reserve;
// use panel::island::island;
use panel::component::wifi_popup::{wifi_hint, wifi_menu};
use panel::component::launcher::launcher;
fn main() {
    // Inside your existing async startup function:
    App::new()
        // panel mode: the top bar plus the notification popups in the top right corner
        .window(panel)
        .window(notifications)
        // the network name shown under the wifi icon, and the menu a click opens
        .window(wifi_hint)
        .window(wifi_menu)
        // the volume/brightness display that rises from the bottom on a change
        .window(osd)
        .window(launcher)
        // island mode instead of panel mode:
        // .window(reserve)
        // .window(island)
        // `aether ipc call theme light` / `dark` / `toggle`, and the shell redraws in it
        .ipc("theme", switch_theme)
        .run();
}

/*
 * Reads the look to switch to off the call, or flips to the other one when it
 * is empty or says `toggle`. Anything the shell does not know is ignored and the
 * current look is reported back, so a mistyped call never blanks the shell.
 */
fn switch_theme(arguments: &[String]) -> String {
    let requested = arguments.first().map(String::as_str).unwrap_or("toggle");

    match requested.to_ascii_lowercase().as_str() {
        "light" => {
            theme::set_mode(theme::Mode::Light);
        }
        "dark" => {
            theme::set_mode(theme::Mode::Dark);
        }
        "toggle" => {
            theme::toggle();
        }
        _ => {}
    }

    theme::mode().name().to_owned()
}

// python3 - "$XDG_RUNTIME_DIR/aether.sock" light <<'PY'
// import socket, sys
// s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.connect(sys.argv[1])
// s.sendall(("theme\n%s\n" % sys.argv[2]).encode()); s.shutdown(socket.SHUT_WR)
// print(s.recv(4096).decode().strip())
// PY