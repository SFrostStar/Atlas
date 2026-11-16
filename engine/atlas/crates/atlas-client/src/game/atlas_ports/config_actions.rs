//! Zoomify's original menu actions, backed by the native settings already in use.
use super::{Game, utility::config};
use atlas_client::{keybinds, lang};
use serde_json::Value;

impl Game<'_> {
    pub(in crate::game) fn atlas_config_action(
        &mut self,
        action: &str,
        draft: &mut Value,
    ) -> Result<(), String> {
        if let Some(name) = action.strip_prefix("zoomify.preset.") {
            let preset =
                config::zoom_preset(name).ok_or_else(|| lang::get("atlas.ports.invalid_config"))?;
            let value = serde_json::to_value(preset).map_err(|e| e.to_string())?;
            self.atlas_utility_set_config("atlas-zoomify", value.clone())?;
            *draft = value;
            return Ok(());
        }
        if action == "zoomify.check_migrations" {
            return Err(lang::get("atlas.config.zoomify.migration_unavailable"));
        }
        if action != "zoomify.unbind_conflicting" {
            return Err(lang::get("atlas.ports.invalid_config"));
        }
        let zoom_key = self.native_mod_key("atlas-zoomify", "zoom", "key.keyboard.c");
        if zoom_key == "key.keyboard.unknown" {
            return Ok(());
        }
        let mut changed = false;
        for &(name, _, category) in keybinds::MAPPINGS {
            if category != "debug" && self.settings.keys.get(name) == zoom_key {
                self.settings.keys.set(name, "key.keyboard.unknown");
                changed = true;
            }
        }
        let native_keys: Vec<_> = self
            .settings
            .mods
            .installed
            .iter()
            .filter(|row| row.id != "atlas-zoomify")
            .flat_map(|row| {
                row.native
                    .iter()
                    .flat_map(|page| page.keys.iter())
                    .map(|binding| (row.id.clone(), binding[0].clone(), binding[2].clone()))
            })
            .collect();
        for (id, name, default) in native_keys {
            if self.native_mod_key(&id, &name, &default) == zoom_key {
                self.settings
                    .mods
                    .keys
                    .entry(id)
                    .or_default()
                    .insert(name, "key.keyboard.unknown".into());
                changed = true;
            }
        }
        if changed {
            self.settings.save(&Self::options_path());
            self.settings.mods.save(&Self::game_dir());
        }
        Ok(())
    }
}
