//! Settings and key defaults for the independently implemented Atlas minimap.
use super::config_model::{Category, Definition, Field, Frame, Kind};
use crate::mod_api::NativeMinimap;
use atlas_client::{extras::Corner, lang};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::Path};

pub(super) const ID: &str = "atlas-minimap";
const FILE: &str = "config/atlas-minimap.json";
const UNBOUND: &str = "key.keyboard.unknown";

pub(super) const BINDINGS: &[(&str, &str)] = &[
    ("settings", "key.keyboard.y"),
    ("new_waypoint", "key.keyboard.b"),
    ("waypoints", "key.keyboard.u"),
    ("enlarge", "key.keyboard.z"),
    ("temporary_waypoint", "key.keyboard.keypad.add"),
    ("zoom_in", UNBOUND),
    ("zoom_out", UNBOUND),
    ("toggle_map", UNBOUND),
    ("toggle_waypoints", UNBOUND),
    ("toggle_map_waypoints", UNBOUND),
    ("toggle_radar", UNBOUND),
    ("toggle_grid", UNBOUND),
    ("toggle_cave", UNBOUND),
];

pub(super) fn keys() -> Vec<[String; 3]> {
    BINDINGS
        .iter()
        .map(|&(id, key)| [id.into(), label(&format!("key.{id}")), key.into()])
        .collect()
}

