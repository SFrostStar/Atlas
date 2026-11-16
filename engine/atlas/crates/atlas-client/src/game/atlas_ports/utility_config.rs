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
pub(in crate::game::atlas_ports) struct Zoom {
    pub initial_zoom: i32,
    pub zoom_in_time: f64,
    pub zoom_out_time: f64,
    #[serde(deserialize_with = "upper", serialize_with = "lower")]
    pub zoom_in_transition: String,
    #[serde(deserialize_with = "upper", serialize_with = "lower")]
    pub zoom_out_transition: String,
    pub affect_hand_fov: bool,
    pub retain_zoom_steps: bool,
    pub scroll_zoom: bool,
    pub scroll_step_count: i32,
    pub zoom_per_step: i32,
    pub scroll_zoom_smoothness: i32,
    #[serde(deserialize_with = "upper", serialize_with = "lower")]
    pub zoom_key_behaviour: String,
    #[serde(rename = "_keybindScrolling", alias = "keybindScrolling")]
    pub keybind_scrolling: bool,
    pub relative_sensitivity: i32,
    pub relative_view_bobbing: bool,
    pub cinematic_camera: i32,
    #[serde(deserialize_with = "upper", serialize_with = "lower")]
    pub spyglass_behaviour: String,
    #[serde(deserialize_with = "upper", serialize_with = "lower")]
    pub spyglass_overlay_visibility: String,
    #[serde(deserialize_with = "upper", serialize_with = "lower")]
    pub spyglass_sound_behaviour: String,
    pub secondary_zoom_amount: i32,
    pub secondary_zoom_in_time: f64,
    pub secondary_zoom_out_time: f64,
    #[serde(rename = "secondaryHideHUDOnZoom", alias = "secondaryHideHudOnZoom")]
    pub secondary_hide_hud_on_zoom: bool,
    #[serde(rename = "_firstLaunch")]
    pub first_launch: bool,
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
            first_launch: false,
        }
    }
}

fn upper<'de, D: serde::Deserializer<'de>>(input: D) -> Result<String, D::Error> {
    String::deserialize(input).map(|s| s.to_ascii_uppercase())
}
fn lower<S: serde::Serializer>(value: &str, output: S) -> Result<S::Ok, S::Error> {
    output.serialize_str(&value.to_ascii_lowercase())
}

pub(in crate::game::atlas_ports) fn zoom_preset(name: &str) -> Option<Zoom> {
    let mut config = Zoom {
        first_launch: true,
        ..Zoom::default()
    };
    match name {
        "default" => {}
        "optifine" => {
            config.zoom_in_transition = "INSTANT".into();
            config.zoom_out_transition = "INSTANT".into();
            config.scroll_zoom = false;
            config.relative_sensitivity = 0;
            config.relative_view_bobbing = false;
            config.cinematic_camera = 100;
        }
        "okzoomer" => {
            config.zoom_in_time = 0.25;
            config.zoom_out_time = 0.25;
            config.scroll_zoom_smoothness = 25;
            config.relative_sensitivity = 50;
            config.relative_view_bobbing = false;
        }
        _ => return None,
    }
    Some(config)
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

#[cfg(test)]
mod tests {
    use super::Zoom;

    #[test]
    fn zoom_scrolling_reads_original_and_legacy_files_and_writes_original_spelling() {
        for key in ["_keybindScrolling", "keybindScrolling"] {
            let config: Zoom = serde_json::from_value(serde_json::json!({(key): true})).unwrap();
            assert!(config.keybind_scrolling);
            let saved = serde_json::to_value(config).unwrap();
            assert_eq!(saved["_keybindScrolling"], true);
            assert!(saved.get("keybindScrolling").is_none());
        }
    }

    #[test]
    fn original_zoom_config_preserves_internal_mechanics_and_native_legacy_files() {
        let config: Zoom = serde_json::from_value(serde_json::json!({
            "zoomInTransition":"instant", "zoomKeyBehaviour":"toggle",
            "spyglassBehaviour":"combine", "secondaryHideHUDOnZoom":false
        }))
        .unwrap();
        assert_eq!(config.zoom_in_transition, "INSTANT");
        assert_eq!(config.zoom_key_behaviour, "TOGGLE");
        assert!(!config.secondary_hide_hud_on_zoom);
        let saved = serde_json::to_value(&config).unwrap();
        assert_eq!(saved["zoomInTransition"], "instant");
        assert_eq!(saved["zoomKeyBehaviour"], "toggle");
        assert_eq!(saved["secondaryHideHUDOnZoom"], false);
        let legacy: Zoom = serde_json::from_value(
            serde_json::json!({"zoomInTransition":"EASE_OUT_EXP","secondaryHideHudOnZoom":false}),
        )
        .unwrap();
        assert_eq!(legacy.zoom_in_transition, "EASE_OUT_EXP");
        assert!(!legacy.secondary_hide_hud_on_zoom);
    }

    #[test]
    fn presets_match_the_observed_original_options() {
        let optifine = super::zoom_preset("optifine").unwrap();
        assert_eq!(optifine.zoom_in_transition, "INSTANT");
        assert_eq!(optifine.relative_sensitivity, 0);
        assert!(!optifine.scroll_zoom);
        assert_eq!(optifine.cinematic_camera, 100);
        let ok = super::zoom_preset("okzoomer").unwrap();
        assert_eq!((ok.zoom_in_time, ok.zoom_out_time), (0.25, 0.25));
        assert_eq!(
            (ok.scroll_zoom_smoothness, ok.relative_sensitivity),
            (25, 50)
        );
        assert!(!ok.relative_view_bobbing);
        assert!(super::zoom_preset("default").unwrap().first_launch);
        assert!(super::zoom_preset("invalid").is_none());
    }
}
