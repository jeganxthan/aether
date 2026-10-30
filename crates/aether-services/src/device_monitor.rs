use std::{fs, io, thread};

use tokio::{io::Interest, io::unix::AsyncFd, runtime::Builder};
use zbus::{Connection, Proxy};
use zvariant::OwnedValue;

const BUS_NAME: &str = "org.freedesktop.Notifications";
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";

pub fn start_device_monitor() -> thread::JoinHandle<()> {
    thread::spawn(|| {
        let runtime = match Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("Device monitor runtime error: {error}");
                return;
            }
        };

        runtime.block_on(async {
            if let Err(error) = run().await {
                eprintln!("Aether device monitor stopped: {error}");
            }
        });
    })
}

async fn run() -> io::Result<()> {
    let connection = match Connection::session().await {
        Ok(connection) => connection,
        Err(error) => {
            return Err(io::Error::other(error));
        }
    };

    let socket = match udev::MonitorBuilder::new()
        .and_then(|builder| builder.match_subsystem_devtype("usb", "usb_device"))
        .and_then(|builder| builder.match_subsystem("input"))
        .and_then(|builder| builder.match_subsystem("sound"))
        .and_then(|builder| builder.match_subsystem("drm"))
        .and_then(|builder| builder.listen())
    {
        Ok(socket) => socket,
        Err(error) => {
            return Err(io::Error::other(error));
        }
    };

    /*
     * The udev socket is nonblocking, so reading it directly would spin at full
     * speed whenever no event is waiting. Watching the same file descriptor with
     * the runtime makes us sleep until the kernel says there is something to read,
     * then the iterator drains whatever arrived.
     *
     * `with_interest` is marked deprecated in favour of `register_with_interest`,
     * but that call does not exist in the tokio we build against; the socket is
     * owned here for the whole loop, so the ownership it takes is the one we want.
     */
    #[allow(deprecated)]
    let async_fd = match AsyncFd::with_interest(socket, Interest::READABLE) {
        Ok(async_fd) => async_fd,
        Err(error) => {
            return Err(error);
        }
    };

    eprintln!("Aether device monitor started");

    loop {
        let mut ready = match async_fd.readable().await {
            Ok(ready) => ready,
            Err(error) => {
                eprintln!("Device monitor wait failed: {error}");
                continue;
            }
        };

        {
            let socket = async_fd.get_ref();

            for event in socket.iter() {
                handle_event(&connection, &event).await;
            }
        }

        // The iterator stops at the first empty read, so clear the readiness flag
        // and wait again; a fresh event re-arms it.
        ready.clear_ready();
    }
}

// sends the notice a single udev event calls for, if it calls for one
async fn handle_event(connection: &Connection, event: &udev::Event) {
    let subsystem = event
        .subsystem()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    let action = event
        .action()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    let device_type = event
        .devtype()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    let is_monitor_hotplug = subsystem == "drm"
        && event
            .property_value("HOTPLUG")
            .and_then(|value| value.to_str())
            == Some("1");

    let connected = match action {
        "add" => true,
        "remove" => false,
        "change" if is_monitor_hotplug => true,
        _ => return,
    };

    let device_name = detect_device_name(event, subsystem, device_type);

    if subsystem == "usb" && !is_notifiable_usb_device(event) {
        return;
    }

    // DRM also emits card-level power events. Only connector nodes represent
    // an actual display hotplug.
    if subsystem == "drm" && !event.sysname().to_string_lossy().contains('-') {
        return;
    }

    // A charger (or other power event) can masquerade as a DRM hotplug on some
    // machines. Only announce a display when an external display is actually
    // reported as connected by the kernel.
    if subsystem == "drm" && connected && !external_display_connected() {
        return;
    }

    let Some((title, body, icon)) = device_message(
        subsystem,
        device_type,
        &device_name,
        connected,
        is_monitor_hotplug,
    ) else {
        return;
    };

    eprintln!("{title}: {body}");

    if let Err(error) = send_notification(connection, title, &body, icon).await {
        eprintln!("Could not send device notification: {error}");
    }
}

fn is_notifiable_usb_device(device: &udev::Device) -> bool {
    let Some(interfaces) = device
        .property_value("ID_USB_INTERFACES")
        .and_then(|value| value.to_str())
    else {
        return false;
    };

    // HID, mass-storage, and USB-audio devices are useful user-facing events.
    is_notifiable_usb_interfaces(interfaces)
}

fn is_notifiable_usb_interfaces(interfaces: &str) -> bool {
    interfaces
        .split(':')
        .any(|class| class.starts_with("03") || class.starts_with("08") || class.starts_with("01"))
}

