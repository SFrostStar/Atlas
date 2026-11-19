//! Atlas implementation of configurable totem presentation.
//! Compatibility reference: Totem Tweaks 1.0.1 by RacialGamer.
//! Original reference license is recorded in MOD-PORT-NOTICES.md.
use super::{
    Game,
    config_model::{Category, Definition, Field, Frame, Kind},
};
use atlas_assets::item_model::DisplayContext;
use atlas_client::{
    hud::{HudBatch, rgba},
    item_render::{self, ItemQuad},
    items::ItemStack,
    lang,
};
use atlas_core::geometry::trig::Mth;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::Cell,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) const ID: &str = "atlas-totemtweaks";
const TOTEM: &str = "minecraft:totem_of_undying";
const FILE: &str = "config/totemtweaks.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Config {
    totem_size: f32,
    pop_size: f32,
    disable_equip_animation: bool,
    #[serde(rename = "TotemPopAnimation")]
    totem_pop_animation: bool,
    animation_speed: i32,
    lock_rotation_position: bool,
    disable_rotations: bool,
    static_size: bool,
    x_position: i32,
    y_position: i32,
    show_totem_count: bool,
    show_overlay: bool,
    overlay_color: i32,
    overlay_opacity: i32,
    enable_totem_size_change: bool,
    totem_size_change_speed: f32,
    min_totem_size: f32,
    max_totem_size: f32,
    enable_totem_pop_size_change: bool,
    totem_pop_size_change_speed: f32,
    min_totem_pop_size: f32,
    max_totem_pop_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            totem_size: 1.0,
            pop_size: 0.3,
            disable_equip_animation: false,
            totem_pop_animation: true,
            animation_speed: 40,
            lock_rotation_position: false,
            disable_rotations: false,
            static_size: false,
            x_position: 50,
            y_position: 50,
            show_totem_count: false,
            show_overlay: false,
            overlay_color: 0xFF0000,
            overlay_opacity: 128,
            enable_totem_size_change: false,
            totem_size_change_speed: 1.0,
            min_totem_size: 0.5,
            max_totem_size: 1.0,
            enable_totem_pop_size_change: false,
            totem_pop_size_change_speed: 1.0,
            min_totem_pop_size: 0.3,
            max_totem_pop_size: 0.9,
        }
    }
}

impl Config {
    fn finite(&self) -> bool {
        [
            self.totem_size,
            self.pop_size,
            self.totem_size_change_speed,
            self.min_totem_size,
            self.max_totem_size,
            self.totem_pop_size_change_speed,
            self.min_totem_pop_size,
            self.max_totem_pop_size,
        ]
        .iter()
        .all(|v| v.is_finite())
    }

    fn held_scale(&self, millis: u64) -> f32 {
        if self.enable_totem_size_change {
            oscillation(
                millis,
                self.totem_size_change_speed,
                self.min_totem_size,
                self.max_totem_size,
            )
        } else {
            self.totem_size
        }
    }

    fn popup_pose(
        &self,
        ticks: i32,
        partial: f32,
        offsets: [f32; 2],
        size: [f32; 2],
        millis: u64,
    ) -> Option<Mat4> {
        if ticks <= 0 || self.animation_speed == 0 {
            return None;
        }
        let elapsed = self.animation_speed.wrapping_sub(ticks);
        let t = ((elapsed as f32 + partial) / 40.0) * 40.0 / self.animation_speed as f32;
        let t2 = t * t;
        let t3 = t * t2;
        let angle = (10.25 * t3 * t2 - 24.95 * t2 * t2 + 25.5 * t3 - 13.8 * t2 + 4.0 * t)
            * std::f32::consts::PI;
        let swing = (angle * 2.0).msin().abs();
        let [width, height] = size.map(|n| n as i32);
        let x = (width / 2) as f32
            + offsets[0] * (width / 4) as f32 * swing
            + (self.x_position.wrapping_sub(50)) as f32 / 100.0 * size[0];
        let y = (height / 2) as f32
            + offsets[1] * (height / 4) as f32 * swing
            + (self.y_position.wrapping_sub(50)) as f32 / 100.0 * size[1];
        let animated = 50.0 + 175.0 * angle.msin();
        let scale = if self.enable_totem_pop_size_change {
            animated
                * oscillation(
                    millis,
                    self.totem_pop_size_change_speed,
                    self.min_totem_pop_size,
                    self.max_totem_pop_size,
                )
        } else if self.static_size {
            225.0 * self.pop_size
        } else {
            animated * self.pop_size
        };
        let mut pose = Mat4::from_translation(Vec3::new(x, y, -50.0))
            * Mat4::from_scale(Vec3::new(scale, -scale, scale));
        if !self.disable_rotations {
            pose = pose
                * Mat4::from_rotation_y((900.0 * angle.msin().abs()).to_radians())
                * Mat4::from_rotation_x((6.0 * (t * 8.0).mcos()).to_radians())
                * Mat4::from_rotation_z((6.0 * (t * 8.0).mcos()).to_radians());
        }
        Some(pose)
    }
}

