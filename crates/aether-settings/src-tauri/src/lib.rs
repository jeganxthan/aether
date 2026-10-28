mod commands;
use commands::display::{display_apply, display_list, display_primary_get, display_primary_set};
use commands::wallpaper::{
    WallpaperState, wallpaper_current, wallpaper_folder, wallpaper_list, wallpaper_restore,
    wallpaper_set,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};

#[cfg(target_os = "linux")]
use gtk::glib::ObjectType;

struct BlurStatus {
    status: Arc<Mutex<String>>,
    rounded: Arc<AtomicBool>,

    #[cfg(target_os = "linux")]
    effect: Arc<Mutex<Option<background_blur::ActiveEffect>>>,
}

#[tauri::command]
fn get_blur_status(status: tauri::State<'_, BlurStatus>) -> String {
    status
        .status
        .lock()
        .expect("blur status lock poisoned")
        .clone()
}

#[tauri::command]
fn window_is_rounded(status: tauri::State<'_, BlurStatus>) -> bool {
    status.rounded.load(Ordering::Relaxed)
}

#[cfg(target_os = "linux")]
mod background_blur {
    use gdk_wayland_sys as gdkwayland_sys;
    use std::mem::ManuallyDrop;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use tauri::Emitter;

    use wayland_client::{
        Connection, Dispatch, Proxy, QueueHandle,
        backend::{Backend, ObjectId},
        globals::{GlobalListContents, registry_queue_init},
        protocol::{
            wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry, wl_surface::WlSurface,
        },
    };

    use wayland_protocols::ext::background_effect::v1::client::{
        ext_background_effect_manager_v1::{self, ExtBackgroundEffectManagerV1},
        ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
    };

    pub struct ActiveEffect {
        connection: Connection,
        compositor: WlCompositor,
        queue_handle: QueueHandle<State>,
        effect: ExtBackgroundEffectSurfaceV1,
    }

    /// Matches the `rounded-4xl` radius (2rem) on the settings window container.
    const BLUR_CORNER_RADIUS: i32 = 32;

    fn add_rounded_region(region: &WlRegion, width: i32, height: i32, radius: i32) {
        let radius = radius.min(width / 2).min(height / 2);
        if radius <= 0 {
            region.add(0, 0, width, height);
            return;
        }

        // Middle section between the two rounded corner bands.
        region.add(0, radius, width, height - radius * 2);

        // One-pixel rows keep the staircase invisible. The inset is taken
        // from the narrow edge of each row so the blur always stays behind
        // the rounded glass instead of leaking a stepped halo outside it.
        let r = radius as f64;

        for y in 0..radius {
            let dy = r - y as f64;
            let inset = (r - (r * r - dy * dy).max(0.0).sqrt()).ceil() as i32;
            let inset = inset.clamp(0, width / 2);
            let row_width = width - inset * 2;

            if row_width > 0 {
                region.add(inset, y, row_width, 1);
                region.add(inset, height - y - 1, row_width, 1);
            }
        }
    }

    impl ActiveEffect {
        fn update_region(&self, width: i32, height: i32, radius: i32) -> Result<(), &'static str> {
            if width <= 0 || height <= 0 {
                return Err("The window has no drawable area yet.");
            }

            let region = self.compositor.create_region(&self.queue_handle, ());
            add_rounded_region(&region, width, height, radius);

            self.effect.set_blur_region(Some(&region));
            region.destroy();

