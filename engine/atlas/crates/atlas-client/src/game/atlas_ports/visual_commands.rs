//! Original client commands and configuration.
use super::{Game, IDS, config, now_ms};
use serde_json::{Value, json};
use std::fs;

impl Game<'_> {
    pub(in crate::game) fn atlas_visual_command(&mut self, command: &str) -> bool {
        let mut args = command.trim_start_matches('/').split_whitespace();
        let Some(name) = args.next() else {
            return false;
        };
        let sub = args.next().unwrap_or("");
        let value = args.next().unwrap_or("");
        match name {
            "friend" if self.atlas_port_on(IDS[2])=>{
                let message=match sub {
                    "add" if !value.is_empty()=>{let v=&mut self.atlas_ports.visual;if v.friends.insert(value.to_lowercase()){v.save_friends();format!("§a✔ {value} добавлен в друзья!")}else{format!("§e{value} уже в списке друзей.")}},
                    "remove" if !value.is_empty()=>{let v=&mut self.atlas_ports.visual;if v.friends.remove(&value.to_lowercase()){v.save_friends();format!("§c✖ {value} удалён из друзей.")}else{format!("§e{value} не найден в списке друзей.")}},
                    "list"=>{let f=&self.atlas_ports.visual.friends;if f.is_empty(){"§eСписок друзей пуст.".into()}else{format!("§aДрузья ({}): §f{}",f.len(),f.iter().cloned().collect::<Vec<_>>().join(", "))}},
                    "clear"=>{let v=&mut self.atlas_ports.visual;let count=v.friends.len();v.friends.clear();v.save_friends();format!("§eСписок друзей очищен (удалено {count}).")},
                    _=>"§6=== [ FriendList ] ===\n§a/friend add <ник>§7 - добавить игрока в друзья\n§c/friend remove <ник>§7 - удалить игрока из друзей\n§e/friend list§7 - список друзей\n§4/friend clear§7 - очистить список друзей".into(),
                };self.native_chat(&message);
            },
            "radar" if self.atlas_port_on(IDS[0])=>{
                if sub=="toggle"{let v=&mut self.atlas_ports.visual;v.radar.enabled=!v.radar.enabled;config::save(&v.dir,"radarmod.json",&v.radar);self.native_chat(&format!("Радар {}",if self.atlas_ports.visual.radar.enabled{"§aВКЛЮЧЕН"}else{"§cВЫКЛЮЧЕН"}));}
                else{self.atlas_ports.visual.open_settings=Some(IDS[0].into())}
            },
            "nofluid" if self.atlas_port_on(IDS[5])=>{
                let c=&mut self.atlas_ports.visual.nofluid;let message=match sub {
                    "toggle"=>{c.enabled=!c.enabled;format!("§b[NoFluid] §fМод {}",if c.enabled{"§aВКЛЮЧЕН"}else{"§cВЫКЛЮЧЕН"})},
                    "water"=>{c.no_water_fog=!c.no_water_fog;format!("§b[NoFluid] §fВодный туман: {}",if c.no_water_fog{"§aУДАЛЕН"}else{"§cСТАНДАРТ"})},
                    "lava"=>{c.no_lava_fog=!c.no_lava_fog;format!("§b[NoFluid] §fТуман лавы: {}",if c.no_lava_fog{"§aУДАЛЕН"}else{"§cСТАНДАРТ"})},
                    "fire"=>{c.no_fire_overlay=!c.no_fire_overlay;format!("§b[NoFluid] §fОгонь на экране: {}",if c.no_fire_overlay{"§aСКРЫТ"}else{"§cВИДЕН"})},
                    "overlay"=>{c.no_water_overlay=!c.no_water_overlay;format!("§b[NoFluid] §fОверлей воды: {}",if c.no_water_overlay{"§aСКРЫТ"}else{"§cВИДЕН"})},
                    "nightvision"=>{c.fluid_night_vision=!c.fluid_night_vision;format!("§b[NoFluid] §fНочное зрение в жидкостях: {}",if c.fluid_night_vision{"§aВКЛ"}else{"§cВЫКЛ"})},
                    _=>"§b=== [NoFluid CPvP 1.21.4] ===\n§f/nofluid toggle\n/nofluid water\n/nofluid lava\n/nofluid fire\n/nofluid overlay\n/nofluid nightvision".into(),
                };config::save(&self.atlas_ports.visual.dir,"nofluid.json",&self.atlas_ports.visual.nofluid);self.native_chat(&message);
            },
            "totempops" if self.atlas_port_on(IDS[7])=>match sub {
                "reload"=>{let v=&mut self.atlas_ports.visual;v.totem=fs::read_to_string(v.dir.join("totempops/config.yml")).map(|s|config::Totem::parse(&s)).unwrap_or_default();self.native_chat("[TotemPops] Конфиг перезагружен")},
                "reset"=>{self.atlas_ports.visual.pops.entries.clear();self.native_chat("[TotemPops] Счётчики сброшены")},
                _=>self.native_chat("/totempops reload | /totempops reset"),
            },
            "killnotification" if self.atlas_port_on(IDS[3])=>{
                let c=&mut self.atlas_ports.visual.kill;let message=match sub {
                    "on"=>{c.enabled=true;"§a[KillNotification] Мод успешно включен!".into()},
                    "off"=>{c.enabled=false;"§c[KillNotification] Мод успешно выключен!".into()},
                    _=>format!("§6[KillNotification] Статус: {}§7. Команды: §e/killnotification on §7| §e/killnotification off",if c.enabled{"§aВКЛЮЧЕН"}else{"§cВЫКЛЮЧЕН"}),
                };config::save(&self.atlas_ports.visual.dir,"killnotification.json",&self.atlas_ports.visual.kill);self.native_chat(&message)
            },
            "pvpsurvived" if self.atlas_port_on(IDS[6])=>{
                match sub {
                    "test"=>{if self.atlas_ports.visual.pvp_state.trigger(now_ms()){self.atlas_visual_pvp_sound()}self.native_chat("§a[PvPSurvived] Тестовое появление надписи под прицелом активировано!")},
                    "reload"=>{let v=&mut self.atlas_ports.visual;v.pvp=config::load(&v.dir,"pvpsurvived.json");self.native_chat("§a[PvPSurvived] Конфигурация успешно перезагружена!")},
                    "toggle"=>{let v=&mut self.atlas_ports.visual;v.pvp.enabled=!v.pvp.enabled;config::save(&v.dir,"pvpsurvived.json",&v.pvp);self.native_chat(&format!("§a[PvPSurvived] Мод {}",if self.atlas_ports.visual.pvp.enabled{"§2включен"}else{"§cвыключен"}))},
                    _=>self.native_chat("§e[PvPSurvived] Команды:\n§7/pvpsurvived test §f- проверить появление надписи под прицелом\n§7/pvpsurvived reload §f- перезагрузить конфиг\n§7/pvpsurvived toggle §f- включить/выключить мод"),
                }
            },
            _=>return false,
        }
        true
    }
    /// Complete original JSON shape, for a text/list/map-capable settings editor.
    pub(in crate::game) fn atlas_visual_config(&self, id: &str) -> Option<Value> {
        let v = &self.atlas_ports.visual;
        Some(match id {
            "atlas-radarmod" => serde_json::to_value(&v.radar).ok()?,
            "atlas-nofluid" => serde_json::to_value(&v.nofluid).ok()?,
            "atlas-pvpsurvived" => serde_json::to_value(&v.pvp).ok()?,
            "atlas-lootbeams" => serde_json::to_value(&v.loot).ok()?,
            "atlas-killnotification" => serde_json::to_value(&v.kill).ok()?,
            "atlas-totempops" => totem_json(&v.totem),
            _ => return None,
        })
    }
    pub(in crate::game) fn atlas_visual_set_config(
        &mut self,
        id: &str,
        value: Value,
    ) -> Result<(), String> {
        let v = &mut self.atlas_ports.visual;
        macro_rules! apply {
            ($field:ident,$file:literal) => {{
                v.$field = serde_json::from_value(value).map_err(|e| e.to_string())?;
                config::save(&v.dir, $file, &v.$field);
            }};
        }
        match id {
            "atlas-radarmod" => {
                apply!(radar, "radarmod.json");
                v.radar.validate();
                config::save(&v.dir, "radarmod.json", &v.radar)
            }
            "atlas-nofluid" => apply!(nofluid, "nofluid.json"),
            "atlas-pvpsurvived" => apply!(pvp, "pvpsurvived.json"),
            "atlas-lootbeams" => apply!(loot, "lootbeams.json"),
            "atlas-killnotification" => apply!(kill, "killnotification.json"),
            "atlas-totempops" => {
                let obj = value.as_object().ok_or("Expected a configuration object")?;
                let yaml = obj
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{k}: {}",
                            match v {
                                Value::String(s) => serde_json::to_string(s).unwrap_or_default(),
                                _ => v.to_string(),
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                v.totem = config::Totem::parse(&yaml);
                v.save_totem()
            }
            _ => return Err("This mod has no configuration file".into()),
        }
        Ok(())
    }
    pub(in crate::game) fn atlas_visual_settings(&self, id: &str, buttons: &mut Vec<String>) {
        if let Some(value) = self.atlas_visual_config(id) {
            buttons.push(atlas_client::lang::get("atlas.ports.edit_config"));
            for (key, value) in flattened(&value) {
                buttons.push(format!(
                    "{key}: {}",
                    if let Some(b) = value.as_bool() {
                        if b { "ON".into() } else { "OFF".into() }
                    } else {
                        value.to_string()
                    }
                ));
            }
        }
        if id == IDS[2] {
            buttons.push(format!(
                "Friends: {} — /friend add <name>",
                self.atlas_ports.visual.friends.len()
            ))
        }
    }
    pub(in crate::game) fn atlas_visual_settings_click(&mut self, id: &str, index: usize) {
        let Some(mut value) = self.atlas_visual_config(id) else {
            return;
        };
        if index == 0 {
            self.atlas_visual_open_editor(id);
            return;
        }
        let Some((key, old)) = flattened(&value).get(index - 1).cloned() else {
            return;
        };
        let next = if let Some(b) = old.as_bool() {
            Value::Bool(!b)
        } else if let Some(n) = old.as_f64() {
            let (step, min, max) = range(id, &key);
            let n = if n + step > max { min } else { n + step };
            if old.is_i64() || old.is_u64() {
                json!(n.round() as i64)
            } else {
                json!((n * 1000.0).round() / 1000.0)
            }
        } else {
            return;
        };
        let ptr = format!("/{}", key.replace('.', "/"));
        if let Some(slot) = value.pointer_mut(&ptr) {
            *slot = next;
        }
        let _ = self.atlas_visual_set_config(id, value);
    }
}
fn totem_json(c: &config::Totem) -> Value {
    json!({"enabled":c.enabled,"label":c.label,"format":c.format,"count_prefix":c.count_prefix,"label_color":format!("#{:06X}",c.label_color),"count_color":format!("#{:06X}",c.count_color),"dynamic_color":c.dynamic_color,"scale":c.scale,"y_offset":c.y_offset,"background":c.background,"show_self":c.show_self,"hide_teammates":c.hide_teammates,"show_zero":c.show_zero,"reset_on_death":c.reset_on_death,"reset_timeout":c.reset_timeout,"max_distance":c.max_distance,"chat_notify":c.chat_notify})
}
fn flattened(value: &Value) -> Vec<(String, Value)> {
    fn walk(out: &mut Vec<(String, Value)>, prefix: &str, value: &Value) {
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                let key = if prefix.is_empty() {
                    k.into()
                } else {
                    format!("{prefix}.{k}")
                };
                if v.is_object() {
                    walk(out, &key, v)
                } else {
                    out.push((key, v.clone()))
                }
            }
        }
    }
    let mut out = vec![];
    walk(&mut out, "", value);
    out
}
fn range(id: &str, key: &str) -> (f64, f64, f64) {
    match key {
        "scale" => (
            0.05,
            if id == IDS[7] { 0.2 } else { 0.5 },
            if id == IDS[7] { 5.0 } else { 2.0 },
        ),
        "maxPlayers" => (1., 1., 20.),
        "maxDistance" | "max_distance" => (10., 10., 200.),
        "backgroundOpacity" => (0.05, 0.1, 1.),
        "hudWidth" => (5., 120., 300.),
        "x" | "y" | "hudX" | "hudY" | "offsetY" | "y_offset" => (1., 0., 500.),
        "targetGrief" => (1., 1., 100.),
        "clickDelayMs" => (50., 200., 10000.),
        "durationMs" => (100., 500., 10000.),
        "soundPitch" => (0.1, 0.1, 2.),
        "soundVolume" => (0.1, 0., 1.),
        "beam.width" => (0.05, 0.05, 2.),
        "beam.height" => (5., 5., 256.),
        "beam.alpha" | "outline.visibleAlpha" | "outline.xrayAlpha" => (0.05, 0., 1.),
        "beam.spinDegreesPerSecond" => (5., 0., 180.),
        "outline.lineWidth" => (0.5, 0.5, 5.),
        "outline.expand" => (0.01, 0., 0.5),
        "maxRenderDistance" => (8., 8., 256.),
        "reset_timeout" => (5., 0., 300.),
        _ => (1., 0., 100.),
    }
}
