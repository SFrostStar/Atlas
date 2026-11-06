//! Atlas feature ports, installed per instance through the launcher.
//! These are original Rust implementations, not Java bytecode or third-party source.
mod armor;
mod food;
mod utility;

use super::{Game, native_hooks::NativeMod};
use atlas_client::{
    hud::{HudAtlas, HudBatch},
    text::{Style, Text},
};
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
                    .filter(|id| IDS.contains(id))
                    .map(str::to_owned),
            );
        }
        state
    }
}

fn compact(batch: &mut HudBatch, atlas: &HudAtlas, label: &str, x: f32, y: f32, color: u32) {
    let text = Text::styled(
        label,
        Style {
            font: Some(atlas_client::font::COMPACT.into()),
            ..Default::default()
        },
    );
    batch.text_styled(atlas, &text, x, y, color, true);
}

impl Game<'_> {
    pub(super) fn atlas_port_on(&self, id: &str) -> bool {
        self.atlas_ports.installed.contains(id) && self.native_on(id)
    }

    pub(super) fn atlas_port_mods(&self, mods: &mut Vec<NativeMod>) {
        let definitions = [
            (
                IDS[0],
                "AppleSkin",
                "Atlas food HUD: hunger, saturation and held-food preview.",
            ),
            (
                IDS[1],
                "ArmorIndicator",
                "Atlas: your equipped armor and remaining durability.",
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
                "Atlas: hold C to zoom; rebind the key on this page.",
            ),
            (
                IDS[5],
                "MentionSound",
                "Atlas: a notification when incoming chat mentions your name.",
            ),
        ];
        for (id, name, description) in definitions {
            if !self.atlas_ports.installed.contains(id) {
                continue;
            }
            mods.push(NativeMod {
                id: id.into(),
                title: name.into(),
                description: description.into(),
                keys: if id == IDS[4] {
                    vec![[
                        "zoom".into(),
                        "Приближение / Zoom".into(),
                        "key.keyboard.c".into(),
                    ]]
                } else {
                    vec![]
                },
            });
        }
    }

    pub(super) fn draw_atlas_ports(&self, batch: &mut HudBatch) {
        if !self.native_window() {
            return;
        }
        if self.atlas_port_on(IDS[0]) {
            food::draw(self, batch);
        }
        if self.atlas_port_on(IDS[1]) {
            armor::draw(self, batch);
        }
    }
}
