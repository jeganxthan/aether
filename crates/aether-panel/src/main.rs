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
use panel::component::battery_menu::battery_menu;
use panel::component::battery_popup::battery_hint;
use panel::component::clipboard_popup;
use panel::component::launcher::launcher;
use panel::component::player_popup::{player_menu, player_popup};
use panel::component::polkit_auth;
use panel::component::settings_popup::settings_hint;
use panel::component::wifi_popup::{wifi_hint, wifi_menu};
use panel::lockscreen::view as lockscreen;

fn main() {
    // Warm up launcher applications and icon cache in the background on startup
    panel::component::launcher::preload();
    clipboard_popup::start();
    polkit_auth::start();

    // Inside your existing async startup function:
    App::new()
        // panel mode: the top bar plus the notification popups in the top right corner
        .window(panel)
        .window(notifications)
        // the network name shown under the wifi icon, and the menu a click opens
        .window(wifi_hint)
        .window(wifi_menu)
        // battery percentage and time estimate on hover
        .window(battery_hint)
        // battery details and power profile island opened from the battery icon
        .window(battery_menu)
        .window(clipboard_popup::view)
        // dynamic island media player popup
        .window(player_popup)
        .window(player_menu)
        .window(polkit_auth::auth_prompt)
        .window(settings_hint)
        // the volume/brightness display that rises from the bottom on a change
        .window(osd)
        .window(launcher)
        .lock(lockscreen)
        .ipc("lock", request_lock)
        // island mode instead of panel mode:
        // .window(reserve)
        // .window(island)
        // `aether ipc call theme light` / `dark` / `toggle`, and the shell redraws in it
        .ipc("theme", switch_theme)
        .ipc("launcher", handle_launcher)
        .ipc("clipboard", handle_clipboard)
        .run();
}

fn request_lock(_: &[String]) -> String {
    aether::Lock::start();

    "locked".to_owned()
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

fn handle_launcher(arguments: &[String]) -> String {
    let requested = arguments.first().map(String::as_str).unwrap_or("toggle");

    match requested.to_ascii_lowercase().as_str() {
        "open" => {
            panel::component::launcher::open_launcher();
        }
        "close" => {
            panel::component::launcher::close();
        }
        _ => {
            panel::component::launcher::toggle();
        }
    }

    "ok".to_owned()
}

fn handle_clipboard(arguments: &[String]) -> String {
    match arguments.first().map(String::as_str).unwrap_or("open") {
        "open" => clipboard_popup::open(),
        "close" => clipboard_popup::close(),
        "toggle" => clipboard_popup::toggle(),
        "pin" => clipboard_popup::pin_selected(),
        _ => return "unknown clipboard action".to_owned(),
    }

    "ok".to_owned()
}

// python3 - "$XDG_RUNTIME_DIR/aether.sock" light <<'PY'
// import socket, sys
// s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.connect(sys.argv[1])
// s.sendall(("theme\n%s\n" % sys.argv[2]).encode()); s.shutdown(socket.SHUT_WR)
// print(s.recv(4096).decode().strip())
// PY

// python3 -c 'import os,socket; s=socket.socket(socket.AF_UNIX); s.connect(os.path.join(os.environ["XDG_RUNTIME_DIR"],"aether.sock")); s.sendall(b"lock\n"); s.shutdown(socket.SHUT_WR); print(s.recv(4096).decode().strip())
