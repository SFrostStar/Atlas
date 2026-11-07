//! Atlas's independent gamma/night-vision controller, implementing Gamma Utils
//! 2.2.6's observable controls and configuration format. No Java implementation
//! code or bundled LGPL assets are copied into this module.
use super::{Game, IDS, now_ms, read_config, save_config};
use atlas_client::{
    keybinds,
    text::{Style, Text},
};
use serde_json::{Value, json};
use std::path::Path;
use winit::keyboard::KeyCode;

pub(super) struct State {
    pub config: Value,
    transitions: [Option<Transition>; 2],
    dynamic_targets: [Option<f64>; 2],
    dimension: String,
    pub dirty: bool,
}
#[derive(Clone, Copy)]
struct Transition {
    target: f64,
    step: f64,
    next: u64,
    message: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            config: defaults(),
            transitions: [None; 2],
            dynamic_targets: [None; 2],
            dimension: String::new(),
            dirty: false,
        }
    }
}
fn defaults() -> Value {
    json!({"gamma":{"value":1.0,"defaultGamma":100,"toggledGamma":1500,"updateToggle":false,"gammaStep":10,"showStatusEffect":false,"resetOnClose":false,"transition":{"smoothTransition":false,"transitionSpeed":3000},"dynamic":{"enabled":false,"paused":false,"minGamma":100,"maxGamma":1000,"transitionSpeed":200,"averagingLightRange":8,"skyBrightnessOverride":0},"dimensionPreference":{"enabled":false,"overworldPreference":1500,"netherPreference":1500,"endPreference":1500},"limiter":{"limitCheck":true,"minGamma":-750,"maxGamma":1500},"hudMessage":{"showMessage":true,"defaultColor":43520,"positiveColor":16755200,"negativeColor":11141120}},"nightVision":{"enabled":false,"value":100.0,"toggledNightVision":100,"updateToggle":false,"nightVisionStep":2,"showStatusEffect":false,"resetOnClose":false,"transition":{"smoothTransition":false,"transitionSpeed":200},"dynamic":{"enabled":false,"paused":false,"minNightVision":0,"maxNightVision":100,"transitionSpeed":15,"averagingLightRange":8,"skyBrightnessOverride":0},"dimensionPreference":{"enabled":false,"overworldPreference":100,"netherPreference":100,"endPreference":100},"limiter":{"limitCheck":true,"minNightVision":0,"maxNightVision":100},"hudMessage":{"showMessage":true,"defaultColor":43520,"positiveColor":16755200,"negativeColor":11141120,"enabledColor":43520,"disabledColor":11141120}},"other":{"namespacedCommands":false}})
}
fn merge(target: &mut Value, source: Value) {
    if let (Value::Object(t), Value::Object(s)) = (target, source) {
        for (k, v) in s {
            if let Some(existing) = t.get_mut(&k) {
                if existing.is_object() && v.is_object() {
                    merge(existing, v);
                } else if (existing.is_boolean() && v.is_boolean())
                    || (existing.is_number() && v.as_f64().is_some_and(f64::is_finite))
                {
                    *existing = v;
                }
            }
        }
    }
}
impl State {
    pub(super) fn apply_config(&mut self, value: Value) {
        let mut config = defaults();
        merge(&mut config, value);
        self.config = config;
        self.transitions = [None; 2];
        self.dynamic_targets = [None; 2];
        self.dirty = true;
    }
    pub(super) fn load(home: &Path) -> Self {
        let mut s = Self::default();
        merge(&mut s.config, read_config(home, "gammautils.json"));
        s
    }
    fn channel(&self, nv: bool) -> &Value {
        &self.config[if nv { "nightVision" } else { "gamma" }]
    }
    fn channel_mut(&mut self, nv: bool) -> &mut Value {
        &mut self.config[if nv { "nightVision" } else { "gamma" }]
    }
    fn value(&self, nv: bool) -> f64 {
        self.channel(nv)["value"]
            .as_f64()
            .unwrap_or(if nv { 100.0 } else { 1.0 })
    }
    fn scale(nv: bool) -> f64 {
        if nv { 1.0 } else { 100.0 }
    }
    fn field(&self, nv: bool, field: &str) -> f64 {
        self.channel(nv)[field].as_f64().unwrap_or(0.0)
    }
    fn set(
        &mut self,
        nv: bool,
        target: f64,
        smooth: bool,
        message: bool,
        dynamic: bool,
        now: u64,
    ) -> Option<&'static str> {
        if !dynamic && self.channel(nv)["dynamic"]["enabled"] == true {
            return message.then_some(if nv {
                "atlas.gamma.dynamic.nightvision.blocked"
            } else {
                "atlas.gamma.dynamic.gamma.blocked"
            });
        }
        let scale = Self::scale(nv);
        let minimum = self.channel(nv)["limiter"][if nv { "minNightVision" } else { "minGamma" }]
            .as_f64()
            .unwrap_or(0.0)
            / scale;
        let maximum = self.channel(nv)["limiter"][if nv { "maxNightVision" } else { "maxGamma" }]
            .as_f64()
            .unwrap_or(100.0)
            / scale;
        let target = if self.channel(nv)["limiter"]["limitCheck"] == true && maximum > minimum {
            target.clamp(minimum, maximum)
        } else {
            target
        };
        let index = usize::from(nv);
        self.transitions[index] = None;
        if smooth && (dynamic || self.channel(nv)["transition"]["smoothTransition"] == true) {
            let speed =
                self.channel(nv)[if dynamic { "dynamic" } else { "transition" }]["transitionSpeed"]
                    .as_f64()
                    .unwrap_or(0.0)
                    / scale
                    / 100.0;
            let step = if target < self.value(nv) {
                -speed
            } else {
                speed
            };
            self.transitions[index] = Some(Transition {
                target,
                step,
                next: now,
                message,
            });
            self.advance(nv, now);
        } else {
            self.channel_mut(nv)["value"] = json!(target);
            if nv && target == 0.0 {
                self.channel_mut(nv)["enabled"] = false.into();
            }
        }
        let default = if nv {
            0.0
        } else {
            self.field(false, "defaultGamma") / 100.0
        };
        if self.channel(nv)["updateToggle"] == true && target != default {
            self.channel_mut(nv)[if nv {
                "toggledNightVision"
            } else {
                "toggledGamma"
            }] = json!((target * scale + 0.5).floor() as i64);
        }
        self.dirty = true;
        None
    }
    fn advance(&mut self, nv: bool, now: u64) -> bool {
        let i = usize::from(nv);
        let Some(mut transition) = self.transitions[i] else {
            return false;
        };
        let mut changed = false;
        for _ in 0..10000 {
            if now < transition.next {
                break;
            }
            let next = self.value(nv) + transition.step;
            let done = (transition.step > 0.0 && next >= transition.target)
                || (transition.step < 0.0 && next <= transition.target);
            let value = if done { transition.target } else { next };
            self.channel_mut(nv)["value"] = json!(value);
            if nv {
                self.channel_mut(nv)["enabled"] = json!(value != 0.0);
            }
            changed |= transition.message;
            self.dirty = true;
            if done {
                self.transitions[i] = None;
                return changed;
            }
            transition.next += 10;
        }
        self.transitions[i] = Some(transition);
        changed
    }
    fn toggle(&mut self, nv: bool, now: u64) -> Option<&'static str> {
        if self.channel(nv)["dynamic"]["enabled"] == true {
            let paused = self.channel(nv)["dynamic"]["paused"] != true;
            self.channel_mut(nv)["dynamic"]["paused"] = json!(paused);
            if paused {
                self.dynamic_targets[usize::from(nv)] = None;
                let value = if nv {
                    0.0
                } else {
                    self.field(false, "defaultGamma") / 100.0
                };
                self.set(nv, value, true, false, true, now);
            }
            return None;
        }
        if nv {
            let enable = self.channel(true)["enabled"] != true;
            let target = if enable {
                self.field(true, "toggledNightVision")
            } else {
                0.0
            };
            if enable {
                self.channel_mut(true)["value"] = 0.0.into();
                self.channel_mut(true)["enabled"] = true.into();
            }
            self.set(true, target, true, true, false, now)
        } else {
            let default = self.field(false, "defaultGamma") / 100.0;
            let target = if self.value(false) == default {
                self.field(false, "toggledGamma") / 100.0
            } else {
                default
            };
            self.set(false, target, true, true, false, now)
        }
    }
    pub(super) fn save(&mut self, home: &Path) {
        if self.dirty {
            save_config(home, "gammautils.json", &self.config);
            self.dirty = false;
        }
    }
}
impl Game<'_> {
    fn atlas_gamma_virtual_effects(&self) -> Vec<(&'static str, bool, i32)> {
        if !self.native_window() || !self.atlas_port_on(IDS[9]) || self.player.is_none() {
            return vec![];
        }
        let state = &self.atlas_ports.actions.gamma;
        let mut effects = Vec::new();
        let gamma = ((state.value(false) * 100.0 + 0.5).floor() as i64) as i32;
        if state.channel(false)["showStatusEffect"] == true {
            if gamma > 100 {
                effects.push(("gammautils:bright", true, gamma));
            } else if gamma < 0 {
                effects.push(("gammautils:dim", false, gamma));
            }
        }
        if state.channel(true)["enabled"] == true && state.channel(true)["showStatusEffect"] == true
        {
            let percentage = ((state.value(true) + 0.5).floor() as i64) as i32;
            effects.push(("gammautils:night_vision", true, percentage));
        }
        effects
    }
    /// The virtual client effects share vanilla's sorted icon rows. They never
    /// enter simulation effects or send effect packets to a multiplayer server.
    pub(in crate::game) fn atlas_gamma_hud_effects(
        &self,
        batch: &mut atlas_client::hud::HudBatch,
    ) -> bool {
        let virtuals = self.atlas_gamma_virtual_effects();
        if virtuals.is_empty() {
            return false;
        }
        let inventory = match self.screen {
            super::Screen::Inventory
                if self.container.is_none() && self.craft_size == 2 && !self.mount_active() =>
            {
                Some((self.inventory_origin(batch).0, 176.0))
            }
            super::Screen::Creative => Some((
                Self::creative_origin(batch).0,
                atlas_client::creative::Creative::WIDTH,
            )),
            _ => None,
        };
        if inventory.is_some_and(|(x, w)| batch.width - (x + w + 2.0) >= 32.0) {
            return true;
        }
        let mut icons: Vec<_> = self
            .potions
            .effects
            .list
            .iter()
            .map(atlas_client::hud_overlays::EffectIcon::from)
            .collect();
        icons.extend(virtuals.into_iter().map(|(id, beneficial, _)| {
            atlas_client::hud_overlays::EffectIcon {
                id: id.into(),
                beneficial,
                color: 0,
                ambient: false,
                duration: -1,
                show_icon: true,
            }
        }));
        let draws = atlas_client::hud_overlays::effects(batch.width as i32, false, &icons);
        atlas_client::hud_layout::paint(batch, &self.gui.atlas, &draws);
        true
    }
    /// Some(None) means this port drew the inventory's effects without a tooltip.
    /// None means the ordinary inventory effects renderer should handle it.
    pub(in crate::game) fn atlas_gamma_inventory_effects(
        &self,
        batch: &mut atlas_client::hud::HudBatch,
        origin: (f32, f32),
        image_width: i32,
    ) -> Option<Option<[(String, u32); 2]>> {
        let virtuals = self.atlas_gamma_virtual_effects();
        if virtuals.is_empty() {
            return None;
        }
        let mut effects = self.potions.effects.list.clone();
        effects.extend(
            virtuals
                .iter()
                .map(|(id, _, _)| atlas_client::effects::EffectInstance::new(id, -1, 0)),
        );
        let mut panels = atlas_client::inventory_effects::panels(
            batch.width as i32,
            [origin.0 as i32, origin.1 as i32],
            image_width,
            &effects,
            self.clock.rate(),
            &|key| self.pack.names.display_get(key),
            &|text| self.gui.atlas.font.width(text).ceil() as i32,
        );
        for panel in &mut panels {
            if let Some((_, beneficial, percentage)) =
                virtuals.iter().find(|(id, _, _)| panel.icon.id == *id)
            {
                panel.icon.beneficial = *beneficial;
                panel.duration = format!("{percentage}%");
                let available = batch.width as i32 - panel.x;
                let maximum = if available >= 120 { available - 7 } else { 32 };
                panel.width = maximum.min(
                    39 + self
                        .gui
                        .atlas
                        .font
                        .width(&panel.name)
                        .max(self.gui.atlas.font.width(&panel.duration))
                        .ceil() as i32,
                );
            }
        }
        Some(atlas_client::inventory_effects::paint(
            batch,
            &self.gui.atlas,
            &panels,
            [self.cursor.0 as i32, self.cursor.1 as i32],
        ))
    }
    pub(in crate::game) fn atlas_actions_gamma(&self, vanilla: f32) -> f32 {
        if self.atlas_port_on(IDS[9]) {
            self.atlas_ports.actions.gamma.value(false) as f32
        } else {
            vanilla
        }
    }
    pub(in crate::game) fn atlas_actions_night_vision(&self, vanilla: f32) -> f32 {
        if self.atlas_port_on(IDS[9]) {
            let g = &self.atlas_ports.actions.gamma;
            if g.channel(true)["enabled"] == true || g.channel(true)["dynamic"]["enabled"] == true {
                return (g.value(true) / 100.0) as f32;
            }
        }
        vanilla
    }
    fn atlas_gamma_message(&mut self, nv: bool) {
        let g = &self.atlas_ports.actions.gamma;
        let cfg = g.channel(nv);
        if cfg["hudMessage"]["showMessage"] != true {
            return;
        }
        let dynamic = cfg["dynamic"]["enabled"] == true;
        let enabled = cfg["enabled"] == true;
        let paused = cfg["dynamic"]["paused"] == true;
        let percent = (g.value(nv) * State::scale(nv) + 0.5).floor() as i64;
        let key = if dynamic {
            match (nv, paused) {
                (true, true) => "atlas.gamma.dynamic.nightvision.paused",
                (true, false) => "atlas.gamma.dynamic.nightvision.active",
                (false, true) => "atlas.gamma.dynamic.gamma.paused",
                (false, false) => "atlas.gamma.dynamic.gamma.active",
            }
        } else if nv && !enabled {
            "atlas.gamma.nightvision.disabled"
        } else if nv {
            "atlas.gamma.nightvision.percent"
        } else {
            "atlas.gamma.gamma.percent"
        };
        let color_key = if dynamic {
            if paused {
                "disabledColor"
            } else {
                "enabledColor"
            }
        } else if nv && !enabled {
            "disabledColor"
        } else if percent < 0 {
            "negativeColor"
        } else if percent > 100 {
            "positiveColor"
        } else {
            "defaultColor"
        };
        let color = cfg["hudMessage"][color_key].as_u64().unwrap_or(43520) as u32;
        let text = Text::tr(key, &[&percent]).with_style(&Style {
            color: Some(color),
            ..Default::default()
        });
        self.titles.set_overlay(text, false);
    }
    pub(in crate::game) fn atlas_gamma_key(&mut self, key: KeyCode) -> bool {
        if !self.atlas_port_on(IDS[9]) {
            return false;
        }
        for (binding, default, nv, operation) in [
            ("gammaToggle", "key.keyboard.g", false, 0i32),
            ("increaseGamma", "key.keyboard.up", false, 1),
            ("decreaseGamma", "key.keyboard.down", false, -1),
            ("maxGamma", "key.keyboard.unknown", false, 2),
            ("minGamma", "key.keyboard.unknown", false, -2),
            ("nightVisionToggle", "key.keyboard.h", true, 0),
            ("increaseNightVision", "key.keyboard.right", true, 1),
            ("decreaseNightVision", "key.keyboard.left", true, -1),
        ] {
            if keybinds::key_code(&self.native_mod_key(IDS[9], binding, default)) != Some(key) {
                continue;
            }
            let now = now_ms();
            let g = &mut self.atlas_ports.actions.gamma;
            let message = if operation == 0 {
                g.toggle(nv, now)
            } else {
                let value = match operation {
                    1 | -1 => {
                        g.value(nv)
                            + g.field(nv, if nv { "nightVisionStep" } else { "gammaStep" })
                                / State::scale(nv)
                                * f64::from(operation)
                    }
                    2 => {
                        g.channel(nv)["limiter"][if nv { "maxNightVision" } else { "maxGamma" }]
                            .as_f64()
                            .unwrap_or(100.0)
                            / State::scale(nv)
                    }
                    _ => {
                        g.channel(nv)["limiter"][if nv { "minNightVision" } else { "minGamma" }]
                            .as_f64()
                            .unwrap_or(0.0)
                            / State::scale(nv)
                    }
                };
                g.set(nv, value, operation.abs() == 2, true, false, now)
            };
            if let Some(key) = message {
                self.native_chat(&atlas_client::lang::get(key));
            } else {
                self.atlas_gamma_message(nv);
            }
            return true;
        }
        false
    }
    fn atlas_gamma_light(&self, range: i64, sky_override: f64) -> f64 {
        let Some(p) = self.player.as_ref() else {
            return 15.0;
        };
        let at = [
            p.position.x.floor() as i32,
            p.position.y.floor() as i32 + 1,
            p.position.z.floor() as i32,
        ];
        let mut sky = self
            .res
            .environment()
            .sample(self.world.day_time, None, None);
        let (rain, thunder) = self.weather_levels(1.0);
        atlas_client::environment::apply_weather(&mut sky, rain, thunder);
        let brightness = f64::from(sky.sky_light_level).max(sky_override);
        let light = |at| {
            let (block, sky) = self.native_light(at);
            f64::from(block).max(f64::from(sky) * brightness)
        };
        let mut total = light(at);
        let mut count = 1;
        for axis in [
            [1, 0, 0],
            [-1, 0, 0],
            [0, 1, 0],
            [0, -1, 0],
            [0, 0, 1],
            [0, 0, -1],
        ] {
            for d in 0..range.max(0).min(128) {
                let p = std::array::from_fn(|i| at[i] + axis[i] * d as i32);
                if !self.res.facts().is_air(self.world.block(p[0], p[1], p[2])) {
                    break;
                }
                total += light(p);
                count += 1;
            }
        }
        total / f64::from(count)
    }
    pub(in crate::game) fn atlas_gamma_tick(&mut self) {
        if !self.atlas_port_on(IDS[9]) {
            return;
        }
        let now = now_ms();
        let dim = self.dim.id().to_owned();
        if self.atlas_ports.actions.gamma.dimension != dim {
            let g = &mut self.atlas_ports.actions.gamma;
            g.dimension = dim.clone();
            for nv in [false, true] {
                if g.channel(nv)["dimensionPreference"]["enabled"] == true {
                    let key = if dim.ends_with("the_nether") {
                        "netherPreference"
                    } else if dim.ends_with("the_end") {
                        "endPreference"
                    } else {
                        "overworldPreference"
                    };
                    let target = g.channel(nv)["dimensionPreference"][key]
                        .as_f64()
                        .unwrap_or(100.0)
                        / State::scale(nv);
                    g.set(nv, target, false, false, false, now);
                }
            }
        }
        for nv in [false, true] {
            let g = &self.atlas_ports.actions.gamma;
            let c = g.channel(nv);
            let changed = if c["dynamic"]["enabled"] == true && c["dynamic"]["paused"] != true {
                let range = c["dynamic"]["averagingLightRange"].as_i64().unwrap_or(8);
                let sky = c["dynamic"]["skyBrightnessOverride"]
                    .as_f64()
                    .unwrap_or(0.0)
                    / 100.0;
                let min = c["dynamic"][if nv { "minNightVision" } else { "minGamma" }]
                    .as_f64()
                    .unwrap_or(0.0)
                    / State::scale(nv);
                let max = c["dynamic"][if nv { "maxNightVision" } else { "maxGamma" }]
                    .as_f64()
                    .unwrap_or(100.0)
                    / State::scale(nv);
                let value = min + (max - min) * (15.0 - self.atlas_gamma_light(range, sky)) / 15.0;
                let g = &mut self.atlas_ports.actions.gamma;
                if g.dynamic_targets[usize::from(nv)] != Some(value) {
                    g.dynamic_targets[usize::from(nv)] = Some(value);
                    g.set(nv, value, true, false, true, now);
                }
                g.advance(nv, now)
            } else {
                self.atlas_ports.actions.gamma.advance(nv, now)
            };
            if changed {
                self.atlas_gamma_message(nv);
            }
        }
    }
    pub(in crate::game) fn atlas_gamma_command(&mut self, message: &str) -> bool {
        if !self.atlas_port_on(IDS[9]) {
            return false;
        }
        let words: Vec<_> = message.trim_start_matches('/').split_whitespace().collect();
        let Some(name) = words.first().copied() else {
            return false;
        };
        let namespace =
            self.atlas_ports.actions.gamma.config["other"]["namespacedCommands"] == true;
        let nv = if name
            == if namespace {
                "gammautils:gamma"
            } else {
                "gamma"
            } {
            false
        } else if name
            == if namespace {
                "gammautils:nightvision"
            } else {
                "nightvision"
            }
        {
            true
        } else {
            return false;
        };
        let operation = words.get(1).copied().unwrap_or("toggle");
        let now = now_ms();
        let g = &mut self.atlas_ports.actions.gamma;
        let mut message = None;
        match operation {
            "toggle" => message = g.toggle(nv, now),
            "dynamic" | "statuseffect" | "transition" => {
                let mode = words.get(2).copied().unwrap_or("toggle");
                let path = if operation == "dynamic" {
                    "dynamic"
                } else if operation == "transition" {
                    "transition"
                } else {
                    "showStatusEffect"
                };
                let key = if operation == "dynamic" {
                    "enabled"
                } else {
                    "smoothTransition"
                };
                if operation == "statuseffect" {
                    let enabled = g.channel(nv)[path] != true;
                    g.channel_mut(nv)[path] = json!(match mode {
                        "enable" => true,
                        "disable" => false,
                        _ => enabled,
                    });
                } else {
                    let enabled = g.channel(nv)[path][key] != true;
                    g.channel_mut(nv)[path][key] = json!(match mode {
                        "enable" | "smooth" => true,
                        "disable" | "none" => false,
                        _ => enabled,
                    });
                }
                g.dirty = true;
            }
            "increase" | "decrease" => {
                let amount = words
                    .get(2)
                    .and_then(|v| v.parse::<i32>().ok())
                    .map_or_else(
                        || g.field(nv, if nv { "nightVisionStep" } else { "gammaStep" }),
                        f64::from,
                    )
                    / State::scale(nv);
                let value = g.value(nv)
                    + if operation == "increase" {
                        amount
                    } else {
                        -amount
                    };
                message = g.set(nv, value, false, true, false, now);
            }
            "min" | "max" if !nv => {
                let target = g.channel(nv)["limiter"][if operation == "min" {
                    "minGamma"
                } else {
                    "maxGamma"
                }]
                .as_f64()
                .unwrap_or(100.0)
                    / 100.0;
                message = g.set(nv, target, true, true, false, now);
            }
            "enable" | "disable" if nv => {
                let enable = operation == "enable";
                if enable {
                    g.channel_mut(true)["enabled"] = true.into();
                    g.channel_mut(true)["value"] = 0.0.into();
                }
                let value = if enable {
                    g.field(true, "toggledNightVision")
                } else {
                    0.0
                };
                message = g.set(true, value, true, true, false, now);
            }
            _ => {
                let number = if operation == "set" {
                    words.get(2).copied().unwrap_or("")
                } else {
                    operation
                };
                if let Ok(n) = number.parse::<i32>() {
                    if nv && g.channel(true)["enabled"] != true {
                        g.channel_mut(true)["enabled"] = true.into();
                        g.channel_mut(true)["value"] = 0.0.into();
                    }
                    message = g.set(nv, f64::from(n) / State::scale(nv), true, true, false, now);
                } else {
                    return false;
                }
            }
        }
        if let Some(key) = message {
            self.native_chat(&atlas_client::lang::get(key));
        } else {
            self.atlas_gamma_message(nv);
        }
        let home = self.atlas_ports.actions.home.clone();
        self.atlas_ports.actions.gamma.save(&home);
        true
    }
    pub(in crate::game) fn atlas_actions_close(&mut self) {
        if !self.native_window() || !self.atlas_port_on(IDS[9]) {
            return;
        }
        let g = &mut self.atlas_ports.actions.gamma;
        if g.channel(false)["resetOnClose"] == true {
            g.channel_mut(false)["value"] = json!(g.field(false, "defaultGamma") / 100.0);
        }
        if g.channel(true)["resetOnClose"] == true {
            g.channel_mut(true)["enabled"] = false.into();
        }
        g.dirty = true;
        let home = self.atlas_ports.actions.home.clone();
        self.atlas_ports.actions.gamma.save(&home);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_toggle_brightness_and_night_vision_independently() {
        let mut s = State::default();
        assert_eq!(s.value(false), 1.0);
        assert_eq!(s.channel(true)["enabled"], false);
        s.toggle(false, 0);
        assert_eq!(s.value(false), 15.0);
        assert_eq!(s.value(true), 100.0);
        s.toggle(false, 10);
        assert_eq!(s.value(false), 1.0);
        s.toggle(true, 20);
        assert_eq!(s.channel(true)["enabled"], true);
        assert_eq!(s.value(true), 100.0);
        s.toggle(true, 30);
        assert_eq!(s.channel(true)["enabled"], false);
        assert_eq!(s.value(true), 0.0);
    }
    #[test]
    fn transition_uses_ten_millisecond_quantization_and_clamps_last_step() {
        let mut s = State::default();
        s.config["gamma"]["transition"]["smoothTransition"] = true.into();
        s.toggle(false, 100);
        assert!((s.value(false) - 1.3).abs() < 1e-9);
        s.advance(false, 109);
        assert!((s.value(false) - 1.3).abs() < 1e-9);
        s.advance(false, 200);
        assert!((s.value(false) - 4.3).abs() < 1e-9);
        s.advance(false, 1000);
        assert_eq!(s.value(false), 15.0);
        assert!(s.transitions[0].is_none());
    }
    #[test]
    fn dynamic_mode_blocks_manual_strength_but_toggle_pauses_to_default() {
        let mut s = State::default();
        s.config["gamma"]["dynamic"]["enabled"] = true.into();
        assert!(s.set(false, 12.0, false, true, false, 0).is_some());
        assert_eq!(s.value(false), 1.0);
        s.set(false, 10.0, false, false, true, 10);
        assert_eq!(s.value(false), 10.0);
        s.toggle(false, 20);
        assert_eq!(s.channel(false)["dynamic"]["paused"], true);
        s.advance(false, 5020);
        assert_eq!(s.value(false), 1.0);
    }
    #[test]
    fn limiter_and_toggle_memory_follow_percent_units() {
        let mut s = State::default();
        s.config["gamma"]["updateToggle"] = true.into();
        s.set(false, 100.0, false, false, false, 0);
        assert_eq!(s.value(false), 15.0);
        assert_eq!(s.channel(false)["toggledGamma"], 1500);
        s.set(false, -20.0, false, false, false, 10);
        assert_eq!(s.value(false), -7.5);
        assert_eq!(s.channel(false)["toggledGamma"], -750);
    }
}
