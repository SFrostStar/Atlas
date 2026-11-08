//! Mod Menu 21.0.0 configuration order, defaults and value cycling (MIT).
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path, sync::RwLock};

// Default attributes are declared explicitly: Java's ordinals are part of the cycle order.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Sorting {
    #[default]
    Ascending,
    Descending,
    HasUpdate,
}
impl Sorting {
    pub fn cycle(&mut self, rev: bool) {
        *self = match (*self, rev) {
            (Self::Ascending, false) | (Self::HasUpdate, true) => Self::Descending,
            (Self::Descending, false) | (Self::Ascending, true) => Self::HasUpdate,
            _ => Self::Ascending,
        };
    }
    pub fn value(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
            Self::HasUpdate => "has_update",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Libraries {
    False,
    #[default]
    WithConfig,
    True,
}
impl Libraries {
    pub fn cycle(&mut self, rev: bool) {
        *self = match (*self, rev) {
            (Self::False, false) | (Self::True, true) => Self::WithConfig,
            (Self::WithConfig, false) | (Self::False, true) => Self::True,
            _ => Self::False,
        };
    }
    pub fn value(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::WithConfig => "with_config",
            Self::True => "true",
        }
    }
    pub fn hide(self, library: bool, config: bool) -> bool {
        library
            && match self {
                Self::False => true,
                Self::WithConfig => !config,
                Self::True => false,
            }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TitleStyle {
    Classic,
    ReplaceRealms,
    Shrink,
    #[default]
    Icon,
}
impl TitleStyle {
    pub fn cycle(&mut self, rev: bool) {
        let a = [Self::Classic, Self::ReplaceRealms, Self::Shrink, Self::Icon];
        let n = a.iter().position(|v| *v == *self).unwrap_or(3);
        *self = a[(n + if rev { 3 } else { 1 }) % 4];
    }
    pub fn value(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::ReplaceRealms => "replace_realms",
            Self::Shrink => "shrink",
            Self::Icon => "icon",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameStyle {
    Insert,
    #[default]
    Icon,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CountLocation {
    #[default]
    TitleScreen,
    ModsButton,
    TitleScreenAndModsButton,
    None,
}
impl CountLocation {
    pub fn title(self) -> bool {
        matches!(self, Self::TitleScreen | Self::TitleScreenAndModsButton)
    }
    pub fn button(self) -> bool {
        matches!(self, Self::ModsButton | Self::TitleScreenAndModsButton)
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Alpha,
    Beta,
    #[default]
    Release,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub sorting: Sorting,
    pub count_libraries: bool,
    pub compact_list: bool,
    pub count_children: bool,
    pub mods_button_style: TitleStyle,
    pub game_menu_button_style: GameStyle,
    pub count_hidden_mods: bool,
    pub mod_count_location: CountLocation,
    pub hide_mod_links: bool,
    pub show_libraries: Libraries,
    pub hide_mod_license: bool,
    pub hide_badges: bool,
    pub hide_mod_credits: bool,
    pub easter_eggs: bool,
    pub random_java_colors: bool,
    pub translate_names: bool,
    pub translate_descriptions: bool,
    pub update_checker: bool,
    pub button_update_badge: bool,
    pub update_channel: Channel,
    pub quick_configure: bool,
    pub modify_title_screen: bool,
    pub modify_game_menu: bool,
    pub hide_config_buttons: bool,
    pub config_mode: bool,
    pub disable_drag_and_drop: bool,
    pub hidden_mods: BTreeSet<String>,
    pub hidden_configs: BTreeSet<String>,
    pub disable_update_checker: BTreeSet<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            sorting: Sorting::Ascending,
            count_libraries: true,
            compact_list: false,
            count_children: true,
            mods_button_style: TitleStyle::Icon,
            game_menu_button_style: GameStyle::Icon,
            count_hidden_mods: true,
            mod_count_location: CountLocation::TitleScreen,
            hide_mod_links: false,
            show_libraries: Libraries::WithConfig,
            hide_mod_license: false,
            hide_badges: false,
            hide_mod_credits: false,
            easter_eggs: true,
            random_java_colors: false,
            translate_names: true,
            translate_descriptions: true,
            update_checker: true,
            button_update_badge: true,
            update_channel: Channel::Release,
            quick_configure: true,
            modify_title_screen: true,
            modify_game_menu: true,
            hide_config_buttons: false,
            config_mode: false,
            disable_drag_and_drop: false,
            hidden_mods: BTreeSet::new(),
            hidden_configs: BTreeSet::new(),
            disable_update_checker: BTreeSet::new(),
        }
    }
}
static CURRENT: RwLock<Option<Config>> = RwLock::new(None);
pub fn current() -> Config {
    CURRENT
        .read()
        .ok()
        .and_then(|v| v.clone())
        .unwrap_or_default()
}
impl Config {
    pub fn publish(&self) {
        if let Ok(mut value) = CURRENT.write() {
            *value = Some(self.clone());
        }
    }
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("config/modmenu.json");
        let value = std::fs::metadata(&path)
            .ok()
            .filter(|m| m.len() <= 128 * 1024)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self::publish(&value);
        value
    }
    pub fn save(&self, dir: &Path) {
        self.publish();
        let path = dir.join("config/modmenu.json");
        if *self == Self::default() && !path.exists() {
            return;
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, bytes);
        }
    }
    pub fn option(&self, key: &str) -> String {
        match key {
            "sorting" => self.sorting.value().into(),
            "show_libraries" => self.show_libraries.value().into(),
            "mods_button_style" => self.mods_button_style.value().into(),
            "game_menu_button_style" => if self.game_menu_button_style == GameStyle::Icon {
                "icon"
            } else {
                "insert"
            }
            .into(),
            "mod_count_location" => match self.mod_count_location {
                CountLocation::TitleScreen => "title_screen",
                CountLocation::ModsButton => "mods_button",
                CountLocation::TitleScreenAndModsButton => "title_screen_and_mods_button",
                CountLocation::None => "none",
            }
            .into(),
            "update_channel" => match self.update_channel {
                Channel::Alpha => "alpha",
                Channel::Beta => "beta",
                Channel::Release => "release",
            }
            .into(),
            _ => serde_json::to_value(self)
                .ok()
                .and_then(|v| v.get(key).and_then(|v| v.as_bool()).map(|v| v.to_string()))
                .unwrap_or_default(),
        }
    }
    pub fn cycle(&mut self, key: &str, reverse: bool) {
        match key {
            "sorting" => self.sorting.cycle(reverse),
            "show_libraries" => self.show_libraries.cycle(reverse),
            "mods_button_style" => self.mods_button_style.cycle(reverse),
            "game_menu_button_style" => {
                self.game_menu_button_style = if self.game_menu_button_style == GameStyle::Icon {
                    GameStyle::Insert
                } else {
                    GameStyle::Icon
                }
            }
            "mod_count_location" => {
                let a = [
                    CountLocation::TitleScreen,
                    CountLocation::ModsButton,
                    CountLocation::TitleScreenAndModsButton,
                    CountLocation::None,
                ];
                let n = a
                    .iter()
                    .position(|v| *v == self.mod_count_location)
                    .unwrap_or(0);
                self.mod_count_location = a[(n + if reverse { 3 } else { 1 }) % 4];
            }
            "update_channel" => {
                let a = [Channel::Alpha, Channel::Beta, Channel::Release];
                let n = a
                    .iter()
                    .position(|v| *v == self.update_channel)
                    .unwrap_or(2);
                self.update_channel = a[(n + if reverse { 2 } else { 1 }) % 3];
            }
            _ => {
                if let Ok(mut v) = serde_json::to_value(&*self) {
                    if let Some(on) = v.get(key).and_then(|v| v.as_bool()) {
                        v[key] = serde_json::Value::Bool(!on);
                        if let Ok(next) = serde_json::from_value(v) {
                            *self = next;
                        }
                    }
                }
            }
        }
        self.publish();
    }
}
pub const OPTIONS: [&str; 21] = [
    "sorting",
    "count_libraries",
    "compact_list",
    "count_children",
    "mods_button_style",
    "game_menu_button_style",
    "count_hidden_mods",
    "mod_count_location",
    "hide_mod_links",
    "show_libraries",
    "hide_mod_license",
    "hide_badges",
    "hide_mod_credits",
    "easter_eggs",
    "random_java_colors",
    "translate_names",
    "translate_descriptions",
    "update_checker",
    "button_update_badge",
    "update_channel",
    "quick_configure",
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_cycle_order_and_defaults() {
        let mut c = Config::default();
        assert_eq!(c.show_libraries, Libraries::WithConfig);
        assert_eq!(c.mods_button_style, TitleStyle::Icon);
        c.cycle("show_libraries", false);
        assert_eq!(c.show_libraries, Libraries::True);
        c.cycle("show_libraries", true);
        assert_eq!(c.show_libraries, Libraries::WithConfig);
        c.cycle("sorting", true);
        assert_eq!(c.sorting, Sorting::HasUpdate);
        c.cycle("mods_button_style", false);
        assert_eq!(c.mods_button_style, TitleStyle::Classic);
        assert_eq!(OPTIONS.len(), 21);
    }
}