pub(super) fn label(name: &str) -> String {
    lang::get(&format!("atlas.minimap.{name}"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::game) enum CaveMode {
    #[default]
    Auto,
    Surface,
    Cave,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::game) enum Position {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl From<Position> for Corner {
    fn from(position: Position) -> Self {
        match position {
            Position::TopLeft => Self::TopLeft,
            Position::TopRight => Self::TopRight,
            Position::BottomLeft => Self::BottomLeft,
            Position::BottomRight => Self::BottomRight,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub(in crate::game) struct Settings {
    pub enabled: bool,
    pub corner: Position,
    /// Zero sizes the map to the viewport.
    pub size: u16,
    pub zoom: u8,
    pub rotate: bool,
    pub round: bool,
    pub cave: CaveMode,
    pub coordinates: bool,
    pub cardinal: bool,
    pub biome: bool,
    pub light: bool,
    pub time: bool,
    pub radar: bool,
    pub radar_players: bool,
    pub radar_mobs: bool,
    pub radar_items: bool,
    /// Maximum difference in Y, with zero allowing every height.
    pub radar_height: u16,
    pub grid: bool,
    pub map_waypoints: bool,
    pub world_waypoints: bool,
    pub death_waypoints: bool,
    pub waypoint_distance: bool,
    pub waypoint_max_distance: u32,
    pub enlarge_toggle: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            corner: Position::TopLeft,
            size: 0,
            zoom: 1,
            rotate: true,
            round: false,
            cave: CaveMode::Auto,
            coordinates: true,
            cardinal: true,
            biome: false,
            light: false,
            time: false,
            radar: true,
            radar_players: true,
            radar_mobs: true,
            radar_items: true,
            radar_height: 20,
            grid: false,
            map_waypoints: true,
            world_waypoints: true,
            death_waypoints: true,
            waypoint_distance: true,
            waypoint_max_distance: 0,
            enlarge_toggle: false,
        }
    }
}

impl Settings {
    pub(super) fn load(home: &Path) -> Self {
        let path = home.join(FILE);
        if !fs::metadata(&path).is_ok_and(|meta| meta.len() <= 16 * 1024) {
            return Self::default();
        }
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .map(Self::normalized)
            .unwrap_or_default()
    }

    pub(super) fn config_value(&self) -> Value {
        serde_json::to_value(self).expect("minimap settings are serializable")
    }

    pub(super) fn default_config() -> Value {
        Self::default().config_value()
    }

    pub(super) fn set_config(&mut self, home: &Path, value: Value) -> Result<(), String> {
        let settings = serde_json::from_value::<Self>(value)
            .map_err(|err| err.to_string())?
            .normalized();
        settings.save(home)?;
        *self = settings;
        Ok(())
    }

    pub(super) fn save(&self, home: &Path) -> Result<(), String> {
        let path = home.join(FILE);
        let pending = path.with_extension(format!("{}.tmp", std::process::id()));
        let bytes = serde_json::to_vec_pretty(&self.normalized()).map_err(|err| err.to_string())?;
        fs::create_dir_all(home.join("config")).map_err(|err| err.to_string())?;
        let result = fs::write(&pending, bytes).and_then(|()| fs::rename(&pending, &path));
        if result.is_err() {
            let _ = fs::remove_file(pending);
        }
        result.map_err(|err| err.to_string())
    }

    fn normalized(mut self) -> Self {
        if self.size > 0 {
            self.size = self.size.clamp(55, 250);
        }
        self.zoom = self.zoom.clamp(1, 6);
        self.radar_height = self.radar_height.min(256);
        self.waypoint_max_distance = self.waypoint_max_distance.min(30_000_000);
        self
    }

    pub(in crate::game) fn size_for_view(&self, view: [f32; 2], enlarged: bool) -> f32 {
        let shortest = view[0].min(view[1]).max(0.0);
        let size = if self.size == 0 {
            (shortest * 260.0 / 1080.0).floor().clamp(55.0, 250.0)
        } else {
            f32::from(self.size)
        };
        let wanted = if enlarged { size * 2.0 } else { size };
        wanted.min((shortest - 20.0).max(16.0)).min(256.0)
    }

    pub(in crate::game) fn request(&self, enlarged: bool) -> Option<NativeMinimap> {
        self.enabled.then(|| NativeMinimap {
            corner: self.corner.into(),
            size: self.size_for_view([640.0, 360.0], enlarged),
            blocks_per_pixel: (if enlarged { 2.0 } else { 1.0 }) / f32::from(self.zoom.max(1)),
            rotate: self.rotate,
            round: self.round,
        })
    }

    pub(super) fn change_zoom(&mut self, direction: i8) {
        self.zoom = self.zoom.saturating_add_signed(direction).clamp(1, 6);
    }
}

fn field(name: &str, kind: Kind) -> Field {
    Field::new(&format!("/{name}"), &format!("atlas.minimap.{name}"), kind)
}

fn toggle(name: &str) -> Field {
    field(name, Kind::Bool)
}

fn number(name: &str, min: i64, max: i64, slider: bool) -> Field {
    field(
        name,
        Kind::Int {
            min,
            max,
            step: 1,
            slider,
        },
    )
}

fn choice(name: &str, values: &[Value], names: &[&str]) -> Field {
    field(
        name,
        Kind::Enum {
            values: values.to_vec(),
            labels: names.iter().map(|name| label(name)).collect(),
        },
    )
}

pub(super) fn definition() -> Definition {
    let category = |name, fields| Category {
        label: label(name),
        fields,
    };
    let mut size = number("size", 0, 250, false);
    size.tooltip.push(label("auto_size_hint"));
    Definition {
        title: label("title"),
        frame: Frame::Cloth,
        categories: vec![
            category(
                "view",
                vec![
                    toggle("enabled"),
                    choice(
                        "corner",
                        &[
                            Value::from("top_left"),
                            Value::from("top_right"),
                            Value::from("bottom_left"),
                            Value::from("bottom_right"),
                        ],
                        &["top_left", "top_right", "bottom_left", "bottom_right"],
                    ),
                    choice(
                        "round",
                        &[Value::Bool(false), Value::Bool(true)],
                        &["square", "circle"],
                    ),
                    size,
                    number("zoom", 1, 6, true),
                    toggle("rotate"),
                    choice(
                        "cave",
                        &[
                            Value::from("auto"),
                            Value::from("surface"),
                            Value::from("cave"),
                        ],
                        &["cave_auto", "cave_surface", "cave_only"],
                    ),
                    toggle("grid"),
                    toggle("enlarge_toggle"),
                ],
            ),
            category(
                "info",
                vec![
                    toggle("coordinates"),
                    toggle("cardinal"),
                    toggle("biome"),
                    toggle("light"),
                    toggle("time"),
                ],
            ),
            category(
                "entities",
                vec![
                    toggle("radar"),
                    toggle("radar_players"),
                    toggle("radar_mobs"),
                    toggle("radar_items"),
                    number("radar_height", 0, 256, false),
                ],
            ),
            category(
                "waypoints",
                vec![
                    toggle("map_waypoints"),
                    toggle("world_waypoints"),
                    toggle("death_waypoints"),
                    toggle("waypoint_distance"),
                    number("waypoint_max_distance", 0, 30_000_000, false),
                ],
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_size_tracks_the_view_and_enlarging_stays_inside_it() {
        let settings = Settings::default();
        assert_eq!(settings.size_for_view([640.0, 360.0], false), 86.0);
        assert_eq!(settings.size_for_view([640.0, 360.0], true), 172.0);
        assert_eq!(settings.size_for_view([640.0, 170.0], false), 55.0);
        assert_eq!(settings.size_for_view([140.0, 48.0], true), 28.0);
    }

    #[test]
    fn zoom_steps_are_clamped_and_change_world_coverage() {
        let mut settings = Settings::default();
        settings.change_zoom(-1);
        assert_eq!(settings.request(false).unwrap().blocks_per_pixel, 1.0);
        settings.change_zoom(1);
        assert_eq!(settings.request(false).unwrap().blocks_per_pixel, 0.5);
        assert_eq!(settings.request(true).unwrap().blocks_per_pixel, 1.0);
        settings.change_zoom(20);
        assert_eq!(settings.zoom, 6);
    }
}
