mod island_state;
mod panel;
mod services;
mod theme;

use aether::App;
use panel::notification::notifications;
use panel::osd::osd;
use panel::top::panel;
use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
// use panel::reserve::reserve;
// use panel::island::island;
use panel::component::battery_menu::battery_menu;
use panel::component::battery_popup::battery_hint;
use panel::component::bluetooth_menu::bluetooth_menu;
use panel::component::bluetooth_popup::bluetooth_hint;
use panel::component::clipboard_popup;
use panel::component::notification_menu::{notification_hint, notification_menu};

use panel::component::launcher::launcher;
use panel::component::player_popup::{player_menu, player_popup};
use panel::component::polkit_auth;
use panel::component::settings_popup::settings_hint;
use panel::component::wifi_popup::{wifi_hint, wifi_menu};
use panel::lockscreen::{self, view as lockscreen};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("ipc") {
        run_ipc_client();
        return;
    }

    lockscreen::preload();
    services::notifications::ensure_started();
    // Warm up launcher applications and icon cache in the background on startup
    panel::component::launcher::preload();
    clipboard_popup::start();
    polkit_auth::start();

    // Inside your existing async startup function:
    App::new()
        // panel mode: the top bar plus the notification popups in the top right corner
        .window_per_monitor(panel)
        .window(notifications)
        .window_per_monitor(notification_hint)
        .window_per_monitor(notification_menu)
        // the network name shown under the wifi icon, and the menu a click opens
        .window_per_monitor(wifi_hint)
        .window_per_monitor(wifi_menu)
        // battery percentage and time estimate on hover
        .window_per_monitor(battery_hint)
        .window_per_monitor(bluetooth_hint)
        // battery details and power profile island opened from the battery icon
        .window_per_monitor(battery_menu)
        .window_per_monitor(bluetooth_menu)
        // dynamic island media player popup
        .window_per_monitor(player_popup)
        .window_per_monitor(player_menu)
        .window(polkit_auth::auth_prompt)
        .window_per_monitor(settings_hint)
        .window(clipboard_popup::view)
        // the volume/brightness display that rises from the bottom on a change
        .window(osd)
        .window_per_monitor(launcher)
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

fn run_ipc_client() {
    let mut arguments = std::env::args().skip(2);
    let Some(name) = arguments.next() else {
        eprintln!("usage: aether-shell ipc <handler> [arguments...]");
        return;
    };

    let socket = aether::ipc_socket();
    let Ok(mut stream) = UnixStream::connect(&socket) else {
        eprintln!("aether-shell: could not connect to {}", socket.display());
        return;
    };

    if let Err(error) = write!(
        stream,
        "{name}\n{}",
        arguments.collect::<Vec<_>>().join("\n")
    ) {
        eprintln!("aether-shell: could not send IPC request: {error}");
        return;
    }

    if let Err(error) = stream.shutdown(Shutdown::Write) {
        eprintln!("aether-shell: could not finish IPC request: {error}");
    }
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

fn handle_clipboard(_arguments: &[String]) -> String {
    match _arguments.first().map(String::as_str) {
        Some("pin") => clipboard_popup::pin_selected(),
        Some("close") => clipboard_popup::close(),
        _ => clipboard_popup::toggle(),
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
