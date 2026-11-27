//! Client-only minimap waypoints. A scope key identifies a world and dimension.
#[path = "minimap_waypoints_ui.rs"]
mod screens;

use super::{Game, minimap_settings::ID};
use atlas_client::{
    hud::{HudAtlas, HudBatch, rgba},
    lang,
    ui_theme::{Face, Ui, color},
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    hash::{BuildHasher, RandomState},
    io::{self, Read, Write},
    path::Path,
};

const FILE: &str = "config/atlas-minimap-waypoints.json";
const MAX_FILE: u64 = 1024 * 1024;
const MAX_SCOPES: usize = 512;
const MAX_POINTS: usize = 256;
const MAX_NAME: usize = 64;
const MAX_SYMBOL: usize = 2;

const COLORS: [u32; 16] = [
    0xffff9c64, 0xfff0ca52, 0xffb4dc6e, 0xff6fd099, 0xff58cbd0, 0xff86b8ff, 0xffa594ff, 0xffe196e8,
    0xfffb8cad, 0xffe36869, 0xffbe9166, 0xff9caa63, 0xff8aa1ab, 0xffb7b7b7, 0xffe9e7de, 0xff6a7486,
];

pub(super) fn label(key: &str) -> String {
    lang::get(&format!("atlas.minimap.waypoints.{key}"))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::game) struct Waypoint {
    pub(in crate::game) name: String,
    pub(in crate::game) symbol: String,
    pub(in crate::game) position: [i32; 3],
    pub(in crate::game) color: u32,
    pub(in crate::game) enabled: bool,
    pub(in crate::game) global: bool,
    #[serde(skip)]
    pub(in crate::game) temporary: bool,
    #[serde(default)]
    death: bool,
    #[serde(default)]
    old_death: bool,
}

impl Waypoint {
    pub(super) fn display_name(&self) -> Cow<'_, str> {
        if self.death {
            label(if self.old_death {
                "old_death"
            } else {
                "recent_death"
            })
            .into()
        } else {
            Cow::Borrowed(&self.name)
        }
    }

    pub(super) fn draw_badge(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        [x, y]: [f32; 2],
        size: f32,
    ) {
        let ink = if self.enabled {
            self.color & 0xffffff
        } else {
            0x737781
        };
        let diamond = |r| [[x, y - r], [x + r, y], [x, y + r], [x - r, y]];
        let edge = rgba(20, 23, 27, 255);
        batch.fill_quad(atlas, diamond(size * 0.60), edge);
        batch.fill_quad(atlas, diamond(size * 0.49), color::hex(ink, 255));
        if self.death {
            let bone = rgba(255, 244, 226, 255);
            let px = size / 10.0;
            for (left, top, w, h, shade) in [
                (-1.8, -3.0, 3.6, 1.0, bone),
                (-3.0, -2.0, 6.0, 3.5, bone),
                (-1.75, 1.5, 3.5, 1.5, bone),
                (-2.2, -1.0, 1.45, 1.55, edge),
                (0.75, -1.0, 1.45, 1.55, edge),
                (-0.35, 1.85, 0.7, 1.15, edge),
            ] {
                batch.fill(atlas, x + left * px, y + top * px, w * px, h * px, shade);
            }
        } else {
            let [_, r, g, b] = ink.to_be_bytes().map(u32::from);
            let text = if 299 * r + 587 * g + 114 * b < 132_000 {
                u32::MAX
            } else {
                edge
            };
            let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
            let font = size * 0.64;
            let width = ui.measure(Face::SemiBold, font, &self.symbol);
            let font = font * (size * 0.68 / width.max(1.0)).min(1.0);
            let width = ui.measure(Face::SemiBold, font, &self.symbol);
            ui.text_line(
                Face::SemiBold,
                font,
                size,
                x - width / 2.0,
                y - size / 2.0,
                text,
                &self.symbol,
            );
            ui.finish();
        }
    }

    fn valid(&self) -> bool {
        let text = |value: &str, max| {
            !value.trim().is_empty()
                && value.encode_utf16().count() <= max
                && !value.chars().any(char::is_control)
        };
        text(&self.name, MAX_NAME)
            && text(&self.symbol, MAX_SYMBOL)
            && self.position.iter().all(|n| n.unsigned_abs() <= 30_000_000)
            && self.color >> 24 == 255
    }

    fn draft(position: [i32; 3]) -> Self {
        Self {
            name: String::new(),
            symbol: String::new(),
            position,
            color: COLORS[RandomState::new().hash_one(position) as usize % COLORS.len()],
            enabled: true,
            global: true,
            temporary: false,
            death: false,
            old_death: false,
        }
    }

    fn record_death(points: &mut Vec<Self>, position: [i32; 3]) {
        for point in points
            .iter_mut()
            .filter(|point| point.death && !point.old_death)
        {
            point.old_death = true;
            point.name = label("old_death");
        }
        points.push(Self {
            name: label("recent_death"),
            symbol: "×".into(),
            color: COLORS[9],
            death: true,
            ..Self::draft(position)
        });
    }
}

