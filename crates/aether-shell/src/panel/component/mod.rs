pub mod battery_menu;
pub mod battery_popup;
pub mod bluetooth_menu;
pub mod bluetooth_popup;
pub mod clipboard_popup;
pub mod launcher;
pub mod notification_menu;
pub mod player_popup;
pub mod polkit_auth;
mod popup_motion;
pub mod settings_popup;
pub mod wifi_popup;

pub(super) fn island_shadow(opacity: f32) -> aether::Shadow {
    aether::Shadow::drop(crate::theme::theme().shadow)
        .opacity(opacity)
        .blur(14.0)
        .offset(0.0, 4.0)
}
