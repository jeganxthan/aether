use std::rc::Rc;

use gpui_kit::{
    App, Hsla, SharedString,
    component::{Theme, ThemeConfig},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThemeKind {
    #[default]
    Black,
    White,
}

impl ThemeKind {
    pub const ALL: [Self; 2] = [Self::Black, Self::White];

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "black" => Some(Self::Black),
            "white" => Some(Self::White),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::White => "White",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ThemeService;

impl ThemeService {
    pub fn current_name(cx: &App) -> SharedString {
        Theme::global(cx).theme_name().clone()
    }

    pub fn available(cx: &App) -> Vec<Rc<ThemeConfig>> {
        crate::registry::available_themes(gpui_kit::component::ThemeRegistry::global(cx).themes())
    }

    pub fn palette(cx: &App) -> ThemePalette {
        ThemePalette::from_theme(Theme::global(cx))
    }

    pub fn apply_config(cx: &mut App, config: Rc<ThemeConfig>) {
        Theme::update(cx, |theme| theme.apply_config(&config));
    }
}

impl ThemePalette {
    pub fn from_theme(theme: &Theme) -> Self {
        let colors = &theme.colors;
        let to_rgb = color_to_rgb;
        let with_alpha = color_to_rgba;

        Self {
            background: to_rgb(colors.background),
            foreground: to_rgb(colors.foreground),
            content_background: to_rgb(colors.background),
            surface_background: to_rgb(colors.secondary),
            input_background: to_rgb(colors.list),
            sidebar_background: to_rgb(colors.sidebar),
            sidebar_blur_background: with_alpha(colors.sidebar, 0.15),
            hover_background: to_rgb(colors.list_hover),
            selected_background: to_rgb(colors.list_active),
            sidebar_foreground: to_rgb(colors.sidebar_foreground),
            sidebar_active_background: to_rgb(colors.sidebar_accent),
            sidebar_active_foreground: to_rgb(colors.sidebar_accent_foreground),
            sidebar_blur_border: with_alpha(colors.sidebar_border, 0.14),
            sidebar_border: to_rgb(colors.sidebar_border),
            border: to_rgb(colors.border),
            input_border: to_rgb(colors.input),
            muted_foreground: to_rgb(colors.muted_foreground),
            accent: to_rgb(colors.primary),
            accent_foreground: to_rgb(colors.primary_foreground),
            success: to_rgb(colors.success),
            success_foreground: to_rgb(colors.success_foreground),
            danger: to_rgb(colors.danger),
            danger_foreground: to_rgb(colors.danger_foreground),
            warning: to_rgb(colors.warning),
            warning_foreground: to_rgb(colors.warning_foreground),
        }
    }
}

fn color_to_rgb(color: Hsla) -> u32 {
    let color = color.to_rgb();
    (color.r * 255.0).round() as u32 * 0x010000
        | (color.g * 255.0).round() as u32 * 0x000100
        | (color.b * 255.0).round() as u32
}

fn color_to_rgba(color: Hsla, alpha: f32) -> u32 {
    let color = color.to_rgb();
    (color.r * 255.0).round() as u32 * 0x01000000
        | (color.g * 255.0).round() as u32 * 0x00010000
        | (color.b * 255.0).round() as u32 * 0x00000100
        | (alpha * 255.0).round() as u32
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemePalette {
    pub background: u32,
    pub foreground: u32,
    pub content_background: u32,
    pub surface_background: u32,
    pub input_background: u32,
    pub sidebar_background: u32,
    pub sidebar_blur_background: u32,
    pub hover_background: u32,
    pub selected_background: u32,
    pub sidebar_foreground: u32,
    pub sidebar_active_background: u32,
    pub sidebar_active_foreground: u32,
    pub sidebar_blur_border: u32,
    pub sidebar_border: u32,
    pub border: u32,
    pub input_border: u32,
    pub muted_foreground: u32,
    pub accent: u32,
    pub accent_foreground: u32,
    pub success: u32,
    pub success_foreground: u32,
    pub danger: u32,
    pub danger_foreground: u32,
    pub warning: u32,
    pub warning_foreground: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsla_colors_convert_to_rgb_hex_without_alpha_byte() {
        assert_eq!(color_to_rgb(Hsla::from(gpui_kit::rgb(0xff0000))), 0xff0000);
        assert_eq!(color_to_rgb(Hsla::from(gpui_kit::rgb(0x00ff00))), 0x00ff00);
        assert_eq!(color_to_rgb(Hsla::from(gpui_kit::rgb(0x0000ff))), 0x0000ff);
    }

    #[test]
    fn palette_uses_selected_theme_sidebar_and_status_roles() {
        let mut theme = Theme::default();
        theme.colors.sidebar_foreground = gpui_kit::rgb(0x112233).into();
        theme.colors.sidebar_accent = gpui_kit::rgb(0x223344).into();
        theme.colors.sidebar_accent_foreground = gpui_kit::rgb(0x334455).into();
        theme.colors.input = gpui_kit::rgb(0x445566).into();
        theme.colors.success = gpui_kit::rgb(0x556677).into();
        theme.colors.danger = gpui_kit::rgb(0x667788).into();

        let palette = ThemePalette::from_theme(&theme);

        assert_eq!(palette.sidebar_foreground, 0x112233);
        assert_eq!(palette.sidebar_active_background, 0x223344);
        assert_eq!(palette.sidebar_active_foreground, 0x334455);
        assert_eq!(palette.input_border, 0x445566);
        assert_eq!(palette.success, 0x556677);
        assert_eq!(palette.danger, 0x667788);
    }
}
