//! Native adapters for the supplied utility mods. No JVM runs in the game.
use super::super::{Game, native_hooks::NativeZoom};
use atlas_client::{chat::ChatSource, keybinds};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    time::Instant,
};
use winit::keyboard::KeyCode;
#[path = "utility_config.rs"]
pub(super) mod config;
#[path = "utility_logic.rs"]
mod logic;
pub(super) const AUTO_SPRINT: &str = "atlas-autosprint";
pub(super) const NO_SHAKE_HAND: &str = "atlas-noshakehand";
pub(super) const ZOOMIFY: &str = "atlas-zoomify";
pub(super) const MENTION_SOUND: &str = "atlas-mentionsound";

pub(super) struct UtilityState {
    home: PathBuf,
    started: Instant,
    mentions: logic::MentionLimiter,
    shake: config::NoShake,
    mention: config::Mention,
    zoom: config::Zoom,
    zooming: bool,
    secondary: bool,
    tiers: i32,
    progress: f64,
    previous_progress: f64,
    scroll: f64,
    previous_scroll: f64,
    secondary_progress: f64,
    previous_secondary: f64,
    was_zooming: bool,
    last_divisor: Cell<f64>,
    reset_divisor: Option<f64>,
}
impl Default for UtilityState {
    fn default() -> Self {
        Self::load(Path::new("."))
    }
}
impl UtilityState {
    pub(super) fn load(home: &Path) -> Self {
        Self {
            home: home.to_owned(),
            started: Instant::now(),
            mentions: Default::default(),
            shake: config::load(&home.join("config/noshakehand.json")),
            mention: config::load(&home.join("config/mentionsound/config.json")),
            zoom: config::load(&home.join("config/zoomify.json")),
            zooming: false,
            secondary: false,
            tiers: 0,
            progress: 0.0,
            previous_progress: 0.0,
            scroll: 0.0,
            previous_scroll: 0.0,
            secondary_progress: 0.0,
            previous_secondary: 0.0,
            was_zooming: false,
            last_divisor: Cell::new(1.0),
            reset_divisor: None,
        }
    }
    fn save(&self) {
        config::save(&self.home.join("config/noshakehand.json"), &self.shake);
        config::save(
            &self.home.join("config/mentionsound/config.json"),
            &self.mention,
        );
        config::save(&self.home.join("config/zoomify.json"), &self.zoom);
    }
}
impl Game<'_> {
    pub(in crate::game) fn atlas_zoom_sensitivity(&self, vanilla: f64) -> f64 {
        if !self.atlas_port_on(ZOOMIFY) {
            return vanilla;
        }
        let u = &self.atlas_ports.utility;
        vanilla
            / (1.0
                + (u.last_divisor.get() - 1.0) * u.zoom.relative_sensitivity.clamp(0, 100) as f64
                    / 100.0)
    }
    pub(in crate::game) fn atlas_zoom_cinematic_divisor(&self) -> f64 {
        let u = &self.atlas_ports.utility;
        if self.atlas_port_on(ZOOMIFY) && u.zooming && u.zoom.cinematic_camera > 0 {
            u.zoom.cinematic_camera as f64 / 100.0
        } else {
            1.0
        }
    }
    pub(in crate::game) fn atlas_zoom_cinematic(&self) -> bool {
        let u = &self.atlas_ports.utility;
        self.atlas_port_on(ZOOMIFY) && (u.secondary || (u.zooming && u.zoom.cinematic_camera > 0))
    }
    pub(in crate::game) fn atlas_zoom_overrides_spyglass(&self) -> bool {
        self.atlas_port_on(ZOOMIFY) && self.atlas_ports.utility.zoom.spyglass_behaviour != "COMBINE"
    }
    pub(in crate::game) fn atlas_zoom_bob_scale(&self) -> f32 {
        if !self.atlas_port_on(ZOOMIFY) || !self.atlas_ports.utility.zoom.relative_view_bobbing {
            return 1.0;
        }
        self.native_zoom_now()
            .map_or(1.0, |z| 1.0 / (1.0 + 0.2 * (z.factor - 1.0)))
    }
    pub(in crate::game) fn atlas_zoom_hand_factor(&self) -> f32 {
        if !self.atlas_port_on(ZOOMIFY) || !self.atlas_ports.utility.zoom.affect_hand_fov {
            return 1.0;
        }
        self.native_zoom_now().map_or(1.0, |z| z.factor)
    }
    pub(in crate::game) fn atlas_utility_config(&self, id: &str) -> Option<serde_json::Value> {
        let u = &self.atlas_ports.utility;
        match id {
            NO_SHAKE_HAND => serde_json::to_value(&u.shake).ok(),
            ZOOMIFY => serde_json::to_value(&u.zoom).ok(),
            MENTION_SOUND => serde_json::to_value(&u.mention).ok(),
            _ => None,
        }
    }
    pub(in crate::game) fn atlas_utility_set_config(
        &mut self,
        id: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let u = &mut self.atlas_ports.utility;
        match id {
            NO_SHAKE_HAND => u.shake = serde_json::from_value(value).map_err(|e| e.to_string())?,
            MENTION_SOUND => {
                u.mention = serde_json::from_value(value).map_err(|e| e.to_string())?
            }
            ZOOMIFY => {
                let config: config::Zoom =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                if !(1..=500).contains(&config.initial_zoom)
                    || !(0..=100).contains(&config.scroll_step_count)
                    || !(100..=1000).contains(&config.zoom_per_step)
                    || !config.zoom_in_time.is_finite()
                    || !config.zoom_out_time.is_finite()
                    || config.zoom_in_time < 0.0
                    || config.zoom_out_time < 0.0
                {
                    return Err("Invalid zoom range".into());
                }
                u.zoom = config;
            }
            _ => return Err("Unknown mod".into()),
        }
        u.save();
        Ok(())
    }
    pub(in crate::game) fn atlas_port_auto_sprint(&self) -> bool {
        self.atlas_port_on(AUTO_SPRINT)
            && self.player.as_ref().is_some_and(|p| {
                logic::auto_sprint(
                    self.focused && self.native_playing(),
                    self.keys.contains(&KeyCode::KeyW),
                    p.shift_key_down || self.keys.contains(&KeyCode::ShiftLeft),
                    p.swimming,
                    p.health.food.food,
                )
            })
    }
    pub(in crate::game) fn atlas_port_hand_sway(&self, partial: f32) -> [f32; 2] {
        if self.atlas_port_on(NO_SHAKE_HAND) && self.atlas_ports.utility.shake.disable_camera_sway {
            [0.0, 0.0]
        } else {
            self.sway
                .rotation(partial, self.view.x_rot, self.view.y_rot)
        }
    }
    pub(in crate::game) fn atlas_port_hand_bobbing(&self) -> bool {
        !(self.atlas_port_on(NO_SHAKE_HAND) && self.atlas_ports.utility.shake.disable_hand_bobbing)
    }
    pub(in crate::game) fn atlas_port_map_pitch(&self, pitch: f32) -> f32 {
        if self.atlas_port_on(NO_SHAKE_HAND) && self.atlas_ports.utility.shake.disable_map_pitch {
            0.0
        } else {
            pitch
        }
    }
    fn atlas_zoom_key_held(&self, action: &str, default: &str) -> bool {
        let key = self.native_mod_key(ZOOMIFY, action, default);
        keybinds::key_code(&key).is_some_and(|k| self.physical_keys.contains(&k))
    }
    pub(in crate::game) fn atlas_utility_tick(&mut self) {
        let active = self.atlas_port_on(ZOOMIFY)
            && self.focused
            && self.native_playing()
            && self.player.is_some();
        let held = active && self.atlas_zoom_key_held("zoom", "key.keyboard.c");
        let holding = self
            .held()
            .into_iter()
            .chain(self.offhand.as_ref())
            .any(|s| &*s.item == "minecraft:spyglass");
        let carrying = self
            .inventory
            .iter()
            .flatten()
            .chain(self.offhand.iter())
            .any(|s| &*s.item == "minecraft:spyglass");
        let scoping = self.scoping();
        let u = &mut self.atlas_ports.utility;
        if u.zoom.zoom_key_behaviour == "HOLD" {
            u.zooming = held;
        }
        if (u.zoom.spyglass_behaviour == "ONLY_ZOOM_WHILE_HOLDING" && !holding)
            || (u.zoom.spyglass_behaviour == "ONLY_ZOOM_WHILE_CARRYING" && !carrying)
        {
            u.zooming = false;
        }
        if u.zoom.spyglass_behaviour != "COMBINE" && scoping {
            u.zooming = true;
        }
        if !active {
            u.zooming = false;
            u.secondary = false;
        }
        if u.was_zooming != u.zooming {
            let old = if u.was_zooming {
                &u.zoom.zoom_in_transition
            } else {
                &u.zoom.zoom_out_transition
            };
            let new = if u.zooming {
                &u.zoom.zoom_in_transition
            } else {
                &u.zoom.zoom_out_transition
            };
            let value = logic::phase(old, u.progress, u.was_zooming);
            u.progress = logic::inverse_phase(new, value, u.zooming);
            if !u.zooming && u.scroll > 0.0 {
                u.reset_divisor = Some(u.last_divisor.get());
                u.scroll = 0.0;
            }
            if u.zooming {
                u.reset_divisor = None;
            }
        }
        u.was_zooming = u.zooming;
        u.previous_progress = u.progress;
        u.previous_scroll = u.scroll;
        u.previous_secondary = u.secondary_progress;
        let seconds = if u.zooming {
            u.zoom.zoom_in_time
        } else {
            u.zoom.zoom_out_time
        };
        let transition = if u.zooming {
            &u.zoom.zoom_in_transition
        } else {
            &u.zoom.zoom_out_transition
        };
        let step = if seconds <= 0.0 || transition == "INSTANT" {
            1.0
        } else {
            0.05 / seconds
        };
        u.progress = (u.progress + if u.zooming { step } else { -step }).clamp(0.0, 1.0);
        if !u.zooming && !u.zoom.retain_zoom_steps {
            u.tiers = 0;
        }
        let speed = 1.0 - (u.zoom.scroll_zoom_smoothness.clamp(0, 100) as f64 / 100.0);
        if u.zooming {
            u.scroll += (u.tiers as f64 - u.scroll) * speed;
        }
        let seconds = if u.secondary {
            u.zoom.secondary_zoom_in_time
        } else {
            u.zoom.secondary_zoom_out_time
        };
        u.secondary_progress = (u.secondary_progress
            + if u.secondary {
                0.05 / seconds.max(0.05)
            } else {
                -0.05 / seconds.max(0.05)
            })
        .clamp(0.0, 1.0);
    }
    pub(in crate::game) fn atlas_utility_key(&mut self, key: KeyCode, pressed: bool) {
        if !pressed {
            return;
        }
        for (action, default) in [
            ("zoom", "key.keyboard.c"),
            ("secondary", "key.keyboard.unknown"),
            ("zoom_in", "key.keyboard.unknown"),
            ("zoom_out", "key.keyboard.unknown"),
        ] {
            if keybinds::key_code(&self.native_mod_key(ZOOMIFY, action, default)) != Some(key) {
                continue;
            }
            if !self.atlas_port_on(ZOOMIFY) || !self.native_playing() {
                continue;
            }
            let u = &mut self.atlas_ports.utility;
            match action {
                "zoom" if u.zoom.zoom_key_behaviour == "TOGGLE" => u.zooming = !u.zooming,
                "secondary" => u.secondary = !u.secondary,
                "zoom_in" if u.zoom.keybind_scrolling => {
                    u.tiers = logic::scroll_tier(u.tiers, 1.0, u.zoom.scroll_step_count)
                }
                "zoom_out" if u.zoom.keybind_scrolling => {
                    u.tiers = logic::scroll_tier(u.tiers, -1.0, u.zoom.scroll_step_count)
                }
                _ => {}
            }
        }
        if self.atlas_port_on(NO_SHAKE_HAND)
            && keybinds::key_code(&self.native_mod_key(
                NO_SHAKE_HAND,
                "toggle",
                "key.keyboard.unknown",
            )) == Some(key)
        {
            let u = &mut self.atlas_ports.utility;
            u.shake.disable_camera_sway = !u.shake.disable_camera_sway;
            u.save();
        }
    }
    pub(in crate::game) fn atlas_utility_wheel(&mut self, delta: f32) -> bool {
        if !self.atlas_port_on(ZOOMIFY) || !self.native_playing() {
            return false;
        }
        let held = self.atlas_zoom_key_held("zoom", "key.keyboard.c");
        let u = &mut self.atlas_ports.utility;
        if !u.zoom.scroll_zoom || u.zoom.keybind_scrolling || !(u.zooming || held) || delta == 0.0 {
            return false;
        }
        u.tiers = logic::scroll_tier(u.tiers, delta, u.zoom.scroll_step_count);
        true
    }
    pub(in crate::game) fn atlas_port_zoom(&self, zoom: &mut Option<NativeZoom>) {
        if zoom.is_some()
            || !self.atlas_port_on(ZOOMIFY)
            || !self.focused
            || !self.native_playing()
            || self.player.is_none()
        {
            return;
        }
        let u = &self.atlas_ports.utility;
        let a = self.clock.fraction().clamp(0.0, 1.0);
        let raw = u.previous_progress + (u.progress - u.previous_progress) * a;
        let p = logic::phase(
            if u.zooming {
                &u.zoom.zoom_in_transition
            } else {
                &u.zoom.zoom_out_transition
            },
            raw,
            u.zooming,
        );
        let tier = u.previous_scroll + (u.scroll - u.previous_scroll) * a;
        let divisor = logic::zoom_divisor(
            u.reset_divisor.unwrap_or(u.zoom.initial_zoom as f64),
            p,
            u.zoom.zoom_per_step.max(1) as f64,
            if u.reset_divisor.is_some() { 0.0 } else { tier },
        );
        u.last_divisor.set(divisor);
        let secondary = u.previous_secondary + (u.secondary_progress - u.previous_secondary) * a;
        let divisor = divisor
            * logic::zoom_divisor(u.zoom.secondary_zoom_amount as f64, secondary, 100.0, 0.0);
        if divisor > 1.00001 {
            *zoom = Some(NativeZoom {
                factor: divisor as f32,
                smooth: false,
                turn: 1.0,
            });
        }
    }
    pub(in crate::game) fn atlas_port_chat(&mut self, message: &str, source: ChatSource) {
        if source != ChatSource::Client {
            self.atlas_visual_chat(message);
        }
        if !self.atlas_port_on(MENTION_SOUND)
            || !self.native_window()
            || self.player.is_none()
            || source == ChatSource::Client
        {
            return;
        }
        let own = self.options.player_name();
        let u = &mut self.atlas_ports.utility;
        if u.mention.ignore_own_messages && logic::own_prefix(message, &own) {
            return;
        }
        if !logic::mentions(
            message,
            std::iter::once(&own).chain(u.mention.extra_names.iter()),
        ) {
            return;
        }
        if !u.mentions.take(
            u.started.elapsed().as_millis() as u64,
            u.mention.cooldown_ms,
        ) {
            return;
        }
        let file = u
            .home
            .join("config/mentionsound")
            .join(&u.mention.sound_file);
        let fallback = u.mention.fallback_to_vanilla_sound;
        let volume = u.mention.volume;
        if !self.audio.play_external(&file, volume) && fallback {
            self.audio
                .play("minecraft:block.note_block.pling", None, volume, 1.0);
        }
    }
    pub(in crate::game) fn atlas_utility_settings(&self, id: &str, buttons: &mut Vec<String>) {
        if self.atlas_utility_config(id).is_some() {
            buttons.push(atlas_client::lang::get("atlas.ports.edit_config"));
        }
        let u = &self.atlas_ports.utility;
        let value = match id {
            NO_SHAKE_HAND => serde_json::to_value(&u.shake),
            ZOOMIFY => serde_json::to_value(&u.zoom),
            MENTION_SOUND => serde_json::to_value(&u.mention),
            _ => return,
        };
        if let Ok(serde_json::Value::Object(map)) = value {
            for (k, v) in map {
                buttons.push(format!("{k}: {v}"));
            }
        }
    }
    pub(in crate::game) fn atlas_utility_settings_click(&mut self, id: &str, index: usize) {
        if self.atlas_utility_config(id).is_some() {
            if index == 0 {
                self.atlas_visual_open_editor(id);
                return;
            }
        }
        let index = index.saturating_sub(1);
        let u = &mut self.atlas_ports.utility;
        let value = match id {
            NO_SHAKE_HAND => serde_json::to_value(&u.shake),
            ZOOMIFY => serde_json::to_value(&u.zoom),
            MENTION_SOUND => serde_json::to_value(&u.mention),
            _ => return,
        };
        if let Ok(serde_json::Value::Object(mut map)) = value {
            if let Some((key, value)) = map.iter_mut().nth(index) {
                match value {
                    serde_json::Value::Bool(v) => *v = !*v,
                    serde_json::Value::Number(n) => {
                        let v = n.as_f64().unwrap_or(0.0);
                        *n = serde_json::Number::from_f64(if v >= 500.0 {
                            0.0
                        } else if key.ends_with("Time") {
                            v + 0.25
                        } else {
                            v + 1.0
                        })
                        .unwrap();
                    }
                    serde_json::Value::String(s) if key == "zoomKeyBehaviour" => {
                        *s = if s == "HOLD" { "TOGGLE" } else { "HOLD" }.into()
                    }
                    _ => {}
                }
            }
            let value = serde_json::Value::Object(map);
            match id {
                NO_SHAKE_HAND => {
                    if let Ok(v) = serde_json::from_value(value) {
                        u.shake = v
                    }
                }
                ZOOMIFY => {
                    if let Ok(v) = serde_json::from_value(value) {
                        u.zoom = v
                    }
                }
                MENTION_SOUND => {
                    if let Ok(v) = serde_json::from_value(value) {
                        u.mention = v
                    }
                }
                _ => {}
            }
            u.save();
        }
    }
    pub(in crate::game) fn atlas_utility_command(&mut self, command: &str) -> bool {
        let words: Vec<_> = command.trim_start_matches('/').split_whitespace().collect();
        if words.first() != Some(&"noshakehand") || !self.atlas_port_on(NO_SHAKE_HAND) {
            return false;
        }
        let u = &mut self.atlas_ports.utility;
        match words.get(1).copied() {
            Some("toggle") => u.shake.disable_camera_sway = !u.shake.disable_camera_sway,
            Some("bobbing") => u.shake.disable_hand_bobbing = !u.shake.disable_hand_bobbing,
            Some("reload") => u.shake = config::load(&u.home.join("config/noshakehand.json")),
            _ => {}
        }
        u.save();
        true
    }
}
