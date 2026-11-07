//! Native adapters of the exact supplied mod versions, not similarly named replacements.
//! MIT: Radar / slmnk; Crystal Optimizer / Author; FriendList / slmnk;
//! KillNotification / Slmnk; RwJoiner / RwJoiner Team; TotemPops / you;
//! CullParticles / quikjay. CC0: Lootbeams / you, NoFluid / slmnk,
//! PvPSurvived / Slmnk. Original notices are retained in the project's mod-port notices.
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
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(in crate::game) const IDS: [&str; 10] = [
    "atlas-radarmod",
    "atlas-crystaloptimizer",
    "atlas-friendlist",
    "atlas-killnotification",
    "atlas-lootbeams",
    "atlas-nofluid",
    "atlas-pvpsurvived",
    "atlas-rwjoiner",
    "atlas-totempops",
    "atlas-cullparticles",
];
pub(super) const DEFINITIONS: [(&str, &str, &str); 10] = [
    (
        IDS[0],
        "Radar",
        "Radar 1.0.0: nearby players, health, distance and direction; R opens settings.",
    ),
    (
        IDS[1],
        "Crystal Optimizer",
        "Crystal Optimizer 1.0.0: remove attacked crystals immediately and clear crystal interaction delay.",
    ),
    (
        IDS[2],
        "FriendList",
        "FriendList 1.0.0: /friend commands, translucent friends and attack protection.",
    ),
    (
        IDS[3],
        "KillNotification",
        "KillNotification 1.0.0: send the original !-<player> chat notification after a kill.",
    ),
    (
        IDS[4],
        "Lootbeams",
        "Lootbeams 1.1.0: rarity silhouettes and rotating gradient loot beams.",
    ),
    (
        IDS[5],
        "NoFluid",
        "NoFluid 1.0.0: configurable fluid fog, overlays and underwater visibility; H toggles.",
    ),
    (
        IDS[6],
        "PvPSurvived",
        "PvPSurvived 1.0.0: combat-end message and sound from server boss bars and chat.",
    ),
    (
        IDS[7],
        "RwJoiner",
        "RwJoiner 1.0.0: the original ReallyWorld Grief menu automation; J toggles, /rwjoin help.",
    ),
    (
        IDS[8],
        "TotemPops",
        "TotemPops 1.0.0: colored pop counters under player names and live YAML settings.",
    ),
    (
        IDS[9],
        "CullParticles",
        "CullParticles 1.21.4_1.0: the supplied JAR disables push-out-of-blocks; it does not cull particles.",
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
        "atlas-rwjoiner" => vec![[
            "toggle".into(),
            "Автозаход / Auto join".into(),
            "key.keyboard.j".into(),
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
    arrow: &'static str,
    skin: String,
    uuid: u128,
}
pub(super) struct State {
    pub(super) dir: PathBuf,
    pub(super) radar: config::Radar,
    pub(super) nofluid: config::NoFluid,
    pub(super) pvp: config::Pvp,
    pub(super) rw: config::RwJoiner,
    pub(super) loot: config::Lootbeams,
    pub(super) kill: config::Kill,
    pub(super) totem: config::Totem,
    pub(super) friends: BTreeSet<String>,
    pub(super) pops: logic::Pops,
    pub(super) kills: logic::Kills,
    pub(super) pvp_state: logic::PvpState,
    pub(super) joiner: logic::Joiner,
    pub(super) open_settings: Option<String>,
    radar_rows: Vec<RadarRow>,
    key_down: [bool; 3],
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
        let friends = fs::read_to_string(dir.join("friendlist.txt"))
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect();
        Self {
            radar,
            nofluid: config::load(&dir, "nofluid.json"),
            pvp: config::load(&dir, "pvpsurvived.json"),
            rw: config::load(&dir, "rwjoiner.json"),
            loot: config::load(&dir, "lootbeams.json"),
            kill: config::load(&dir, "killnotification.json"),
            totem,
            friends,
            dir,
            pops: Default::default(),
            kills: Default::default(),
            pvp_state: Default::default(),
            joiner: Default::default(),
            open_settings: None,
            radar_rows: vec![],
            key_down: [false; 3],
            totem_modified: fs::metadata(totem_file).and_then(|m| m.modified()).ok(),
            ui: Default::default(),
            materialized: false,
        }
    }
    fn save_friends(&self) {
        let _ = fs::create_dir_all(&self.dir).and_then(|()| {
            fs::write(
                self.dir.join("friendlist.txt"),
                self.friends.iter().cloned().collect::<Vec<_>>().join("\n"),
            )
        });
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
                        "atlas-rwjoiner" => vec![[
                            "toggle".into(),
                            "Автозаход / Auto join".into(),
                            "key.keyboard.j".into(),
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
    pub(in crate::game) fn atlas_visual_friend(&self, name: &str) -> bool {
        self.atlas_port_on(IDS[2])
            && self
                .atlas_ports
                .visual
                .friends
                .contains(&name.to_lowercase())
    }
    pub(in crate::game) fn atlas_visual_no_fluid(&self) -> Option<&config::NoFluid> {
        let c = &self.atlas_ports.visual.nofluid;
        (self.atlas_port_on(IDS[5]) && c.enabled).then_some(c)
    }
    pub(in crate::game) fn atlas_visual_crystal(&self) -> bool {
        self.native_window() && self.atlas_port_on(IDS[1])
    }
    /// This is the actual sole mixin in the provided CullParticles JAR, verified against Yarn.
    pub(in crate::game) fn atlas_visual_no_push_out(&self) -> bool {
        self.native_window() && self.atlas_port_on(IDS[9])
    }
    pub(in crate::game) fn atlas_visual_disconnect(&mut self) {
        let v = &mut self.atlas_ports.visual;
        v.pops = Default::default();
        v.pvp_state.was_in_pvp = false;
        v.pvp_state.duration_ticks = 0;
        v.pvp_state.display_start = None;
        v.radar_rows.clear();
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
                (IDS[0], "radarmod.json"),
                (IDS[3], "killnotification.json"),
                (IDS[4], "lootbeams.json"),
                (IDS[5], "nofluid.json"),
                (IDS[6], "pvpsurvived.json"),
                (IDS[7], "rwjoiner.json"),
            ] {
                if self.atlas_ports.installed.contains(id)
                    && !v.dir.join(name).exists()
                    && let Some(value) = self.atlas_visual_config(id)
                {
                    config::save(&v.dir, name, &value);
                }
            }
            if self.atlas_ports.installed.contains(IDS[8])
                && !v.dir.join("totempops/config.yml").exists()
            {
                v.save_totem();
            }
            if self.atlas_ports.installed.contains(IDS[2]) && !v.dir.join("friendlist.txt").exists()
            {
                v.save_friends();
            }
        }
        let now = now_ms();
        for (index, (id, name, default)) in [
            (IDS[0], "open_config", "key.keyboard.r"),
            (IDS[5], "toggle", "key.keyboard.h"),
            (IDS[7], "toggle", "key.keyboard.j"),
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
                    _ => self.atlas_visual_rw_enabled(!self.atlas_ports.visual.rw.enabled),
                }
            }
        }
        if let Some(id) = self.atlas_visual_take_settings_request() {
            self.atlas_visual_open_editor(&id);
        }
        if self.player.is_none() {
            self.atlas_ports.visual.radar_rows.clear();
            return;
        }
        let names = self.remote_player_names();
        if self.atlas_port_on(IDS[0]) {
            let pos = self.player.as_ref().unwrap().position;
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
                    Some((
                        distance,
                        RadarRow {
                            name: name.clone(),
                            hp: (m.health + m.absorption).ceil().max(0.0) as i32,
                            distance: (distance.sqrt() as f32).round() as i32,
                            arrow: logic::direction(delta.x, delta.z, self.yaw),
                            skin: m.skin.clone().unwrap_or_else(|| {
                                atlas_client::tab_list::default_skin(uuid).into()
                            }),
                            uuid,
                        },
                    ))
                })
                .collect();
            rows.sort_by(|a, b| a.0.total_cmp(&b.0));
            rows.truncate(cfg.max_players);
            for (_, row) in &mut rows {
                row.skin = self.face_skin(row.uuid, &row.skin)
            }
            self.atlas_ports.visual.radar_rows = rows.into_iter().map(|(_, r)| r).collect();
        }
        if self.atlas_port_on(IDS[8]) {
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
        if self.atlas_port_on(IDS[3]) {
            let local = self.own_profile_name();
            let enabled = self.atlas_ports.visual.kill.enabled;
            for (id, name) in &names {
                if let Some(m) = self.mobs.iter().find(|m| m.id == *id && m.health <= 0.0) {
                    let _ = m;
                    let uuid = self.remote_player_uuid(*id).unwrap_or(*id as u128);
                    self.atlas_ports
                        .visual
                        .kills
                        .death(uuid, name, now, &local, enabled)
                }
            }
            if let Some(msg) = self
                .atlas_ports
                .visual
                .kills
                .tick(now, self.remote.is_some())
            {
                self.send_chat(&msg)
            }
        }
        if self.atlas_port_on(IDS[6]) && self.atlas_ports.visual.pvp.enabled {
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
        if self.atlas_port_on(IDS[7]) {
            self.atlas_visual_rw_tick(now)
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
    pub(in crate::game) fn atlas_visual_attacked(&mut self, id: u32) {
        if self.atlas_port_on(IDS[3])
            && let Some(name) = self.remote_player_name(id).map(str::to_owned)
        {
            let uuid = self.remote_player_uuid(id).unwrap_or(id as u128);
            self.atlas_ports.visual.kills.attack(uuid, &name, now_ms())
        }
    }
    pub(in crate::game) fn atlas_visual_entity_status(&mut self, id: u32, status: u8) {
        if status != 35 || !self.atlas_port_on(IDS[8]) {
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
        if !self.atlas_port_on(IDS[8]) {
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
    pub(in crate::game) fn atlas_visual_chat(&mut self, message: &str) {
        let now = now_ms();
        if self.atlas_port_on(IDS[3]) {
            let local = self.own_profile_name();
            let enabled = self.atlas_ports.visual.kill.enabled;
            self.atlas_ports
                .visual
                .kills
                .chat(message, &local, now, enabled)
        }
        self.atlas_visual_overlay(message);
    }
    /// Only PvPSurvived consumes actionbar/overlay messages; KillNotification excludes them.
    pub(in crate::game) fn atlas_visual_overlay(&mut self, message: &str) {
        let now = now_ms();
        if self.atlas_port_on(IDS[6])
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
        if self.atlas_port_on(IDS[0]) && self.atlas_ports.visual.radar.enabled {
            self.atlas_visual_radar_draw(batch)
        }
        if self.atlas_port_on(IDS[6]) && self.atlas_ports.visual.pvp.enabled {
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
        if self.atlas_port_on(IDS[7]) && self.atlas_ports.visual.rw.show_hud {
            self.atlas_visual_rw_draw(batch)
        }
    }
    fn atlas_visual_radar_draw(&self, batch: &mut HudBatch) {
        let c = &self.atlas_ports.visual.radar;
        let rows = &self.atlas_ports.visual.radar_rows;
        let atlas = &self.gui.atlas;
        let start = batch.vertices.len();
        let (x, y, w, h) = (
            c.x as f32,
            c.y as f32,
            c.hud_width as f32,
            if rows.is_empty() {
                40.0
            } else {
                26.0 + rows.len() as f32 * 24.0
            },
        );
        let alpha = (c.background_opacity * 255.0) as u8;
        let bg = rgba(17, 17, 24, alpha);
        let outline = rgba(43, 43, 58, alpha.saturating_add(40));
        batch.fill(atlas, x + 1.0, y, w - 2.0, h, bg);
        batch.fill(atlas, x, y + 1.0, w, h - 2.0, bg);
        for (dx, dy, dw, dh) in [
            (1., 0., w - 2., 1.),
            (1., h - 1., w - 2., 1.),
            (0., 1., 1., h - 2.),
            (w - 1., 1., 1., h - 2.),
        ] {
            batch.fill(atlas, x + dx, y + dy, dw, dh, outline)
        }
        self.native_text(batch, "Игроки рядом", [x + 8., y + 7.], 0xeeeeee, true);
        for (dx, dy, dw, dh) in [
            (3., 0., 4., 1.),
            (3., 9., 4., 1.),
            (0., 3., 1., 4.),
            (9., 3., 1., 4.),
            (1., 1., 2., 2.),
            (7., 1., 2., 2.),
            (1., 7., 2., 2.),
            (7., 7., 2., 2.),
            (4., 2., 2., 2.),
            (3., 6., 4., 2.),
        ] {
            batch.fill(
                atlas,
                x + w - 18. + dx,
                y + 6. + dy,
                dw,
                dh,
                rgba(204, 204, 204, 255),
            )
        }
        batch.fill(atlas, x + 4., y + 21., w - 8., 1., argb(587202559));
        if rows.is_empty() {
            self.native_text(
                batch,
                "Поблизости никого нет",
                [x + 8., y + 26.],
                0x888888,
                false,
            )
        }
        for (index, row) in rows.iter().enumerate() {
            let ry = y + 24. + index as f32 * 24.;
            if let Some(r) = atlas.sprites.get(&row.skin).copied() {
                for (u, v) in [(8., 8.), (40., 8.)] {
                    let mut face = r;
                    let du = r.u1 - r.u0;
                    let dv = r.v1 - r.v0;
                    face.u0 = r.u0 + du * u / 64.;
                    face.v0 = r.v0 + dv * v / 64.;
                    face.u1 = face.u0 + du * 8. / 64.;
                    face.v1 = face.v0 + dv * 8. / 64.;
                    batch.rect_uv(x + 6., ry + 3., 18., 18., face, u32::MAX)
                }
            }
            let width = |s: &str| HudBatch::text_width(atlas, s);
            let ax = if c.show_arrows {
                x + w - width(row.arrow) - 8.
            } else {
                x + w - 4.
            };
            if c.show_arrows {
                self.native_text(batch, row.arrow, [ax, ry + 7.], 0xa855f7, true)
            }
            let dist = format!("{}м", row.distance);
            let dx = if c.show_distance {
                ax - width(&dist) - 8.
            } else {
                ax
            };
            if c.show_distance {
                self.native_text(batch, &dist, [dx, ry + 7.], 0xffffff, true)
            }
            let available = dx - x - 28. - 4.;
            let mut name = row.name.clone();
            if width(&name) > available && available > 15. {
                while !name.is_empty() && width(&name) > available - 6. {
                    name.pop();
                }
                name.push('…')
            }
            self.native_text(batch, &name, [x + 28., ry + 2.], 0xffffff, true);
            if c.show_health {
                self.native_text(
                    batch,
                    "HP: ",
                    [x + 28., ry + 12.],
                    if row.hp < 20 { 0xff6666 } else { 0x999999 },
                    true,
                );
                self.native_text(
                    batch,
                    &row.hp.to_string(),
                    [x + 28. + width("HP: "), ry + 12.],
                    logic::health_color(row.hp),
                    true,
                )
            }
        }
        batch.scale_since(start, x, y, c.scale, c.scale);
    }
}

fn argb(v: u32) -> u32 {
    rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8)
}
