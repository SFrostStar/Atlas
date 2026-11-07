use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct NoShake {
    pub disable_camera_sway: bool,
    pub disable_hand_bobbing: bool,
    pub disable_map_pitch: bool,
}
impl Default for NoShake {
    fn default() -> Self {
        Self {
            disable_camera_sway: true,
            disable_hand_bobbing: false,
            disable_map_pitch: false,
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Mention {
    pub extra_names: Vec<String>,
    pub sound_file: String,
    pub volume: f32,
    pub ignore_own_messages: bool,
    pub fallback_to_vanilla_sound: bool,
    pub cooldown_ms: u64,
}
impl Default for Mention {
    fn default() -> Self {
        Self {
            extra_names: vec![],
            sound_file: "sound.wav".into(),
            volume: 1.0,
            ignore_own_messages: true,
            fallback_to_vanilla_sound: true,
            cooldown_ms: 500,
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Zoom {
    pub initial_zoom: i32,
    pub zoom_in_time: f64,
    pub zoom_out_time: f64,
    pub zoom_in_transition: String,
    pub zoom_out_transition: String,
    pub affect_hand_fov: bool,
    pub retain_zoom_steps: bool,
    pub scroll_zoom: bool,
    pub scroll_step_count: i32,
    pub zoom_per_step: i32,
    pub scroll_zoom_smoothness: i32,
    pub zoom_key_behaviour: String,
    pub keybind_scrolling: bool,
    pub relative_sensitivity: i32,
    pub relative_view_bobbing: bool,
    pub cinematic_camera: i32,
    pub spyglass_behaviour: String,
    pub spyglass_overlay_visibility: String,
    pub spyglass_sound_behaviour: String,
    pub secondary_zoom_amount: i32,
    pub secondary_zoom_in_time: f64,
    pub secondary_zoom_out_time: f64,
    pub secondary_hide_hud_on_zoom: bool,
}
impl Default for Zoom {
    fn default() -> Self {
        Self {
            initial_zoom: 4,
            zoom_in_time: 1.0,
            zoom_out_time: 0.5,
            zoom_in_transition: "EASE_OUT_EXP".into(),
            zoom_out_transition: "EASE_OUT_EXP".into(),
            affect_hand_fov: true,
            retain_zoom_steps: false,
            scroll_zoom: true,
            scroll_step_count: 10,
            zoom_per_step: 150,
            scroll_zoom_smoothness: 70,
            zoom_key_behaviour: "HOLD".into(),
            keybind_scrolling: false,
            relative_sensitivity: 100,
            relative_view_bobbing: true,
            cinematic_camera: 0,
            spyglass_behaviour: "COMBINE".into(),
            spyglass_overlay_visibility: "HOLDING".into(),
            spyglass_sound_behaviour: "WITH_OVERLAY".into(),
            secondary_zoom_amount: 4,
            secondary_zoom_in_time: 10.0,
            secondary_zoom_out_time: 1.0,
            secondary_hide_hud_on_zoom: true,
        }
    }
}
pub(in crate::game::atlas_ports) fn load<T: Default + for<'a> Deserialize<'a>>(path: &Path) -> T {
    fs::metadata(path)
        .ok()
        .filter(|m| m.len() <= 256 * 1024)
        .and_then(|_| fs::read(path).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub(in crate::game::atlas_ports) fn save<T: Serialize>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(value) {
        let _ = fs::write(path, bytes);
    }
}