type Catalog = BTreeMap<String, Vec<Waypoint>>;

#[derive(Default)]
pub(super) struct State {
    worlds: Catalog,
    saved: Option<Vec<u8>>,
    unreadable: bool,
    screen: Option<screens::Screen>,
}

#[derive(Debug)]
enum SaveError {
    Limit,
    Data,
    Io(io::Error),
}

impl From<io::Error> for SaveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl SaveError {
    fn message(&self) -> String {
        match self {
            Self::Limit => label("limit"),
            Self::Data => label("load_error"),
            Self::Io(error) => {
                log::warn!("Minimap waypoints: {error}");
                label("save_error")
            }
        }
    }
}

fn read_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(io::Error::other("waypoint file exceeds its size limit"));
    }
    Ok(Some(bytes))
}

fn valid_scope(key: &str) -> bool {
    !key.is_empty() && key.len() <= 1024
}

impl State {
    pub(super) fn load(home: &Path) -> Self {
        let mut state = Self::default();
        state.refresh(home);
        state
    }

    fn refresh(&mut self, home: &Path) {
        let Ok(saved) = read_file(&home.join(FILE)) else {
            self.unreadable = true;
            return;
        };
        if saved == self.saved && !self.unreadable {
            return;
        }
        let mut worlds: Catalog = match saved.as_deref().map(serde_json::from_slice) {
            None => Catalog::new(),
            Some(Ok(worlds)) => worlds,
            Some(Err(_)) => {
                self.unreadable = true;
                return;
            }
        };
        if worlds.len() > MAX_SCOPES
            || worlds.iter().any(|(key, points)| {
                !valid_scope(key) || points.len() > MAX_POINTS || points.iter().any(|p| !p.valid())
            })
        {
            self.unreadable = true;
            return;
        }
        for (key, points) in std::mem::take(&mut self.worlds) {
            let temporary = points.into_iter().filter(|p| p.temporary);
            let target = worlds.entry(key).or_default();
            target.extend(temporary.take(MAX_POINTS.saturating_sub(target.len())));
        }
        worlds.retain(|_, points| !points.is_empty());
        self.worlds = worlds;
        self.saved = saved;
        self.unreadable = false;
    }

    pub(in crate::game) fn entries(&self, key: &str) -> &[Waypoint] {
        self.worlds.get(key).map_or(&[], Vec::as_slice)
    }

    pub(in crate::game) fn clear_temporary(&mut self) {
        for points in self.worlds.values_mut() {
            points.retain(|point| !point.temporary);
        }
        self.worlds.retain(|_, points| !points.is_empty());
        self.screen = None;
    }