            self.connection
                .flush()
                .map_err(|_| "Could not send the blur request to the compositor.")
        }
    }

    fn window_is_rounded(window: &gtk::Window) -> bool {
        use gtk::prelude::*;

        let fullscreen = window
            .window()
            .map(|window| window.state().contains(gtk::gdk::WindowState::FULLSCREEN))
            .unwrap_or(false);

        !fullscreen && !window.is_maximized()
    }

    fn sync_window_state(
        window: &gtk::Window,
        is_rounded: bool,
        effect: &Arc<Mutex<Option<ActiveEffect>>>,
        rounded: &Arc<AtomicBool>,
        app_handle: &tauri::AppHandle,
    ) {
        use gtk::prelude::*;

        let radius = if is_rounded { BLUR_CORNER_RADIUS } else { 0 };

        if let Ok(effect) = effect.lock() {
            if let Some(effect) = effect.as_ref() {
                let _ = effect.update_region(
                    window.allocated_width(),
                    window.allocated_height(),
                    radius,
                );
            }
        }

        let was_rounded = rounded.swap(is_rounded, Ordering::Relaxed);

        if was_rounded != is_rounded {
            let _ = app_handle.emit("window-rounded-changed", is_rounded);
        }
    }

    fn sync_window(
        window: &gtk::Window,
        effect: &Arc<Mutex<Option<ActiveEffect>>>,
        rounded: &Arc<AtomicBool>,
        app_handle: &tauri::AppHandle,
    ) {
        let is_rounded = window_is_rounded(window);
        sync_window_state(window, is_rounded, effect, rounded, app_handle);
    }

    /// Reapply the blur region when the GTK window is resized or changes state.
    pub unsafe fn track_size(
        widget: *mut gtk::ffi::GtkWidget,
        effect: Arc<Mutex<Option<ActiveEffect>>>,
        rounded: Arc<AtomicBool>,
        app_handle: tauri::AppHandle,
    ) {
        use gtk::prelude::*;

        let toplevel = unsafe { gtk::ffi::gtk_widget_get_toplevel(widget) };

        if toplevel.is_null() {
            return;
        }

        let toplevel: gtk::Widget = unsafe { gtk::glib::translate::from_glib_none(toplevel) };
        let Ok(window) = toplevel.downcast::<gtk::Window>() else {
            return;
        };

        sync_window(&window, &effect, &rounded, &app_handle);

        let size_effect = Arc::clone(&effect);
        let size_rounded = Arc::clone(&rounded);
        let size_app_handle = app_handle.clone();

        window.connect_size_allocate(move |window, _| {
            sync_window(window, &size_effect, &size_rounded, &size_app_handle);
        });

        let state_effect = Arc::clone(&effect);
        let state_rounded = Arc::clone(&rounded);

        window.connect_window_state_event(move |window, event| {
            let is_rounded = !event
                .new_window_state()
                .contains(gtk::gdk::WindowState::FULLSCREEN | gtk::gdk::WindowState::MAXIMIZED);
            sync_window_state(
                window,
                is_rounded,
                &state_effect,
                &state_rounded,
                &app_handle,
            );
            gtk::glib::Propagation::Proceed
        });
    }

    struct ForeignSurface {
        wrapper: *mut wayland_sys::client::wl_proxy,
        surface: ManuallyDrop<WlSurface>,
    }

    impl Drop for ForeignSurface {
        fn drop(&mut self) {
            unsafe {
                wayland_sys::client::wl_proxy_wrapper_destroy(self.wrapper);
            }
        }
    }

    #[derive(Default)]
    struct State {
        capabilities: u32,
    }

    impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
        fn event(
            _: &mut Self,
            _: &wl_registry::WlRegistry,
            _: wl_registry::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ExtBackgroundEffectManagerV1, ()> for State {
        fn event(
            state: &mut Self,
            _: &ExtBackgroundEffectManagerV1,
            event: ext_background_effect_manager_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let ext_background_effect_manager_v1::Event::Capabilities { flags } = event {
                state.capabilities = flags.into();
            }
        }
    }

    wayland_client::delegate_noop!(State: ignore WlCompositor);
    wayland_client::delegate_noop!(State: ignore WlRegion);
    wayland_client::delegate_noop!(State: ignore WlSurface);
    wayland_client::delegate_noop!(State: ignore ExtBackgroundEffectSurfaceV1);

    pub unsafe fn enable(widget: *mut gtk::ffi::GtkWidget) -> Result<ActiveEffect, &'static str> {
        let toplevel = unsafe { gtk::ffi::gtk_widget_get_toplevel(widget) };

        if toplevel.is_null() {
            return Err("The native window surface is not available yet.");
        }

        let window = unsafe { gtk::ffi::gtk_widget_get_window(toplevel) };

        if window.is_null() {
            return Err("The native window surface is not available yet.");
        }

        let is_wayland_window = unsafe {
            gtk::glib::gobject_ffi::g_type_check_instance_is_a(
                window.cast(),
                gdkwayland_sys::gdk_wayland_window_get_type(),
            )
        };

        if is_wayland_window == 0 {
            return Err("Background blur requires a Wayland session.");
        }

        let display = unsafe { gtk::gdk::ffi::gdk_window_get_display(window) };

        let wayland_display =
            unsafe { gdkwayland_sys::gdk_wayland_display_get_wl_display(display.cast()) };

        let native_surface =
            unsafe { gdkwayland_sys::gdk_wayland_window_get_wl_surface(window.cast()) };

        if wayland_display.is_null() || native_surface.is_null() {
            return Err("Could not access the Wayland window surface.");
        }

        let backend = unsafe { Backend::from_foreign_display(wayland_display.cast()) };

        let connection = Connection::from_backend(backend);

        let (globals, mut queue) = registry_queue_init::<State>(&connection)
            .map_err(|_| "Could not read Wayland globals.")?;

        let queue_handle = queue.handle();

        let manager = globals
            .bind::<ExtBackgroundEffectManagerV1, _, _>(&queue_handle, 1..=1, ())
            .map_err(|_| "This compositor does not support background blur.")?;

        let compositor = globals
            .bind::<WlCompositor, _, _>(&queue_handle, 1..=6, ())
            .map_err(|_| "The compositor does not provide the Wayland compositor interface.")?;

        let wrapper =
            unsafe { wayland_sys::client::wl_proxy_create_wrapper(native_surface.cast()) };

        if wrapper.is_null() {
            return Err("Could not create a safe Wayland surface handle.");
        }

        let surface_id = match unsafe { ObjectId::from_ptr(WlSurface::interface(), wrapper) } {
            Ok(id) => id,
            Err(_) => {
                unsafe {
                    wayland_sys::client::wl_proxy_wrapper_destroy(wrapper);
                }
                return Err("Could not attach blur to the native window surface.");
            }
        };

        let surface = match WlSurface::from_id(&connection, surface_id) {
            Ok(surface) => surface,
            Err(_) => {
                unsafe {
                    wayland_sys::client::wl_proxy_wrapper_destroy(wrapper);
                }
                return Err("Could not attach blur to the native window surface.");
            }
        };

        let surface = ForeignSurface {
            wrapper,
            surface: ManuallyDrop::new(surface),
        };

        let mut state = State::default();

        queue
            .roundtrip(&mut state)
            .map_err(|_| "Could not read compositor blur capabilities.")?;

        if state.capabilities & 1 == 0 {
            return Err("This compositor does not advertise ext-background-effect-v1 blur.");
        }

        let effect = manager.get_background_effect(&surface.surface, &queue_handle, ());

        let active_effect = ActiveEffect {
            connection,
            compositor,
            queue_handle,
            effect,
        };

        let width = unsafe { gtk::ffi::gtk_widget_get_allocated_width(toplevel) };

        let height = unsafe { gtk::ffi::gtk_widget_get_allocated_height(toplevel) };

        active_effect.update_region(width, height, BLUR_CORNER_RADIUS)?;
        surface.surface.commit();

        Ok(active_effect)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(BlurStatus {
            status: Arc::new(Mutex::new("Checking compositor blur support…".into())),
            rounded: Arc::new(AtomicBool::new(true)),

            #[cfg(target_os = "linux")]
            effect: Arc::new(Mutex::new(None)),
        })
        // Add this immediately after BlurStatus:
        .manage(WallpaperState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::wifi::wifi_is_enabled,
            commands::wifi::wifi_set_enabled,
            commands::wifi::wifi_scan_networks,
            get_blur_status,
            window_is_rounded,
            commands::bluetooth::bluetooth_status,
            commands::bluetooth::bluetooth_set_enabled,
            commands::bluetooth::bluetooth_devices,
            commands::bluetooth::bluetooth_start_scan,
            commands::bluetooth::bluetooth_stop_scan,
            commands::bluetooth::bluetooth_pair,
            commands::bluetooth::bluetooth_connect,
            commands::bluetooth::bluetooth_disconnect,
            wallpaper_list,
            wallpaper_current,
            wallpaper_folder,
            wallpaper_set,
            wallpaper_restore,
            display_list,
            display_apply,
            display_primary_get,
            display_primary_set,
            commands::sound::sound_get_state,
            commands::sound::sound_set_volume,
            commands::sound::sound_toggle_mute,
            commands::sound::sound_increase_volume,
            commands::sound::sound_decrease_volume,
        ])
        .setup(|app| {
            #[cfg(target_os = "linux")]
            {
                let window = app
                    .get_webview_window("main")
                    .ok_or("The main Tauri window was not created.")?;

                let app_handle = app.handle().clone();
                let state = app.state::<BlurStatus>();

                let status = Arc::clone(&state.status);
                let active_effect = Arc::clone(&state.effect);
                let rounded = Arc::clone(&state.rounded);

                window.with_webview(move |webview| {
                    let widget = webview.inner().as_ptr() as *mut gtk::ffi::GtkWidget;

                    let result = unsafe { background_blur::enable(widget) };

                    let message = match result {
                        Ok(effect) => {
                            *active_effect.lock().expect("blur effect lock poisoned") =
                                Some(effect);

                            unsafe {
                                background_blur::track_size(
                                    widget,
                                    Arc::clone(&active_effect),
                                    Arc::clone(&rounded),
                                    app_handle.clone(),
                                );
                            }

                            "Blur active — using compositor ext-background-effect-v1.".to_string()
                        }
                        Err(error) => error.to_string(),
                    };

                    *status.lock().expect("blur status lock poisoned") = message.clone();

                    if let Err(error) = app_handle.emit("background-blur-status", message) {
                        eprintln!("Could not publish background blur status: {error}");
                    }
                })?;
            }

            #[cfg(not(target_os = "linux"))]
            {
                *app.state::<BlurStatus>()
                    .status
                    .lock()
                    .expect("blur status lock poisoned") =
                    "Compositor background blur is currently supported on Linux Wayland only."
                        .into();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Aether Settings");
}
