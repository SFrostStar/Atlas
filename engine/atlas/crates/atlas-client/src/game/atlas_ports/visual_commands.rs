//! Original client commands, configuration and ReallyWorld menu input.
use super::{Game, IDS, argb, config, logic, now_ms};
use atlas_client::{hud::HudBatch, menu::Input, text::Text};
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
            "totempops" if self.atlas_port_on(IDS[8])=>match sub {
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
            "rwjoin"|"rw" if self.atlas_port_on(IDS[7])=>{
                match sub {
                    "on"=>self.atlas_visual_rw_enabled(true),"off"=>self.atlas_visual_rw_enabled(false),""|"toggle"=>self.atlas_visual_rw_enabled(!self.atlas_ports.visual.rw.enabled),
                    "target"=>{if let Ok(n)=value.parse::<i32>(){if (1..=100).contains(&n){self.atlas_ports.visual.rw.target_grief=n;self.native_chat(&format!("§6[RwJoiner] §fЦелевой гриф изменён на: §bГриф {n}"))}}},
                    "delay"=>{if let Ok(n)=value.parse::<u64>(){if (200..=10000).contains(&n){self.atlas_ports.visual.rw.click_delay_ms=n;self.native_chat(&format!("§6[RwJoiner] §fЗадержка кликов установлена на: §a{n} мс"))}}},
                    "autorestart"|"hud"=>{if let Ok(b)=value.parse::<bool>(){if sub=="hud"{self.atlas_ports.visual.rw.show_hud=b;self.native_chat(&format!("§6[RwJoiner] §fОтображение HUD: {}",if b{"§aВКЛЮЧЕНО"}else{"§cВЫКЛЮЧЕНО"}))}else{self.atlas_ports.visual.rw.auto_restart_rejoin=b;self.native_chat(&format!("§6[RwJoiner] §fАвто-перезаход после рестарта: {}",if b{"§aВКЛЮЧЕН"}else{"§cВЫКЛЮЧЕН"}))}}},
                    "dump"=>self.atlas_visual_rw_dump(),
                    "status"=>{let v=&self.atlas_ports.visual;let lines=["§6§m--------------------------------".into(),"§6✦ §e§lRwJoiner v1.1.0 §7(1.21.4 Fabric)".into(),format!("§fСтатус: {}",if v.rw.enabled{"§aВКЛЮЧЕН"}else{"§cВЫКЛЮЧЕН"}),format!("§fТекущее действие: §e{}",v.joiner.phase.description()),format!("§fЦелевой гриф: §bГриф {} §8(верстак 1.21, слот 22)",v.rw.target_grief),format!("§fЗадержка попыток: §a{} мс",v.rw.click_delay_ms),format!("§fАвто-рестарт заход: {}",if v.rw.auto_restart_rejoin{"§aДа"}else{"§cНет"}),format!("§fПопыток кликов: §e{}",v.joiner.attempts),"§8Клавиша быстрого переключения: §6[J]".into(),"§6§m--------------------------------".into()];for s in lines{self.native_chat(&s)}},
                    "help"=>for s in ["§6§m--------------------------------","§6✦ §eКоманды §lRwJoiner§r:","§e/rwjoin §7или §e[J] §8- §fвключить/выключить","§e/rwjoin <номер> §8- §fзадать гриф и сразу начать заход (напр. §b/rwjoin 11§f)","§e/rwjoin target <номер> §8- §fустановить целевой гриф","§e/rwjoin delay <мс> §8- §fзадержка кликов (напр. 850)","§e/rwjoin autorestart <true|false> §8- §fперезаход при рестарте","§e/rwjoin hud <true|false> §8- §fвкл/выкл оверлей","§e/rwjoin dump §8- §fраспечатать все слоты открытого меню","§e/rwjoin status §8- §fтекущий статус","§6§m--------------------------------"]{self.native_chat(s)},
                    _=>if let Ok(n)=sub.parse::<i32>(){if (1..=100).contains(&n){self.atlas_ports.visual.rw.target_grief=n;self.native_chat(&format!("§6[RwJoiner] §fЦелевой гриф изменён на: §bГриф {n}"));self.atlas_visual_rw_enabled(true)}},
                }let v=&self.atlas_ports.visual;config::save(&v.dir,"rwjoiner.json",&v.rw);
            },
            _=>return false,
        }
        true
    }
    pub(in crate::game) fn atlas_visual_rw_enabled(&mut self, enabled: bool) {
        let v = &mut self.atlas_ports.visual;
        v.rw.enabled = enabled;
        v.joiner.toggle(enabled, v.rw.target_grief);
        config::save(&v.dir, "rwjoiner.json", &v.rw);
        let target = v.rw.target_grief;
        let sound = v.rw.sound_alert;
        if self.player.is_some() {
            self.native_chat(&if enabled {
                format!("§6[RwJoiner] §aАвтозаход ВКЛЮЧЕН §7-> §bГриф {target} §8(верстак 1.21)")
            } else {
                "§6[RwJoiner] §cАвтозаход ВЫКЛЮЧЕН".into()
            });
            if sound {
                self.audio.play(
                    "minecraft:ui.button.click",
                    None,
                    1.0,
                    if enabled { 1.2 } else { 0.8 },
                )
            }
        }
    }
    fn atlas_visual_rw_menu(&mut self) -> Option<Vec<logic::MenuItem>> {
        let (_, name) = self.remote_menu();
        let rows = name
            .strip_prefix("minecraft:generic_9x")?
            .parse::<usize>()
            .ok()?;
        if !(1..=6).contains(&rows) {
            return None;
        }
        Some(
            (0..rows * 9)
                .map(|index| {
                    self.slot_view(300 + index)
                        .map(|s| logic::MenuItem {
                            id: s.item.to_string(),
                            name: self
                                .stack_tooltip(&s)
                                .first()
                                .map(|(t, _)| t.plain())
                                .unwrap_or_default(),
                            lore: atlas_client::item_tooltip::lore_texts(&s)
                                .into_iter()
                                .map(|t| t.plain())
                                .collect(),
                            count: s.count as i32,
                        })
                        .unwrap_or(logic::MenuItem {
                            id: "minecraft:air".into(),
                            name: String::new(),
                            lore: vec![],
                            count: 0,
                        })
                })
                .collect(),
        )
    }
    fn atlas_visual_rw_board(&self) -> String {
        let board = &self.score.board;
        let Some(o) = board.display_objective("sidebar") else {
            return String::new();
        };
        let mut text = o.display_text().plain();
        for entry in board.list_scores(&o.name) {
            text.push(' ');
            text.push_str(&entry.owner_name().plain());
            if let Some(t) = board.players_team(&entry.owner) {
                text.push(' ');
                text.push_str(&atlas_client::scoreboard::comp_text(&t.prefix).plain());
                text.push(' ');
                text.push_str(&atlas_client::scoreboard::comp_text(&t.suffix).plain());
            }
        }
        text
    }
    pub(super) fn atlas_visual_rw_tick(&mut self, now: u64) {
        if self.remote.is_none() {
            return;
        }
        let menu = self.atlas_visual_rw_menu();
        let board = self.atlas_visual_rw_board();
        let c = self.atlas_ports.visual.rw.clone();
        let action = self.atlas_ports.visual.joiner.tick(
            now,
            c.enabled,
            c.target_grief,
            c.auto_restart_rejoin,
            c.click_delay_ms,
            &board,
            menu.as_deref(),
        );
        match action {
            Some(logic::JoinAction::Rejoin(target)) => {
                self.native_chat(&format!("§6[RwJoiner] §eОбнаружен рестарт / переход в Хаб! Начинаем автозаход на Гриф {target}..."));
                self.atlas_ports.visual.rw.target_grief = target;
                self.atlas_visual_rw_enabled(true)
            }
            Some(logic::JoinAction::Connected) => {
                self.native_chat(&format!(
                    "§6[RwJoiner] §a✔ Успешно зашли на Гриф {}! (Попыток: {})",
                    c.target_grief, self.atlas_ports.visual.joiner.attempts
                ));
                if c.sound_alert {
                    self.audio
                        .play("minecraft:entity.player.levelup", None, 1.0, 1.0)
                }
                self.atlas_ports.visual.rw.enabled = false;
                config::save(
                    &self.atlas_ports.visual.dir,
                    "rwjoiner.json",
                    &self.atlas_ports.visual.rw,
                )
            }
            Some(logic::JoinAction::Click(index)) => {
                self.remote_menu_ops(&[(Some(300 + index), 0, Input::Pickup)]);
            }
            Some(logic::JoinAction::Close) => self.remote_close_container(),
            Some(logic::JoinAction::Open) => {
                let compass = self.inventory.iter().take(9).position(|s| {
                    s.as_ref().is_some_and(|s| {
                        s.item.as_ref() == "minecraft:compass" || {
                            let name = self
                                .stack_tooltip(s)
                                .first()
                                .map(|(t, _)| t.plain().to_lowercase())
                                .unwrap_or_default();
                            ["выбор", "сервер", "режим", "компас", "меню"]
                                .iter()
                                .any(|s| name.contains(s))
                        }
                    })
                });
                if let Some(slot) = compass {
                    self.selected = slot;
                    let _ = self.use_item();
                } else {
                    self.send_command("menu")
                }
            }
            None => {}
        }
    }
    fn atlas_visual_rw_dump(&mut self) {
        if let Some(menu) = self.atlas_visual_rw_menu() {
            let title = self
                .remote
                .as_ref()
                .and_then(|r| r.menu_title.as_ref())
                .map(|t| t.plain())
                .unwrap_or_default();
            self.native_chat(&format!(
                "§6§m---§r §eМеню: §f{title} §7(Слотов: {}) §6§m---",
                menu.len()
            ));
            let mut n = 0;
            for (i, item) in menu.iter().enumerate() {
                if !item.empty() {
                    n += 1;
                    self.native_chat(&format!(
                        "§7[Слот §e{i}§7] §f{} §8(Item: {}, count: {})",
                        item.name, item.id, item.count
                    ))
                }
            }
            self.native_chat(&format!("§6[RwJoiner] Всего предметов найдено: §e{n}"))
        } else {
            self.native_chat("§c[RwJoiner] Сейчас не открыто ни одно меню сундука!")
        }
    }
    pub(super) fn atlas_visual_rw_draw(&self, batch: &mut HudBatch) {
        let v = &self.atlas_ports.visual;
        let c = &v.rw;
        let x = c.hud_x as f32;
        let y = c.hud_y as f32;
        let (w, h, bg, border) = if c.enabled {
            (180., 58., 0xb00f172a, 0x80d97706)
        } else {
            (120., 18., 0x80101726, 0x40334155)
        };
        batch.fill(&self.gui.atlas, x, y, w, h, argb(bg));
        for (dx, dy, dw, dh) in [
            (0., 0., w, 1.),
            (0., h - 1., w, 1.),
            (0., 0., 1., h),
            (w - 1., 0., 1., h),
        ] {
            batch.fill(&self.gui.atlas, x + dx, y + dy, dw, dh, argb(border))
        }
        let lines = if c.enabled {
            vec![
                ("§6✦ §e§lRwJoiner §7[1.21.4] §a• ВКЛ".into(), 6.),
                (
                    format!(
                        "§fЦель: §bГриф {} §8(верстак) §7| Попыток: §e{}",
                        c.target_grief, v.joiner.attempts
                    ),
                    19.,
                ),
                (
                    format!("§fДействие: §e{}", v.joiner.phase.description()),
                    32.,
                ),
                ("§8Нажмите [J] для выключения".into(), 45.),
            ]
        } else {
            vec![("§6✦ §fRwJoiner §7[§cВЫКЛ§7] §8[J]".into(), 5.)]
        };
        for (text, dy) in lines {
            batch.text_styled(
                &self.gui.atlas,
                &Text::legacy(&text),
                x + 6.,
                y + dy,
                u32::MAX,
                true,
            );
        }
    }

    /// Complete original JSON shape, for a text/list/map-capable settings editor.
    pub(in crate::game) fn atlas_visual_config(&self, id: &str) -> Option<Value> {
        let v = &self.atlas_ports.visual;
        Some(match id {
            "atlas-radarmod" => serde_json::to_value(&v.radar).ok()?,
            "atlas-nofluid" => serde_json::to_value(&v.nofluid).ok()?,
            "atlas-pvpsurvived" => serde_json::to_value(&v.pvp).ok()?,
            "atlas-rwjoiner" => serde_json::to_value(&v.rw).ok()?,
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
            "atlas-rwjoiner" => {
                let before = v.rw.enabled;
                apply!(rw, "rwjoiner.json");
                if v.rw.enabled != before {
                    v.joiner.toggle(v.rw.enabled, v.rw.target_grief)
                }
            }
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
            if id == IDS[8] { 0.2 } else { 0.5 },
            if id == IDS[8] { 5.0 } else { 2.0 },
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
