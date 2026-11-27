//! Atlas feature ports, installed per instance through the launcher.
//! AppleSkin follows squeek502's Unlicense implementation; other ports are Atlas implementations.
mod actions;
mod armor;
mod aspect_ratio;
mod config_actions;
mod config_model;
mod config_ui;
mod esc_music;
mod esc_music_ui;
mod food;
mod ias;
mod inventory_hud;
mod macro_keybinds;
mod minimap_port;
mod minimap_settings;
mod minimap_waypoints;
mod mouse_tweaks;
pub(super) mod saturation_plus;
mod totem_tweaks;
mod utility;
mod visual;

use super::{Game, native_hooks::NativeMod};
use atlas_client::hud::HudBatch;
use std::{collections::BTreeSet, fs, path::Path};

const IDS: [&str; 6] = [
    "atlas-appleskin",
    "atlas-armorindicator",
    "atlas-autosprint",
    "atlas-noshakehand",
    "atlas-zoomify",
    "atlas-mentionsound",
];

#[derive(Default)]
pub(super) struct State {
    installed: BTreeSet<String>,
    utility: utility::UtilityState,
    food: food::State,
    ias: ias::State,
    inventory_hud: inventory_hud::State,
    macro_keybinds: macro_keybinds::State,
    minimap: minimap_settings::Settings,
    minimap_runtime: minimap_port::Runtime,
    minimap_waypoints: minimap_waypoints::State,
    armor: armor::State,
    aspect_ratio: aspect_ratio::State,
    esc_music: esc_music::State,
    esc_music_ui: esc_music_ui::State,
    saturation_plus: saturation_plus::State,
    mouse_tweaks: mouse_tweaks::State,
    totem_tweaks: totem_tweaks::State,
    actions: actions::State,
    visual: visual::State,
    config_ui: config_ui::State,
    pub(in crate::game) return_screen: Option<super::Screen>,
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        let mut state = Self::default();
        let file = home.join("atlas-ports.json");
        if fs::metadata(&file).is_ok_and(|m| m.len() <= 16 * 1024)
            && let Ok(bytes) = fs::read(file)
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && value["schema"] == 1
            && let Some(installed) = value["installed"].as_array()
        {
            state.installed.extend(
                installed
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter(|id| {
                        IDS.contains(id)
                            || actions::IDS.contains(id)
                            || visual::IDS.contains(id)
                            || *id == aspect_ratio::ID
                            || *id == esc_music::ID
                            || *id == ias::ID
                            || *id == inventory_hud::ID
                            || *id == macro_keybinds::ID
                            || *id == minimap_settings::ID
                            || *id == saturation_plus::ID
                            || *id == mouse_tweaks::ID
                            || *id == totem_tweaks::ID
                    })
                    .map(str::to_owned),
            );
        }
        if state.installed.contains(IDS[0]) {
            state.food = food::State::load(home);
        }
        if state.installed.contains(ias::ID) {
            state.ias.load_config(home);
        }
        if state.installed.contains(inventory_hud::ID) {
            state.inventory_hud = inventory_hud::State::load(home);
        }
        if state.installed.contains(macro_keybinds::ID) {
            state.macro_keybinds = macro_keybinds::State::load(home);
        }
        if state.installed.contains(minimap_settings::ID) {
            state.minimap = minimap_settings::Settings::load(home);
            state.minimap_waypoints = minimap_waypoints::State::load(home);
        }
        if state.installed.contains(saturation_plus::ID) {
            state.saturation_plus = saturation_plus::State::load(home);
        }
        if state.installed.contains(mouse_tweaks::ID) {
            state.mouse_tweaks = mouse_tweaks::State::load(home);
        }
        if state.installed.contains(totem_tweaks::ID) {
            state.totem_tweaks = totem_tweaks::State::load(home);
        }
        state.utility = utility::UtilityState::load(home);
        state.armor = armor::State::load(home);
        state.actions = actions::State::load(home);
        state.visual = visual::State::load(home);
        state
    }
}