fn oscillation(millis: u64, speed: f32, min: f32, max: f32) -> f32 {
    let phase =
        (atlas_core::jmath::sin(millis as f64 / 1000.0 * f64::from(speed)) / 2.0 + 0.5) as f32;
    phase * (max - min) + min
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Default)]
pub(super) struct State {
    config: Config,
    overlay_left: Cell<i32>,
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        let mut state = Self::default();
        let path = home.join(FILE);
        match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Config>(&bytes) {
                Ok(config) if config.finite() => state.config = config,
                _ => log::warn!("Totem settings could not be parsed: {}", path.display()),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if let Err(err) = state.persist(home) {
                    log::warn!("Totem settings could not be saved: {err}");
                }
            }
            Err(err) => log::warn!("Totem settings could not be read: {err}"),
        }
        state
    }

    fn persist(&self, home: &Path) -> Result<(), String> {
        let path = home.join(FILE);
        let bytes = serde_json::to_vec_pretty(&self.config).map_err(|err| err.to_string())?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|err| err.to_string())?;
        fs::write(path, bytes).map_err(|err| err.to_string())
    }

    pub(super) fn config_value(&self) -> Value {
        serde_json::from_str(&serde_json::to_string(&self.config).unwrap()).unwrap()
    }

    pub(super) fn default_config() -> Value {
        serde_json::from_str(&serde_json::to_string(&Config::default()).unwrap()).unwrap()
    }

    pub(super) fn set_config(&mut self, home: &Path, value: Value) -> Result<(), String> {
        let config: Config = serde_json::from_value(value).map_err(|err| err.to_string())?;
        if !config.finite() {
            return Err(lang::get("atlas.ports.invalid_config"));
        }
        let proposed = Self {
            config,
            overlay_left: Cell::new(self.overlay_left.get()),
        };
        proposed.persist(home)?;
        self.config = proposed.config;
        Ok(())
    }

    fn overlay_alpha(&self, floating_totem: bool, ticks: i32) -> Option<u8> {
        let mut alpha = None;
        if self.config.show_totem_count
            && floating_totem
            && self.config.show_overlay
            && self.overlay_left.get() > 0
        {
            let remaining = self.overlay_left.get();
            // Compatibility behavior: the reference couples the color effect to the counter.
            let value = f64::from(self.config.overlay_opacity) / 255.0
                * f64::from(remaining as f32 / self.config.animation_speed as f32)
                * 255.0;
            alpha = Some((value as i32) as u8);
            self.overlay_left.set(remaining - 1);
        }
        if self.config.show_overlay {
            self.overlay_left.set(ticks);
        }
        alpha
    }
}

