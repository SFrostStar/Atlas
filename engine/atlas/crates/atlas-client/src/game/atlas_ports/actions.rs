//! Native client utility ports from the user-supplied JARs.
//! Original licenses and attribution are listed in THIRD_PARTY_NOTICES.md.
use super::super::{BlockEntity, Game, Screen};
use atlas_client::{atlas::chunk_animation as chunk, items::ItemStack, keybinds, menu::Input};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use winit::keyboard::KeyCode;
#[path = "actions_gamma.rs"]
mod gamma;
#[path = "actions_inventory.rs"]
mod inventory;
#[path = "actions_logic.rs"]
mod logic;
#[path = "actions_ui.rs"]
mod ui;
#[path = "actions_viewmodel.rs"]
pub(super) mod viewmodel;

pub(super) const IDS: [&str; 10] = [
    "atlas-autocraft",
    "atlas-dropall",
    "atlas-hotcommands",
    "atlas-inventory-manager",
    "atlas-middleclickpearl",
    "atlas-sortmod",
    "atlas-streamermode",
    "atlas-viewmodel",
    "atlas-chunkanimator",
    "atlas-gammautils",
];
pub(super) const DEFINITIONS: [(&str, &str, &str); 10] = [
    (
        IDS[0],
        "AutoCraft",
        "Stack crafting for crystals, ender eyes and harming arrows.",
    ),
    (
        IDS[1],
        "DropAll",
        "Original inventory and chest Drop All buttons.",
    ),
    (
        IDS[2],
        "HotCommands",
        "Nested command buttons and an in-game command editor.",
    ),
    (
        IDS[3],
        "Inventory Manager",
        "Save, rename and apply 41-slot inventory layouts.",
    ),
    (
        IDS[4],
        "MiddleClickPearl",
        "Use a hotbar ender pearl with the middle mouse button.",
    ),
    (
        IDS[5],
        "Chest Sorter",
        "Sort chest stacks using the supplied mod's click schedule.",
    ),
    (
        IDS[6],
        "StreamerMode",
        "Hide auth passwords while preserving the real sent command.",
    ),
    (
        IDS[7],
        "ViewModel",
        "Hand transforms, thirteen swing styles and interactive chat controls.",
    ),
    (
        IDS[8],
        "Chunk Animator",
        "Five chunk entrance modes with eleven easing functions.",
    ),
    (
        IDS[9],
        "Gamma Utils",
        "Gamma and night vision controls with transitions and dynamic lighting.",
    ),
];
pub(super) fn keys(id: &str) -> Vec<[String; 3]> {
    let bindings: Vec<(&str, &str, &str)> = match id {
        id if id == IDS[2] => vec![("open", "key.hotcommands.open", "key.keyboard.h")],
        id if id == IDS[4] => vec![("throw", "key.middleclickpearl.throw", "key.mouse.middle")],
        id if id == IDS[7] => vec![("open", "key.viewmodel.open_gui", "key.keyboard.v")],
        id if id == IDS[9] => vec![
            (
                "gammaToggle",
                "key.gammautils.gammaToggle",
                "key.keyboard.g",
            ),
            (
                "increaseGamma",
                "key.gammautils.increaseGamma",
                "key.keyboard.up",
            ),
            (
                "decreaseGamma",
                "key.gammautils.decreaseGamma",
                "key.keyboard.down",
            ),
            (
                "maxGamma",
                "key.gammautils.maxGamma",
                "key.keyboard.unknown",
            ),
            (
                "minGamma",
                "key.gammautils.minGamma",
                "key.keyboard.unknown",
            ),
            (
                "nightVisionToggle",
                "key.gammautils.nightVisionToggle",
                "key.keyboard.h",
            ),
            (
                "increaseNightVision",
                "key.gammautils.increaseNightVision",
                "key.keyboard.right",
            ),
            (
                "decreaseNightVision",
                "key.gammautils.decreaseNightVision",
                "key.keyboard.left",
            ),
        ],
        _ => vec![],
    };
    bindings
        .into_iter()
        .map(|(binding, label, default)| {
            [
                binding.into(),
                atlas_client::lang::get(label),
                default.into(),
            ]
        })
        .collect()
}
pub(super) struct State {
    home: PathBuf,
    password: logic::PasswordOptions,
    protect_history: bool,
    suppress_suggestions: bool,
    pub(super) viewmodel: viewmodel::Config,
    pub(super) viewmodel_editor: viewmodel::ChatEditor,
    chunk_mode: chunk::Mode,
    easing: chunk::Easing,
    duration: u64,
    disable_near: bool,
    sort_queue: VecDeque<usize>,
    sort_cooldown: u8,
    pub(super) active_screen: Option<String>,
    pub(super) commands: Vec<HotCommand>,
    command_path: Vec<usize>,
    pub(super) autocraft: Option<usize>,
    craft_delay: u8,
    craft_count: u64,
    ui: ui::State,
    inventory: inventory::State,
    gamma: gamma::State,
}
impl Default for State {
    fn default() -> Self {
        Self {
            home: PathBuf::new(),
            password: logic::PasswordOptions::default(),
            protect_history: true,
            suppress_suggestions: true,
            viewmodel: viewmodel::Config::default(),
            viewmodel_editor: viewmodel::ChatEditor::default(),
            chunk_mode: chunk::Mode::Below,
            easing: chunk::Easing::Sine,
            duration: 1000,
            disable_near: false,
            sort_queue: VecDeque::new(),
            sort_cooldown: 0,
            active_screen: None,
            commands: default_commands(),
            command_path: vec![],
            autocraft: None,
            craft_delay: 0,
            craft_count: 0,
            ui: ui::State::default(),
            inventory: inventory::State::default(),
            gamma: gamma::State::default(),
        }
    }
}
fn read_config(home: &Path, name: &str) -> Value {
    let path = home.join("config").join(name);
    if fs::metadata(&path).is_ok_and(|m| m.len() <= 1024 * 1024) {
        fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    }
}
fn save_config(home: &Path, name: &str, value: &Value) {
    let dir = home.join("config");
    if fs::create_dir_all(&dir).is_ok()
        && let Ok(bytes) = serde_json::to_vec_pretty(value)
    {
        let _ = fs::write(dir.join(name), bytes);
    }
}
fn number(v: &Value, key: &str, default: f32) -> f32 {
    v[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .map_or(default, |v| v as f32)
}
fn boolean(v: &Value, key: &str, default: bool) -> bool {
    v[key].as_bool().unwrap_or(default)
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
impl State {
    pub(super) fn load(home: &Path) -> Self {
        let mut s = Self {
            home: home.to_owned(),
            ..Self::default()
        };
        let p = read_config(home, "streamermode.json");
        s.password.enabled = boolean(&p, "enabled", true);
        s.password.preserve_spaces = boolean(&p, "preserveSpaces", true);
        s.password.mask = p["maskCharacter"].as_str().unwrap_or("*").into();
        if let Some(list) = p["protectedCommands"].as_array() {
            s.password.protected_commands = list
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
        }
        s.protect_history = boolean(&p, "protectChatHistory", true);
        s.suppress_suggestions = boolean(&p, "suppressSuggestions", true);
        let v = read_config(home, "viewmodel.json");
        for (key, hand) in [
            ("mainHand", &mut s.viewmodel.main),
            ("offHand", &mut s.viewmodel.off),
        ] {
            let v = &v[key];
            hand.position = [
                number(v, "posX", 0.0),
                number(v, "posY", 0.0),
                number(v, "posZ", 0.0),
            ];
            hand.rotation = [
                number(v, "pitch", 0.0),
                number(v, "yaw", 0.0),
                number(v, "roll", 0.0),
            ];
            hand.scale = number(v, "scale", 1.0);
        }
        s.viewmodel.swing = viewmodel::Swing::ALL
            .into_iter()
            .find(|k| Some(k.name()) == v["swingType"].as_str())
            .unwrap_or_default();
        s.viewmodel.strength = number(&v, "swingStrength", 1.0);
        s.viewmodel.duration = number(&v, "swingDuration", 1.0);
        s.viewmodel.only_target = boolean(&v, "onlyOnTarget", false);
        s.viewmodel.auto_swing = boolean(&v, "autoSwing", false);
        s.viewmodel.interactive = boolean(&v, "interactiveInChat", true);
        s.viewmodel.outline = boolean(&v, "outlineSelected", true);
        let c = read_config(home, "chunkanimator.json");
        s.chunk_mode = match c["mode"].as_str() {
            Some("ABOVE") => chunk::Mode::Above,
            Some("HYBRID") => chunk::Mode::Hybrid,
            Some("HORIZONTAL_SLIDE") => chunk::Mode::HorizontalSlide,
            Some("HORIZONTAL_SLIDE_ALTERNATE") => chunk::Mode::HorizontalSlideAlternate,
            _ => chunk::Mode::Below,
        };
        s.easing = match c["easingFunction"].as_str() {
            Some("LINEAR") => chunk::Easing::Linear,
            Some("QUAD") => chunk::Easing::Quad,
            Some("CUBIC") => chunk::Easing::Cubic,
            Some("QUART") => chunk::Easing::Quart,
            Some("QUINT") => chunk::Easing::Quint,
            Some("EXPO") => chunk::Easing::Expo,
            Some("CIRC") => chunk::Easing::Circ,
            Some("BACK") => chunk::Easing::Back,
            Some("BOUNCE") => chunk::Easing::Bounce,
            Some("ELASTIC") => chunk::Easing::Elastic,
            _ => chunk::Easing::Sine,
        };
        s.duration = c["animationDuration"].as_i64().unwrap_or(1000).max(0) as u64;
        s.disable_near = boolean(&c, "disableAroundPlayer", false);
        if let Some(list) = read_config(home, "hotcommands.json").as_array() {
            let commands: Vec<_> = list.iter().filter_map(HotCommand::from_json).collect();
            if !commands.is_empty() {
                s.commands = commands;
            }
        }
        s.inventory = inventory::State::load(home);
        s.gamma = gamma::State::load(home);
        s
    }
    fn save_password(&self) {
        save_config(
            &self.home,
            "streamermode.json",
            &json!({"enabled":self.password.enabled,"maskCharacter":self.password.mask,"preserveSpaces":self.password.preserve_spaces,"protectedCommands":self.password.protected_commands,"protectChatHistory":self.protect_history,"suppressSuggestions":self.suppress_suggestions}),
        );
    }
    pub(super) fn save_viewmodel(&self) {
        let hand = |h: &viewmodel::HandSettings| json!({"posX":h.position[0],"posY":h.position[1],"posZ":h.position[2],"scale":h.scale,"pitch":h.rotation[0],"yaw":h.rotation[1],"roll":h.rotation[2]});
        save_config(
            &self.home,
            "viewmodel.json",
            &json!({"mainHand":hand(&self.viewmodel.main),"offHand":hand(&self.viewmodel.off),"swingType":self.viewmodel.swing.name(),"swingStrength":self.viewmodel.strength,"swingDuration":self.viewmodel.duration,"onlyOnTarget":self.viewmodel.only_target,"autoSwing":self.viewmodel.auto_swing,"interactiveInChat":self.viewmodel.interactive,"outlineSelected":self.viewmodel.outline}),
        );
    }
    fn save_chunks(&self) {
        let mode = match self.chunk_mode {
            chunk::Mode::Below => "BELOW",
            chunk::Mode::Above => "ABOVE",
            chunk::Mode::Hybrid => "HYBRID",
            chunk::Mode::HorizontalSlide => "HORIZONTAL_SLIDE",
            chunk::Mode::HorizontalSlideAlternate => "HORIZONTAL_SLIDE_ALTERNATE",
        };
        let easing = format!("{:?}", self.easing).to_uppercase();
        save_config(
            &self.home,
            "chunkanimator.json",
            &json!({"mode":mode,"easingFunction":easing,"animationDuration":self.duration,"disableAroundPlayer":self.disable_near}),
        );
    }
    fn save_commands(&self) {
        save_config(
            &self.home,
            "hotcommands.json",
            &Value::Array(self.commands.iter().map(HotCommand::json).collect()),
        );
    }
}
#[derive(Clone, Debug)]
pub(super) struct HotCommand {
    pub id: String,
    pub name: String,
    pub command: String,
    pub icon: String,
    pub children: Vec<HotCommand>,
}
impl HotCommand {
    fn new(name: &str, command: &str, icon: &str) -> Self {
        Self {
            id: format!("atlas-{}-{name}", now_ms()),
            name: name.into(),
            command: command.into(),
            icon: icon.into(),
            children: vec![],
        }
    }
    fn group(name: &str, icon: &str, children: &[(&str, &str, &str)]) -> Self {
        let mut s = Self::new(name, "", icon);
        s.children = children
            .iter()
            .map(|(n, c, i)| Self::new(n, c, i))
            .collect();
        s
    }
    fn from_json(v: &Value) -> Option<Self> {
        v.as_object()?;
        Some(Self {
            id: v["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("atlas-{}", now_ms()), str::to_owned),
            name: v["name"].as_str().unwrap_or_default().into(),
            command: v["command"].as_str().unwrap_or_default().into(),
            icon: v["icon"].as_str().unwrap_or_default().into(),
            children: v["children"]
                .as_array()
                .map(|v| v.iter().filter_map(Self::from_json).collect())
                .unwrap_or_default(),
        })
    }
    fn json(&self) -> Value {
        json!({"id":self.id,"name":self.name,"command":self.command,"icon":self.icon,"children":self.children.iter().map(Self::json).collect::<Vec<_>>()})
    }
}
fn default_commands() -> Vec<HotCommand> {
    vec![
        HotCommand::group(
            "/home",
            "red_bed",
            &[
                ("Основной дом", "/home", "red_bed"),
                ("Реклама / Магазин", "/home ad", "emerald"),
                ("Ферма", "/home farm", "wheat"),
                ("Шахта", "/home mine", "iron_pickaxe"),
                ("Поставить точку дома", "/sethome", "anvil"),
            ],
        ),
        HotCommand::group(
            "/tpa",
            "ender_pearl",
            &[
                ("Друг 1 (/tpa Friend1)", "/tpa Friend1", "player_head"),
                ("Друг 2 (/tpa Friend2)", "/tpa Friend2", "player_head"),
                ("Принять ТП (/tpaccept)", "/tpaccept", "lime_dye"),
                ("Отклонить ТП (/tpdeny)", "/tpdeny", "red_dye"),
            ],
        ),
        HotCommand::new("/spawn", "/spawn", "compass"),
        HotCommand::group(
            "/warp",
            "beacon",
            &[
                ("Магазин (/warp shop)", "/warp shop", "gold_ingot"),
                ("ПВП Арена (/warp pvp)", "/warp pvp", "diamond_sword"),
                ("Эндер-мир (/warp end)", "/warp end", "ender_eye"),
            ],
        ),
        HotCommand::new("/back", "/back", "clock"),
        HotCommand::new("/feed", "/feed", "cooked_beef"),
        HotCommand::new("/heal", "/heal", "golden_apple"),
        HotCommand::group(
            "/kit",
            "chest",
            &[
                ("Кит Старт (/kit start)", "/kit start", "stone_sword"),
                ("Кит Бонус (/kit bonus)", "/kit bonus", "iron_chestplate"),
            ],
        ),
    ]
}
impl Game<'_> {
    fn atlas_craft_name(&self, recipe: usize) -> String {
        atlas_client::lang::get(match recipe {
            0 => "atlas.autocraft.name.crystal",
            1 => "atlas.autocraft.name.eye",
            _ => "atlas.autocraft.name.arrows",
        })
    }
    fn atlas_craft_notice(&mut self, key: &str, args: &[&dyn std::fmt::Display], action_bar: bool) {
        let text =
            atlas_client::text::Text::legacy(&atlas_client::text::Text::tr(key, args).plain());
        if action_bar {
            self.titles.set_overlay(text, false);
        } else {
            let ticks = self.visual_ticks();
            let focused = self.screen == Screen::Chat;
            self.ui.chat.add(
                &self.gui.atlas,
                text,
                None,
                atlas_client::chat::ChatSource::Client,
                ticks,
                focused,
            );
        }
    }
    fn atlas_craft_stop(&mut self, by_user: bool) {
        let Some(recipe) = self.atlas_ports.actions.autocraft.take() else {
            return;
        };
        self.atlas_ports.actions.craft_delay = 0;
        if by_user {
            self.atlas_craft_notice("atlas.autocraft.stopped", &[], true);
        } else {
            let name = self.atlas_craft_name(recipe);
            self.atlas_craft_notice("atlas.autocraft.exhausted", &[&name], false);
        }
    }
    fn atlas_craft_missing(&self, recipe: usize) -> Option<(String, usize, u32)> {
        ingredients(recipe).iter().find_map(|g| {
            let count = self
                .inventory
                .iter()
                .flatten()
                .filter(|s| matches_ingredient(s, g))
                .map(|s| s.count)
                .sum::<u32>()
                + g.targets
                    .iter()
                    .filter_map(|i| self.craft_grid[i - 100].as_ref())
                    .filter(|s| matches_ingredient(s, g))
                    .map(|s| s.count)
                    .sum::<u32>();
            (count < g.targets.len() as u32).then(|| {
                let name = if g.item == "minecraft:lingering_potion" {
                    atlas_client::lang::get("atlas.autocraft.ingredient.harming")
                } else {
                    self.pack.names.display_item(g.item)
                };
                (name, g.targets.len(), count)
            })
        })
    }
    pub(in crate::game) fn atlas_actions_config(&self, id: &str) -> Option<Value> {
        let s = &self.atlas_ports.actions;
        match id {
            id if id == IDS[6] => Some(
                json!({"enabled":s.password.enabled,"maskCharacter":s.password.mask,"preserveSpaces":s.password.preserve_spaces,"protectedCommands":s.password.protected_commands,"protectChatHistory":s.protect_history,"suppressSuggestions":s.suppress_suggestions}),
            ),
            id if id == IDS[9] => Some(s.gamma.config.clone()),
            _ => None,
        }
    }
    pub(in crate::game) fn atlas_actions_set_config(
        &mut self,
        id: &str,
        value: Value,
    ) -> Result<(), String> {
        if !value.is_object() {
            return Err(atlas_client::lang::get("atlas.ports.invalid_config"));
        }
        let s = &mut self.atlas_ports.actions;
        if id == IDS[6] {
            for k in [
                "enabled",
                "preserveSpaces",
                "protectChatHistory",
                "suppressSuggestions",
            ] {
                if !value[k].is_boolean() {
                    return Err(format!("{k}: bool"));
                }
            }
            let Some(mask) = value["maskCharacter"].as_str() else {
                return Err("maskCharacter: string".into());
            };
            let Some(commands) = value["protectedCommands"].as_array() else {
                return Err("protectedCommands: array".into());
            };
            if commands.len() > 256 || commands.iter().any(|v| !v.is_string()) {
                return Err("protectedCommands: string[]".into());
            }
            s.password.enabled = value["enabled"] == true;
            s.password.preserve_spaces = value["preserveSpaces"] == true;
            s.protect_history = value["protectChatHistory"] == true;
            s.suppress_suggestions = value["suppressSuggestions"] == true;
            s.password.mask = mask.into();
            s.password.protected_commands = commands
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            s.save_password();
            Ok(())
        } else if id == IDS[9] {
            s.gamma.apply_config(value);
            s.gamma.save(&s.home);
            Ok(())
        } else {
            Err(atlas_client::lang::get("atlas.ports.invalid_config"))
        }
    }
    pub(in crate::game) fn atlas_actions_mask_chat(&self, input: &str) -> String {
        if self.atlas_port_on(IDS[6]) {
            logic::mask_password(input, &self.atlas_ports.actions.password)
        } else {
            input.into()
        }
    }
    pub(in crate::game) fn atlas_actions_suppress_history(&self, input: &str) -> bool {
        self.atlas_port_on(IDS[6])
            && self.atlas_ports.actions.protect_history
            && logic::password_prefix(input, &self.atlas_ports.actions.password).is_some()
    }
    pub(in crate::game) fn atlas_actions_suppress_suggestions(&self, input: &str) -> bool {
        self.atlas_port_on(IDS[6])
            && self.atlas_ports.actions.suppress_suggestions
            && logic::password_prefix(input, &self.atlas_ports.actions.password).is_some()
    }
    pub(in crate::game) fn atlas_actions_middle_click(&mut self) -> bool {
        self.atlas_port_on(IDS[4])
            && self.screen == Screen::Playing
            && self.player.is_some()
            && self
                .inventory
                .iter()
                .take(9)
                .flatten()
                .any(|s| &*s.item == "minecraft:ender_pearl")
    }
    fn atlas_actions_throw_pearl(&mut self) -> bool {
        if !self.atlas_port_on(IDS[4]) || self.screen != Screen::Playing || self.player.is_none() {
            return false;
        }
        let items: Vec<_> = self
            .inventory
            .iter()
            .take(9)
            .map(|s| s.as_ref().map(|s| s.item.to_string()))
            .collect();
        let Some(slot) = logic::pearl_slot(&items) else {
            return false;
        };
        if !self.spectator() && !self.cooldowns.is_on_cooldown("minecraft:ender_pearl") {
            self.atlas_pearl_use(slot);
        }
        true
    }
    fn atlas_actions_chest(&self) -> bool {
        self.screen == Screen::Inventory
            && (self.remote_menu().1.starts_with("minecraft:generic_9x")
                || matches!(self.container_kind(), Some(BlockEntity::Chest(_))))
    }
    pub(in crate::game) fn atlas_actions_drop_all(&mut self) -> bool {
        if !self.atlas_port_on(IDS[1]) || self.screen != Screen::Inventory {
            return false;
        }
        let slots: Vec<_> = if self.atlas_actions_chest() {
            self.slots()
                .into_iter()
                .filter(|i| *i >= 300 && *i < 500)
                .collect()
        } else if self.container.is_none()
            && self.remote_menu().1 == "inventory"
            && self.craft_size == 2
        {
            logic::drop_inventory_slots().collect()
        } else {
            return false;
        };
        for slot in slots {
            if self.slot_view(slot).is_some() {
                self.menu_ops(&[(Some(slot), 1, Input::Throw)]);
            }
        }
        true
    }
    pub(in crate::game) fn atlas_actions_start_sort(&mut self) -> bool {
        if !self.atlas_port_on(IDS[5]) || !self.atlas_actions_chest() {
            return false;
        }
        let slots: Vec<_> = self
            .slots()
            .into_iter()
            .filter(|i| *i >= 300 && *i < 500)
            .collect();
        let mut snapshot = Vec::new();
        for slot in slots {
            snapshot.push(self.slot_view(slot).map(|stack| {
                let max = self.items().get(&stack.item).map_or(64, |i| i.max_stack);
                let mut identity = stack.clone();
                identity.count = 1;
                logic::SortStack {
                    item: stack.item.to_string(),
                    name: atlas_client::item_tooltip::name_line(&stack, &|key| {
                        self.pack.names.display_get(key)
                    }),
                    identity: format!("{identity:?}"),
                    count: stack.count as i32,
                    max: max as i32,
                }
            }));
        }
        self.atlas_ports.actions.sort_queue = logic::chest_sort_plan(&snapshot);
        self.atlas_ports.actions.sort_cooldown = 0;
        true
    }
    pub(in crate::game) fn atlas_actions_tick(&mut self) {
        if !self.native_window() {
            return;
        }
        if !self.atlas_actions_chest() || !self.atlas_port_on(IDS[5]) {
            self.atlas_ports.actions.sort_queue.clear();
        } else if self.atlas_ports.actions.sort_cooldown > 0 {
            self.atlas_ports.actions.sort_cooldown -= 1;
        } else if let Some(slot) = self.atlas_ports.actions.sort_queue.pop_front() {
            self.menu_ops(&[(Some(300 + slot), 0, Input::Pickup)]);
            self.atlas_ports.actions.sort_cooldown = 2;
        }
        self.atlas_actions_craft_tick();
        self.atlas_inventory_tick();
        self.atlas_gamma_tick();
        if self.atlas_actions_auto_swing()
            && self.screen == Screen::Playing
            && self.player.is_some()
            && self.left
        {
            self.clicks.miss_time = 0;
            self.attack_clicks = self.attack_clicks.saturating_add(1);
        }
    }
    pub(in crate::game) fn atlas_actions_key(&mut self, key: KeyCode) -> bool {
        if self.screen != Screen::Playing {
            return false;
        }
        let pearl = keybinds::key_code(&self.native_mod_key(IDS[4], "throw", "key.mouse.middle"))
            == Some(key)
            && self.atlas_actions_throw_pearl();
        let gamma = self.atlas_gamma_key(key);
        for (id, binding, default) in [
            (IDS[2], "open", "key.keyboard.h"),
            (IDS[7], "open", "key.keyboard.v"),
        ] {
            if self.atlas_port_on(id)
                && keybinds::key_code(&self.native_mod_key(id, binding, default)) == Some(key)
            {
                self.atlas_ports.actions.active_screen = Some(id.into());
                self.atlas_ports.actions.command_path.clear();
                return self.atlas_open_port_screen(id);
            }
        }
        gamma || pearl
    }
    pub(in crate::game) fn atlas_actions_mouse_pearl(&mut self, button: i32) -> bool {
        let name = match button {
            0 => "key.mouse.left".to_owned(),
            1 => "key.mouse.right".to_owned(),
            2 => "key.mouse.middle".to_owned(),
            _ => format!("key.mouse.{}", button + 1),
        };
        if self.native_mod_key(IDS[4], "throw", "key.mouse.middle") == name {
            self.atlas_actions_throw_pearl()
        } else {
            false
        }
    }
    pub(in crate::game) fn atlas_actions_chunk_config(&self) -> Option<chunk::Config> {
        if !self.atlas_port_on(IDS[8]) {
            return None;
        }
        let s = &self.atlas_ports.actions;
        let player = self.player.as_ref().map_or([0, 0], |p| {
            [p.position.x.floor() as i32, p.position.z.floor() as i32]
        });
        let (min_y, height) = self.native_world_height();
        Some(chunk::Config {
            mode: s.chunk_mode,
            easing: s.easing,
            duration: s.duration,
            disable_near: s.disable_near,
            min_y,
            max_y: min_y + height,
            player,
        })
    }
    pub(in crate::game) fn atlas_actions_viewmodel_pose(
        &self,
        main: bool,
        left_dominant: bool,
    ) -> Vec<viewmodel::PoseOp> {
        if self.atlas_port_on(IDS[7]) {
            self.atlas_ports.actions.viewmodel.hand_pose(
                main,
                left_dominant,
                self.atlas_ports.actions.viewmodel_editor.selected == Some(main),
                now_ms(),
            )
        } else {
            vec![]
        }
    }
    pub(in crate::game) fn atlas_actions_swing_pose(
        &self,
        right: bool,
        progress: f32,
        entity_target: bool,
    ) -> Option<Vec<viewmodel::PoseOp>> {
        self.atlas_port_on(IDS[7])
            .then(|| {
                self.atlas_ports
                    .actions
                    .viewmodel
                    .swing_pose(right, progress, entity_target)
            })
            .flatten()
    }
    pub(in crate::game) fn atlas_actions_swing_duration(&self, original: i32) -> i32 {
        if self.atlas_port_on(IDS[7]) {
            self.atlas_ports.actions.viewmodel.swing_duration(original)
        } else {
            original
        }
    }
    pub(in crate::game) fn atlas_actions_auto_swing(&self) -> bool {
        self.atlas_port_on(IDS[7]) && self.atlas_ports.actions.viewmodel.auto_swing
    }
    pub(in crate::game) fn atlas_actions_hand_adjustment(
        &self,
        hand: super::super::Hand,
    ) -> glam::Mat4 {
        pose_matrix(self.atlas_actions_viewmodel_pose(
            hand == super::super::Hand::Main,
            self.settings.main_hand_left,
        ))
    }
    pub(in crate::game) fn atlas_actions_item_pose(
        &self,
        left: bool,
        equip: f32,
        attack: f32,
        pose: atlas_client::first_person::ItemPose,
        targeted: bool,
    ) -> glam::Mat4 {
        if matches!(pose, atlas_client::first_person::ItemPose::Swing)
            && let Some(ops) = self.atlas_actions_swing_pose(!left, attack, targeted)
        {
            atlas_client::first_person::item_pose(
                left,
                equip,
                0.0,
                atlas_client::first_person::ItemPose::Using,
            ) * pose_matrix(ops)
        } else {
            atlas_client::first_person::item_pose(left, equip, attack, pose)
        }
    }
    pub(in crate::game) fn atlas_actions_hand_selected(&self, hand: super::super::Hand) -> bool {
        self.atlas_port_on(IDS[7])
            && self.atlas_ports.actions.viewmodel.outline
            && self.atlas_ports.actions.viewmodel_editor.selected
                == Some(hand == super::super::Hand::Main)
    }
    pub(in crate::game) fn atlas_actions_settings(&self, id: &str, buttons: &mut Vec<String>) {
        let s = &self.atlas_ports.actions;
        match id {
            id if id == IDS[2] || id == IDS[7] => {
                buttons.push(atlas_client::lang::get("atlas.ports.open_settings"))
            }
            id if id == IDS[6] => {
                for (key, value) in [
                    ("atlas.streamermode.enabled", s.password.enabled),
                    ("atlas.streamermode.spaces", s.password.preserve_spaces),
                    ("atlas.streamermode.history", s.protect_history),
                    ("atlas.streamermode.suggestions", s.suppress_suggestions),
                ] {
                    buttons.push(format!(
                        "{}: {}",
                        atlas_client::lang::get(key),
                        atlas_client::lang::get(if value { "options.on" } else { "options.off" })
                    ));
                }
                buttons.push(atlas_client::lang::get("atlas.ports.edit_config"));
            }
            id if id == IDS[8] => {
                buttons.push(format!(
                    "{}: {:?}",
                    atlas_client::lang::get("atlas.chunkanimator.mode"),
                    s.chunk_mode
                ));
                buttons.push(format!(
                    "{}: {:?}",
                    atlas_client::lang::get("atlas.chunkanimator.easing"),
                    s.easing
                ));
                buttons.push(format!(
                    "{}: {} ms",
                    atlas_client::lang::get("atlas.chunkanimator.duration"),
                    s.duration
                ));
                buttons.push(format!(
                    "{}: {}",
                    atlas_client::lang::get("atlas.chunkanimator.disable_near"),
                    atlas_client::lang::get(if s.disable_near {
                        "options.on"
                    } else {
                        "options.off"
                    })
                ));
            }
            id if id == IDS[9] => buttons.push(atlas_client::lang::get("atlas.ports.edit_config")),
            _ => {}
        }
    }
    pub(in crate::game) fn atlas_actions_settings_click(&mut self, id: &str, index: usize) {
        if id == IDS[2] || id == IDS[7] {
            if index == 0 {
                self.atlas_ports.actions.active_screen = Some(id.into());
                self.atlas_open_port_screen(id);
            }
            return;
        }
        if (id == IDS[6] && index == 4) || (id == IDS[9] && index == 0) {
            self.atlas_visual_open_editor(id);
            return;
        }
        let s = &mut self.atlas_ports.actions;
        if id == IDS[6] {
            match index {
                0 => s.password.enabled = !s.password.enabled,
                1 => s.password.preserve_spaces = !s.password.preserve_spaces,
                2 => s.protect_history = !s.protect_history,
                3 => s.suppress_suggestions = !s.suppress_suggestions,
                _ => return,
            }
            s.save_password();
        } else if id == IDS[8] {
            match index {
                0 => {
                    let modes = [
                        chunk::Mode::Below,
                        chunk::Mode::Above,
                        chunk::Mode::Hybrid,
                        chunk::Mode::HorizontalSlide,
                        chunk::Mode::HorizontalSlideAlternate,
                    ];
                    let i = modes.iter().position(|v| *v == s.chunk_mode).unwrap_or(0);
                    s.chunk_mode = modes[(i + 1) % modes.len()];
                }
                1 => {
                    let easings = [
                        chunk::Easing::Linear,
                        chunk::Easing::Quad,
                        chunk::Easing::Cubic,
                        chunk::Easing::Quart,
                        chunk::Easing::Quint,
                        chunk::Easing::Expo,
                        chunk::Easing::Sine,
                        chunk::Easing::Circ,
                        chunk::Easing::Back,
                        chunk::Easing::Bounce,
                        chunk::Easing::Elastic,
                    ];
                    let i = easings.iter().position(|v| *v == s.easing).unwrap_or(6);
                    s.easing = easings[(i + 1) % easings.len()];
                }
                2 => {
                    s.duration = if s.duration >= 10000 {
                        0
                    } else {
                        (s.duration + 100).min(10000)
                    }
                }
                3 => s.disable_near = !s.disable_near,
                _ => return,
            }
            s.save_chunks();
        }
    }
}
fn pose_matrix(ops: impl IntoIterator<Item = viewmodel::PoseOp>) -> glam::Mat4 {
    let mut matrix = glam::Mat4::IDENTITY;
    for op in ops {
        let next = match op {
            viewmodel::PoseOp::Translate(v) => {
                glam::Mat4::from_translation(glam::Vec3::from_array(v))
            }
            viewmodel::PoseOp::Scale(v) => glam::Mat4::from_scale(glam::Vec3::from_array(v)),
            viewmodel::PoseOp::RotateX(v) => glam::Mat4::from_rotation_x(v.to_radians()),
            viewmodel::PoseOp::RotateY(v) => glam::Mat4::from_rotation_y(v.to_radians()),
            viewmodel::PoseOp::RotateZ(v) => glam::Mat4::from_rotation_z(v.to_radians()),
        };
        matrix *= next;
    }
    matrix
}

// AutoCraft: preserve the original order of slot clicks and tick delays. No
// recipe injection or direct inventory writes: the ordinary menu predicts and
// sends each operation, so the server remains authoritative.
struct Ingredient {
    item: &'static str,
    targets: &'static [usize],
}
fn ingredients(recipe: usize) -> &'static [Ingredient] {
    const CRYSTAL: &[Ingredient] = &[
        Ingredient {
            item: "minecraft:glass",
            targets: &[100, 101, 102, 103, 105, 106, 108],
        },
        Ingredient {
            item: "minecraft:ender_eye",
            targets: &[104],
        },
        Ingredient {
            item: "minecraft:ghast_tear",
            targets: &[107],
        },
    ];
    const EYE: &[Ingredient] = &[
        Ingredient {
            item: "minecraft:ender_pearl",
            targets: &[100],
        },
        Ingredient {
            item: "minecraft:blaze_powder",
            targets: &[101],
        },
    ];
    const ARROW: &[Ingredient] = &[
        Ingredient {
            item: "minecraft:arrow",
            targets: &[100, 101, 102, 103, 105, 106, 107, 108],
        },
        Ingredient {
            item: "minecraft:lingering_potion",
            targets: &[104],
        },
    ];
    match recipe {
        0 => CRYSTAL,
        1 => EYE,
        _ => ARROW,
    }
}
fn matches_ingredient(s: &ItemStack, g: &Ingredient) -> bool {
    &*s.item == g.item
        && (g.item != "minecraft:lingering_potion"
            || matches!(
                s.potion(),
                Some(
                    "minecraft:harming" | "minecraft:strong_harming" | "harming" | "strong_harming"
                )
            )
            || s.custom_potion_effects()
                .iter()
                .any(|(id, _, _)| id == "instant_damage" || id == "minecraft:instant_damage"))
}
impl Game<'_> {
    fn atlas_crafting_table(&self) -> bool {
        self.screen == Screen::Inventory
            && self.craft_size == 3
            && (self.remote.is_none() || self.remote_menu().1 == "minecraft:crafting")
    }
    fn atlas_craft_empty(&self) -> Option<usize> {
        (9..36).chain(0..9).find(|i| self.inventory[*i].is_none())
    }
    fn atlas_craft_find(&self, g: &Ingredient) -> Option<usize> {
        (9..36).chain(0..9).find(|i| {
            self.inventory[*i]
                .as_ref()
                .is_some_and(|s| matches_ingredient(s, g))
        })
    }
    fn atlas_craft_available(&self, recipe: usize) -> bool {
        ingredients(recipe).iter().all(|g| {
            self.inventory
                .iter()
                .flatten()
                .filter(|s| matches_ingredient(s, g))
                .map(|s| s.count)
                .sum::<u32>()
                + g.targets
                    .iter()
                    .filter_map(|i| self.craft_grid[i - 100].as_ref())
                    .filter(|s| matches_ingredient(s, g))
                    .map(|s| s.count)
                    .sum::<u32>()
                >= g.targets.len() as u32
        })
    }
    fn atlas_craft_clear(&mut self) {
        for i in 100..109 {
            if self.craft_grid[i - 100].is_some() {
                self.menu_ops(&[(Some(i), 0, Input::QuickMove)]);
            }
        }
    }
    pub(in crate::game) fn atlas_actions_craft_toggle(&mut self, recipe: usize) -> bool {
        if !self.atlas_port_on(IDS[0]) || !self.atlas_crafting_table() || recipe >= 3 {
            return false;
        }
        if self.atlas_ports.actions.autocraft == Some(recipe) {
            self.atlas_craft_stop(true);
        } else if self.atlas_craft_available(recipe) {
            let s = &mut self.atlas_ports.actions;
            s.autocraft = Some(recipe);
            s.craft_delay = 0;
            s.craft_count = 0;
            let name = self.atlas_craft_name(recipe);
            self.atlas_craft_notice("atlas.autocraft.started", &[&name], true);
        } else {
            if let Some((name, required, available)) = self.atlas_craft_missing(recipe) {
                self.atlas_craft_notice(
                    "atlas.autocraft.missing_detail",
                    &[&name, &required, &available],
                    false,
                );
            }
        }
        true
    }
    fn atlas_craft_fill(&mut self, g: &Ingredient) {
        if g.targets.len() == 1 {
            let slot = g.targets[0];
            if let Some(s) = self.craft_grid[slot - 100].as_ref() {
                if !matches_ingredient(s, g) {
                    self.menu_ops(&[(Some(slot), 0, Input::QuickMove)]);
                } else if s.count >= self.items().get(&s.item).map_or(64, |i| i.max_stack)
                    || self.items().get(&s.item).is_some_and(|i| i.max_stack == 1)
                {
                    return;
                }
            }
            if let Some(source) = self.atlas_craft_find(g) {
                self.menu_ops(&[(Some(source), 0, Input::Pickup)]);
                if self.carried.is_some() {
                    self.menu_ops(&[(Some(slot), 0, Input::Pickup)]);
                    self.atlas_craft_return(source);
                }
            }
        } else {
            for _ in 0..8 {
                let Some(source) = self.atlas_craft_find(g) else {
                    break;
                };
                if !g.targets.iter().any(|i| {
                    self.craft_grid[i - 100].as_ref().is_none_or(|s| {
                        s.count < self.items().get(&s.item).map_or(64, |i| i.max_stack)
                    })
                }) {
                    break;
                }
                self.menu_ops(&[(Some(source), 0, Input::Pickup)]);
                if let Some(carried) = self.carried.as_ref() {
                    if carried.count >= g.targets.len() as u32 {
                        let mut ops = vec![(None, 0, Input::QuickCraft)];
                        ops.extend(g.targets.iter().map(|i| (Some(*i), 1, Input::QuickCraft)));
                        ops.push((None, 2, Input::QuickCraft));
                        self.menu_ops(&ops);
                    } else {
                        for slot in g.targets {
                            if self.carried.is_none() {
                                break;
                            }
                            if self.craft_grid[slot - 100].is_none() {
                                self.menu_ops(&[(Some(*slot), 1, Input::Pickup)]);
                            }
                        }
                    }
                    self.atlas_craft_return(source);
                }
            }
        }
    }
    fn atlas_craft_return(&mut self, source: usize) {
        if self.carried.is_some() {
            self.menu_ops(&[(Some(source), 0, Input::Pickup)]);
            if self.carried.is_some()
                && let Some(empty) = self.atlas_craft_empty()
            {
                self.menu_ops(&[(Some(empty), 0, Input::Pickup)]);
            }
        }
    }
    fn atlas_actions_craft_tick(&mut self) {
        let Some(recipe) = self.atlas_ports.actions.autocraft else {
            return;
        };
        if !self.atlas_crafting_table() || !self.atlas_port_on(IDS[0]) {
            self.atlas_craft_stop(false);
            return;
        }
        if self.atlas_ports.actions.craft_delay > 0 {
            self.atlas_ports.actions.craft_delay -= 1;
            return;
        }
        if self.carried.is_some() {
            let target = self.atlas_craft_empty().unwrap_or(9);
            self.menu_ops(&[(Some(target), 0, Input::Pickup)]);
            self.atlas_ports.actions.craft_delay = 1;
            return;
        }
        if self.craft_result().is_some() {
            self.menu_ops(&[(Some(200), 0, Input::QuickMove)]);
            self.atlas_ports.actions.craft_count += 1;
            let name = self.atlas_craft_name(recipe);
            self.atlas_craft_notice("atlas.autocraft.crafting", &[&name], true);
            self.atlas_ports.actions.craft_delay = 1;
            return;
        }
        if self.craft_grid[4]
            .as_ref()
            .is_some_and(|s| &*s.item == "minecraft:glass_bottle")
        {
            self.menu_ops(&[(Some(104), 0, Input::QuickMove)]);
            self.atlas_ports.actions.craft_delay = 1;
            return;
        }
        if (100..109).any(|i| {
            self.craft_grid[i - 100].as_ref().is_some_and(|s| {
                !ingredients(recipe)
                    .iter()
                    .any(|g| g.targets.contains(&i) && matches_ingredient(s, g))
            })
        }) {
            self.atlas_craft_clear();
            self.atlas_ports.actions.craft_delay = 1;
            return;
        }
        if !self.atlas_craft_available(recipe) {
            self.atlas_craft_clear();
            self.atlas_craft_stop(false);
            return;
        }
        if self.atlas_craft_empty().is_none() {
            self.atlas_craft_notice("atlas.autocraft.full", &[], false);
            self.atlas_craft_stop(true);
            return;
        }
        for g in ingredients(recipe) {
            self.atlas_craft_fill(g);
        }
        if !self.atlas_craft_available(recipe)
            && ingredients(recipe)
                .iter()
                .any(|g| g.targets.iter().any(|i| self.craft_grid[i - 100].is_none()))
        {
            self.atlas_craft_clear();
            self.atlas_craft_stop(false);
            return;
        }
        if self.craft_result().is_some() {
            self.menu_ops(&[(Some(200), 0, Input::QuickMove)]);
            self.atlas_ports.actions.craft_count += 1;
            let name = self.atlas_craft_name(recipe);
            self.atlas_craft_notice("atlas.autocraft.crafting", &[&name], true);
            if self.craft_grid[4]
                .as_ref()
                .is_some_and(|s| &*s.item == "minecraft:glass_bottle")
            {
                self.menu_ops(&[(Some(104), 0, Input::QuickMove)]);
            }
        }
        self.atlas_ports.actions.craft_delay = 2;
    }
}