fn detect_device_name(device: &udev::Device, subsystem: &str, device_type: &str) -> String {
    if subsystem == "drm" {
        return "External display".to_string();
    }

    for key in ["ID_MODEL_FROM_DATABASE", "ID_MODEL", "NAME"] {
        if let Some(value) = device.property_value(key).and_then(|value| value.to_str()) {
            let value = value.replace('_', " ").trim().to_string();

            if !value.is_empty() {
                return value;
            }
        }
    }

    if let Some(name) = device
        .attribute_value("name")
        .and_then(|value| value.to_str())
    {
        let name = name.trim();

        if !name.is_empty() {
            return name.to_string();
        }
    }

    match subsystem {
        "input" => "Input device".to_string(),
        "sound" => "Audio device".to_string(),
        "usb" if device_type == "usb_device" => "USB device".to_string(),
        _ => "Device".to_string(),
    }
}

fn external_display_connected() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        // If sysfs cannot be read, keep the previous behavior rather than
        // dropping legitimate display notifications.
        return true;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        // Only connector entries look like "card0-HDMI-A-1".
        if !name.contains('-') {
            continue;
        }

        let lower = name.to_lowercase();
        if lower.contains("edp")
            || lower.contains("lvds")
            || lower.contains("dsi")
            || lower.contains("writeback")
        {
            continue;
        }

        let status_path = entry.path().join("status");
        if let Ok(status) = fs::read_to_string(&status_path)
            && status.trim() == "connected"
        {
            return true;
        }
    }

    false
}

fn device_message(
    subsystem: &str,
    device_type: &str,
    name: &str,
    connected: bool,
    monitor_hotplug: bool,
) -> Option<(&'static str, String, &'static str)> {
    let (category, icon) = match subsystem {
        "input" => {
            // Only report recognized input devices.
            // Other input subsystem events can be noisy.
            return None;
        }

        "usb" => {
            // Notify once for physical USB devices, not every interface.
            if device_type != "usb_device" {
                return None;
            }

            // HID and USB audio devices may also generate input/sound
            // events. Skip generic USB notifications for these devices
            // to reduce duplicates.
            //
            // This is a heuristic; composite devices may need refinement.
            return Some((
                if connected {
                    "Device connected"
                } else {
                    "Device disconnected"
                },
                format!(
                    "{} {}",
                    name,
                    if connected {
                        "was connected."
                    } else {
                        "was disconnected."
                    }
                ),
                "drive-removable-media",
            ));
        }

        "sound" => ("Audio device", "audio-card"),
        "drm" if monitor_hotplug => ("Display", "video-display"),
        _ => return None,
    };

    Some((
        if connected {
            "Device connected"
        } else {
            "Device disconnected"
        },
        format!(
            "{}: {} {}",
            category,
            name,
            if connected {
                "connected."
            } else {
                "disconnected."
            }
        ),
        icon,
    ))
}

async fn send_notification(
    connection: &Connection,
    summary: &str,
    body: &str,
    icon: &str,
) -> zbus::Result<()> {
    let proxy = Proxy::new(connection, BUS_NAME, OBJECT_PATH, INTERFACE).await?;

    let actions: Vec<String> = Vec::new();
    let hints: std::collections::HashMap<String, OwnedValue> = std::collections::HashMap::new();

    let args = (
        "Aether", 0_u32, icon, summary, body, actions, hints, 5000_i32,
    );

    let _: u32 = proxy.call("Notify", &args).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_usb_device_gets_a_notice() {
        let message = device_message("usb", "usb_device", "Some Drive", true, false);

        let (title, body, icon) = message.expect("a usb device should be announced");

        assert_eq!(title, "Device connected");
        assert!(body.contains("Some Drive"));
        assert_eq!(icon, "drive-removable-media");
    }

    #[test]
    fn a_usb_interface_is_skipped() {
        // a composite device sends one event per interface; only the device itself is news
        assert!(device_message("usb", "usb_interface", "Some Drive", true, false).is_none());
    }

    #[test]
    fn only_user_facing_usb_classes_are_notifiable() {
        assert!(is_notifiable_usb_interfaces(":030101:"));
        assert!(is_notifiable_usb_interfaces(":080650:"));
        assert!(!is_notifiable_usb_interfaces(":090000:"));
    }

    #[test]
    fn input_events_stay_quiet() {
        // input events are frequent and mostly uninteresting
        assert!(device_message("input", "input", "Mouse", true, false).is_none());
    }

    #[test]
    fn an_audio_device_gets_a_notice_either_way() {
        let (title, _, icon) = device_message("sound", "sound", "Card", true, false)
            .expect("audio should be announced");

        assert_eq!(title, "Device connected");
        assert_eq!(icon, "audio-card");

        let (title, _, _) = device_message("sound", "sound", "Card", false, false)
            .expect("audio should be announced");

        assert_eq!(title, "Device disconnected");
    }

    #[test]
    fn a_display_is_only_announced_on_a_real_hotplug() {
        // a drm change without the hotplug property is not a display being added
        assert!(device_message("drm", "drm_minor", "HDMI", true, false).is_none());

        let (title, _, icon) =
            device_message("drm", "drm_minor", "HDMI", true, true).expect("a hotplug is news");

        assert_eq!(title, "Device connected");
        assert_eq!(icon, "video-display");
    }

    #[test]
    fn an_unknown_subsystem_stays_quiet() {
        assert!(device_message("tty", "tty", "tty0", true, false).is_none());
    }
}
