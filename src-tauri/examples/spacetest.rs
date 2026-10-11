//! throwaway diagnostic: replicate space.rs's shm-pool flow standalone so we
//! can see whether labwc rejects our fd outside the app.

use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_registry, wl_region, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, queue) = registry_queue_init::<State>(&conn).unwrap();
    let qh = queue.handle();

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=6, ()).unwrap();
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
    let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ()).unwrap();

    let surface = compositor.create_surface(&qh, ());
    let empty = compositor.create_region(&qh, ());
    surface.set_input_region(Some(&empty));
    empty.destroy();

    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None,
        zwlr_layer_shell_v1::Layer::Background,
        "aether-space-test".to_string(),
        &qh,
        (),
    );
    layer_surface.set_anchor(
        zwlr_layer_surface_v1::Anchor::Top
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layer_surface.set_size(0, 28);
    layer_surface.set_exclusive_zone(28);
    layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
    surface.commit();

    let mut state = State {
        shm,
        qh: qh.clone(),
        surface,
        buffer_file: None,
        buffer: None,
    };
    let mut queue = queue;
    if queue.blocking_dispatch(&mut state).is_err() {
        eprintln!("TEST: dispatch error");
        std::process::exit(2);
    }
    eprintln!("TEST: mapped buffer file stays alive: {:?}", state.buffer_file.is_some());
    std::process::exit(0);
}

struct State {
    shm: wl_shm::WlShm,
    qh: QueueHandle<State>,
    surface: wl_surface::WlSurface,
    buffer_file: Option<File>,
    buffer: Option<wl_buffer::WlBuffer>,
}

impl State {
    fn make_buffer(&mut self, width: i32, height: i32) -> bool {
        let width = width.max(1);
        let height = height.max(1);
        let stride = width * 4;
        let size = (stride * height) as usize;

        let path = std::env::temp_dir().join(format!("aether-space-test-{}", std::process::id()));
        let mut file = File::create(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        file.set_len(size as u64).unwrap();
        file.write_all(&vec![0u8; size]).unwrap();

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
        self.buffer = Some(buffer);
        self.buffer_file = Some(file);
        true
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _s: &mut Self, _p: &wl_registry::WlRegistry, _e: wl_registry::Event, _d: &GlobalListContents,
        _c: &Connection, _q: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn event(_s: &mut Self, _p: &wl_compositor::WlCompositor, _e: wl_compositor::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<wl_region::WlRegion, ()> for State {
    fn event(_s: &mut Self, _p: &wl_region::WlRegion, _e: wl_region::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<wl_shm::WlShm, ()> for State {
    fn event(_s: &mut Self, _p: &wl_shm::WlShm, _e: wl_shm::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<wl_shm_pool::WlShmPool, ()> for State {
    fn event(_s: &mut Self, _p: &wl_shm_pool::WlShmPool, _e: wl_shm_pool::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<wl_buffer::WlBuffer, ()> for State {
    fn event(_s: &mut Self, _p: &wl_buffer::WlBuffer, _e: wl_buffer::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(_s: &mut Self, _p: &wl_surface::WlSurface, _e: wl_surface::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<ZwlrLayerShellV1, ()> for State {
    fn event(_s: &mut Self, _p: &ZwlrLayerShellV1, _e: zwlr_layer_shell_v1::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {}
}
impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(s: &mut Self, ls: &ZwlrLayerSurfaceV1, e: zwlr_layer_surface_v1::Event, _d: &(), _c: &Connection, _q: &QueueHandle<Self>) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, width, height } = e {
            eprintln!("TEST: configure {width}x{height}");
            ls.ack_configure(serial);
            if s.make_buffer(width as i32, height as i32) {
                if let Some(buffer) = &s.buffer {
                    s.surface.attach(Some(buffer), 0, 0);
                }
                s.surface.commit();
            }
        }
    }
}