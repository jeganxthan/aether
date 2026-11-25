use crate::ThemeService;
use gpui_kit::{
    App,
    component::{ThemeConfig, ThemeRegistry, ThemeSet},
};

fn register_themes(cx: &mut App) {
    let mut themes = ThemeSet::default();
    themes.themes = vec![
        serde_json::from_str::<ThemeConfig>(include_str!("themes/aether-dark.json"))
            .expect("Black theme configuration must be valid"),
        serde_json::from_str::<ThemeConfig>(include_str!("themes/aether-light.json"))
            .expect("Light theme configuration must be valid"),
    ];
    let themes = serde_json::to_string(&themes).expect("themes must serialize");
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(&themes)
        .expect("Black and Light themes must load");
}

pub fn init(cx: &mut App) {
    register_themes(cx);

    let selected = ThemeRegistry::global(cx)
        .themes()
        .get("Black")
        .cloned()
        .expect("the default Black theme must be registered");
    ThemeService::apply_config(cx, selected);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_black_and_light_theme_configs_match_their_modes() {
        let black: ThemeConfig =
            serde_json::from_str(include_str!("themes/aether-dark.json")).unwrap();
        let light: ThemeConfig =
            serde_json::from_str(include_str!("themes/aether-light.json")).unwrap();

        assert_eq!(black.name.as_ref(), "Black");
        assert_eq!(light.name.as_ref(), "Light");
        assert!(black.mode.is_dark());
        assert!(!light.mode.is_dark());
    }

    #[test]
    fn available_themes_are_limited_to_black_and_light() {
        let themes = vec![
            serde_json::from_str::<ThemeConfig>(include_str!("themes/aether-dark.json")).unwrap(),
            serde_json::from_str::<ThemeConfig>(include_str!("themes/aether-light.json")).unwrap(),
            serde_json::from_str::<ThemeConfig>(r#"{"name":"Unused"}"#).unwrap(),
        ]
        .into_iter()
        .map(|theme| (theme.name.clone(), std::rc::Rc::new(theme)))
        .collect();

        let available = available_themes(&themes);
        assert_eq!(
            available
                .iter()
                .map(|theme| theme.name.as_ref())
                .collect::<Vec<_>>(),
            ["Light", "Black"]
        );
    }
}

pub(crate) fn available_themes(
    themes: &std::collections::HashMap<gpui_kit::SharedString, std::rc::Rc<ThemeConfig>>,
) -> Vec<std::rc::Rc<ThemeConfig>> {
    ["Light", "Black"]
        .into_iter()
        .filter_map(|name| themes.get(name).cloned())
        .collect()
}
