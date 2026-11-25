//! Native adapters of the exact supplied mod versions, not similarly named replacements.
//! MIT: Radar / slmnk; Crystal Optimizer / Author; TotemPops / you.
//! CC0: NoFluid / slmnk, PvPSurvived / Slmnk. Original notices are retained in the project's mod-port notices.
//! Java entry points/mixins are adapted to Atlas client hooks; no Java bytecode is shipped.
#[path = "visual_commands.rs"]
mod commands;
#[path = "visual_config.rs"]
pub(in crate::game) mod config;
#[path = "visual_logic.rs"]
pub(in crate::game) mod logic;
#[path = "visual_ui.rs"]
mod ui;
#[path = "visual_world.rs"]
mod world;

use super::super::{Game, native_hooks::NativeMod};
use atlas_client::{
    hud::{HudBatch, rgba},
    keybinds,
    text::{Style, Text},
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const RADAR: &str = "atlas-radarmod";
const CRYSTAL_OPTIMIZER: &str = "atlas-crystaloptimizer";
const NO_FLUID: &str = "atlas-nofluid";
const PVP_SURVIVED: &str = "atlas-pvpsurvived";
const TOTEM_POPS: &str = "atlas-totempops";
pub(in crate::game) const IDS: [&str; 5] = [
    RADAR, CRYSTAL_OPTIMIZER, NO_FLUID, PVP_SURVIVED, TOTEM_POPS,
];
pub(super) const DEFINITIONS: [(&str, &str, &str); 5] = [
    (
        RADAR,
        "Radar",
        "Radar 1.0.0: nearby players, health, distance and direction; R opens settings.",
    ),
    (
        CRYSTAL_OPTIMIZER,
        "Crystal Optimizer",
        "Crystal Optimizer 1.0.0: remove attacked crystals immediately and clear crystal interaction delay.",
    ),
    (
        NO_FLUID,
        "NoFluid",
        "NoFluid 1.0.0: configurable fluid fog, overlays and underwater visibility; H toggles.",
    ),
    (
        PVP_SURVIVED,
        "PvPSurvived",
        "PvPSurvived 1.0.0: combat-end message and sound from server boss bars and chat.",
    ),
    (
        TOTEM_POPS,
        "TotemPops",
        "TotemPops 1.0.0: colored pop counters under player names and live YAML settings.",
    ),
];

pub(super) fn keys(id: &str) -> Vec<[String; 3]> {
    match id {
        "atlas-radarmod" => vec![[
            "open_config".into(),
            "Настройки радара / Radar settings".into(),
            "key.keyboard.r".into(),
        ]],
        "atlas-nofluid" => vec![[
            "toggle".into(),
            "Переключить / Toggle".into(),
            "key.keyboard.h".into(),
        ]],
        _ => Vec::new(),
    }
}

pub(in crate::game) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[derive(Clone)]
struct RadarRow {
    name: String,
    hp: i32,
    distance: i32,
    arrow: logic::RadarTurn,
    skin: String,
    uuid: u128,
}
pub(super) struct State {
    pub(super) dir: PathBuf,
    pub(super) radar: config::Radar,
    pub(super) nofluid: config::NoFluid,
    pub(super) pvp: config::Pvp,
    pub(super) totem: config::Totem,
    pub(super) pops: logic::Pops,
    pub(super) pvp_state: logic::PvpState,
    pub(super) open_settings: Option<String>,
    radar_rows: Vec<RadarRow>,
    radar_count: usize,
    key_down: [bool; 2],
    totem_modified: Option<SystemTime>,
    ui: ui::State,
    materialized: bool,
}
impl Default for State {
    fn default() -> Self {
        Self::load(Path::new("."))
    }
}
impl State {
    pub(super) fn load(home: &Path) -> Self {
        let dir = home.join("config");
        let mut radar: config::Radar = config::load(&dir, "radarmod.json");
        radar.validate();
        let totem_file = dir.join("totempops/config.yml");
        let totem = fs::read_to_string(&totem_file)
            .map(|s| config::Totem::parse(&s))
            .unwrap_or_default();
        Self {
            radar,
            nofluid: config::load(&dir, "nofluid.json"),
            pvp: config::load(&dir, "pvpsurvived.json"),
            totem,
            dir,
            pops: Default::default(),
            pvp_state: Default::default(),
            open_settings: None,
            radar_rows: vec![],
            radar_count: 0,
            key_down: [false; 2],
            totem_modified: fs::metadata(totem_file).and_then(|m| m.modified()).ok(),
            ui: Default::default(),
            materialized: false,
        }
    }
    fn save_totem(&self) {
        let path = self.dir.join("totempops/config.yml");
        let _ = fs::create_dir_all(path.parent().unwrap())
            .and_then(|()| fs::write(path, self.totem.yaml()));
    }
}

impl Game<'_> {
    pub(in crate::game) fn atlas_visual_mods(&self, mods: &mut Vec<NativeMod>) {
        for (id, title, description) in DEFINITIONS {
            if self.atlas_ports.installed.contains(id) {
                mods.push(NativeMod {
                    id: id.into(),
                    title: title.into(),
                    description: description.into(),
                    keys: match id {
                        "atlas-radarmod" => vec![[
                            "open_config".into(),
                            "Настройки радара / Radar settings".into(),
                            "key.keyboard.r".into(),
                        ]],
                        "atlas-nofluid" => vec![[
                            "toggle".into(),
                            "Переключить / Toggle".into(),
                            "key.keyboard.h".into(),
                        ]],
                        _ => vec![],
                    },
                })
            }
        }
    }
    pub(in crate::game) fn atlas_visual_take_settings_request(&mut self) -> Option<String> {
        self.atlas_ports.visual.open_settings.take()
    }
    pub(in crate::game) fn atlas_visual_no_fluid(&self) -> Option<&config::NoFluid> {
        let c = &self.atlas_ports.visual.nofluid;
        (self.atlas_port_on(NO_FLUID) && c.enabled).then_some(c)
    }
    pub(in crate::game) fn atlas_visual_crystal(&self) -> bool {
        self.native_window() && self.atlas_port_on(CRYSTAL_OPTIMIZER)
    }
    pub(in crate::game) fn atlas_visual_disconnect(&mut self) {
        let v = &mut self.atlas_ports.visual;
        v.pops = Default::default();
        v.pvp_state.was_in_pvp = false;
        v.pvp_state.duration_ticks = 0;
        v.pvp_state.display_start = None;
        v.radar_rows.clear();
        v.radar_count = 0;
    }
    pub(in crate::game) fn atlas_visual_tick(&mut self) {
        if !self.native_window() {
            return;
        }
        // Like the original mod initializers, create defaults only when a file is absent.
        // Existing malformed files are preserved for the user's inspection and repair.
        if !self.atlas_ports.visual.materialized {
            self.atlas_ports.visual.materialized = true;
            let v = &self.atlas_ports.visual;
            for (id, name) in [
                (RADAR, "radarmod.json"),
                (NO_FLUID, "nofluid.json"),
                (PVP_SURVIVED, "pvpsurvived.json"),
            ] {
                if self.atlas_ports.installed.contains(id)
                    && !v.dir.join(name).exists()
                    && let Some(value) = self.atlas_visual_config(id)
                {
                    config::save(&v.dir, name, &value);
                }
            }
            if self.atlas_ports.installed.contains(TOTEM_POPS)
                && !v.dir.join("totempops/config.yml").exists()
            {
                v.save_totem();
            }
        }
        let now = now_ms();
        for (index, (id, name, default)) in [
            (RADAR, "open_config", "key.keyboard.r"),
            (NO_FLUID, "toggle", "key.keyboard.h"),
        ]
        .into_iter()
        .enumerate()
        {
            let key = self.native_mod_key(id, name, default);
            let down = self.focused
                && self.native_playing()
                && keybinds::key_code(&key).is_some_and(|k| self.physical_keys.contains(&k));
            let edge = down && !self.atlas_ports.visual.key_down[index];
            self.atlas_ports.visual.key_down[index] = down;
            if edge && self.atlas_port_on(id) {
                match index {
                    0 => self.atlas_ports.visual.open_settings = Some(id.into()),
                    1 => {
                        self.atlas_ports.visual.nofluid.enabled =
                            !self.atlas_ports.visual.nofluid.enabled;
                        config::save(
                            &self.atlas_ports.visual.dir,
                            "nofluid.json",
                            &self.atlas_ports.visual.nofluid,
                        );
                        self.titles.set_overlay(
                            Text::legacy(&format!(
                                "§b[NoFluid] §fМод {}",
                                if self.atlas_ports.visual.nofluid.enabled {
                                    "§aВКЛЮЧЕН"
                                } else {
                                    "§cВЫКЛЮЧЕН"
                                }
                            )),
                            false,
                        );
                    }
                    _ => {}
                }
            }
        }
        if let Some(id) = self.atlas_visual_take_settings_request() {
            self.atlas_visual_open_editor(&id);
        }
        if self.player.is_none() {
            self.atlas_ports.visual.radar_rows.clear();
            self.atlas_ports.visual.radar_count = 0;
            return;
        }
        let names = self.remote_player_names();
        if self.atlas_port_on(RADAR) {
            let pos = self.player.as_ref().unwrap().position;
            let clock = self.hud_clock();
            let cfg = &self.atlas_ports.visual.radar;
            let mut rows: Vec<_> = names
                .iter()
                .filter_map(|(id, name)| {
                    let m = self
                        .mobs
                        .iter()
                        .find(|m| m.id == *id && !m.removed && m.health > 0.0)?;
                    let delta = m.position.subtract(pos);
                    let distance = delta.length_squared();
                    if distance > cfg.max_distance * cfg.max_distance {
                        return None;
                    }
                    let uuid = self.remote_player_uuid(*id)?;
                    let angle = logic::direction(delta.x, delta.z, self.yaw);
                    let arrow = self.atlas_ports.visual.radar_rows.iter().find(|row| row.uuid == uuid)
                        .map_or_else(|| logic::RadarTurn::new(angle, clock), |row| row.arrow.toward(angle, clock));
                    Some((
                        distance,
                        RadarRow {
                            name: name.clone(),
                            hp: (m.health + m.absorption).ceil().max(0.0) as i32,
                            distance: (distance.sqrt() as f32).round() as i32,
                            arrow,
                            skin: m.skin.clone().unwrap_or_else(|| {
                                atlas_client::tab_list::default_skin(uuid).into()
                            }),
                            uuid,
                        },
                    ))
                })
                .collect();
            let count = rows.len();
            rows.sort_by(|a, b| a.0.total_cmp(&b.0));
            rows.truncate(cfg.max_players);
            for (_, row) in &mut rows {
                row.skin = self.face_skin(row.uuid, &row.skin)
            }
            self.atlas_ports.visual.radar_rows = rows.into_iter().map(|(_, r)| r).collect();
            self.atlas_ports.visual.radar_count = count;
        }
        if self.atlas_port_on(TOTEM_POPS) {
            let v = &mut self.atlas_ports.visual;
            v.pops.tick += 1;
            if v.pops.tick % 40 == 0 {
                let path = v.dir.join("totempops/config.yml");
                let modified = fs::metadata(&path).and_then(|m| m.modified()).ok();
                if modified.is_some() && modified != v.totem_modified {
                    v.totem = fs::read_to_string(path)
                        .map(|s| config::Totem::parse(&s))
                        .unwrap_or_default();
                    v.totem_modified = modified;
                }
            }
            let max = self.atlas_ports.visual.totem.max_distance.powi(2);
            let pos = self.player.as_ref().unwrap().position;
            for (id, _) in &names {
                if let Some(uuid) = self.remote_player_uuid(*id) {
                    if let Some(m) = self.mobs.iter().find(|m| m.id == *id) {
                        let v = &mut self.atlas_ports.visual;
                        if v.totem.reset_on_death && (m.health <= 0.0 || m.death_time > 0) {
                            v.pops.entries.remove(&uuid);
                        } else if m.hurt_time > 0 && m.position.distance_squared(pos) <= max {
                            v.pops.touch(uuid)
                        }
                    }
                }
            }
            let me = self.atlas_visual_own_uuid();
            if self.atlas_ports.visual.totem.reset_on_death
                && self
                    .player
                    .as_ref()
                    .is_some_and(|p| p.health.dead_or_dying())
            {
                self.atlas_ports.visual.pops.entries.remove(&me);
            }
            let timeout = self.atlas_ports.visual.totem.reset_timeout;
            if timeout > 0.0 {
                self.atlas_ports.visual.pops.expire((timeout * 20.0) as u64)
            }
        }
        if self.atlas_port_on(PVP_SURVIVED) && self.atlas_ports.visual.pvp.enabled {
            if self
                .player
                .as_ref()
                .is_some_and(|p| p.health.dead_or_dying())
            {
                {
                    let v = &mut self.atlas_ports.visual.pvp_state;
                    v.was_in_pvp = false;
                    v.duration_ticks = 0;
                    v.display_start = None;
                }
            } else {
                let titles = self
                    .boss_overlay
                    .bars(now as u128)
                    .into_iter()
                    .map(|b| b.name.plain())
                    .collect::<Vec<_>>();
                let v = &mut self.atlas_ports.visual;
                if v.pvp_state.bosses(&titles, &v.pvp.custom_keywords, now) {
                    self.atlas_visual_pvp_sound()
                }
            }
        }
    }
    pub(in crate::game) fn atlas_visual_own_uuid(&self) -> u128 {
        if let Some(remote) = &self.remote {
            return remote.profile();
        }
        self.level.player_uuid.map_or(0, |u| {
            u.iter()
                .fold(0u128, |a, &w| (a << 32) | u128::from(w as u32))
        })
    }
    pub(in crate::game) fn atlas_visual_entity_status(&mut self, id: u32, status: u8) {
        if status != 35 || !self.atlas_port_on(TOTEM_POPS) {
            return;
        }
        let own = self
            .remote
            .as_ref()
            .is_some_and(|r| r.entity_id as u32 == id);
        let name = if own {
            self.options.player_name()
        } else {
            self.remote_player_name(id).unwrap_or("").into()
        };
        if name.is_empty() {
            return;
        }
        let uuid = if own {
            self.atlas_visual_own_uuid()
        } else {
            self.remote_player_uuid(id).unwrap_or(id as u128)
        };
        self.atlas_visual_totem(uuid, &name, own);
    }
    pub(in crate::game) fn atlas_visual_totem(&mut self, uuid: u128, name: &str, is_self: bool) {
        if !self.atlas_port_on(TOTEM_POPS) {
            return;
        }
        let count = self.atlas_ports.visual.pops.pop(uuid);
        if self.atlas_ports.visual.totem.chat_notify && !is_self {
            self.native_chat(&format!(
                "§6[TotemPops] §f{name} §7потерял тотем §c(-{count})"
            ))
        }
    }
    fn atlas_visual_pvp_sound(&mut self) {
        let c = &self.atlas_ports.visual.pvp;
        if c.play_sound {
            self.audio.play(
                "minecraft:entity.player.levelup",
                None,
                c.sound_volume,
                c.sound_pitch,
            )
        }
    }
    pub(in crate::game) fn atlas_visual_overlay(&mut self, message: &str) {
        let now = now_ms();
        if self.atlas_port_on(PVP_SURVIVED)
            && self.atlas_ports.visual.pvp.enabled
            && self
                .player
                .as_ref()
                .is_some_and(|p| !p.health.dead_or_dying())
            && logic::pvp_end(message, &self.atlas_ports.visual.pvp.custom_keywords)
        {
            let v = &mut self.atlas_ports.visual;
            v.pvp_state.was_in_pvp = false;
            v.pvp_state.duration_ticks = 0;
            if v.pvp_state.trigger(now) {
                self.atlas_visual_pvp_sound()
            }
        }
    }
    pub(in crate::game) fn atlas_visual_hud(&self, batch: &mut HudBatch) {
        if !self.native_window() || self.player.is_none() || self.ui.hide_gui {
            return;
        }
        if self.atlas_port_on(RADAR) && self.atlas_ports.visual.radar.enabled {
            self.atlas_visual_radar_draw(batch)
        }
        if self.atlas_port_on(PVP_SURVIVED) && self.atlas_ports.visual.pvp.enabled {
            let c = &self.atlas_ports.visual.pvp;
            if let Some(alpha) =
                self.atlas_ports
                    .visual
                    .pvp_state
                    .alpha(now_ms(), c.duration_ms, c.fade_effect)
            {
                let start = batch.vertices.len();
                let text = Text::styled(
                    &c.text,
                    Style {
                        bold: Some(c.bold),
                        ..Default::default()
                    },
                );
                let x = batch.width / 2.0;
                let y = batch.height / 2.0 + c.offset_y as f32;
                let rgb = config::hex(c.color_hex.trim_start_matches("0x")).unwrap_or(0x00ff55);
                let color = argb((alpha as u32) << 24 | rgb);
                let width = self.gui.atlas.font.width_text(&text);
                batch.text_styled(&self.gui.atlas, &text, x - width / 2.0, y, color, true);
                batch.scale_since(start, x, y, c.scale, c.scale);
            }
        }
    }
}

fn argb(v: u32) -> u32 {
    rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8)
}
