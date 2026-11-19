//! Mouse Tweaks 2.27, adapted from Ivan Molodetskikh's BSD-3-Clause implementation.
//! Reference 653985b63ca1a47e741150c9f691a1fc42a07d27; original notice is in mod-port-licenses.
use super::super::{Game, Screen, stations};
use atlas_client::{containers::BlockEntity, items::ItemStack, menu::Input};
use atlas_optifine::Properties;
use std::{fs, io::Write, path::Path};
#[path = "mouse_tweaks_ui.rs"]
mod screen;

pub(super) const ID: &str = "atlas-mousetweaks";
pub(super) const CONFIG_PATH: &str = "config/MouseTweaks.cfg";
const DEFAULT_CONFIG: &str = "RMBTweak=1\nLMBTweakWithItem=1\nLMBTweakWithoutItem=1\nWheelTweak=1\nWheelSearchOrder=1\nWheelScrollDirection=0\nScrollItemScaling=0\nDebug=0\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Config {
    rmb: bool,
    lmb_with_item: bool,
    lmb_without_item: bool,
    wheel: bool,
    search_backwards: bool,
    direction: i32,
    always_one: bool,
    debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::parse(DEFAULT_CONFIG)
    }
}

impl Config {
    fn parse(text: &str) -> Self {
        let properties = Properties::parse_untrimmed(text);
        let integer = |key, fallback| {
            properties
                .get(key)
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(fallback)
        };
        let direction = match integer("WheelScrollDirection", 0) {
            value @ 0..=2 => value,
            _ => 3,
        };
        Self {
            rmb: integer("RMBTweak", 1) != 0,
            lmb_with_item: integer("LMBTweakWithItem", 1) != 0,
            lmb_without_item: integer("LMBTweakWithoutItem", 1) != 0,
            wheel: integer("WheelTweak", 1) != 0,
            search_backwards: integer("WheelSearchOrder", 1) != 0,
            direction,
            always_one: integer("ScrollItemScaling", 0) != 0,
            debug: integer("Debug", 0) != 0,
        }
    }

    fn load(home: &Path) -> Self {
        let path = home.join(CONFIG_PATH);
        match fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let create = || -> std::io::Result<()> {
                    fs::create_dir_all(home.join("config"))?;
                    fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)?
                        .write_all(DEFAULT_CONFIG.as_bytes())
                };
                if let Err(error) = create() {
                    log::warn!("Mouse Tweaks: cannot create {}: {error}", path.display());
                }
                Self::default()
            }
            Err(error) => {
                log::warn!("Mouse Tweaks: cannot read {}: {error}", path.display());
                Self::default()
            }
        }
    }

    fn push(self, delta: i32, above: bool) -> bool {
        let position_aware = matches!(self.direction, 2 | 3);
        let inverted = matches!(self.direction, 1 | 3);
        (delta < 0) ^ (position_aware && above) ^ inverted
    }

    fn save(self, home: &Path) -> std::io::Result<()> {
        let text = format!(
            "RMBTweak={}\nLMBTweakWithItem={}\nLMBTweakWithoutItem={}\nWheelTweak={}\nWheelSearchOrder={}\nWheelScrollDirection={}\nScrollItemScaling={}\nDebug={}\n",
            u8::from(self.rmb),
            u8::from(self.lmb_with_item),
            u8::from(self.lmb_without_item),
            u8::from(self.wheel),
            u8::from(self.search_backwards),
            self.direction,
            u8::from(self.always_one),
            u8::from(self.debug),
        );
        fs::create_dir_all(home.join("config"))?;
        fs::write(home.join(CONFIG_PATH), text)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Context {
    screen: Screen,
    container: Option<(i32, i32, i32)>,
    station: Option<stations::Kind>,
    craft_size: usize,
    remote_menu: i32,
}

#[derive(Default)]
pub(super) struct State {
    config: Config,
    context: Option<Context>,
    previous: Option<usize>,
    button: Option<bool>,
    left_drag: bool,
    right_drag: bool,
    left_original: bool,
    wheel: f64,
    gui_focus: Option<usize>,
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        Self {
            config: Config::load(home),
            ..Self::default()
        }
    }

    fn screen(&mut self, context: Option<Context>, home: &Path) {
        if self.context == context {
            return;
        }
        self.context = context;
        self.previous = None;
        self.button = None;
        self.left_drag = false;
        self.right_drag = false;
        self.left_original = false;
        self.wheel = 0.0;
        if context.is_some() {
            self.config = Config::load(home);
            if self.config.debug {
                log::info!(
                    "Mouse Tweaks: opened a container screen; wheel tweak enabled: {}",
                    self.config.wheel
                );
            }
        }
    }

    fn scroll(&mut self, delta: f64) -> i32 {
        let scaled = if self.config.always_one {
            if delta == 0.0 { 0.0 } else { delta.signum() }
        } else {
            delta
        };
        if self.wheel != 0.0 && (scaled == 0.0 || scaled.signum() != self.wheel.signum()) {
            self.wheel = 0.0;
        }
        self.wheel += scaled;
        let steps = self.wheel as i32;
        self.wheel -= f64::from(steps);
        steps
    }
}

