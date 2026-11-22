use gpui_kit::{
    App, Global,
    component::{Theme, ThemeMode},
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

    pub const fn palette(self) -> ThemePalette {
        match self {
            Self::Black => ThemePalette {
                name: "Black",
                background: 0x000000,
                foreground: 0xf5f5f5,
                content_background: 0x111111,
                sidebar_background: 0x141416,
                sidebar_blur_background: 0x1a1a2826,
                sidebar_blur_border: 0xffffff20,
                border: 0x2a2a2e,
                accent: 0x9d9d9d,
            },
            Self::White => ThemePalette {
                name: "White",
                background: 0xffffff,
                foreground: 0x171717,
                content_background: 0xffffff,
                sidebar_background: 0xf4f4f6,
                sidebar_blur_background: 0xffffff26,
                sidebar_blur_border: 0x00000020,
                border: 0xe2e2e6,
                accent: 0x333333,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ThemeService {
    current: ThemeKind,
}

impl Global for ThemeService {}

impl ThemeService {
    pub fn initialize(cx: &mut App) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }

        Self::apply_current(cx);
    }

    pub fn current(cx: &App) -> ThemeKind {
        cx.try_global::<Self>()
            .map_or(ThemeKind::default(), |service| service.current)
    }

    pub fn set(cx: &mut App, theme: ThemeKind) {
        cx.set_global(Self { current: theme });
        Self::apply_current(cx);
    }

    fn apply_current(cx: &mut App) {
        let mode = match Self::current(cx) {
            ThemeKind::Black => ThemeMode::Dark,
            ThemeKind::White => ThemeMode::Light,
        };
        Theme::change(mode, None, cx);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemePalette {
    pub name: &'static str,
    pub background: u32,
    pub foreground: u32,
    pub content_background: u32,
    pub sidebar_background: u32,
    pub sidebar_blur_background: u32,
    pub sidebar_blur_border: u32,
    pub border: u32,
    pub accent: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_contains_only_black_and_white() {
        assert_eq!(ThemeKind::ALL, [ThemeKind::Black, ThemeKind::White]);
        assert_eq!(ThemeKind::Black.palette().name, "Black");
        assert_eq!(ThemeKind::White.palette().name, "White");
    }
}
