// Reserves the 28px top strip for the island by mapping a transparent
// `wlr-layer-shell` surface across the top of the output. Unlike labwc's
// `<margin>` rc.xml setting (which is only read at compositor startup), a
// layer surface's exclusive zone is re-applied live: labwc reflows maximized
// and tiled windows the moment the zone changes.
//
// The strip is fully transparent and has an empty input region, so it is
// invisible and never intercepts pointer events. It keeps its 28px exclusive
// zone at all times, so maximized / tiled windows are always anchored below
// it and the island's transparent strip is static — hovering the island or
// opening a panel never reflows windows.
//
// The strip is pinned to the output the island window lives on. A layer
// surface created with a NULL output is placed on the output under the
// pointer, which is nondeterministic on a multi-monitor setup.

use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::os::unix::io::OwnedFd;
use std::sync::{Arc, Mutex};
use std::thread;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

/// Height of the reserved strip, in logical pixels.
const RESERVED_HEIGHT: i32 = 28;

// The fields are never read: the Mutex<Shared> exists purely to *own* the
// connection, layer-shell proxies and the chosen output for the lifetime of
// the app, keeping the strip mapped. The set_reserved toggle was removed —
// the strip is static.
#[allow(dead_code)]
struct Shared {
    conn: Connection,
    surface: wl_surface::WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    // Keeping the output proxy alive keeps the layer surface pinned to it.
    output: Option<wl_output::WlOutput>,
}

/// Handle to the background strip. Cheap to clone the inner `Arc`.
#[allow(dead_code)]
pub struct SpaceReserver {
    shared: Arc<Mutex<Shared>>,
}

#[derive(Debug, Default)]
struct OutputInfo {
    name: Option<String>,
    description: Option<String>,
    mode: Option<(i32, i32)>,
}

impl SpaceReserver {
    /// Connect and map the strip on the output the island is on. Returns
    /// `None` (quietly) when layer-shell is unavailable or we are not on
    /// Wayland, so the rest of the app still runs.
    ///
    /// `monitor_name` / `monitor_size` come from the island window's current
    /// monitor and are matched against every `wl_output`: first by connector
    /// name / description, then by mode size.
    pub fn start(monitor_name: Option<&str>, monitor_size: Option<(i32, i32)>) -> Option<Self> {
        let conn = Connection::connect_to_env().ok()?;
        let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();

        let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=6, ()).ok()?;
        let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).ok()?;
        let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ()).ok()?;

        let surface = compositor.create_surface(&qh, ());

        // Never take pointer input over the strip.
        let empty = compositor.create_region(&qh, ());
        surface.set_input_region(Some(&empty));
        empty.destroy();

        // Bind every output (v4 when available) so we can read its connector
        // name, description and mode, then pin the strip to the island's.
        let registry = globals.registry();
        let mut outputs: Vec<(wl_output::WlOutput, Arc<Mutex<OutputInfo>>)> = Vec::new();
        for global in globals.contents().clone_list() {
            if global.interface == "wl_output" {
                let info = Arc::new(Mutex::new(OutputInfo::default()));
                let version = global.version.min(4);
                let output: wl_output::WlOutput =
                    registry.bind(global.name, version, &qh, info.clone());
                outputs.push((output, info));
            }
        }

        let mut state = State {
            shm,
            qh: qh.clone(),
            surface: surface.clone(),
            buffer: None,
            buffer_file: None,
        };

        // Let the compositor send each output's info before we choose.
        for _ in 0..3 {
            if queue.roundtrip(&mut state).is_err() {
                return None;
            }
        }

        let chosen = choose_index(&outputs, monitor_name, monitor_size);
        let expected = chosen
            .and_then(|index| outputs[index].1.lock().ok().map(|info| info.name.clone()))
            .flatten();
        eprintln!(
            "[space] reserving top strip on {:?} (matched {})",
            expected,
            chosen.is_some()
        );

        let target = chosen.map(|index| outputs[index].0.clone());

        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            target.as_ref(),
            zwlr_layer_shell_v1::Layer::Background,
            "aether-space".to_string(),
            &qh,
            (),
        );

        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface.set_size(0, RESERVED_HEIGHT as u32);
        layer_surface.set_exclusive_zone(RESERVED_HEIGHT);
        layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);

        surface.commit();

        // Keep the chosen output (if any) alive for the process lifetime; the
        // rest are released when `outputs` drops at the end of this function.
        let output = target;

        let shared = Arc::new(Mutex::new(Shared {
            conn: conn.clone(),
            surface: surface.clone(),
            layer_surface: layer_surface.clone(),
            output,
        }));

        thread::spawn(move || loop {
            if queue.blocking_dispatch(&mut state).is_err() {
                break;
            }
        });

        // The registry and shell proxies are only needed to create the
        // surface; the live objects keep the connection alive from here on.
        std::mem::forget(layer_shell);

        Some(Self { shared })
    }
}