impl Game<'_> {
    pub(in crate::game) fn atlas_totem_tweaks_enabled(&self) -> bool {
        self.atlas_port_on(ID)
    }

    pub(in crate::game) fn atlas_totem_tweaks_activated(&mut self) {
        if !self.atlas_totem_tweaks_enabled() {
            return;
        }
        let state = &self.atlas_ports.totem_tweaks;
        let config = &state.config;
        self.item_activation.configure(
            if config.totem_pop_animation {
                config.animation_speed
            } else {
                0
            },
            config.lock_rotation_position,
        );
        if config.show_overlay {
            state.overlay_left.set(config.animation_speed);
        }
    }

    pub(in crate::game) fn atlas_totem_tweaks_equip(
        &self,
        hand: atlas_client::first_person::Hand,
        original: f32,
    ) -> f32 {
        use atlas_client::first_person::Hand;
        let actual = match hand {
            Hand::Main => self.held(),
            Hand::Off => self.offhand.as_ref(),
        };
        if self.atlas_totem_tweaks_enabled()
            && self.atlas_ports.totem_tweaks.config.disable_equip_animation
            && actual.is_some_and(|item| &*item.item == TOTEM)
        {
            0.0
        } else {
            original
        }
    }

    pub(in crate::game) fn atlas_totem_tweaks_hand_scale(&self, item: &ItemStack) -> f32 {
        if self.atlas_totem_tweaks_enabled() && &*item.item == TOTEM {
            self.atlas_ports
                .totem_tweaks
                .config
                .held_scale(now_millis())
        } else {
            1.0
        }
    }

    pub(in crate::game) fn atlas_totem_tweaks_draw(&self, batch: &mut HudBatch, partial: f32) {
        if !self.atlas_totem_tweaks_enabled() || self.ui.hide_gui {
            return;
        }
        let state = &self.atlas_ports.totem_tweaks;
        let (ticks, offsets) = self.item_activation.parameters();
        let item = self.item_activation.current_item();
        if let Some(item) = item
            && let Some(pose) = state.config.popup_pose(
                ticks,
                partial,
                offsets,
                [batch.width, batch.height],
                now_millis(),
            )
        {
            let resolved = self.resolve_item(item, DisplayContext::Fixed, None, 0);
            let mut quads: Vec<ItemQuad> = Vec::new();
            resolved.submit(
                &self.pack.item_renderer,
                &self.gui.atlas.sprites,
                &self.item_host(),
                pose,
                &mut quads,
            );
            let rect = [0.0, 0.0, batch.width, batch.height];
            item_render::gui::draw_quads(
                &quads,
                item_render::gui::items_3d_lights(),
                batch,
                rect,
                self.glint().as_ref(),
            );
        }
        let floating_totem = item.is_some_and(|item| &*item.item == TOTEM);
        if state.config.show_totem_count && floating_totem {
            let count = self
                .inventory
                .iter()
                .flatten()
                .filter(|item| &*item.item == TOTEM)
                .map(|item| item.count)
                .sum::<u32>();
            let text =
                atlas_client::text::Text::tr("atlas.totem_tweaks.count", &[&count.to_string()])
                    .plain();
            let x = ((batch.width - HudBatch::text_width(&self.gui.atlas, &text)) / 2.0).floor();
            batch.text(
                &self.gui.atlas,
                &text,
                x,
                (batch.height as i32 / 2 + 20) as f32,
                u32::MAX,
                true,
            );
        }
        if let Some(alpha) = state.overlay_alpha(floating_totem, ticks) {
            let color = state.config.overlay_color as u32;
            batch.fill(
                &self.gui.atlas,
                0.0,
                0.0,
                batch.width,
                batch.height,
                rgba((color >> 16) as u8, (color >> 8) as u8, color as u8, alpha),
            );
        }
    }

    pub(in crate::game) fn atlas_totem_tweaks_command(&mut self, command: &str) -> bool {
        if !self.atlas_totem_tweaks_enabled() {
            return false;
        }
        match command.trim_start_matches('/').trim() {
            "totemTweaks" => {
                self.atlas_config_open(ID);
                true
            }
            "simulatePop" => {
                if self.player.is_some() {
                    self.item_activation
                        .activate(ItemStack::new(TOTEM, 1), self.particles.random());
                    self.atlas_totem_tweaks_activated();
                }
                true
            }
            _ => false,
        }
    }
}

