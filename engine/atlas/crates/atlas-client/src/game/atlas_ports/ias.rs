//! Atlas adaptation of IAS 9.0.7 (VidTu, LGPL-3.0-or-later).
//! The launcher brokers saved profiles and Microsoft authorization.
#[path = "ias_ui.rs"]
mod ui;

use super::Game;
use atlas_client::{lang, ui::EditBox};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, Instant},
};

pub(super) const ID: &str = "atlas-ias";
const ACCOUNT_SCREEN: &str = "atlas-ias-accounts";

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Config {
    title_text: bool,
    title_button: bool,
    servers_text: bool,
    servers_button: bool,
    close_on_login: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title_text: true,
            title_button: true,
            servers_text: true,
            servers_button: true,
            close_on_login: false,
        }
    }
}

impl Config {
    fn button(self, mode: &str) -> bool {
        if mode == "title" {
            self.title_button
        } else {
            self.servers_button
        }
    }

    fn text(self, mode: &str) -> bool {
        if mode == "title" {
            self.title_text
        } else {
            self.servers_text
        }
    }
}

pub(super) fn definition() -> super::config_model::Definition {
    use super::config_model::{Category, Definition, Field, Frame, Kind};
    Definition {
        title: lang::get("ias.config"),
        frame: Frame::Cloth,
        categories: vec![Category {
            label: lang::get("ias.config"),
            fields: [
                "titleText",
                "titleButton",
                "serversText",
                "serversButton",
                "closeOnLogin",
            ]
            .map(|key| Field::new(&format!("/{key}"), &format!("ias.config.{key}"), Kind::Bool))
            .into(),
        }],
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    current_id: String,
    accounts: Vec<Account>,
    #[serde(default)]
    microsoft: Microsoft,
}

#[derive(Clone, Deserialize)]
struct Account {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    uuid: Option<String>,
}

impl Account {
    fn online(&self) -> bool {
        self.kind == "microsoft"
    }
    fn uuid(&self) -> u128 {
        self.uuid
            .as_deref()
            .and_then(|v| u128::from_str_radix(&v.replace('-', ""), 16).ok())
            .unwrap_or_else(|| atlas_protocol::crypto::offline_uuid(&self.name))
    }
    fn uuid_text(&self) -> String {
        let uuid = self.uuid();
        atlas_client::save::uuid_text(std::array::from_fn(|i| (uuid >> (96 - 32 * i)) as i32))
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Microsoft {
    configured: bool,
    active: bool,
    user_code: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    Switch,
    Add,
    Edit,
    Delete,
    Move,
    Microsoft,
    CancelMicrosoft,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    request_id: &'a str,
    action: Option<Action>,
    account_id: Option<&'a str>,
    name: Option<&'a str>,
    direction: Option<i8>,
    offline: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reply {
    request_id: String,
    account_id: Option<String>,
    name: Option<String>,
    session_path: Option<PathBuf>,
    error: Option<String>,
}

struct Paths {
    catalog: PathBuf,
    request: PathBuf,
    reply: PathBuf,
}

fn paths() -> Option<&'static Paths> {
    static PATHS: OnceLock<Option<Paths>> = OnceLock::new();
    PATHS
        .get_or_init(|| {
            let absolute = |key| {
                std::env::var_os(key)
                    .map(PathBuf::from)
                    .filter(|path| path.is_absolute())
            };
            Some(Paths {
                catalog: absolute("ATLAS_ACCOUNT_SWITCH_FILE")?,
                request: absolute("ATLAS_ACCOUNT_SWITCH_REQUEST")?,
                reply: absolute("ATLAS_ACCOUNT_SWITCH_REPLY")?,
            })
        })
        .as_ref()
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    const LIMIT: u64 = 48 * 1024;
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > LIMIT {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

pub(super) struct State {
    config: Config,
    catalog: Catalog,
    last_read: Option<Instant>,
    search: EditBox,
    nickname: EditBox,
    selected: Option<String>,
    scroll: usize,
    popup: Option<Popup>,
    pending: Option<Pending>,
    last_click: Option<(String, Instant)>,
    hover: Option<(String, Instant)>,
    swallow_release: bool,
    notice: Option<String>,
}

#[derive(Clone, Copy)]
enum Popup {
    Provider(bool),
    Offline(bool),
    Delete,
    Microsoft,
    Login,
}

impl Default for State {
    fn default() -> Self {
        let mut search = EditBox::new(0.0, 11.0, 150.0, 20.0);
        search.max_length = 64;
        let mut nickname = EditBox::new(0.0, 0.0, 148.0, 20.0);
        nickname.max_length = 16;
        Self {
            catalog: Catalog::default(),
            config: Config::default(),
            last_read: None,
            search,
            nickname,
            selected: None,
            scroll: 0,
            popup: None,
            pending: None,
            last_click: None,
            hover: None,
            swallow_release: false,
            notice: None,
        }
    }
}

struct Pending {
    id: String,
    action: Action,
    started: Instant,
}

impl State {
    pub(super) fn load_config(&mut self, home: &Path) {
        self.config = read_json(&home.join("config/ias.json")).unwrap_or_default();
    }

    pub(super) fn config_value(&self) -> serde_json::Value {
        serde_json::to_value(self.config).expect("IAS config serializes")
    }

    pub(super) fn config_default() -> serde_json::Value {
        serde_json::to_value(Config::default()).expect("IAS config serializes")
    }

    pub(super) fn apply_config(
        &mut self,
        home: &Path,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let config: Config = serde_json::from_value(value).map_err(|e| e.to_string())?;
        let folder = home.join("config");
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let id = atlas_client::save::uuid_text(atlas_client::save::random_uuid());
        let tmp = folder.join(format!("ias-{id}.tmp"));
        let json = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        let result = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .and_then(|mut f| f.write_all(&json))
            .and_then(|_| fs::rename(&tmp, folder.join("ias.json")));
        if let Err(error) = result {
            let _ = fs::remove_file(&tmp);
            return Err(error.to_string());
        }
        self.config = config;
        Ok(())
    }

    fn refresh(&mut self) {
        if let Some(paths) = paths()
            && let Some(catalog) = read_json(&paths.catalog)
        {
            self.catalog = catalog;
        }
    }

    fn matching(&self) -> Vec<&Account> {
        let query = self.search.value.trim().to_lowercase();
        let mut matches = self
            .catalog
            .accounts
            .iter()
            .filter(|account| query.is_empty() || account.name.to_lowercase().contains(&query))
            .collect::<Vec<_>>();
        if !query.is_empty() {
            matches.sort_by_key(|account| !account.name.to_lowercase().starts_with(&query));
        }
        matches
    }

    fn selection(&self) -> Option<&Account> {
        let id = self.selected.as_deref()?;
        self.matching().into_iter().find(|a| a.id == id)
    }

    fn update_search(&mut self) {
        if self.selection().is_none() {
            self.selected = None;
        }
        self.scroll = 0;
    }

    fn request(&mut self, request: Request<'_>) -> bool {
        let Some(paths) = paths() else { return false };
        if self.pending.is_some() {
            return false;
        }
        let action = request.action.unwrap_or(Action::Switch);
        let id = atlas_client::save::uuid_text(atlas_client::save::random_uuid()).replace('-', "");
        let temporary = paths.request.with_extension(format!("{id}.tmp"));
        // Rebind the request's lifetime to the unique correlation id.
        let message = Request {
            request_id: &id,
            ..request
        };
        let write = || -> Result<(), Box<dyn std::error::Error>> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            serde_json::to_writer(&mut file, &message)?;
            file.flush()?;
            drop(file);
            fs::rename(&temporary, &paths.request)?;
            Ok(())
        };
        if write().is_err() {
            let _ = fs::remove_file(&temporary);
            self.notice = Some(lang::get("atlas.ias.failed"));
            return false;
        }
        self.notice = None;
        self.pending = Some(Pending {
            id,
            action,
            started: Instant::now(),
        });
        true
    }

    fn field(&mut self) -> Option<&mut EditBox> {
        match self.popup {
            Some(Popup::Offline(_)) if self.nickname.focused => Some(&mut self.nickname),
            None if self.search.focused => Some(&mut self.search),
            _ => None,
        }
    }
}

impl Game<'_> {
    fn ias_opener(&self) -> Option<&'static str> {
        if !self.atlas_port_on(ID)
            || self.screen != super::super::Screen::Playing
            || paths().is_none()
        {
            return None;
        }
        match self.title.as_ref()?.mode.id() {
            "title" => Some("title"),
            "multiplayer" => Some("multiplayer"),
            _ => None,
        }
    }

    pub(in crate::game) fn atlas_ias_active(&self) -> bool {
        self.native_mod_screen() == Some(ACCOUNT_SCREEN)
    }

    pub(in crate::game) fn atlas_ias_poll(&mut self) {
        let Some(paths) = paths() else { return };
        if !self.atlas_ias_active() && self.ias_opener().is_none() {
            return;
        }
        let now = Instant::now();
        let state = &mut self.atlas_ports.ias;
        if state
            .last_read
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_millis(150))
        {
            return;
        }
        state.last_read = Some(now);
        state.refresh();
        let Some(pending) = &state.pending else {
            return;
        };
        let action = pending.action;
        if now.saturating_duration_since(pending.started) > Duration::from_secs(90) {
            state.pending = None;
            state.notice = Some(lang::get("atlas.ias.failed"));
            return;
        }
        let Some(reply) = read_json::<Reply>(&paths.reply).filter(|r| r.request_id == pending.id)
        else {
            return;
        };
        let _ = fs::remove_file(&paths.reply);
        state.pending = None;
        if let Some(error) = reply.error {
            let error = if error == "atlas.ias.microsoft_setup" {
                "atlas.ias.microsoft_unconfigured"
            } else {
                &error
            };
            state.notice = Some(lang::get_or(error, &lang::get("atlas.ias.failed")));
            return;
        }
        if matches!(action, Action::Switch) {
            let result = reply
                .name
                .ok_or_else(|| "atlas.ias.failed".to_owned())
                .and_then(|name| self.atlas_switch_account(name, reply.session_path.as_deref()));
            match result {
                Ok(()) => {
                    if self.atlas_ports.ias.config.close_on_login {
                        self.native_mod_screen_close(ACCOUNT_SCREEN);
                    } else {
                        self.atlas_ports.ias.notice = Some(lang::get("ias.login.done"));
                    }
                }
                Err(error) => self.atlas_ports.ias.notice = Some(lang::get(&error)),
            }
            return;
        }
        if let Some(id) = reply.account_id {
            state.selected = Some(id);
        }
        match action {
            Action::Microsoft => {}
            Action::CancelMicrosoft => state.popup = None,
            _ => state.popup = None,
        }
    }
}

fn valid_name(name: &str) -> bool {
    (3..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