#[derive(Clone)]
struct Slot {
    id: usize,
    player: bool,
    y: f32,
    output: bool,
    ignored: bool,
    stack: Option<ItemStack>,
    capacity: u32,
    limit: u32,
    accepts: bool,
}

fn compatible(a: Option<&ItemStack>, b: Option<&ItemStack>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.same_item_same_components(b),
        _ => true,
    }
}

fn other_inventory_above(selected: &Slot, slots: &[Slot]) -> bool {
    let mut above = 0;
    let mut below = 0;
    for slot in slots.iter().filter(|slot| slot.player != selected.player) {
        if slot.y < selected.y {
            above += 1;
        } else {
            below += 1;
        }
    }
    above > below
}

fn push_slots(slots: &[Slot], selected: &Slot, mut count: u32, all: bool) -> Option<Vec<usize>> {
    let mut targets = Vec::new();
    let mut empty = Vec::new();
    for slot in slots {
        if count == 0 {
            break;
        }
        if slot.ignored || slot.output || slot.player == selected.player {
            continue;
        }
        match &slot.stack {
            None if slot.accepts => empty.push(slot),
            Some(stack)
                if compatible(selected.stack.as_ref(), Some(stack)) && stack.count < slot.limit =>
            {
                targets.push(slot.id);
                count = count.saturating_sub(slot.limit - stack.count);
            }
            _ => {}
        }
    }
    for slot in empty {
        if count == 0 {
            break;
        }
        targets.push(slot.id);
        count = count.saturating_sub(slot.capacity);
    }
    if all && count > 0 {
        None
    } else {
        Some(targets)
    }
}

fn pull_slot<'a>(slots: &'a [Slot], selected: &Slot, backwards: bool) -> Option<&'a Slot> {
    let eligible = |slot: &&Slot| {
        !slot.ignored
            && slot.player != selected.player
            && slot.stack.is_some()
            && compatible(selected.stack.as_ref(), slot.stack.as_ref())
    };
    if backwards {
        slots.iter().rev().find(eligible)
    } else {
        slots.iter().find(eligible)
    }
}

