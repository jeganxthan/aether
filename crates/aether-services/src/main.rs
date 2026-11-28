mod daemon;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    daemon::start().await
}
// eval "$(dbus-launch --sh-syntax)"
// export XDG_CURRENT_DESKTOP=aether XDG_SESSION_TYPE=wayland
// dbus-update-activation-environment XDG_CURRENT_DESKTOP=aether XDG_SESSION_TYPE=wayland WAYLAND_DISPLAY="$WAYLAND_DISPLAY"
// pkill -f xdg-desktop-portal
// cargo run -p aether-services &
// cargo run -p aether-settings