/// Pick the output that matches the island's monitor. Prefers a connector
/// name / description match, then falls back to the mode size. Returns `None`
/// when nothing matches (the compositor then places the surface on the output
/// under the pointer, the pre-existing behaviour).
fn choose_index(
    outputs: &[(wl_output::WlOutput, Arc<Mutex<OutputInfo>>)],
    monitor_name: Option<&str>,
    monitor_size: Option<(i32, i32)>,
) -> Option<usize> {
    let matches_name = |info: &OutputInfo| {
        let Some(target) = monitor_name else {
            return false;
        };
        let target = target.to_lowercase();
        info.name
            .as_deref()
            .map(|name| name.to_lowercase() == target)
            .unwrap_or(false)
            || info
                .description
                .as_deref()
                .map(|desc| desc.to_lowercase().contains(&target))
                .unwrap_or(false)
    };

    if monitor_name.is_some() {
        if let Some(index) = outputs.iter().position(|(_, info)| {
            info.lock().map(|info| matches_name(&info)).unwrap_or(false)
        }) {
            return Some(index);
        }
    }

    if let Some(size) = monitor_size {
        if let Some(index) = outputs.iter().position(|(_, info)| {
            info.lock().map(|info| info.mode == Some(size)).unwrap_or(false)
        }) {
            return Some(index);
        }
    }

    None
}

struct State {
    shm: wl_shm::WlShm,
    qh: QueueHandle<State>,
    surface: wl_surface::WlSurface,
    buffer: Option<wl_buffer::WlBuffer>,
    buffer_file: Option<File>,
}

impl State {
    /// Build a fully transparent ARGB buffer of `width` x `height`.
    fn make_buffer(&mut self, width: i32, height: i32) -> Option<wl_buffer::WlBuffer> {
        let width = width.max(1);
        let height = height.max(1);
        let stride = width * 4;
        let size = (stride * height) as usize;

        // Anonymous shm via memfd. This is the same mechanism as libwayland,
        // and the compositor can always mmap it back (unlike a reused /tmp fd).
        let raw = CString::new("aether-space").ok()?;
        let raw_fd = unsafe { libc::memfd_create(raw.as_ptr(), libc::MFD_CLOEXEC) };
        if raw_fd < 0 {
            eprintln!(
                "[space] memfd_create failed: {}",
                std::io::Error::last_os_error()
            );
            return None;
        }
        let mut file = File::from(unsafe { OwnedFd::from_raw_fd(raw_fd) });
        file.set_len(size as u64).ok()?;
        file.write_all(&vec![0u8; size]).ok()?;

        eprintln!(
            "[space] make_buffer fd={} size={} ({width}x{height})",
            file.as_raw_fd(),
            size
        );

        let pool = self.shm.create_pool(file.as_fd(), size as i32, &self.qh, ());
        let buffer = pool.create_buffer(
            0,
            width,
            height,
            stride,
            wl_shm::Format::Argb8888,
            &self.qh,
            (),
        );
        pool.destroy();

        self.buffer_file = Some(file);
        Some(buffer)
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

impl Dispatch<wl_output::WlOutput, Arc<Mutex<OutputInfo>>> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &Arc<Mutex<OutputInfo>>,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let Ok(mut info) = data.lock() else {
            return;
        };
        match event {
            wl_output::Event::Name { name } => info.name = Some(name),
            wl_output::Event::Description { description } => info.description = Some(description),
            wl_output::Event::Mode { width, height, .. } => info.mode = Some((width, height)),
            _ => {}
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_compositor::WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_region::WlRegion,
        _event: wl_region::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_shm::WlShm,
        _event: wl_shm::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_shm_pool::WlShmPool,
        _event: wl_shm_pool::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for State {
    fn event(
        state: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            if state.buffer.as_ref() == Some(buffer) {
                state.buffer = None;
            }
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrLayerShellV1,
        _event: zwlr_layer_shell_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                layer_surface.ack_configure(serial);

                if let Some(buffer) = state.make_buffer(width as i32, height as i32) {
                    state.surface.attach(Some(&buffer), 0, 0);
                    state.surface.damage(0, 0, width as i32, height as i32);
                    state.surface.commit();
                    state.buffer = Some(buffer);
                }

                // Serialize the queued create_pool now, while the memfd is
                // provably open — a later flush from another dispatch could
                // race with whatever has been reusing these fd numbers.
                let _ = conn.flush();
                let _ = qh;
            }
            zwlr_layer_surface_v1::Event::Closed => {}
            _ => {}
        }
    }
}