impl Game<'_> {
    pub(super) fn atlas_saturation_plus_enabled(&self) -> bool {
        self.atlas_port_on(saturation_plus::ID)
    }

    pub(in crate::game) fn atlas_config_value(&self, id: &str) -> Option<serde_json::Value> {
        match id {
            ias::ID => Some(self.atlas_ports.ias.config_value()),
            inventory_hud::ID => Some(self.atlas_ports.inventory_hud.config_value()),
            minimap_settings::ID => Some(self.atlas_ports.minimap.config_value()),
            "atlas-appleskin" => Some(self.atlas_ports.food.config_value()),
            "atlas-zoomify" => self.atlas_utility_config(id),
            totem_tweaks::ID => Some(self.atlas_ports.totem_tweaks.config_value()),
            "atlas-chunkanimator" | "atlas-gammautils" => self.atlas_actions_config(id),
            _ => None,
        }
    }
    pub(in crate::game) fn atlas_config_default(&self, id: &str) -> Option<serde_json::Value> {
        match id {
            ias::ID => Some(ias::State::config_default()),
            inventory_hud::ID => Some(inventory_hud::State::default_config()),
            minimap_settings::ID => Some(minimap_settings::Settings::default_config()),
            "atlas-appleskin" => Some(food::State::default_config()),
            "atlas-zoomify" => serde_json::to_value(utility::config::Zoom::default()).ok(),
            totem_tweaks::ID => Some(totem_tweaks::State::default_config()),
            _ => self.atlas_actions_default_config(id),
        }
    }
    pub(in crate::game) fn atlas_config_apply(
        &mut self,
        id: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        match id {
            ias::ID => self.atlas_ports.ias.apply_config(&Self::game_dir(), value),
            inventory_hud::ID => self
                .atlas_ports
                .inventory_hud
                .set_config(&Self::game_dir(), value),
            minimap_settings::ID => self
                .atlas_ports
                .minimap
                .set_config(&Self::game_dir(), value),
            "atlas-appleskin" => self.atlas_ports.food.set_config(&Self::game_dir(), value),
            "atlas-zoomify" => self.atlas_utility_set_config(id, value),
            totem_tweaks::ID => self
                .atlas_ports
                .totem_tweaks
                .set_config(&Self::game_dir(), value),
            _ => self.atlas_actions_set_config(id, value),
        }
    }
    pub(in crate::game) fn atlas_open_original_mod_config(&mut self, id: &str) -> bool {
        match id {
            mouse_tweaks::ID => self.atlas_mouse_tweaks_open(),
            macro_keybinds::ID => self.atlas_macros_open(),
            ias::ID
            | "atlas-appleskin"
            | "atlas-chunkanimator"
            | "atlas-zoomify"
            | "atlas-gammautils"
            | totem_tweaks::ID
            | inventory_hud::ID
            | minimap_settings::ID => self.atlas_config_open(id),
            "atlas-armorindicator" | "atlas-radarmod" => self.atlas_original_open_screen(id),
            _ => false,
        }
    }
    /// Port-owned configuration can open from the title's Options as well as from a world.
    pub(in crate::game) fn atlas_open_port_screen(&mut self, id: &str) -> bool {
        if !self.atlas_ports.installed.contains(id)
            || self.renderer.is_none()
            || self.remote_loading()
        {
            return false;
        }
        if self.screen == super::Screen::Mod {
            return self.native_mod_screen() == Some(id);
        }
        self.atlas_ports.return_screen = Some(self.screen);
        self.native_mod_screen = Some(id.to_owned());
        self.screen = super::Screen::Mod;
        self.keys.clear();
        self.left = false;
        self.right = false;
        self.mining = None;
        self.capture(false);
        true
    }
    pub(super) fn atlas_port_on(&self, id: &str) -> bool {
        self.atlas_ports.installed.contains(id) && self.native_on(id)
    }

    pub(super) fn atlas_port_mods(&self, mods: &mut Vec<NativeMod>) {
        let definitions = [
            (
                IDS[0],
                "AppleSkin",
                "AppleSkin 3.0.10: saturation, food/health previews and food tooltips.",
            ),
            (
                IDS[1],
                "ArmorIndicator",
                "ArmorIndicator 1.0.0: target armor, durability and damage tint.",
            ),
            (
                IDS[2],
                "AutoSprint",
                "Atlas: hold forward to request normal vanilla sprinting.",
            ),
            (
                IDS[3],
                "NoShakeHand",
                "Atlas: remove inertial hand sway while turning the camera.",
            ),
            (
                IDS[4],
                "Zoomify",
                "Zoomify: hold C and use the wheel for geometric zoom steps.",
            ),
            (
                IDS[5],
                "MentionSound",
                "Atlas: a notification when incoming chat mentions your name.",
            ),
            (
                aspect_ratio::ID,
                "Aspect Ratio",
                "Adjust the game's aspect ratio in real time with a slider and presets.",
            ),
            (
                esc_music::ID,
                "ESC Music Controller",
                "Control the current Windows media session from the pause menu.",
            ),
            (
                ias::ID,
                "In-Game Account Switcher",
                "Choose and manage saved Atlas accounts from the title screen and server list.",
            ),
            (
                saturation_plus::ID,
                "Saturation Plus",
                "Expanded saturation and exhaustion rules, commands, and HUD.",
            ),
            (
                mouse_tweaks::ID,
                "Mouse Tweaks",
                "Drag and scroll to move items in inventory screens.",
            ),
            (
                totem_tweaks::ID,
                "Totem Tweaks",
                "Customize held totems and their activation animation.",
            ),
            (
                inventory_hud::ID,
                "Inventory HUD +",
                "atlas.inventory_hud.description",
            ),
            (
                macro_keybinds::ID,
                "Macro Keybinds",
                "atlas.macro_keybinds.description",
            ),
            (
                minimap_settings::ID,
                "Atlas Minimap",
                "atlas.minimap.description",
            ),
        ];
        for (id, name, description) in definitions
            .into_iter()
            .chain(visual::DEFINITIONS)
            .chain(actions::DEFINITIONS)
        {
            if !self.atlas_ports.installed.contains(id) {
                continue;
            }
            mods.push(NativeMod {
                id: id.into(),
                title: name.into(),
                description: if matches!(
                    id,
                    inventory_hud::ID | macro_keybinds::ID | minimap_settings::ID
                ) {
                    atlas_client::lang::get(description)
                } else {
                    description.into()
                },
                keys: if matches!(
                    id,
                    esc_music::ID
                        | ias::ID
                        | saturation_plus::ID
                        | mouse_tweaks::ID
                        | totem_tweaks::ID
                ) {
                    Vec::new()
                } else if id == IDS[1] {
                    [
                        (
                            "toggle_hud",
                            "key.armorindicator.toggle_hud",
                            "key.keyboard.h",
                        ),
                        (
                            "toggle_tint",
                            "key.armorindicator.toggle_tint",
                            "key.keyboard.j",
                        ),
                        (
                            "open_config",
                            "key.armorindicator.open_config",
                            "key.keyboard.k",
                        ),
                    ]
                    .into_iter()
                    .map(|(name, label, key)| {
                        [name.into(), atlas_client::lang::get(label), key.into()]
                    })
                    .collect()
                } else if id == IDS[4] {
                    vec![
                        [
                            "zoom".into(),
                            "Приближение / Zoom".into(),
                            "key.keyboard.c".into(),
                        ],
                        [
                            "secondary".into(),
                            "Secondary zoom".into(),
                            "key.keyboard.unknown".into(),
                        ],
                        [
                            "zoom_in".into(),
                            "Zoom in".into(),
                            "key.keyboard.unknown".into(),
                        ],
                        [
                            "zoom_out".into(),
                            "Zoom out".into(),
                            "key.keyboard.unknown".into(),
                        ],
                    ]
                } else if id == IDS[3] {
                    vec![[
                        "toggle".into(),
                        "Toggle camera sway".into(),
                        "key.keyboard.unknown".into(),
                    ]]
                } else if id == aspect_ratio::ID {
                    aspect_ratio::keys()
                } else if id == inventory_hud::ID {
                    inventory_hud::keys()
                } else if id == macro_keybinds::ID {
                    macro_keybinds::keys()
                } else if id == minimap_settings::ID {
                    minimap_settings::keys()
                } else {
                    if visual::IDS.contains(&id) {
                        visual::keys(id)
                    } else {
                        actions::keys(id)
                    }
                },
            });
        }
    }

    pub(super) fn draw_atlas_ports(&self, batch: &mut HudBatch) {
        if !self.native_window() {
            return;
        }
        if self.atlas_port_on(IDS[1]) {
            armor::draw(self, batch);
        }
        self.atlas_visual_hud(batch);
        self.draw_inventory_hud(batch);
    }
}
