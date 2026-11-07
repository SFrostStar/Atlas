//! AppleSkin 3.0.10 by squeek502, Unlicense. Adapted for Atlas's renderer.
//! Reference: appleskin-fabric-mc26.3-3.0.10.jar supplied by SFrostStar.
use super::super::Game;
use atlas_client::{
    hud::{HudAtlas, HudBatch, rgba},
    hud_layout::HealthInputs,
    items::ItemStack,
};
use atlas_core::random::{LegacyRandom, RandomSource};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::Path,
};
#[path = "food_logic.rs"]
mod logic;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Config {
    show_food_values_in_tooltip: bool,
    show_food_values_in_tooltip_always: bool,
    show_saturation_hud_overlay: bool,
    show_food_values_hud_overlay: bool,
    show_food_values_hud_overlay_when_offhand: bool,
    show_food_exhaustion_hud_underlay: bool,
    show_food_health_hud_overlay: bool,
    show_vanilla_animations_overlay: bool,
    max_hud_overlay_flash_alpha: f32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            show_food_values_in_tooltip: true,
            show_food_values_in_tooltip_always: true,
            show_saturation_hud_overlay: true,
            show_food_values_hud_overlay: true,
            show_food_values_hud_overlay_when_offhand: true,
            show_food_exhaustion_hud_underlay: true,
            show_food_health_hud_overlay: true,
            show_vanilla_animations_overlay: true,
            max_hud_overlay_flash_alpha: 0.65,
        }
    }
}
#[derive(Default)]
pub(super) struct State {
    config: Config,
    flash: Cell<logic::Flash>,
    last_tick: Cell<Option<u64>>,
    pub(super) natural_regeneration: Cell<Option<bool>>,
    synced: RefCell<HashMap<u32, (f32, f32, bool)>>,
}
impl State {
    pub(super) fn load(home: &Path) -> Self {
        let path = home.join("config/appleskin.json5");
        let mut state = Self::default();
        if let Ok(text) = std::fs::read_to_string(&path) {
            // Jankson writes this flat config with comments and optional quoted keys.
            let mut values = serde_json::Map::new();
            for line in text.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().trim_matches('"');
                    let value = value
                        .split("//")
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_end_matches(',');
                    if let Ok(value) = serde_json::from_str(value) {
                        values.insert(key.into(), value);
                    }
                }
            }
            state.config = serde_json::from_str(&text)
                .or_else(|_| serde_json::from_value(serde_json::Value::Object(values)))
                .unwrap_or_default();
        } else if let Ok(text) = serde_json::to_string_pretty(&state.config) {
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(path, text);
        }
        state
    }
    fn alpha(&self, tick: u64, has_food: bool) -> f32 {
        if !has_food {
            self.flash.set(logic::Flash::default());
            self.last_tick.set(Some(tick));
            return 0.0;
        }
        let mut flash = self.flash.get();
        let elapsed = self
            .last_tick
            .replace(Some(tick))
            .map_or(1, |old| tick.saturating_sub(old).min(32));
        for _ in 0..elapsed {
            flash.tick();
        }
        self.flash.set(flash);
        flash.alpha(self.config.max_hud_overlay_flash_alpha)
    }
}
#[derive(Clone, Copy)]
struct Food {
    nutrition: i32,
    saturation: f32,
    always: bool,
    rotten: bool,
    regeneration: f32,
}
impl Game<'_> {
    pub(in crate::game) fn appleskin_reset(&self) {
        self.atlas_ports.food.natural_regeneration.set(None);
    }
    pub(in crate::game) fn appleskin_sync_guests(&self, host: &super::super::lan::LanHost) {
        if !self.atlas_port_on("atlas-appleskin") {
            return;
        }
        use atlas_protocol::packets::{configuration::CustomPayload, play::Clientbound};
        let mut synced = self.atlas_ports.food.synced.borrow_mut();
        synced.retain(|conn, _| host.hub.guests.iter().any(|g| g.conn == *conn));
        for guest in host
            .hub
            .guests
            .iter()
            .filter(|g| g.loaded && host.hub.here(g))
        {
            let Some(motion) = &guest.motion else {
                continue;
            };
            let food = &motion.health.food;
            let regen = self.world.health_rules.natural_regeneration;
            let old = synced.get(&guest.conn).copied();
            let mut next = old.unwrap_or((food.saturation, food.exhaustion, regen));
            let send = |channel: &str, data: Vec<u8>| {
                host.hub.send_to(
                    guest.conn,
                    &Clientbound::CustomPayload(CustomPayload {
                        channel: channel.into(),
                        data,
                    }),
                )
            };
            if old.is_none_or(|v| v.0 != food.saturation) {
                send(
                    "appleskin:saturation",
                    food.saturation.to_be_bytes().to_vec(),
                );
                next.0 = food.saturation;
            }
            if old.is_none_or(|v| (v.1 - food.exhaustion).abs() >= 0.01) {
                send(
                    "appleskin:exhaustion",
                    food.exhaustion.to_be_bytes().to_vec(),
                );
                next.1 = food.exhaustion;
            }
            if old.is_none_or(|v| v.2 != regen) {
                send("appleskin:natural_regeneration", vec![u8::from(regen)]);
                next.2 = regen;
            }
            synced.insert(guest.conn, next);
        }
    }
    fn appleskin_food(&self, stack: &ItemStack) -> Option<Food> {
        if stack.count == 0 {
            return None;
        }
        let view = atlas_client::item_render::stack_view(stack);
        let components = self.pack.item_renderer.models.components(&view);
        let food = components.get("minecraft:food")?;
        let consumable = components.get("minecraft:consumable")?;
        let mut result = Food {
            nutrition: food.get("nutrition")?.as_i64()?.clamp(0, i32::MAX as i64) as i32,
            saturation: food
                .get("saturation")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            always: food
                .get("can_always_eat")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            rotten: false,
            regeneration: 0.0,
        };
        if let Some(effects) = consumable
            .get("on_consume_effects")
            .and_then(|v| v.as_array())
        {
            for effect in effects {
                if !matches!(
                    effect.get("type").and_then(|v| v.as_str()),
                    Some("minecraft:apply_effects" | "apply_effects")
                ) {
                    continue;
                }
                if let Some(list) = effect.get("effects").and_then(|v| v.as_array()) {
                    let mut regeneration_added = false;
                    for entry in list {
                        let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        if let Some(def) = atlas_client::brewing::effect(id) {
                            result.rotten |=
                                def.category == atlas_client::brewing::Category::Harmful;
                        }
                        if id.trim_start_matches("minecraft:") == "regeneration"
                            && !regeneration_added
                        {
                            let duration =
                                entry.get("duration").and_then(|v| v.as_i64()).unwrap_or(0);
                            let amp =
                                entry.get("amplifier").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                            result.regeneration +=
                                (duration as f32 / 50_i32.wrapping_shr(amp).max(1) as f32).floor();
                            regeneration_added = true;
                        }
                    }
                }
            }
        }
        Some(result)
    }
    fn appleskin_held(&self, food: i32) -> Option<Food> {
        let main = self
            .held()
            .and_then(|s| self.appleskin_food(s))
            .filter(|f| food < 20 || f.always);
        if main.is_some()
            || !self
                .atlas_ports
                .food
                .config
                .show_food_values_hud_overlay_when_offhand
        {
            return main;
        }
        self.offhand
            .as_ref()
            .and_then(|s| self.appleskin_food(s))
            .filter(|f| food < 20 || f.always)
    }
    pub(in crate::game) fn appleskin_underlay(&self, batch: &mut HudBatch, i: &HealthInputs) {
        if !self.atlas_port_on("atlas-appleskin")
            || i.vehicle_hearts != 0
            || !self
                .atlas_ports
                .food
                .config
                .show_food_exhaustion_hud_underlay
        {
            return;
        }
        let exhaustion = self
            .player
            .as_ref()
            .map_or(0.0, |p| p.health.food.exhaustion);
        let width = ((exhaustion / 4.0).clamp(0.0, 1.0) * 81.0) as i32 as f32;
        icon(
            batch,
            &self.gui.atlas,
            (batch.width as i32 / 2 + 91) as f32 - width,
            (batch.height as i32 - 39) as f32,
            81.0 - width,
            18.0,
            width,
            9.0,
            0.75,
        );
    }
    pub(in crate::game) fn appleskin_overlay(&self, batch: &mut HudBatch, i: &HealthInputs) {
        if !self.atlas_port_on("atlas-appleskin") {
            return;
        }
        let state = &self.atlas_ports.food;
        let config = &state.config;
        let held = self.appleskin_held(i.food);
        let alpha = state.alpha(i.tick as u64, held.is_some());
        let health_bars = ((i.max_health + i.absorption.ceil()) / 2.0).ceil() as i32;
        let rows = (health_bars as f32 / 10.0).ceil() as i32;
        let row_height = (10 - (rows - 2)).max(3);
        let mut random = LegacyRandom::new(i64::from(i.tick.wrapping_mul(312871)));
        let mut hearts = Vec::new();
        if (0..=1000).contains(&health_bars) {
            hearts.resize(health_bars as usize, (0, 0));
            for n in (0..health_bars).rev() {
                let dy = if config.show_vanilla_animations_overlay && i.health.ceil() <= 4.0 {
                    random.next_int_bound(2).unwrap_or(0)
                } else {
                    0
                };
                hearts[n as usize] = (n % 10 * 8, -(n / 10) * row_height + dy);
            }
        }
        let right = (batch.width as i32 / 2 + 91) as f32;
        let left = (batch.width as i32 / 2 - 91) as f32;
        let top = (batch.height as i32 - 39) as f32;
        let atlas = &self.gui.atlas;
        let offsets: Vec<_> = (0..10)
            .map(|n| {
                let dy = if config.show_vanilla_animations_overlay
                    && i.saturation <= 0.0
                    && i.tick % (i.food * 3 + 1) == 0
                {
                    random.next_int_bound(3).unwrap_or(0) - 1
                } else {
                    0
                };
                (right - (n * 8 + 9) as f32, top + dy as f32)
            })
            .collect();
        if i.vehicle_hearts == 0 {
            if config.show_saturation_hud_overlay {
                saturation(batch, atlas, &offsets, i.saturation, 0.0, 1.0);
            }
            if let Some(food) = held.filter(|_| config.show_food_values_hud_overlay) {
                let modified = (i.food + food.nutrition).clamp(0, 20);
                for n in (i.food / 2).max(0)..((modified as f32 / 2.0).ceil() as i32).min(10) {
                    let (x, y) = offsets[n as usize];
                    hunger_icon(batch, atlas, x, y, food.rotten, "empty", alpha * 0.25);
                    hunger_icon(
                        batch,
                        atlas,
                        x,
                        y,
                        food.rotten,
                        if n * 2 + 1 == modified {
                            "half"
                        } else {
                            "full"
                        },
                        alpha,
                    );
                }
                if config.show_saturation_hud_overlay {
                    let gained = (i.saturation + food.saturation)
                        .min((i.food + food.nutrition) as f32)
                        - i.saturation;
                    saturation(batch, atlas, &offsets, i.saturation, gained, alpha);
                }
            }
        }
        if let Some(food) = held.filter(|_| {
            config.show_food_health_hud_overlay
                && i.food < 18
                && !i.regeneration
                && self.potions.effects.get("poison").is_none()
                && self.potions.effects.get("wither").is_none()
                && self.ui.difficulty != 0
                && i.health < i.max_health
        }) {
            let food_level = (i.food + food.nutrition).min(20);
            let exhaustion = self
                .player
                .as_ref()
                .map_or(0.0, |p| p.health.food.exhaustion);
            let regenerate = state
                .natural_regeneration
                .get()
                .unwrap_or(self.world.health_rules.natural_regeneration);
            let healing = if food_level >= 18 && regenerate {
                logic::estimated_health(
                    food_level,
                    (i.saturation + food.saturation).min(food_level as f32),
                    exhaustion,
                )
            } else {
                0.0
            };
            let modified = (i.health + healing + food.regeneration).min(i.max_health);
            let start = (i.health.ceil() / 2.0).max(0.0) as usize;
            let end = (modified / 2.0).ceil().max(0.0) as usize;
            for (n, &(dx, dy)) in hearts.iter().enumerate().take(end).skip(start) {
                let part = if (n * 2 + 1) as i32 == modified.ceil() as i32 {
                    "half"
                } else {
                    "full"
                };
                let prefix = if i.hardcore { "hardcore_" } else { "" };
                let container = if i.hardcore {
                    "gui/sprites/hud/heart/container_hardcore"
                } else {
                    "gui/sprites/hud/heart/container"
                };
                batch.sprite_scaled(
                    atlas,
                    container,
                    left + dx as f32,
                    top + dy as f32,
                    9.0,
                    9.0,
                    white(alpha * 0.25),
                );
                batch.sprite_scaled(
                    atlas,
                    &format!("gui/sprites/hud/heart/{prefix}{part}"),
                    left + dx as f32,
                    top + dy as f32,
                    9.0,
                    9.0,
                    white(alpha),
                );
            }
        }
    }
    pub(in crate::game) fn appleskin_tooltip_size(&self, stack: &ItemStack) -> Option<[f32; 2]> {
        if !self.atlas_port_on("atlas-appleskin") {
            return None;
        }
        let config = &self.atlas_ports.food.config;
        if !config.show_food_values_in_tooltip_always
            && !(config.show_food_values_in_tooltip && self.has_shift_down())
        {
            return None;
        }
        let food = self.appleskin_food(stack)?;
        if food.nutrition == 0 {
            return None;
        }
        let h = ((food.nutrition as f32 / 2.0).ceil() as i32).abs();
        let s = (food.saturation.abs() / 2.0).ceil() as i32;
        let hw = if h > 10 {
            9.0 + HudBatch::text_width(&self.gui.atlas, &format!("x{h}"))
        } else {
            h as f32 * 9.0
        };
        let sw = if s > 10 || s == 0 {
            7.0 + HudBatch::text_width(&self.gui.atlas, &format!("x{s}"))
        } else {
            s as f32 * 7.0
        };
        Some([hw.max(sw), 20.0])
    }
    pub(in crate::game) fn appleskin_tooltip_draw(
        &self,
        batch: &mut HudBatch,
        stack: &ItemStack,
        x: f32,
        y: f32,
    ) {
        let Some(food) = self.appleskin_food(stack) else {
            return;
        };
        let atlas = &self.gui.atlas;
        let hunger = (food.nutrition as f32 / 2.0).ceil().abs() as i32;
        let count = if hunger > 10 { 1 } else { hunger };
        for n in 0..count {
            let xx = x + ((count - 1 - n) * 9) as f32;
            hunger_icon(batch, atlas, xx, y, false, "empty", 1.0);
            hunger_icon(
                batch,
                atlas,
                xx,
                y,
                food.rotten,
                if food.nutrition - 1 == n * 2 {
                    "half"
                } else {
                    "full"
                },
                1.0,
            );
        }
        if hunger > 10 {
            tooltip_multiple(batch, atlas, &format!("x{hunger}"), x + 9.0, y, 2.0);
        }
        let sat = (food.saturation.abs() / 2.0).ceil() as i32;
        let count = if sat > 10 || sat == 0 { 1 } else { sat };
        for n in 0..count {
            let effective = (food.saturation.abs() - (n * 2) as f32) / 2.0;
            let u = if effective >= 1.0 {
                21.0
            } else if effective > 0.5 {
                14.0
            } else if effective > 0.25 {
                7.0
            } else if effective > 0.0 {
                0.0
            } else {
                28.0
            };
            icon(
                batch,
                atlas,
                x + ((count - 1 - n) * 7) as f32,
                y + 10.0,
                u,
                if food.saturation >= 0.0 { 27.0 } else { 34.0 },
                7.0,
                7.0,
                if food.saturation.abs() <= (n * 2) as f32 {
                    0.5
                } else {
                    1.0
                },
            );
        }
        if sat > 10 || sat == 0 {
            tooltip_multiple(
                batch,
                atlas,
                &format!("x{}", if food.saturation < 0.0 { -sat } else { sat }),
                x + 7.0,
                y + 10.0,
                1.0,
            );
        }
    }
    pub(in crate::game) fn appleskin_payload(&mut self, channel: &str, data: &[u8]) {
        if !self.atlas_port_on("atlas-appleskin") {
            return;
        }
        if channel == "appleskin:natural_regeneration" && data.len() == 1 {
            self.atlas_ports
                .food
                .natural_regeneration
                .set(Some(data[0] != 0));
        } else if data.len() == 4 {
            let value = f32::from_be_bytes(data.try_into().unwrap());
            if !value.is_finite() {
                return;
            }
            if let Some(player) = self.player.as_mut() {
                match channel {
                    "appleskin:exhaustion" => {
                        player.health.food.exhaustion = value.clamp(0.0, 40.0)
                    }
                    "appleskin:saturation" => {
                        player.health.food.saturation = value.clamp(0.0, 20.0)
                    }
                    _ => {}
                }
            }
        }
    }
}
fn white(alpha: f32) -> u32 {
    rgba(255, 255, 255, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}
fn icon(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    w: f32,
    h: f32,
    alpha: f32,
) {
    if let Some(mut r) = atlas.sprites.get("atlas/appleskin-icons").copied() {
        let du = (r.u1 - r.u0) / 256.0;
        let dv = (r.v1 - r.v0) / 256.0;
        r.u0 += u * du;
        r.v0 += v * dv;
        r.u1 = r.u0 + w * du;
        r.v1 = r.v0 + h * dv;
        batch.rect_uv(x, y, w, h, r, white(alpha));
    }
}
fn saturation(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    offsets: &[(f32, f32)],
    current: f32,
    gained: f32,
    alpha: f32,
) {
    if current + gained < 0.0 {
        return;
    }
    let total = (current + gained).clamp(0.0, 20.0);
    let start = if gained == 0.0 {
        0
    } else {
        (current / 2.0).max(0.0) as usize
    };
    for (n, &(x, y)) in offsets
        .iter()
        .enumerate()
        .take((total / 2.0).ceil() as usize)
        .skip(start)
    {
        icon(
            batch,
            atlas,
            x,
            y,
            logic::saturation_sprite(total / 2.0 - n as f32),
            0.0,
            9.0,
            9.0,
            alpha,
        );
    }
}
fn hunger_icon(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    rotten: bool,
    part: &str,
    alpha: f32,
) {
    batch.sprite_scaled(
        atlas,
        &format!(
            "gui/sprites/hud/food_{part}{}",
            if rotten { "_hunger" } else { "" }
        ),
        x,
        y,
        9.0,
        9.0,
        white(alpha),
    );
}
fn tooltip_multiple(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    label: &str,
    x: f32,
    y: f32,
    local_y: f32,
) {
    let start = batch.vertices.len();
    batch.text(
        atlas,
        label,
        x + 2.0,
        y + local_y,
        rgba(170, 170, 170, 255),
        true,
    );
    for vertex in &mut batch.vertices[start..] {
        vertex.pos[0] = x + (vertex.pos[0] - x) * 0.75;
        vertex.pos[1] = y + (vertex.pos[1] - y) * 0.75;
    }
}
