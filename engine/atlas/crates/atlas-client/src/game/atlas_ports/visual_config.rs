//! Configuration adapters for the supplied Java mod versions. See visual.rs for authorship.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Radar {
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub max_players: usize,
    pub max_distance: f64,
    pub background_opacity: f32,
    pub show_health: bool,
    pub show_distance: bool,
    pub show_arrows: bool,
    pub hud_width: i32,
}
impl Default for Radar {
    fn default() -> Self {
        Self {
            enabled: true,
            x: 10,
            y: 10,
            scale: 1.0,
            max_players: 8,
            max_distance: 64.0,
            background_opacity: 0.65,
            show_health: true,
            show_distance: true,
            show_arrows: true,
            hud_width: 175,
        }
    }
}
impl Radar {
    pub fn validate(&mut self) {
        self.x = self.x.max(0);
        self.y = self.y.max(0);
        self.scale = self.scale.clamp(0.4, 2.5);
        self.max_players = self.max_players.clamp(1, 30);
        self.max_distance = self.max_distance.clamp(5.0, 500.0);
        self.background_opacity = self.background_opacity.clamp(0.0, 1.0);
        self.hud_width = self.hud_width.clamp(120, 300);
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(in crate::game) struct NoFluid {
    pub enabled: bool,
    pub no_water_fog: bool,
    pub no_lava_fog: bool,
    pub no_powder_snow_fog: bool,
    pub no_water_overlay: bool,
    pub no_fire_overlay: bool,
    pub max_water_vision: bool,
    pub fluid_night_vision: bool,
}
impl Default for NoFluid {
    fn default() -> Self {
        Self {
            enabled: true,
            no_water_fog: true,
            no_lava_fog: true,
            no_powder_snow_fog: true,
            no_water_overlay: true,
            no_fire_overlay: true,
            max_water_vision: true,
            fluid_night_vision: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Pvp {
    pub enabled: bool,
    pub text: String,
    pub duration_ms: u64,
    pub offset_y: i32,
    pub color_hex: String,
    pub bold: bool,
    pub play_sound: bool,
    pub sound_pitch: f32,
    pub sound_volume: f32,
    pub fade_effect: bool,
    pub scale: f32,
    pub custom_keywords: Vec<String>,
}
impl Default for Pvp {
    fn default() -> Self {
        Self {
            enabled: true,
            text: "ТЫ ВЫЖИЛ!".into(),
            duration_ms: 2000,
            offset_y: 12,
            color_hex: "#00FF55".into(),
            bold: true,
            play_sound: true,
            sound_pitch: 1.3,
            sound_volume: 0.8,
            fade_effect: true,
            scale: 1.0,
            custom_keywords: Vec::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct RwJoiner {
    pub enabled: bool,
    pub target_grief: i32,
    pub target_mode: String,
    pub click_delay_ms: u64,
    pub auto_restart_rejoin: bool,
    pub sound_alert: bool,
    pub show_hud: bool,
    pub hud_x: i32,
    pub hud_y: i32,
}
impl Default for RwJoiner {
    fn default() -> Self {
        Self {
            enabled: false,
            target_grief: 11,
            target_mode: "1.21".into(),
            click_delay_ms: 900,
            auto_restart_rejoin: true,
            sound_alert: true,
            show_hud: true,
            hud_x: 10,
            hud_y: 10,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Outline {
    pub enabled: bool,
    pub through_walls: bool,
    pub line_width: f32,
    pub expand: f32,
    pub visible_alpha: f32,
    pub xray_alpha: f32,
}
impl Default for Outline {
    fn default() -> Self {
        Self {
            enabled: true,
            through_walls: true,
            line_width: 1.5,
            expand: 0.05,
            visible_alpha: 1.0,
            xray_alpha: 0.35,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Beam {
    pub enabled: bool,
    pub through_walls: bool,
    pub width: f32,
    pub height: f32,
    pub alpha: f32,
    pub spin_degrees_per_second: f32,
}
impl Default for Beam {
    fn default() -> Self {
        Self {
            enabled: true,
            through_walls: true,
            width: 0.25,
            height: 60.0,
            alpha: 0.5,
            spin_degrees_per_second: 20.0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Lootbeams {
    pub enabled: bool,
    pub max_render_distance: f64,
    pub outline: Outline,
    pub beam: Beam,
    pub rarity_colors: BTreeMap<String, String>,
    pub item_color_overrides: BTreeMap<String, String>,
    pub whitelist_mode: bool,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
}
impl Default for Lootbeams {
    fn default() -> Self {
        Self {
            enabled: true,
            max_render_distance: 48.0,
            outline: Outline::default(),
            beam: Beam::default(),
            rarity_colors: [
                ("COMMON", "#FFFFFF"),
                ("UNCOMMON", "#FFFF55"),
                ("RARE", "#55FFFF"),
                ("EPIC", "#FF55FF"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
            item_color_overrides: [
                ("minecraft:netherite_ingot", "#B24544"),
                ("minecraft:diamond", "#4DE6E0"),
                ("minecraft:nether_star", "#FFD966"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
            whitelist_mode: false,
            whitelist: Vec::new(),
            blacklist: ["arrow", "snowball", "egg", "experience_bottle"]
                .map(|s| format!("minecraft:{s}"))
                .to_vec(),
        }
    }
}
impl Lootbeams {
    pub fn color(&self, id: &str, rarity: &str) -> Option<u32> {
        if (self.whitelist_mode && !self.whitelist.iter().any(|s| s == id))
            || (!self.whitelist_mode && self.blacklist.iter().any(|s| s == id))
        {
            return None;
        }
        self.item_color_overrides
            .get(id)
            .and_then(|s| hex(s))
            .or_else(|| self.rarity_colors.get(rarity).and_then(|s| hex(s)))
            .or(Some(0xffffff))
    }
}
pub(super) fn hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s.strip_prefix('#').unwrap_or(s), 16)
        .ok()
        .map(|v| v & 0xffffff)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Kill {
    pub enabled: bool,
}
impl Default for Kill {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// The original uses a deliberately small YAML dialect rather than a YAML library.
#[derive(Clone)]
pub(super) struct Totem {
    pub enabled: bool,
    pub label: String,
    pub format: String,
    pub count_prefix: String,
    pub label_color: u32,
    pub count_color: u32,
    pub dynamic_color: bool,
    pub scale: f32,
    pub y_offset: f32,
    pub background: bool,
    pub show_self: bool,
    pub hide_teammates: bool,
    pub show_zero: bool,
    pub reset_on_death: bool,
    pub reset_timeout: f64,
    pub max_distance: f64,
    pub chat_notify: bool,
}
impl Default for Totem {
    fn default() -> Self {
        Self {
            enabled: true,
            label: "Totems".into(),
            format: "{label}: {count}".into(),
            count_prefix: "-".into(),
            label_color: 0xffffff,
            count_color: 0xff5555,
            dynamic_color: true,
            scale: 1.0,
            y_offset: 10.0,
            background: true,
            show_self: false,
            hide_teammates: false,
            show_zero: false,
            reset_on_death: true,
            reset_timeout: 30.0,
            max_distance: 64.0,
            chat_notify: false,
        }
    }
}
impl Totem {
    pub fn parse(input: &str) -> Self {
        let mut c = Self::default();
        for line in input.lines() {
            let line = strip_comment(line);
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim().to_lowercase();
            let value = unquote(value.trim());
            let number = value.replace(',', ".").parse::<f64>().ok();
            let boolean = match value.to_lowercase().as_str() {
                "true" | "yes" | "on" => Some(true),
                "false" | "no" | "off" => Some(false),
                _ => None,
            };
            macro_rules! b {
                ($field:ident) => {
                    if let Some(v) = boolean {
                        c.$field = v;
                    }
                };
            }
            match key.as_str() {
                "enabled" => b!(enabled),
                "label" => c.label = value,
                "format" => c.format = value,
                "count_prefix" => c.count_prefix = value,
                "label_color" => {
                    if let Some(v) = named_color(&value) {
                        c.label_color = v
                    }
                }
                "count_color" => {
                    if let Some(v) = named_color(&value) {
                        c.count_color = v
                    }
                }
                "dynamic_color" => b!(dynamic_color),
                "background" => b!(background),
                "show_self" => b!(show_self),
                "hide_teammates" => b!(hide_teammates),
                "show_zero" => b!(show_zero),
                "reset_on_death" => b!(reset_on_death),
                "chat_notify" => b!(chat_notify),
                "scale" => {
                    if let Some(v) = number {
                        c.scale = (v as f32).clamp(0.2, 5.0)
                    }
                }
                "y_offset" => {
                    if let Some(v) = number {
                        c.y_offset = v as f32
                    }
                }
                "reset_timeout" => {
                    if let Some(v) = number {
                        c.reset_timeout = v.max(0.0)
                    }
                }
                "max_distance" => {
                    if let Some(v) = number {
                        c.max_distance = v.max(1.0)
                    }
                }
                _ => {}
            }
        }
        c
    }
    pub fn yaml(&self) -> String {
        let quote = |s: &str| serde_json::to_string(s).unwrap_or_default();
        format!(
            "enabled: {}\nlabel: {}\nformat: {}\ncount_prefix: {}\nlabel_color: \"#{:06X}\"\ncount_color: \"#{:06X}\"\ndynamic_color: {}\nscale: {}\ny_offset: {}\nbackground: {}\nshow_self: {}\nhide_teammates: {}\nshow_zero: {}\nreset_on_death: {}\nreset_timeout: {}\nmax_distance: {}\nchat_notify: {}\n",
            self.enabled,
            quote(&self.label),
            quote(&self.format),
            quote(&self.count_prefix),
            self.label_color,
            self.count_color,
            self.dynamic_color,
            self.scale,
            self.y_offset,
            self.background,
            self.show_self,
            self.hide_teammates,
            self.show_zero,
            self.reset_on_death,
            self.reset_timeout,
            self.max_distance,
            self.chat_notify
        )
    }
}
fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escape = false;
    let mut previous_space = true;
    for (i, ch) in line.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if let Some(q) = quote {
            if ch == '\\' && q == '"' {
                escape = true
            } else if ch == q {
                quote = None
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch)
        } else if ch == '#' && previous_space {
            return &line[..i];
        }
        previous_space = ch.is_whitespace();
    }
    line
}
fn unquote(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        let mut out = String::new();
        let mut chars = s[1..s.len() - 1].chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        v => v,
                    })
                }
            } else {
                out.push(c)
            }
        }
        out
    } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].replace("''", "'")
    } else {
        s.into()
    }
}
fn named_color(s: &str) -> Option<u32> {
    let s = s.trim().to_lowercase();
    Some(match s.as_str() {
        "black" => 0,
        "dark_blue" => 0xaa,
        "dark_green" => 0xaa00,
        "dark_aqua" => 0xaaaa,
        "dark_red" => 0xaa0000,
        "dark_purple" => 0xaa00aa,
        "gold" | "orange" => 0xffaa00,
        "gray" | "grey" => 0xaaaaaa,
        "dark_gray" | "dark_grey" => 0x555555,
        "blue" => 0x5555ff,
        "green" => 0x55ff55,
        "aqua" => 0x55ffff,
        "red" => 0xff5555,
        "light_purple" | "pink" => 0xff55ff,
        "yellow" => 0xffff55,
        "white" => 0xffffff,
        _ => {
            let s = s
                .strip_prefix('#')
                .or_else(|| s.strip_prefix("0x"))
                .unwrap_or(&s);
            let s = if s.len() == 3 {
                s.chars().flat_map(|c| [c, c]).collect::<String>()
            } else {
                s.into()
            };
            u32::from_str_radix(&s, 16).ok()? & 0xffffff
        }
    })
}

pub(super) fn load<T: for<'a> Deserialize<'a> + Default>(dir: &Path, name: &str) -> T {
    fs::read(dir.join(name))
        .ok()
        .and_then(|s| serde_json::from_slice(&s).ok())
        .unwrap_or_default()
}
pub(super) fn save<T: Serialize>(dir: &Path, name: &str, value: &T) {
    if let Ok(bytes) = serde_json::to_vec_pretty(value) {
        let _ = fs::create_dir_all(dir).and_then(|()| fs::write(dir.join(name), bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn totem_yaml_preserves_quoted_colors_comments_and_literals() {
        let config = Totem::parse(
            "label: 'O''Brien #7' # a comment\nformat: \"{label}: {count}\"\ncount_prefix: \"\"\nlabel_color: \"#abc\"\ncount_color: orange\nscale: 1,25\nshow_self: yes\nreset_timeout: -5\nmax_distance: 0\n",
        );
        assert_eq!(config.label, "O'Brien #7");
        assert_eq!(config.format, "{label}: {count}");
        assert_eq!(config.count_prefix, "");
        assert_eq!(config.label_color, 0xaabbcc);
        assert_eq!(config.count_color, 0xffaa00);
        assert_eq!(config.scale, 1.25);
        assert!(config.show_self);
        assert_eq!(config.reset_timeout, 0.0);
        assert_eq!(config.max_distance, 1.0);
        let restored = Totem::parse(&config.yaml());
        assert_eq!(restored.label, config.label);
        assert_eq!(restored.label_color, config.label_color);
    }
    #[test]
    fn loot_filters_and_overrides_use_original_precedence() {
        let mut c = Lootbeams::default();
        assert_eq!(c.color("minecraft:diamond", "COMMON"), Some(0x4de6e0));
        assert_eq!(c.color("minecraft:arrow", "EPIC"), None);
        assert_eq!(c.color("minecraft:stone", "EPIC"), Some(0xff55ff));
        c.whitelist_mode = true;
        c.whitelist.push("minecraft:arrow".into());
        assert_eq!(c.color("minecraft:arrow", "UNCOMMON"), Some(0xffff55));
        assert_eq!(c.color("minecraft:diamond", "EPIC"), None);
    }
}
