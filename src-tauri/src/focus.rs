// Restores keyboard focus to the window that was focused before the island
// grabbed it (launcher / clipboard).
//
// Under Wayland a client cannot focus another client's window directly. labwc,
// however, implements `zwlr_foreign_toplevel_management_v1` and honours a
// client's `activate` request (src/foreign-toplevel/wlr-foreign.c:
// handle_request_activate -> desktop_focus_view). So we keep a lightweight
// second Wayland connection that watches every toplevel and remembers the last
// one that was activated while it was *not* us; when the island is done with
// the keyboard we ask labwc to focus that toplevel again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self as toplevel_handle, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self as toplevel_manager, ZwlrForeignToplevelManagerV1},
};

/// `enum state` values from the protocol (`activated` is 2).
const STATE_ACTIVATED: u8 = 2;

struct Shared {
    conn: Connection,
    seat: wl_seat::WlSeat,
    target: Option<ZwlrForeignToplevelHandleV1>,
}

/// Handle to the background watcher. Cheap to clone the inner `Arc`.
pub struct FocusTracker {
    shared: Arc<Mutex<Shared>>,
}

impl FocusTracker {
    /// Connect to the compositor and start watching toplevel focus. Returns
    /// `None` (quietly) when not running under a Wayland session, so the rest
    /// of the app keeps working.
    pub fn start() -> Option<Self> {
        let conn = Connection::connect_to_env().ok()?;
        let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();

        let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).ok()?;
        let manager: ZwlrForeignToplevelManagerV1 = globals.bind(&qh, 1..=3, ()).ok()?;

        let shared = Arc::new(Mutex::new(Shared {
            conn: conn.clone(),
            seat,
            target: None,
        }));

        let mut state = State {
            shared: shared.clone(),
            titles: HashMap::new(),
            app_ids: HashMap::new(),
        };

        thread::spawn(move || loop {
            if queue.blocking_dispatch(&mut state).is_err() {
                break;
            }
        });

        // The manager only starts emitting toplevels after a roundtrip, which
        // `blocking_dispatch` performs. Keep the manager alive for the process.
        std::mem::forget(manager);

        Some(Self { shared })
    }

    /// Ask the compositor to focus the last non-island toplevel.
    pub fn restore(&self) {
        let shared = self.shared.lock().unwrap();
        if let Some(target) = &shared.target {
            target.activate(&shared.seat);
            let _ = shared.conn.flush();
        }
    }
}

struct State {
    shared: Arc<Mutex<Shared>>,
    titles: HashMap<ZwlrForeignToplevelHandleV1, String>,
    app_ids: HashMap<ZwlrForeignToplevelHandleV1, String>,
}

impl State {
    fn is_self(&self, handle: &ZwlrForeignToplevelHandleV1) -> bool {
        let looks_like_self = |s: &String| {
            let s = s.to_lowercase();
            s.contains("aether")
        };
        self.app_ids.get(handle).map(looks_like_self).unwrap_or(false)
            || self.titles.get(handle).map(looks_like_self).unwrap_or(false)
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        _event: toplevel_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }

    // The manager's `toplevel` event creates a handle object; without this the
    // queue panics on the first toplevel.
    wayland_client::event_created_child!(State, ZwlrForeignToplevelManagerV1, [
        toplevel_manager::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: toplevel_handle::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            toplevel_handle::Event::Title { title } => {
                state.titles.insert(handle.clone(), title);
            }
            toplevel_handle::Event::AppId { app_id } => {
                state.app_ids.insert(handle.clone(), app_id);
            }
            toplevel_handle::Event::State { state: raw } => {
                let activated = raw.contains(&STATE_ACTIVATED);
                if activated && !state.is_self(handle) {
                    state.shared.lock().unwrap().target = Some(handle.clone());
                }
            }
            toplevel_handle::Event::Closed => {
                state.titles.remove(handle);
                state.app_ids.remove(handle);
                let mut shared = state.shared.lock().unwrap();
                if shared.target.as_ref() == Some(handle) {
                    shared.target = None;
                }
            }
            _ => {}
        }
    }
}
