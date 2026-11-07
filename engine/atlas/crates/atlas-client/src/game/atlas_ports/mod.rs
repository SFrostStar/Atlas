//! Atlas feature ports, installed per instance through the launcher.
//! AppleSkin follows squeek502's Unlicense implementation; other ports are Atlas implementations.
mod actions;
mod armor;
mod food;
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
    armor: armor::State,
    actions: actions::State,
    visual: visual::State,
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
                        IDS.contains(id) || actions::IDS.contains(id) || visual::IDS.contains(id)
                    })
                    .map(str::to_owned),
            );
        }
        if state.installed.contains(IDS[0]) {
            state.food = food::State::load(home);
        }
        state.utility = utility::UtilityState::load(home);
        state.armor = armor::State::load(home);
        state.actions = actions::State::load(home);
        state.visual = visual::State::load(home);
        state
    }
}

impl Game<'_> {
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
                description: description.into(),
                keys: if id == IDS[4] {
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
        self.atlas_actions_viewmodel_hud(batch);
    }
}
