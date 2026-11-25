//! In-game chat macros, based on the behavior of Macro Keybinds 1.3.1 by Thomas7520.
#[path = "macro_keybinds_ui.rs"]
mod screens;

use super::Game;
use atlas_client::lang;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
    time::{Duration, Instant},
};

pub(super) const ID: &str = "atlas-macrokeybinds";
const CONFIG: &str = "config/macrokeybinds.json";
const OPEN: &str = "key.keyboard.n";

pub(super) fn keys() -> Vec<[String; 3]> {
    vec![["open_config".into(), label("open"), OPEN.into()]]
}

fn label(key: &str) -> String {
    lang::get(&format!("atlas.macro_keybinds.{key}"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    #[default]
    Simple,
    Toggle,
    Repeat,
    Delayed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    #[default]
    Message,
    Command,
    FillChat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Modifier {
    #[default]
    None,
    Shift,
    Alt,
    Control,
}

impl Modifier {
    fn current(modifiers: winit::keyboard::ModifiersState) -> Self {
        if modifiers.shift_key() {
            Self::Shift
        } else if modifiers.alt_key() {
            Self::Alt
        } else if modifiers.control_key() {
            Self::Control
        } else {
            Self::None
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Macro {
    name: String,
    text: String,
    key: String,
    mode: Mode,
    action: Action,
    modifier: Modifier,
    milliseconds: u32,
    enabled: bool,
    #[serde(skip)]
    due: Option<Instant>,
    #[serde(skip)]
    last_action: Option<Instant>,
}

impl Default for Macro {
    fn default() -> Self {
        Self {
            name: String::new(),
            text: String::new(),
            key: String::new(),
            mode: Mode::Simple,
            action: Action::Message,
            modifier: Modifier::None,
            milliseconds: 0,
            enabled: true,
            due: None,
            last_action: None,
        }
    }
}

impl Macro {
    fn input(&mut self, key: &str, modifier: Modifier, pressed: bool, now: Instant) {
        if !self.enabled || self.key != key {
            return;
        }
        if !pressed {
            if self.mode == Mode::Toggle {
                self.due = None;
            }
            return;
        }
        if self.modifier != Modifier::None && self.modifier != modifier {
            return;
        }
        let interval = Duration::from_millis(u64::from(self.milliseconds));
        self.due = match self.mode {
            Mode::Simple => Some(now + Duration::from_millis(50)),
            Mode::Delayed => self.due.or_else(|| now.checked_add(interval)),
            Mode::Repeat if self.due.is_some() => None,
            Mode::Toggle | Mode::Repeat => self
                .last_action
                .and_then(|last| last.checked_add(interval))
                .or(Some(now)),
        };
    }

    fn tick(&mut self, now: Instant) -> bool {
        if self.due.is_none_or(|due| now < due) || !self.enabled {
            return false;
        }
        self.last_action = Some(now);
        self.due = match self.mode {
            Mode::Simple | Mode::Delayed => None,
            Mode::Toggle | Mode::Repeat => {
                now.checked_add(Duration::from_millis(u64::from(self.milliseconds)))
            }
        };
        true
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    global: Vec<Macro>,
    servers: BTreeMap<String, Vec<Macro>>,
}

#[derive(Default)]
pub(super) struct State {
    config: Config,
    screen: screens::Page,
    config_error: bool,
    pressed: BTreeSet<String>,
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        let config = match fs::read(home.join(CONFIG)) {
            Ok(bytes) => serde_json::from_slice(&bytes).ok(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Some(Config::default()),
            Err(_) => None,
        };
        Self {
            config_error: config.is_none(),
            config: config.unwrap_or_default(),
            ..Self::default()
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.config_error {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "invalid macro configuration").into(),
            );
        }
        let path = Game::game_dir().join(CONFIG);
        fs::create_dir_all(path.parent().unwrap())?;
        let pending = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&pending, serde_json::to_vec_pretty(&self.config)?)?;
        if let Err(error) = fs::rename(&pending, &path) {
            let _ = fs::remove_file(&pending);
            return Err(error.into());
        }
        Ok(())
    }

    fn entries(&self, server: Option<&str>) -> &[Macro] {
        match server {
            None => &self.config.global,
            Some(address) => self.config.servers.get(address).map_or(&[], Vec::as_slice),
        }
    }

    fn entries_mut(&mut self, server: Option<&str>) -> &mut Vec<Macro> {
        match server {
            None => &mut self.config.global,
            Some(address) => self.config.servers.entry(address.into()).or_default(),
        }
    }

    fn active_mut(&mut self, server: Option<&str>) -> impl Iterator<Item = &mut Macro> {
        self.config.global.iter_mut().chain(
            server
                .and_then(|address| self.config.servers.get_mut(address))
                .into_iter()
                .flatten(),
        )
    }

    fn clear_running(&mut self) {
        self.pressed.clear();
        for binding in self
            .config
            .global
            .iter_mut()
            .chain(self.config.servers.values_mut().flatten())
        {
            binding.due = None;
            binding.last_action = None;
        }
    }
}

impl Game<'_> {
    fn macro_server(&self) -> Option<&str> {
        self.remote
            .as_ref()
            .filter(|remote| remote.joined && self.integrated.is_none())
            .map(|remote| remote.session.address.as_str())
            .filter(|address| !address.is_empty() && *address != "local")
    }

    pub(in crate::game) fn atlas_macros_input(&mut self, key: &str, pressed: bool) {
        if !self.native_window() || !self.atlas_port_on(ID) {
            return;
        }
        if pressed && (!self.native_playing() || self.player.is_none()) {
            return;
        }
        if pressed && self.native_mod_key(ID, "open_config", OPEN) == key {
            self.atlas_macros_open();
            return;
        }
        let server = self.macro_server().map(str::to_owned);
        let modifier = if key.starts_with("key.mouse.") {
            Modifier::None
        } else {
            Modifier::current(self.modifiers)
        };
        let state = &mut self.atlas_ports.macro_keybinds;
        let now = Instant::now();
        let mut held = false;
        for binding in state.active_mut(server.as_deref()) {
            binding.input(key, modifier, pressed, now);
            held |= binding.key == key && binding.mode == Mode::Toggle && binding.due.is_some();
        }
        if held {
            state.pressed.insert(key.to_owned());
        } else {
            state.pressed.remove(key);
        }
    }

    pub(in crate::game) fn atlas_macros_stop(&mut self) {
        self.atlas_ports.macro_keybinds.clear_running();
    }

    pub(in crate::game) fn atlas_macros_tick(&mut self) {
        if !self.native_window() || !self.atlas_ports.installed.contains(ID) {
            return;
        }
        if !self.atlas_port_on(ID) || self.player.is_none() || self.remote_loading() {
            self.atlas_macros_stop();
            return;
        }
        let server = self.macro_server().map(str::to_owned);
        let now = Instant::now();
        let playing = self.native_playing();
        let focused = self.focused || self.control.is_some();
        let modifiers = Modifier::current(self.modifiers);
        let state = &mut self.atlas_ports.macro_keybinds;
        let pressed = &state.pressed;
        let cfg = &mut state.config;
        let mut ready = Vec::new();
        for binding in cfg.global.iter_mut().chain(
            server
                .as_deref()
                .and_then(|key| cfg.servers.get_mut(key))
                .into_iter()
                .flatten(),
        ) {
            if binding.due.is_none() {
                continue;
            }
            if binding.mode == Mode::Toggle
                && (!playing
                    || !focused
                    || !pressed.contains(&binding.key)
                    || (binding.modifier != Modifier::None && binding.modifier != modifiers))
            {
                binding.due = None;
            }
            if (binding.action != Action::FillChat || playing) && binding.tick(now) {
                ready.push((binding.action, binding.text.clone()));
            }
        }
        for (action, text) in ready {
            match action {
                Action::FillChat => {
                    let _ = self.native_open_chat(&text);
                }
                Action::Message if self.remote.is_some() && self.chat_abilities().allows(&text) => {
                    self.send_signed_chat(&text)
                }
                Action::Message if self.remote.is_none() && !text.starts_with('/') => {
                    self.handle_chat_input(&text)
                }
                Action::Message => {}
                Action::Command => {
                    let line = format!("/{}", text.trim_start_matches('/'));
                    if self.remote.is_some() {
                        let _ = self.native_send_chat(&line);
                    } else {
                        self.handle_chat_input(&line);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_modes_fire_once_hold_repeat_or_delay() {
        let now = Instant::now();
        let later = |ms| now + Duration::from_millis(ms);
        let mut binding = Macro {
            key: "key.keyboard.g".into(),
            modifier: Modifier::Control,
            milliseconds: 100,
            ..Macro::default()
        };
        binding.input("key.keyboard.g", Modifier::None, true, now);
        assert!(!binding.tick(later(50)));
        binding.input("key.keyboard.g", Modifier::Control, true, now);
        assert!(!binding.tick(later(49)));
        assert!(binding.tick(later(50)));
        assert!(!binding.tick(later(51)));
        binding.mode = Mode::Delayed;
        binding.input("key.keyboard.g", Modifier::Control, true, now);
        binding.input("key.keyboard.g", Modifier::Control, true, later(50));
        assert!(!binding.tick(later(99)));
        assert!(binding.tick(later(100)));
        binding.mode = Mode::Toggle;
        binding.input("key.keyboard.g", Modifier::Control, true, later(150));
        assert!(!binding.tick(later(199)));
        assert!(binding.tick(later(200)));
        binding.input("key.keyboard.g", Modifier::Control, false, later(220));
        assert!(!binding.tick(later(300)));
        binding.mode = Mode::Repeat;
        binding.input("key.keyboard.g", Modifier::Control, true, later(400));
        assert!(binding.tick(later(400)));
        assert!(binding.tick(later(500)));
        binding.input("key.keyboard.g", Modifier::Control, true, later(520));
        assert!(!binding.tick(later(600)));
    }
}