pub(super) fn definition() -> Definition {
    fn field(name: &str, fallback: &str, kind: Kind) -> Field {
        let mut field = Field::new(&format!("/{name}"), "", kind);
        field.label = lang::get_or(&format!("atlas.totem_tweaks.{name}"), fallback);
        field
    }
    fn boolean(name: &str, fallback: &str) -> Field {
        field(name, fallback, Kind::Bool)
    }
    fn float(name: &str, fallback: &str) -> Field {
        field(
            name,
            fallback,
            Kind::Float {
                min: -(f32::MAX as f64),
                max: f32::MAX as f64,
                step: 0.0,
                slider: false,
            },
        )
    }
    fn slider(name: &str, fallback: &str, max: i64) -> Field {
        field(
            name,
            fallback,
            Kind::Int {
                min: 0,
                max,
                step: 1,
                slider: true,
            },
        )
    }
    fn category(key: &str, fallback: &str, fields: Vec<Field>) -> Category {
        Category {
            label: lang::get_or(&format!("atlas.totem_tweaks.{key}"), fallback),
            fields,
        }
    }
    Definition {
        title: lang::get_or("atlas.totem_tweaks.title", "Totem Tweaks"),
        frame: Frame::Yacl,
        categories: vec![
            category(
                "held",
                "Held totem",
                vec![
                    float("totemSize", "Held size"),
                    boolean("disableEquipAnimation", "Instant equip"),
                    boolean("enableTotemSizeChange", "Animate held size"),
                    float("totemSizeChangeSpeed", "Held animation speed"),
                    float("minTotemSize", "Minimum held size"),
                    float("maxTotemSize", "Maximum held size"),
                ],
            ),
            category(
                "popup",
                "Pop animation",
                vec![
                    boolean("TotemPopAnimation", "Show pop animation"),
                    float("popSize", "Pop size"),
                    slider("animationSpeed", "Duration in ticks", 100),
                    boolean("lockRotationPosition", "Keep pop centered"),
                    boolean("disableRotations", "Disable pop rotations"),
                    boolean("staticSize", "Use fixed pop size"),
                    slider("xPosition", "Horizontal position", 100),
                    slider("yPosition", "Vertical position", 100),
                ],
            ),
            category(
                "overlay",
                "HUD",
                vec![
                    boolean("showTotemCount", "Show inventory totem count"),
                    boolean("showOverlay", "Show pop color overlay"),
                    field(
                        "overlayColor",
                        "Overlay color",
                        Kind::Color { alpha: false },
                    ),
                    slider("overlayOpacity", "Overlay opacity", 255),
                ],
            ),
            category(
                "size_cycle",
                "Pop size cycle",
                vec![
                    boolean("enableTotemPopSizeChange", "Animate pop size"),
                    float("totemPopSizeChangeSpeed", "Pop size cycle speed"),
                    float("minTotemPopSize", "Minimum pop size"),
                    float("maxTotemPopSize", "Maximum pop size"),
                ],
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_roundtrip_and_import_keeps_reference_keys() {
        let value = State::default_config();
        assert_eq!(value.as_object().unwrap().len(), 22);
        assert_eq!(value["TotemPopAnimation"], true);
        assert_eq!(value["popSize"], 0.3);
        assert_eq!(value["animationSpeed"], 40);
        let config: Config = serde_json::from_value(value).unwrap();
        assert!(config.finite());
        assert_eq!(config.held_scale(1234), 1.0);
    }

    #[test]
    fn static_pop_size_and_position_are_independent_from_elapsed_ticks() {
        let config = Config {
            static_size: true,
            disable_rotations: true,
            x_position: 25,
            y_position: 75,
            ..Config::default()
        };
        let first = config
            .popup_pose(40, 0.0, [0.0; 2], [640.0, 360.0], 0)
            .unwrap();
        let last = config
            .popup_pose(1, 0.0, [0.0; 2], [640.0, 360.0], 0)
            .unwrap();
        assert_eq!(first, last);
        assert_eq!(first.w_axis.truncate(), Vec3::new(160.0, 270.0, -50.0));
        assert_eq!(first.x_axis.x, 67.5);
        assert_eq!(first.y_axis.y, -67.5);
    }

    #[test]
    fn size_cycle_uses_wall_clock_and_overlay_follows_the_counter_dependency() {
        let config = Config {
            enable_totem_size_change: true,
            show_overlay: true,
            ..Config::default()
        };
        assert_eq!(config.held_scale(0), 0.75);
        let mut state = State {
            config,
            ..State::default()
        };
        state.overlay_left.set(40);
        assert_eq!(state.overlay_alpha(true, 39), None);
        assert_eq!(state.overlay_left.get(), 39);
        state.config.show_totem_count = true;
        assert_eq!(state.overlay_alpha(true, 38), Some(124));
        assert_eq!(state.overlay_left.get(), 38);
        assert_eq!(state.overlay_alpha(false, 37), None);
    }
}