    fn edit(
        &mut self,
        home: &Path,
        key: &str,
        change: impl FnOnce(&mut Vec<Waypoint>),
    ) -> Result<(), SaveError> {
        let old = self.entries(key);
        let mut next = old.to_vec();
        change(&mut next);
        if !valid_scope(key)
            || next.len() > MAX_POINTS
            || (old.is_empty() && self.worlds.len() >= MAX_SCOPES)
        {
            return Err(SaveError::Limit);
        }
        if next.iter().any(|point| !point.valid()) {
            return Err(SaveError::Data);
        }

        if !old
            .iter()
            .filter(|p| !p.temporary)
            .eq(next.iter().filter(|p| !p.temporary))
        {
            if self.unreadable {
                return Err(SaveError::Data);
            }
            let mut persisted: BTreeMap<&str, Vec<&Waypoint>> = self
                .worlds
                .iter()
                .map(|(key, points)| {
                    (
                        key.as_str(),
                        points.iter().filter(|point| !point.temporary).collect(),
                    )
                })
                .collect();
            persisted.insert(key, next.iter().filter(|point| !point.temporary).collect());
            persisted.retain(|_, points| !points.is_empty());
            let bytes = serde_json::to_vec(&persisted).map_err(io::Error::other)?;
            if bytes.len() as u64 > MAX_FILE {
                return Err(SaveError::Limit);
            }
            self.write(home, &bytes)?;
            self.saved = Some(bytes);
        }
        if next.is_empty() {
            self.worlds.remove(key);
        } else {
            self.worlds.insert(key.to_owned(), next);
        }
        Ok(())
    }

    fn write(&self, home: &Path, bytes: &[u8]) -> io::Result<()> {
        let path = home.join(FILE);
        fs::create_dir_all(path.parent().expect("waypoint configuration directory"))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))?;
        lock.try_lock()?;
        if read_file(&path)? != self.saved {
            return Err(io::Error::other("waypoints changed in another session"));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let pending = path.with_extension(format!("{}-{}.tmp", std::process::id(), now.as_nanos()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        let written = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&pending, &path)
        })();
        if written.is_err() {
            let _ = fs::remove_file(pending);
        }
        written
    }
}

impl Game<'_> {
    pub(in crate::game) fn atlas_minimap_death_waypoint(&mut self, position: [i32; 3]) {
        if !self.atlas_port_on(ID) {
            return;
        }
        let Some(scope) = self.atlas_minimap_world_key() else {
            return;
        };
        let home = Self::game_dir();
        let state = &mut self.atlas_ports.minimap_waypoints;
        state.refresh(&home);
        let result = state.edit(&home, &scope, |points| {
            Waypoint::record_death(points, position);
        });
        if let Err(error) = result {
            log::warn!("Could not save the latest death waypoint: {error:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn scope_and_temporary_markers_survive_only_their_intended_sessions() {
        let home = std::env::temp_dir().join(format!(
            "atlas-waypoint-tests-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&home).unwrap();
        let spawn = Waypoint {
            name: "Test base".into(),
            symbol: "B".into(),
            ..Waypoint::draft([21, 65, -18])
        };
        let mut state = State::load(&home);
        let mut stale = State::load(&home);
        state
            .edit(&home, "overworld", |points| points.push(spawn.clone()))
            .unwrap();
        assert!(
            stale
                .edit(&home, "the_nether", |points| points.push(spawn.clone()))
                .is_err()
        );
        stale.refresh(&home);
        stale
            .edit(&home, "the_nether", |points| points.push(spawn.clone()))
            .unwrap();
        stale
            .edit(&home, "overworld", |points| {
                points.push(Waypoint {
                    temporary: true,
                    ..spawn.clone()
                })
            })
            .unwrap();
        assert_eq!(stale.entries("overworld").len(), 2);
        stale.clear_temporary();
        let reloaded = State::load(&home);
        for state in [stale, reloaded] {
            assert_eq!(state.entries("overworld"), std::slice::from_ref(&spawn));
            assert_eq!(state.entries("the_nether"), std::slice::from_ref(&spawn));
            assert!(state.entries("different_world").is_empty());
        }
        let path = home.join(FILE);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("lock")).unwrap();
        fs::remove_dir(home.join("config")).unwrap();
        fs::remove_dir(&home).unwrap();
    }
}
