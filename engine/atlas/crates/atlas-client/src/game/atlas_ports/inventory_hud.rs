//! Inventory HUD + 3.4.27 behavior, drawn with Atlas and vanilla UI primitives.
use super::{
    Game,
    config_model::{Category, Definition, Field, Frame, Kind},
};
use atlas_client::{
    brewing,
    hud::{HudBatch, rgba},
    items::ItemStack,
    keybinds, lang,
    ui_theme::{Face, Ui, color},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, fs, path::Path, sync::Arc};
use winit::keyboard::KeyCode;
#[path = "inventory_hud_layout.rs"]
mod layout;
pub(super) use layout::PositionEditor;

pub(super) const ID: &str = "atlas-inventoryhud";
const FILE: &str = "config/inventoryhud.json";
const KEYS: [(&str, &str); 5] = [
    ("toggle_inventory", "key.keyboard.i"),
    ("open_config", "key.keyboard.o"),
    ("toggle_effects", "key.keyboard.unknown"),
    ("toggle_armor", "key.keyboard.unknown"),
    ("toggle_all", "key.keyboard.unknown"),
];
const EQUIPMENT: [&str; 8] = [
    "helmet",
    "chestplate",
    "leggings",
    "boots",
    "offhand",
    "mainhand",
    "free_slots",
    "arrows",
];

pub(super) fn keys() -> Vec<[String; 3]> {
    KEYS.into_iter()
        .map(|(id, key)| [id.into(), label(id), key.into()])
        .collect()
}