impl Game<'_> {
    pub(in crate::game) fn atlas_mouse_tweaks_on(&self) -> bool {
        self.atlas_port_on(ID)
    }

    pub(in crate::game) fn atlas_mouse_tweaks_tick(&mut self) {
        let context = (self.atlas_mouse_tweaks_on()
            && matches!(self.screen, Screen::Inventory | Screen::Creative)
            && !self.prompt_open()
            && !self.dialog_open())
        .then(|| Context {
            screen: self.screen,
            container: self.container,
            station: self.station.as_ref().map(|station| station.kind),
            craft_size: self.craft_size,
            remote_menu: self.remote_menu_id(),
        });
        self.atlas_ports
            .mouse_tweaks
            .screen(context, &Self::game_dir());
    }

    pub(in crate::game) fn atlas_mouse_tweaks_press(&mut self, slot: Option<usize>, right: bool) {
        self.atlas_mouse_tweaks_tick();
        let state = &mut self.atlas_ports.mouse_tweaks;
        if state.context.is_none() {
            return;
        }
        state.previous = slot;
        state.button = Some(right);
        if right {
            if self.carried.is_some() && state.config.rmb {
                state.right_drag = true;
                state.left_original = false;
            }
        } else if self.carried.is_none() {
            state.left_drag = true;
        }
    }

    pub(in crate::game) fn atlas_mouse_tweaks_release(&mut self, right: bool) {
        if right {
            self.atlas_ports.mouse_tweaks.right_drag = false;
        } else {
            self.atlas_ports.mouse_tweaks.left_drag = false;
        }
        if self.atlas_ports.mouse_tweaks.button == Some(right) {
            self.atlas_ports.mouse_tweaks.button = None;
        }
    }

    fn mouse_tweaks_ignored(&self, slot: usize) -> bool {
        self.screen == Screen::Creative && !(slot < 36 || (500..505).contains(&slot))
    }

    fn mouse_tweaks_result(&self, slot: usize) -> bool {
        slot == 200
            || (slot == 302 && matches!(self.container_kind(), Some(BlockEntity::Furnace(_))))
            || (slot == stations::RESULT
                && self
                    .station
                    .as_ref()
                    .is_some_and(|s| s.kind == stations::Kind::Merchant))
    }

    fn mouse_tweaks_click(&mut self, slot: usize, right: bool, shift: bool) {
        self.menu_click(
            Some(slot),
            i32::from(right),
            if shift {
                Input::QuickMove
            } else {
                Input::Pickup
            },
        );
        if let Some(key) = self.container {
            self.container_changed(key);
        }
    }

    fn mouse_tweaks_slot(&mut self, id: usize, probe: &ItemStack) -> Option<Slot> {
        let mut rules = None;
        self.menu_session(|menu, env, to_menu| {
            let index = to_menu(id)?;
            let stack = menu.slot(index);
            rules = Some((
                menu.slot_capacity(index),
                menu.slot_max(env, index, stack.unwrap_or(probe)),
                menu.may_place(env, index, probe),
            ));
            Some(false)
        });
        let (capacity, limit, accepts) = rules?;
        Some(Slot {
            id,
            player: id < 36 || (500..505).contains(&id),
            y: self.slot_position(id).1,
            output: self.mouse_tweaks_result(id),
            ignored: self.mouse_tweaks_ignored(id),
            stack: self.slot_view(id),
            capacity,
            limit,
            accepts,
        })
    }

    fn mouse_tweaks_slots(&mut self, probe: &ItemStack) -> Vec<Slot> {
        let ids = self.slots();
        let mut order = Vec::new();
        self.menu_session(|_, _, to_menu| {
            order = ids
                .iter()
                .filter_map(|&id| to_menu(id).map(|index| (index, id)))
                .collect();
            Some(false)
        });
        order.sort_unstable_by_key(|&(index, _)| index);
        order
            .into_iter()
            .filter_map(|(_, id)| self.mouse_tweaks_slot(id, probe))
            .collect()
    }

    fn mouse_tweaks_place_one(&mut self, slot: usize) {
        if self.mouse_tweaks_ignored(slot) || self.mouse_tweaks_result(slot) {
            return;
        }
        let Some(held) = self.carried.clone() else {
            return;
        };
        let Some(target) = self.mouse_tweaks_slot(slot, &held) else {
            return;
        };
        if self.items().get(&held.item).is_some_and(|item| item.bundle)
            || (compatible(target.stack.as_ref(), Some(&held))
                && target
                    .stack
                    .as_ref()
                    .is_none_or(|stack| stack.count != target.limit))
        {
            self.mouse_tweaks_click(slot, true, false);
        }
    }

    /// True only after RMB has replaced vanilla's pending quick-craft drag.
    pub(in crate::game) fn atlas_mouse_tweaks_drag(&mut self, slot: Option<usize>) -> bool {
        self.atlas_mouse_tweaks_tick();
        let state = &mut self.atlas_ports.mouse_tweaks;
        let Some(right) = state.button.filter(|_| state.context.is_some()) else {
            return false;
        };
        if slot == state.previous {
            return right && state.right_drag && state.left_original;
        }
        let previous = state.previous;
        let start_right = right && state.right_drag && !state.left_original;
        if start_right {
            state.left_original = true;
        }
        state.previous = slot;
        if start_right {
            self.inv_drag = None;
            self.inv_clicks.skip_release();
            if let Some(previous) = previous {
                self.mouse_tweaks_place_one(previous);
            }
        }
        let owned = right && self.atlas_ports.mouse_tweaks.right_drag;
        let Some(slot) = slot.filter(|&slot| !self.mouse_tweaks_ignored(slot)) else {
            return owned;
        };
        if right {
            if owned {
                self.mouse_tweaks_place_one(slot);
            }
        } else if self.atlas_ports.mouse_tweaks.left_drag {
            let config = self.atlas_ports.mouse_tweaks.config;
            let Some(selected) = self.slot_view(slot) else {
                return false;
            };
            let shift = self.has_shift_down();
            match self.carried.clone() {
                None if config.lmb_without_item && shift => {
                    self.mouse_tweaks_click(slot, false, true)
                }
                Some(held) if config.lmb_with_item && compatible(Some(&selected), Some(&held)) => {
                    if shift {
                        self.mouse_tweaks_click(slot, false, true);
                    } else if held.count.saturating_add(selected.count) <= self.max_stack(&held) {
                        self.mouse_tweaks_click(slot, false, false);
                        if !self.mouse_tweaks_result(slot) {
                            self.mouse_tweaks_click(slot, false, false);
                        }
                    }
                }
                _ => {}
            }
        }
        owned
    }

    pub(in crate::game) fn atlas_mouse_tweaks_scroll(
        &mut self,
        delta: f64,
        slot: Option<usize>,
    ) -> bool {
        self.atlas_mouse_tweaks_tick();
        let config = self.atlas_ports.mouse_tweaks.config;
        if self.atlas_ports.mouse_tweaks.context.is_none() || !config.wheel || !delta.is_finite() {
            return false;
        }
        let Some(id) = slot.filter(|&slot| !self.mouse_tweaks_ignored(slot)) else {
            return false;
        };
        let selected_stack = self.slot_view(id);
        if selected_stack.as_ref().is_some_and(|stack| {
            self.items()
                .get(&stack.item)
                .is_some_and(|item| item.bundle)
        }) {
            return false;
        }
        let steps = self.atlas_ports.mouse_tweaks.scroll(delta);
        let Some(stack) = selected_stack else {
            return true;
        };
        let Some(selected) = self.mouse_tweaks_slot(id, &stack) else {
            return false;
        };
        let slots = self.mouse_tweaks_slots(&stack);
        let push = config.push(steps, other_inventory_above(&selected, &slots));
        let mut remaining = steps.unsigned_abs();
        if remaining == 0 {
            return true;
        }
        let held = self.carried.clone();
        if selected.output {
            if !compatible(Some(&stack), held.as_ref()) {
                return true;
            }
            if held.is_none() {
                if !push {
                    return true;
                }
                while remaining > 0 {
                    remaining -= 1;
                    let Some(stack) = self.slot_view(id) else {
                        break;
                    };
                    let slots = self.mouse_tweaks_slots(&stack);
                    let Some(selected) = slots.iter().find(|slot| slot.id == id) else {
                        break;
                    };
                    let Some(targets) = push_slots(&slots, selected, stack.count, true) else {
                        break;
                    };
                    self.mouse_tweaks_click(id, false, false);
                    for (index, target) in targets.iter().enumerate() {
                        if index + 1 == targets.len() {
                            self.mouse_tweaks_click(*target, false, false);
                        } else if let Some(target) = self.mouse_tweaks_slot(*target, &stack) {
                            let count = target.stack.as_ref().map_or(0, |stack| stack.count);
                            for _ in count..target.limit {
                                self.mouse_tweaks_click(target.id, true, false);
                            }
                        }
                    }
                }
            } else {
                for _ in 0..remaining {
                    self.mouse_tweaks_click(id, false, false);
                }
            }
            return true;
        }
        if held.is_some() && compatible(Some(&stack), held.as_ref()) {
            return true;
        }
        if push {
            if held.as_ref().is_some_and(|held| {
                self.mouse_tweaks_slot(id, held)
                    .is_none_or(|slot| !slot.accepts)
            }) {
                return true;
            }
            remaining = remaining.min(stack.count);
            let targets = push_slots(&slots, &selected, remaining, false).unwrap_or_default();
            if targets.is_empty() {
                return true;
            }
            self.mouse_tweaks_click(id, false, false);
            for target in targets {
                if let Some(target) = self.mouse_tweaks_slot(target, &stack) {
                    let count = target.stack.as_ref().map_or(0, |stack| stack.count);
                    let moved = remaining.min(target.limit.saturating_sub(count));
                    remaining -= moved;
                    for _ in 0..moved {
                        self.mouse_tweaks_click(target.id, true, false);
                    }
                }
            }
            self.mouse_tweaks_click(id, false, false);
        } else {
            let mut room = selected.limit.saturating_sub(stack.count);
            remaining = remaining.min(room);
            while remaining > 0 {
                let slots = self.mouse_tweaks_slots(&stack);
                let Some(target) = pull_slot(&slots, &selected, config.search_backwards).cloned()
                else {
                    break;
                };
                let count = target.stack.as_ref().map_or(0, |stack| stack.count);
                if target.output {
                    if room < count {
                        break;
                    }
                    room -= count;
                    remaining = remaining.saturating_sub(1).min(room);
                    if held.as_ref().is_some_and(|held| {
                        self.mouse_tweaks_slot(id, held)
                            .is_none_or(|slot| !slot.accepts)
                    }) {
                        break;
                    }
                    self.mouse_tweaks_click(id, false, false);
                    self.mouse_tweaks_click(target.id, false, false);
                    self.mouse_tweaks_click(id, false, false);
                } else {
                    let moved = remaining.min(count);
                    room -= moved;
                    remaining -= moved;
                    if held.as_ref().is_some_and(|held| {
                        self.mouse_tweaks_slot(target.id, held)
                            .is_none_or(|slot| !slot.accepts)
                    }) {
                        break;
                    }
                    self.mouse_tweaks_click(target.id, false, false);
                    if moved == count {
                        self.mouse_tweaks_click(id, false, false);
                    } else {
                        for _ in 0..moved {
                            self.mouse_tweaks_click(id, true, false);
                        }
                    }
                    self.mouse_tweaks_click(target.id, false, false);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(count: u32) -> ItemStack {
        ItemStack {
            item: "minecraft:stone".into(),
            count,
            damage: 0,
            extra: None,
        }
    }

    fn slot(id: usize, player: bool, count: u32) -> Slot {
        Slot {
            id,
            player,
            y: 0.0,
            output: false,
            ignored: false,
            stack: (count > 0).then(|| stack(count)),
            capacity: 64,
            limit: 64,
            accepts: true,
        }
    }

    #[test]
    fn properties_defaults_and_invalid_enum_ids_follow_original_config() {
        assert_eq!(Config::parse(""), Config::default());
        let config = Config::parse(
            "RMBTweak=0 \nLMBTweakWithItem:0\nLMBTweakWithoutItem=-2\nWheelSearchOrder=9\nWheelScrollDirection=8\nScrollItemScaling=-1\n",
        );
        assert!(config.rmb);
        assert!(!config.lmb_with_item);
        assert!(config.lmb_without_item && config.search_backwards && config.always_one);
        assert_eq!(config.direction, 3);
    }

    #[test]
    fn fractional_wheel_restarts_on_direction_changes_and_scales_before_accumulating() {
        let mut state = State::default();
        assert_eq!(state.scroll(0.75), 0);
        assert_eq!(state.scroll(-0.5), 0);
        assert_eq!(state.scroll(-0.6), -1);
        assert!((state.wheel + 0.1).abs() < 1e-12);
        state.config.always_one = true;
        assert_eq!(state.scroll(5.0), 1);
        assert_eq!(state.scroll(0.0), 0);
        assert_eq!(state.wheel, 0.0);
        state.config.always_one = false;
        assert_eq!(state.scroll(0.4), 0);
        assert_eq!(state.scroll(0.0), 0);
        assert_eq!(state.wheel, 0.0);
    }

    #[test]
    fn all_four_wheel_directions_follow_inventory_geometry() {
        let mut config = Config::default();
        for (direction, above, expected) in [
            (0, false, true),
            (0, true, true),
            (1, false, false),
            (1, true, false),
            (2, false, true),
            (2, true, false),
            (3, false, false),
            (3, true, true),
        ] {
            config.direction = direction;
            assert_eq!(config.push(-1, above), expected);
            assert_ne!(config.push(1, above), expected);
        }
    }

    #[test]
    fn push_prefers_partial_stacks_and_crafting_requires_every_item_to_fit() {
        let selected = slot(200, false, 4);
        let mut slots = vec![slot(9, true, 0), slot(10, true, 62), slot(11, true, 63)];
        assert_eq!(
            push_slots(&slots, &selected, 4, true),
            Some(vec![10, 11, 9])
        );
        slots[0].accepts = false;
        assert_eq!(push_slots(&slots, &selected, 4, true), None);
        assert_eq!(push_slots(&slots, &selected, 4, false), Some(vec![10, 11]));
    }

    #[test]
    fn pull_search_order_is_the_original_menu_order_and_can_take_crafting_results() {
        let selected = slot(1, true, 10);
        let mut result = slot(200, false, 4);
        result.output = true;
        let slots = vec![result, slot(100, false, 6), slot(2, true, 7)];
        assert_eq!(
            pull_slot(&slots, &selected, false).map(|slot| slot.id),
            Some(200)
        );
        assert_eq!(
            pull_slot(&slots, &selected, true).map(|slot| slot.id),
            Some(100)
        );
    }
}