fn label(name: &str) -> String {
    lang::get(&format!("atlas.inventory_hud.{name}"))
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum Horizontal {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum Vertical {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
struct Place {
    horizontal: Horizontal,
    vertical: Vertical,
    x: i32,
    y: i32,
}

impl Place {
    const fn bottom(x: i32, y: i32) -> Self {
        Self {
            horizontal: Horizontal::Middle,
            vertical: Vertical::Bottom,
            x,
            y,
        }
    }

    fn raw(self, window: [f32; 2], size: [f32; 2]) -> [f32; 2] {
        let [w, h] = window;
        let [width, height] = size;
        let x = self.x as f32;
        let y = self.y as f32;
        [
            match self.horizontal {
                Horizontal::Left => x,
                Horizontal::Middle => ((w - width) / 2.0).floor() - x,
                Horizontal::Right => w - x,
            },
            match self.vertical {
                Vertical::Top => y,
                Vertical::Center => ((h - height) / 2.0).floor() - y,
                Vertical::Bottom => h - y,
            },
        ]
    }

    fn clamped(self, window: [f32; 2], size: [f32; 2]) -> [f32; 2] {
        let [x, y] = self.raw(window, size);
        [
            x.clamp(0.0, (window[0] - size[0]).max(0.0)),
            y.clamp(0.0, (window[1] - size[1]).max(0.0)),
        ]
    }

    fn on_right(self) -> bool {
        self.horizontal == Horizontal::Right
            || (self.horizontal == Horizontal::Middle && self.x > 0)
    }

    fn valid(self) -> bool {
        (-4096..=4096).contains(&self.x) && (-4096..=4096).contains(&self.y)
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
struct Inventory {
    enabled: bool,
    vertical: bool,
    compact: bool,
    opacity: u8,
    hide_empty: bool,
    animated: bool,
    debug: bool,
    position: Place,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            enabled: true,
            vertical: false,
            compact: false,
            opacity: 0,
            hide_empty: false,
            animated: false,
            debug: true,
            position: Place::bottom(0, 150),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
struct Effects {
    enabled: bool,
    compact: bool,
    horizontal: bool,
    opacity: u8,
    gap: i8,
    bar_seconds: u32,
    hidden: bool,
    high_levels: bool,
    debug: bool,
    position: Place,
    blocked: BTreeMap<String, bool>,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            enabled: false,
            compact: false,
            horizontal: false,
            opacity: 100,
            gap: 0,
            bar_seconds: 300,
            hidden: true,
            high_levels: false,
            debug: true,
            position: Place {
                horizontal: Horizontal::Left,
                vertical: Vertical::Center,
                x: 30,
                y: 0,
            },
            blocked: brewing::all_effects()
                .map(|effect| (effect.id.into(), false))
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Durability {
    Percentage,
    Damage,
    DamageLeft,
    Off,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
struct Equipment {
    enabled: bool,
    armor: bool,
    mainhand: bool,
    offhand: bool,
    free_slots: bool,
    arrows: bool,
    arrows_without_weapon: bool,
    show_empty: bool,
    counts: bool,
    bars: bool,
    debug: bool,
    move_together: bool,
    max_remaining: u8,
    scale: u8,
    durability: Durability,
    position: Place,
    slots: [Place; 8],
}

impl Default for Equipment {
    fn default() -> Self {
        Self {
            enabled: false,
            armor: true,
            mainhand: true,
            offhand: true,
            free_slots: true,
            arrows: true,
            arrows_without_weapon: true,
            show_empty: true,
            counts: false,
            bars: false,
            debug: true,
            move_together: true,
            max_remaining: 100,
            scale: 100,
            durability: Durability::Percentage,
            position: Place::bottom(0, 70),
            slots: [
                Place::bottom(103, 54),
                Place::bottom(103, 37),
                Place::bottom(-103, 54),
                Place::bottom(-103, 37),
                Place::bottom(-103, 71),
                Place::bottom(103, 71),
                Place::bottom(-103, 20),
                Place::bottom(103, 20),
            ],
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct Config {
    inventory: Inventory,
    effects: Effects,
    equipment: Equipment,
}

impl Config {
    fn normalize(mut self) -> Option<Self> {
        if self.inventory.opacity > 100
            || self.effects.opacity > 100
            || !(-5..=5).contains(&self.effects.gap)
            || self.effects.bar_seconds == 0
            || !matches!(self.equipment.scale, 50 | 75 | 100 | 125 | 150)
            || self.equipment.max_remaining > 100
            || !self.inventory.position.valid()
            || !self.effects.position.valid()
            || !self.equipment.position.valid()
            || self.equipment.slots.iter().any(|slot| !slot.valid())
        {
            return None;
        }
        for effect in brewing::all_effects() {
            self.effects
                .blocked
                .entry(effect.id.into())
                .or_insert(false);
        }
        Some(self)
    }
}

#[derive(Default)]
struct ItemPulse {
    item: Option<Arc<str>>,
    count: u32,
    started: Option<u64>,
}

impl ItemPulse {
    fn update(&mut self, stack: Option<&ItemStack>, tick: u64, animate_gain: bool) {
        if let Some(stack) = stack {
            let changed = self.item.as_ref() != Some(&stack.item);
            if animate_gain && (changed || self.count < stack.count) {
                self.started = Some(tick);
            }
            if changed {
                self.item = Some(Arc::clone(&stack.item));
            }
            self.count = stack.count;
        } else {
            *self = Self::default();
        }
    }

    fn scale(&self, tick: u64, fraction: f32) -> f32 {
        let left = self.started.map_or(0.0, |start| {
            5.0 - tick.saturating_sub(start) as f32 - fraction
        });
        1.0 + left.max(0.0) / 10.0
    }
}

#[derive(Default)]
pub(super) struct State {
    config: Config,
    inventory_pulses: RefCell<Option<[ItemPulse; 27]>>,
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        let config = fs::read(home.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Config>(&bytes).ok())
            .and_then(Config::normalize)
            .unwrap_or_default();
        Self {
            config,
            ..Self::default()
        }
    }

    pub(super) fn config_value(&self) -> Value {
        serde_json::to_value(&self.config).expect("inventory HUD configuration is serializable")
    }

    pub(super) fn default_config() -> Value {
        serde_json::to_value(Config::default()).expect("inventory HUD defaults are serializable")
    }

    pub(super) fn set_config(&mut self, home: &Path, value: Value) -> Result<(), String> {
        let config = serde_json::from_value::<Config>(value)
            .map_err(|e| e.to_string())?
            .normalize()
            .ok_or_else(|| lang::get("atlas.ports.invalid_config"))?;
        self.save(home, config)
    }

    fn save(&mut self, home: &Path, config: Config) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        fs::create_dir_all(home.join("config"))
            .and_then(|()| fs::write(home.join(FILE), bytes))
            .map_err(|e| e.to_string())?;
        self.config = config;
        Ok(())
    }
}

fn setting(path: &str, name: &str, kind: Kind) -> Field {
    Field::new(path, &format!("atlas.inventory_hud.{name}"), kind)
}

fn number(path: &str, name: &str, min: i64, max: i64, slider: bool) -> Field {
    setting(
        path,
        name,
        Kind::Int {
            min,
            max,
            step: 1,
            slider,
        },
    )
}

fn enumerate(path: &str, name: &str, modes: &[(&str, &str)]) -> Field {
    setting(
        path,
        name,
        Kind::Enum {
            values: modes.iter().map(|(id, _)| Value::from(*id)).collect(),
            labels: modes.iter().map(|(_, name)| label(name)).collect(),
        },
    )
}

fn position(path: &str, name: &str) -> Field {
    setting(
        path,
        name,
        Kind::Group {
            fields: vec![
                number(&format!("{path}/x"), "offset_x", -4096, 4096, false),
                number(&format!("{path}/y"), "offset_y", -4096, 4096, false),
                enumerate(
                    &format!("{path}/horizontal"),
                    "anchor_x",
                    &[("Left", "left"), ("Middle", "center"), ("Right", "right")],
                ),
                enumerate(
                    &format!("{path}/vertical"),
                    "anchor_y",
                    &[("Top", "top"), ("Center", "center"), ("Bottom", "bottom")],
                ),
            ],
        },
    )
}

fn bools(category: &str, options: &[&str]) -> Vec<Field> {
    options
        .iter()
        .map(|name| setting(&format!("/{category}/{name}"), name, Kind::Bool))
        .collect()
}

pub(super) fn definition() -> Definition {
    let mut inventory = bools(
        "inventory",
        &[
            "enabled",
            "vertical",
            "compact",
            "animated",
            "hide_empty",
            "debug",
        ],
    );
    inventory.extend([
        number("/inventory/opacity", "opacity", 0, 100, true),
        position("/inventory/position", "position"),
    ]);
    let mut effects = bools(
        "effects",
        &[
            "enabled",
            "horizontal",
            "compact",
            "hidden",
            "high_levels",
            "debug",
        ],
    );
    effects.extend([
        number("/effects/opacity", "opacity", 0, 100, true),
        number("/effects/gap", "gap", -5, 5, false),
        number(
            "/effects/bar_seconds",
            "bar_seconds",
            1,
            i64::from(i32::MAX),
            false,
        ),
        position("/effects/position", "position"),
        setting(
            "/effects/blocked",
            "blacklist",
            Kind::Group {
                fields: brewing::all_effects()
                    .map(|effect| {
                        let pointer = effect.id.replace('~', "~0").replace('/', "~1");
                        Field::new(
                            &format!("/effects/blocked/{pointer}"),
                            &brewing::description_id(effect.id),
                            Kind::Bool,
                        )
                    })
                    .collect(),
            },
        ),
    ]);
    let mut equipment = bools(
        "equipment",
        &[
            "enabled",
            "armor",
            "mainhand",
            "offhand",
            "free_slots",
            "arrows",
            "arrows_without_weapon",
            "show_empty",
            "counts",
            "bars",
            "debug",
            "move_together",
        ],
    );
    equipment.extend([
        enumerate(
            "/equipment/durability",
            "durability",
            &[
                ("percentage", "percent"),
                ("damage", "damage"),
                ("damage-left", "remaining"),
                ("off", "off"),
            ],
        ),
        number("/equipment/max_remaining", "max_remaining", 0, 100, false),
        setting(
            "/equipment/scale",
            "scale",
            Kind::Int {
                min: 50,
                max: 150,
                step: 25,
                slider: true,
            },
        ),
        position("/equipment/position", "position"),
        setting(
            "/equipment/slots",
            "individual_positions",
            Kind::Group {
                fields: EQUIPMENT
                    .into_iter()
                    .enumerate()
                    .map(|(index, name)| position(&format!("/equipment/slots/{index}"), name))
                    .collect(),
            },
        ),
    ]);
    for (category, fields) in [
        ("inventory", &mut inventory),
        ("effects", &mut effects),
        ("equipment", &mut equipment),
    ] {
        fields.insert(
            0,
            setting(
                &format!("/{category}/arrange"),
                "position",
                Kind::Action {
                    action: "inventoryhud.arrange".into(),
                    button: label("edit_positions"),
                },
            ),
        );
    }
    Definition {
        title: label("title"),
        frame: Frame::Yacl,
        categories: [
            ("inventory", inventory),
            ("effects", effects),
            ("equipment", equipment),
        ]
        .into_iter()
        .map(|(name, fields)| Category {
            label: label(name),
            fields,
        })
        .collect(),
    }
}

impl Game<'_> {
    pub(in crate::game) fn atlas_inventory_hud_key(&mut self, key: KeyCode) -> bool {
        if !self.atlas_port_on(ID) || !self.native_playing() || self.player.is_none() {
            return false;
        }
        let Some((binding, _)) = KEYS.into_iter().find(|(name, default)| {
            keybinds::key_code(&self.native_mod_key(ID, name, default)) == Some(key)
        }) else {
            return false;
        };
        if binding == "open_config" {
            return self.atlas_config_open(ID);
        }
        let state = &mut self.atlas_ports.inventory_hud;
        let mut config = state.config.clone();
        match binding {
            "toggle_inventory" => config.inventory.enabled = !config.inventory.enabled,
            "toggle_effects" => config.effects.enabled = !config.effects.enabled,
            "toggle_armor" => config.equipment.enabled = !config.equipment.enabled,
            _ => {
                let turn_on = !(config.inventory.enabled
                    || config.effects.enabled
                    || config.equipment.enabled);
                config.inventory.enabled = turn_on;
                config.effects.enabled = turn_on;
                config.equipment.enabled = turn_on;
            }
        }
        if let Err(error) = state.save(&Self::game_dir(), config) {
            log::warn!("Inventory HUD settings could not be saved: {error}");
        }
        true
    }

    pub(in crate::game) fn atlas_inventory_replaces_effect_icons(&self) -> bool {
        self.native_window()
            && self.atlas_port_on(ID)
            && self.atlas_ports.inventory_hud.config.effects.enabled
    }

    pub(in crate::game) fn draw_inventory_hud(&self, batch: &mut HudBatch) {
        if !self.atlas_port_on(ID) || self.player.is_none() {
            return;
        }
        let state = &self.atlas_ports.inventory_hud;
        let config = &state.config;
        if config.effects.enabled && (!self.debug || config.effects.debug) {
            self.draw_inventory_hud_effects(batch, &config.effects);
        }
        if config.equipment.enabled && (!self.debug || config.equipment.debug) {
            self.draw_inventory_hud_armor(batch, &config.equipment);
        }
        let inventory_screen = matches!(
            self.screen,
            super::super::Screen::Inventory | super::super::Screen::Creative
        );
        if !inventory_screen && config.inventory.enabled && (!self.debug || config.inventory.debug)
        {
            self.draw_inventory_hud_slots(batch, state, &config.inventory);
        } else {
            state.inventory_pulses.borrow_mut().take();
        }
    }

    fn draw_inventory_hud_slots(&self, batch: &mut HudBatch, state: &State, config: &Inventory) {
        let scale = if config.compact { 0.75 } else { 1.0 };
        let (columns, rows) = if config.vertical { (3, 9) } else { (9, 3) };
        let size = [
            (18 * columns - 2) as f32 * scale,
            (18 * rows - 2) as f32 * scale,
        ];
        let [x, y] = config.position.clamped([batch.width, batch.height], size);
        let items = &self.inventory[9..36];
        let atlas = &self.gui.atlas;
        if config.opacity > 0 && (!config.hide_empty || items.iter().any(Option::is_some)) {
            let alpha = (u16::from(config.opacity) * 255 / 100) as u8;
            batch.fill(
                atlas,
                x - 3.0,
                y - 3.0,
                size[0] + 6.0,
                size[1] + 6.0,
                rgba(25, 29, 36, alpha),
            );
            for row in 0..rows {
                for column in 0..columns {
                    batch.fill(
                        atlas,
                        x + (column * 18) as f32 * scale,
                        y + (row * 18) as f32 * scale,
                        16.0 * scale,
                        16.0 * scale,
                        rgba(8, 11, 17, alpha / 2),
                    );
                }
            }
        }
        let mut pulses = state.inventory_pulses.borrow_mut();
        let tick = self.visual_ticks();
        let observed = pulses.is_some();
        for (index, stack) in items.iter().enumerate() {
            let pop = if config.animated {
                let pulse = &mut pulses
                    .get_or_insert_with(|| std::array::from_fn(|_| ItemPulse::default()))[index];
                pulse.update(stack.as_ref(), tick, observed);
                pulse.scale(tick, self.clock.fraction() as f32)
            } else {
                1.0
            };
            let Some(stack) = stack else {
                continue;
            };
            let [column, row] = inventory_cell(index, config.vertical);
            let ix = x + (column * 18) as f32 * scale;
            let iy = y + (row * 18) as f32 * scale;
            let before = batch.vertices.len();
            self.draw_item_model(batch, stack, ix, iy);
            if pop != 1.0 {
                batch.scale_since(before, ix + 12.0, iy + 12.0, pop, pop);
            }
            self.draw_item_decorations(batch, stack, ix, iy, None);
            if scale != 1.0 {
                batch.scale_since(before, ix, iy, scale, scale);
            }
        }
        if !config.animated {
            pulses.take();
        }
    }
}

fn inventory_cell(index: usize, vertical: bool) -> [usize; 2] {
    if vertical {
        [2 - index / 9, index % 9]
    } else {
        [index % 9, index / 9]
    }
}

impl Equipment {
    fn at_hotbar(&self) -> bool {
        self.move_together
            && self.position.horizontal == Horizontal::Middle
            && self.position.vertical == Vertical::Bottom
            && self.position.y <= 90
            && self.position.x.abs() <= 90
    }

    fn element(
        &self,
        window: [f32; 2],
        slot: usize,
        right_handed: bool,
        offhand_empty: bool,
    ) -> ([f32; 2], bool) {
        let scale = f32::from(self.scale) / 100.0;
        if !self.move_together {
            let position = self.slots[slot];
            return (
                position.clamped(window, [16.0 * scale; 2]),
                !position.on_right(),
            );
        }
        if !self.at_hotbar() {
            let on_right = self.position.on_right();
            let mut p = self.position.clamped(window, [90.0 * scale, 70.0 * scale]);
            let mut y = match slot {
                4 if self.mainhand => 18.0,
                5 => 0.0,
                6 | 7 => {
                    36.0 - 18.0 * f32::from(u8::from(!self.mainhand) + u8::from(!self.offhand))
                        + if slot == 7 && self.free_slots {
                            18.0
                        } else {
                            0.0
                        }
                }
                _ => (slot % 4 * 18) as f32,
            };
            if slot == 4 && !self.mainhand {
                y = 0.0;
            }
            let x = if on_right { 75.0 } else { 0.0 }
                + if slot >= 4 && self.armor {
                    if on_right { -42.0 } else { 42.0 }
                } else {
                    0.0
                };
            p[0] += x * scale;
            p[1] += y * scale;
            return (p, !on_right);
        }

        let left = matches!(slot, 0 | 1 | 5 | 7);
        let durability_off = self.durability == Durability::Off;
        let offhand_on_side = !offhand_empty && (left == right_handed);
        let offset = if !durability_off || offhand_on_side {
            if offhand_on_side && scale < 1.0 {
                26.0
            } else {
                (24.0 * scale).floor()
            }
        } else {
            0.0
        };
        let center = (window[0] / 2.0).floor();
        let (mut x, mut text_right) = if left {
            (
                center - 96.0 - (16.0 * scale).floor() - offset,
                !durability_off && !offhand_on_side,
            )
        } else {
            (center + 96.0 + offset, durability_off || offhand_on_side)
        };
        if slot < 6 && scale >= 1.0 {
            x = center
                + if left {
                    -96.0 - (16.0 * scale).floor()
                } else {
                    96.0
                };
            if !durability_off {
                let text_offset = (24.0 * scale).floor();
                x += if left { -text_offset } else { text_offset };
            }
            text_right = left != durability_off;
        }
        let y = [56.0, 38.0, 56.0, 38.0, 74.0, 74.0, 20.0, 20.0][slot]
            - if slot < 6 && !self.armor { 36.0 } else { 0.0 };
        ([x, (window[1] - y * scale).floor()], text_right)
    }
}

fn percent_left(max: u32, damage: u32) -> u32 {
    (u64::from(max.saturating_sub(damage)) * 100 / u64::from(max)) as u32
}

fn durability_label(
    mode: Durability,
    damage: u32,
    max: u32,
    percent: u32,
) -> Option<(String, u32)> {
    let color = match percent {
        100 => rgba(85, 255, 85, 255),
        0 => rgba(170, 0, 0, 255),
        1..=10 => rgba(255, 85, 85, 255),
        11..=25 => rgba(255, 170, 0, 255),
        26..=50 => rgba(255, 255, 85, 255),
        _ => u32::MAX,
    };
    match mode {
        Durability::Percentage => Some((format!("{percent}%"), color)),
        Durability::Damage => Some((damage.to_string(), color)),
        Durability::DamageLeft => Some((max.saturating_sub(damage).to_string(), color)),
        Durability::Off => None,
    }
}

impl Game<'_> {
    fn draw_inventory_hud_armor(&self, batch: &mut HudBatch, config: &Equipment) {
        let _font_scope = atlas_client::font::java_mod_font_scope();
        let main = self.inventory[self.selected].as_ref();
        let offhand = self.offhand.as_ref();
        let weapon = [main, offhand]
            .into_iter()
            .flatten()
            .map(|item| &*item.item)
            .find(|id| matches!(*id, "minecraft:bow" | "minecraft:crossbow"));
        let atlas = &self.gui.atlas;
        let scale = f32::from(config.scale) / 100.0;
        let enabled = [config.armor; 4].into_iter().chain([
            config.offhand,
            config.mainhand,
            config.free_slots,
            config.arrows && (config.arrows_without_weapon || weapon.is_some()),
        ]);
        let stacks = self
            .armor
            .iter()
            .map(Option::as_ref)
            .chain([offhand, main, None, None]);
        for (index, (enabled, stack)) in enabled.zip(stacks).enumerate() {
            if !enabled {
                continue;
            }
            let ([x, y], text_right) = config.element(
                [batch.width, batch.height],
                index,
                !self.settings.main_hand_left,
                offhand.is_none(),
            );
            let start = batch.vertices.len();
            let mut text = None;
            if index == 6 {
                let free = self
                    .inventory
                    .iter()
                    .take(36)
                    .filter(|slot| slot.is_none())
                    .count();
                batch.sprite(atlas, "atlas/inventoryhud/backpack", x, y);
                text = Some((free.to_string(), u32::MAX));
            } else if index == 7 {
                let count = self
                    .inventory
                    .iter()
                    .take(36)
                    .filter_map(Option::as_ref)
                    .chain(offhand)
                    .filter(|stack| {
                        matches!(
                            &*stack.item,
                            "minecraft:arrow"
                                | "minecraft:spectral_arrow"
                                | "minecraft:tipped_arrow"
                        ) || self.items().item_in_tag("minecraft:arrows", &stack.item)
                            || weapon == Some("minecraft:crossbow")
                                && &*stack.item == "minecraft:firework_rocket"
                                && [offhand, main]
                                    .into_iter()
                                    .flatten()
                                    .any(|held| std::ptr::eq(*stack, held))
                    })
                    .map(|stack| u64::from(stack.count))
                    .sum::<u64>();
                batch.sprite(atlas, "atlas/inventoryhud/ammo", x, y);
                text = Some((count.to_string(), u32::MAX));
            } else {
                let damage = stack
                    .and_then(|item| self.items().get(&item.item).map(|info| info.max_damage))
                    .filter(|max| *max != 0);
                let visible = stack.filter(|item| {
                    damage.is_none_or(|max| {
                        percent_left(max, item.damage) <= u32::from(config.max_remaining)
                    })
                });
                if let Some(stack) = visible {
                    self.draw_item_model(batch, stack, x, y);
                    if let Some(max) = damage {
                        text = durability_label(
                            config.durability,
                            stack.damage,
                            max,
                            percent_left(max, stack.damage),
                        );
                        if config.bars {
                            self.draw_item_decorations(batch, stack, x, y, None);
                        }
                    } else if index >= 4 && config.counts {
                        let count = self
                            .inventory
                            .iter()
                            .take(36)
                            .filter_map(Option::as_ref)
                            .chain(offhand)
                            .filter(|other| stack.item == other.item)
                            .map(|other| u64::from(other.count))
                            .sum::<u64>();
                        text = Some((count.to_string(), u32::MAX));
                    } else {
                        self.draw_item_decorations(batch, stack, x, y, None);
                    }
                } else if config.show_empty && index >= 4 {
                    batch.fill(atlas, x, y, 16.0, 16.0, rgba(18, 22, 29, 155));
                    if index == 4 {
                        batch.sprite(atlas, "gui/sprites/container/slot/shield", x, y);
                    }
                }
            }
            if let Some((text, color)) = text {
                let tx = if text_right {
                    x + 17.0
                } else {
                    x - 1.0 - atlas.font.width(&text)
                };
                batch.text(atlas, &text, tx, y + 4.0, color, true);
            }
            if scale != 1.0 {
                batch.scale_since(start, x, y, scale, scale);
            }
        }
    }
}

fn remaining_time(ticks: i32) -> (String, u32) {
    let seconds = ticks / 20;
    if ticks < 0 || seconds >= 1600 {
        return ("**:**".into(), u32::MAX);
    }
    let color = if seconds <= 10 {
        rgba(255, 85, 85, 255)
    } else {
        u32::MAX
    };
    (format!("{:02}:{:02}", seconds / 60, seconds % 60), color)
}

impl Effects {
    fn dimensions(&self) -> [f32; 2] {
        match (self.horizontal, self.compact) {
            (true, true) => [18.0, 22.0],
            (true, false) => [38.0, 36.0],
            (false, true) => [22.0, 18.0],
            (false, false) => [60.0, 24.0],
        }
    }

    fn origin(&self, window: [f32; 2], count: usize) -> ([f32; 2], f32) {
        let size = self.dimensions();
        let [mut x, mut y] = self.position.raw(window, size);
        let gap = f32::from(self.gap);
        let additional = count.saturating_sub(1) as f32;
        if self.horizontal {
            let step = (size[0] - if self.compact { 0.0 } else { 2.0 }) + gap;
            x -= match self.position.horizontal {
                Horizontal::Right => (additional + 1.0) * step,
                Horizontal::Middle if self.position.x < 0 => additional * step,
                Horizontal::Middle if self.position.x == 0 => (additional * step / 2.0).floor(),
                _ => 0.0,
            };
            return ([x, y.clamp(0.0, (window[1] - size[1]).max(0.0))], step);
        }
        let step = size[1] + gap;
        if self.position.horizontal == Horizontal::Right {
            x -= size[0];
        }
        y -= match self.position.vertical {
            Vertical::Bottom => additional * step,
            Vertical::Center if self.position.y < 0 => additional * step,
            Vertical::Center if self.position.y == 0 => (additional * step / 2.0).floor(),
            _ => 0.0,
        };
        ([x.clamp(0.0, (window[0] - size[0]).max(0.0)), y], step)
    }
}

impl Game<'_> {
    fn draw_inventory_hud_effects(&self, batch: &mut HudBatch, config: &Effects) {
        let filtered = self.potions.effects.list.iter().filter(|effect| {
            (effect.visible || config.hidden)
                && !config.blocked.get(effect.id).copied().unwrap_or(false)
        });
        let count = filtered.clone().count();
        if count == 0 {
            return;
        }
        let [w, h] = config.dimensions();
        let ([start_x, start_y], step) = config.origin([batch.width, batch.height], count);
        let atlas = &self.gui.atlas;
        let opacity = f32::from(config.opacity) / 100.0;
        let tile_fill = color::with_alpha(color::bg(), opacity * 0.94);
        let tile_border = color::with_alpha(color::muted(), opacity * 0.23);
        let flip = !config.horizontal && config.position.on_right();
        let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
        for (index, effect) in filtered.enumerate() {
            let offset = index as f32 * step;
            let x = start_x + if config.horizontal { offset } else { 0.0 };
            let y = start_y + if config.horizontal { 0.0 } else { offset };
            let (tile, timer) = if config.compact {
                ([x, y + 1.0, w, h - 2.0], None)
            } else if config.horizontal {
                (
                    [x + 8.0, y, 22.0, 22.0],
                    Some([x + 2.0, y + 24.0, w - 4.0, 12.0]),
                )
            } else {
                let tile_x = if flip { x + w - 22.0 } else { x };
                let timer_x = if flip { x } else { x + 24.0 };
                (
                    [tile_x, y + 1.0, 22.0, 22.0],
                    Some([timer_x, y + 3.0, w - 24.0, 18.0]),
                )
            };
            let [tile_x, tile_y, tile_w, tile_h] = tile;
            ui.rounded(tile_x, tile_y, tile_w, tile_h, 4.0, tile_fill);
            ui.border(tile_x, tile_y, tile_w, tile_h, 4.0, tile_border);

            let icon = if config.compact { 12.0 } else { 18.0 };
            let icon_x = tile_x
                + if config.compact && !config.horizontal {
                    if flip { tile_w - icon - 3.0 } else { 3.0 }
                } else {
                    (tile_w - icon) / 2.0
                };
            let icon_y = tile_y
                + if config.compact && config.horizontal {
                    2.0
                } else {
                    (tile_h - icon) / 2.0
                };
            let sprite = atlas_client_core::potions::sprite(effect.id);
            ui.batch
                .sprite_scaled(atlas, &sprite, icon_x, icon_y, icon, icon, u32::MAX);

            let level = effect.amplifier + 1;
            let level = if level > 9 && !config.high_levels {
                "*".into()
            } else {
                level.to_string()
            };
            let rank_font = if config.compact { 6.0 } else { 7.0 };
            let rank_width = ui.measure(Face::SemiBold, rank_font, &level).ceil() + 4.0;
            let rank_height = rank_font + 1.0;
            let rank_edge = (icon_x + icon + 2.0).min(tile_x + tile_w - 1.0);
            let rank_x = (rank_edge - rank_width).max(tile_x + 1.0);
            let rank_y = tile_y + 1.0;
            ui.rounded(rank_x, rank_y, rank_width, rank_height, 2.0, tile_fill);
            ui.text_line(
                Face::SemiBold,
                rank_font,
                rank_height,
                rank_x + 2.0,
                rank_y + 0.5,
                rgba(0, 0, 0, 210),
                &level,
            );
            ui.text_line(
                Face::SemiBold,
                rank_font,
                rank_height,
                rank_x + 2.0,
                rank_y,
                color::accent(),
                &level,
            );

            if let Some([tx, ty, tw, th]) = timer {
                let (duration, warning) = remaining_time(effect.duration);
                let (ink, edge) = if warning == u32::MAX {
                    (color::text(), tile_border)
                } else {
                    (warning, color::with_alpha(warning, opacity * 0.55))
                };
                let size = if config.horizontal { 8.5 } else { 10.0 };
                let text_x = tx + (tw - ui.measure(Face::Mono, size, &duration)) / 2.0;
                ui.rounded(
                    tx,
                    ty,
                    tw,
                    th,
                    4.0,
                    color::with_alpha(color::border(), opacity * 0.96),
                );
                ui.border(tx, ty, tw, th, 4.0, edge);
                ui.text_line(
                    Face::Mono,
                    size,
                    th,
                    text_x,
                    ty + 0.5,
                    rgba(0, 0, 0, 210),
                    &duration,
                );
                ui.text_line(Face::Mono, size, th, text_x, ty, ink, &duration);
            } else {
                let fraction = if effect.duration < 0 {
                    1.0
                } else {
                    (effect.duration as f32 / (config.bar_seconds as f32 * 20.0)).clamp(0.0, 1.0)
                };
                let color = if fraction < 0.5 {
                    rgba(255, (510.0 * fraction) as u8, 0, 255)
                } else {
                    rgba((510.0 * (1.0 - fraction)) as u8, 255, 0, 255)
                };
                let length = (12.0 * fraction).floor().max(1.0);
                if config.horizontal {
                    let bar_y = y + h - 4.0;
                    ui.rounded(x + 3.0, bar_y, 12.0, 2.0, 1.0, tile_border);
                    ui.rounded(x + 3.0, bar_y, length, 2.0, 1.0, color);
                } else {
                    let bar_x = if flip { x + 2.0 } else { x + w - 4.0 };
                    let bottom = y + h - 3.0;
                    ui.rounded(bar_x, bottom - 12.0, 2.0, 12.0, 1.0, tile_border);
                    ui.rounded(bar_x, bottom - length, 2.0, length, 1.0, color);
                }
            }
        }
        ui.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_preview_excludes_hotbar_and_rotates_the_three_storage_rows() {
        assert_eq!(inventory_cell(0, false), [0, 0]);
        assert_eq!(inventory_cell(26, false), [8, 2]);
        assert_eq!(inventory_cell(0, true), [2, 0]);
        assert_eq!(inventory_cell(26, true), [0, 8]);
    }

    #[test]
    fn duration_and_durability_use_reference_boundaries() {
        assert_eq!(remaining_time(-1).0, "**:**");
        assert_eq!(remaining_time(32_000).0, "**:**");
        assert_eq!(
            remaining_time(200),
            ("00:10".into(), rgba(255, 85, 85, 255))
        );
        assert_eq!(percent_left(156, 3), 98);
        assert_eq!(
            durability_label(Durability::DamageLeft, 3, 156, 98)
                .unwrap()
                .0,
            "153"
        );
    }
}
