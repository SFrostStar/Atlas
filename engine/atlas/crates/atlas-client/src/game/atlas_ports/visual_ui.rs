//! A complete configuration editor for ports whose Java mod uses JSON/YAML configuration.
//! This Atlas editor intentionally does not claim to reproduce RadarConfigScreen's layout.
use super::Game;
use atlas_client::{
    clipboard::EditShortcut,
    hud::{HudBatch, rgba},
    text::Text,
    ui::{self, EditBox},
};
use serde_json::Value;
use winit::keyboard::KeyCode;

#[derive(Default)]
pub(super) struct State {
    id: String,
    value: Value,
    scroll: usize,
    dialog: Option<Dialog>,
    error: Option<String>,
}
struct Dialog {
    key: String,
    field: EditBox,
    string: bool,
}
fn ru() -> bool {
    atlas_client::lang::code().starts_with("ru")
}
fn local(a: &str, b: &str) -> String {
    if ru() { a } else { b }.into()
}
fn hit(r: [f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}
fn label(key: &str) -> String {
    let (a, b) = match key {
        "enabled" => ("Включено", "Enabled"),
        "scale" => ("Масштаб", "Scale"),
        "x" | "hudX" => ("Позиция X", "Position X"),
        "y" | "hudY" => ("Позиция Y", "Position Y"),
        "maxPlayers" => ("Максимум игроков", "Maximum players"),
        "maxDistance" | "max_distance" => ("Расстояние", "Distance"),
        "backgroundOpacity" => ("Непрозрачность фона", "Background opacity"),
        "showHealth" => ("Показывать здоровье", "Show health"),
        "showDistance" => ("Показывать расстояние", "Show distance"),
        "showArrows" => ("Показывать стрелки", "Show arrows"),
        "hudWidth" => ("Ширина радара", "Radar width"),
        "noWaterFog" => ("Убрать водный туман", "Remove water fog"),
        "noLavaFog" => ("Убрать туман лавы", "Remove lava fog"),
        "noPowderSnowFog" => ("Убрать туман рыхлого снега", "Remove powder-snow fog"),
        "noWaterOverlay" => ("Убрать оверлей воды", "Remove water overlay"),
        "noFireOverlay" => ("Убрать оверлей огня", "Remove fire overlay"),
        "maxWaterVision" => ("Видимость под водой", "Maximum water vision"),
        "fluidNightVision" => ("Ночное зрение в жидкостях", "Night vision in fluids"),
        "text" => ("Текст", "Text"),
        "durationMs" => ("Длительность (мс)", "Duration (ms)"),
        "offsetY" | "y_offset" => ("Смещение по Y", "Y offset"),
        "colorHex" => ("Цвет", "Color"),
        "bold" => ("Полужирный текст", "Bold text"),
        "playSound" => ("Звук", "Play sound"),
        "soundPitch" => ("Высота звука", "Sound pitch"),
        "soundVolume" => ("Громкость", "Volume"),
        "fadeEffect" => ("Плавное исчезновение", "Fade effect"),
        "customKeywords" => ("Ключевые слова", "Custom keywords"),
        "targetGrief" => ("Номер грифа", "Target Grief"),
        "targetMode" => ("Режим", "Target mode"),
        "clickDelayMs" => ("Задержка кликов (мс)", "Click delay (ms)"),
        "autoRestartRejoin" => ("Перезаход после рестарта", "Rejoin after restart"),
        "soundAlert" => ("Звуковые уведомления", "Sound alerts"),
        "showHud" => ("Показывать HUD", "Show HUD"),
        "maxRenderDistance" => ("Дальность отображения", "Render distance"),
        "outline" => ("Обводка", "Outline"),
        "beam" => ("Луч", "Beam"),
        "rarityColors" => ("Цвета редкости", "Rarity colors"),
        "itemColorOverrides" => ("Цвета предметов", "Item color overrides"),
        "whitelistMode" => ("Режим белого списка", "Whitelist mode"),
        "whitelist" => ("Белый список", "Whitelist"),
        "blacklist" => ("Чёрный список", "Blacklist"),
        "label" => ("Название счётчика", "Counter label"),
        "format" => ("Формат счётчика", "Counter format"),
        "count_prefix" => ("Префикс количества", "Count prefix"),
        "label_color" => ("Цвет названия", "Label color"),
        "count_color" => ("Цвет количества", "Count color"),
        "dynamic_color" => ("Цвет по количеству", "Dynamic count color"),
        "background" => ("Подложка", "Background"),
        "show_self" => ("Показывать у себя", "Show self"),
        "hide_teammates" => ("Скрывать сокомандников", "Hide teammates"),
        "show_zero" => ("Показывать нулевой счётчик", "Show zero"),
        "reset_on_death" => ("Сброс при смерти", "Reset on death"),
        "reset_timeout" => ("Время сброса (с)", "Reset timeout (s)"),
        "chat_notify" => ("Уведомления в чате", "Chat notifications"),
        _ => return key.to_owned(),
    };
    local(a, b)
}
impl Game<'_> {
    fn atlas_editor_config(&self, id: &str) -> Option<Value> {
        self.atlas_visual_config(id)
            .or_else(|| self.atlas_utility_config(id))
            .or_else(|| self.atlas_armor_config(id))
            .or_else(|| self.atlas_actions_config(id))
    }
    fn atlas_editor_set(&mut self, id: &str, value: Value) -> Result<(), String> {
        if self.atlas_visual_config(id).is_some() {
            self.atlas_visual_set_config(id, value)
        } else if self.atlas_utility_config(id).is_some() {
            self.atlas_utility_set_config(id, value)
        } else if self.atlas_actions_config(id).is_some() {
            self.atlas_actions_set_config(id, value)
        } else {
            self.atlas_armor_set_config(id, value)
        }
    }
    pub(in crate::game) fn atlas_visual_open_editor(&mut self, id: &str) -> bool {
        let Some(value) = self.atlas_editor_config(id) else {
            return false;
        };
        if !self.atlas_open_port_screen(id) {
            return false;
        }
        self.atlas_ports.visual.ui = State {
            id: id.into(),
            value,
            ..Default::default()
        };
        true
    }
    fn atlas_visual_editor_active(&self) -> bool {
        let ui = &self.atlas_ports.visual.ui;
        !ui.id.is_empty() && self.native_mod_screen() == Some(ui.id.as_str())
    }
    pub(in crate::game) fn atlas_visual_screen_render(&mut self, batch: &mut HudBatch) -> bool {
        if !self.atlas_visual_editor_active() {
            return false;
        }
        let atlas = &self.gui.atlas;
        let w = batch.width;
        let h = batch.height;
        let width = (w - 32.0).min(500.0);
        let x = (w - width) / 2.0;
        batch.fill(atlas, 0.0, 0.0, w, h, rgba(8, 12, 18, 225));
        let s = &self.atlas_ports.visual.ui;
        let title = self
            .settings
            .mods
            .installed
            .iter()
            .find(|m| m.id == s.id)
            .map(|m| m.title.as_str())
            .unwrap_or(s.id.as_str());
        let title = format!("{} · {}", title, local("Настройки", "Settings"));
        batch.text(
            atlas,
            &title,
            (w - HudBatch::text_width(atlas, &title)) / 2.0,
            15.0,
            u32::MAX,
            true,
        );
        if s.dialog.is_none() {
            let max = ((h - 94.0) / 26.0).max(1.0) as usize;
            let Some(obj) = s.value.as_object() else {
                return true;
            };
            for (index, (key, value)) in obj.iter().enumerate().skip(s.scroll).take(max) {
                let y = 42.0 + (index - s.scroll) as f32 * 26.0;
                let name = label(key);
                let value = match value {
                    Value::Bool(b) => local(
                        if *b { "Вкл" } else { "Выкл" },
                        if *b { "On" } else { "Off" },
                    ),
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                batch.text(
                    atlas,
                    &name,
                    x + 4.0,
                    y + 6.0,
                    rgba(220, 225, 233, 255),
                    true,
                );
                let field = [x + width * 0.48, y, width * 0.52, 22.0];
                let text = if HudBatch::text_width(atlas, &value) > field[2] - 12.0 {
                    let n = atlas.font.fit(&value, field[2] - 24.0);
                    format!("{}…", value.chars().take(n).collect::<String>())
                } else {
                    value
                };
                ui::button(
                    batch,
                    atlas,
                    field[0],
                    field[1],
                    field[2],
                    field[3],
                    &Text::literal(text),
                    true,
                    hit(field, self.cursor.0, self.cursor.1),
                );
            }
            if obj.len() > max {
                let hint = local("Прокрутка колёсиком мыши", "Scroll with the mouse wheel");
                batch.text(atlas, &hint, x, h - 45.0, rgba(150, 160, 175, 255), false);
            }
            let done = local("Готово", "Done");
            ui::button(
                batch,
                atlas,
                w / 2.0 - 70.0,
                h - 30.0,
                140.0,
                20.0,
                &Text::literal(done),
                true,
                hit(
                    [w / 2.0 - 70.0, h - 30.0, 140., 20.],
                    self.cursor.0,
                    self.cursor.1,
                ),
            );
        } else {
            let s = &mut self.atlas_ports.visual.ui;
            let dialog = s.dialog.as_mut().unwrap();
            let title = label(&dialog.key);
            batch.text(atlas, &title, x + 4.0, 58.0, u32::MAX, false);
            dialog.field.x = x + 4.0;
            dialog.field.y = 78.0;
            dialog.field.width = width - 8.0;
            dialog.field.draw(batch, atlas, None);
            let hint = if dialog.string {
                local("Введите текст", "Enter text")
            } else {
                local(
                    "Числа, списки и объекты: формат JSON",
                    "Numbers, lists and objects use JSON",
                )
            };
            batch.text(
                atlas,
                &hint,
                x + 4.0,
                105.0,
                rgba(160, 170, 185, 255),
                false,
            );
            if let Some(error) = &s.error {
                let n = atlas.font.fit(error, width - 8.0);
                batch.text(
                    atlas,
                    &error.chars().take(n).collect::<String>(),
                    x + 4.0,
                    122.0,
                    rgba(255, 105, 105, 255),
                    false,
                );
            }
            for (dx, label) in [
                (-144., local("Сохранить", "Save")),
                (4., local("Отмена", "Cancel")),
            ] {
                let rect = [w / 2.0 + dx, 148., 140., 20.];
                ui::button(
                    batch,
                    atlas,
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    &Text::literal(label),
                    true,
                    hit(rect, self.cursor.0, self.cursor.1),
                );
            }
        }
        true
    }
    fn atlas_visual_dialog_save(&mut self) {
        let s = &mut self.atlas_ports.visual.ui;
        let Some(dialog) = s.dialog.as_ref() else {
            return;
        };
        let new = if dialog.string {
            Value::String(dialog.field.value.clone())
        } else {
            match serde_json::from_str(&dialog.field.value) {
                Ok(v) => v,
                Err(e) => {
                    s.error = Some(e.to_string());
                    return;
                }
            }
        };
        let id = s.id.clone();
        let mut value = s.value.clone();
        if let Some(obj) = value.as_object_mut() {
            obj.insert(dialog.key.clone(), new);
        }
        match self.atlas_editor_set(&id, value.clone()) {
            Ok(()) => {
                let s = &mut self.atlas_ports.visual.ui;
                s.value = value;
                s.dialog = None;
                s.error = None
            }
            Err(e) => self.atlas_ports.visual.ui.error = Some(e),
        }
    }
    pub(in crate::game) fn atlas_visual_screen_mouse(
        &mut self,
        button: i32,
        pressed: bool,
    ) -> bool {
        if !self.atlas_visual_editor_active() {
            return false;
        }
        if button != 0 || !pressed {
            return true;
        }
        let Some(r) = self.renderer.as_ref() else {
            return true;
        };
        let batch = HudBatch::new([r.config.width, r.config.height]);
        let w = batch.width;
        let h = batch.height;
        let width = (w - 32.).min(500.);
        let x = (w - width) / 2.;
        let (cx, cy) = self.cursor;
        if self.atlas_ports.visual.ui.dialog.is_some() {
            if hit([w / 2. - 144., 148., 140., 20.], cx, cy) {
                self.atlas_visual_dialog_save()
            } else if hit([w / 2. + 4., 148., 140., 20.], cx, cy) {
                self.atlas_ports.visual.ui.dialog = None;
                self.atlas_ports.visual.ui.error = None
            } else if let Some(d) = self.atlas_ports.visual.ui.dialog.as_mut() {
                let rect = [d.field.x, d.field.y, d.field.width, d.field.height];
                d.field.focused = hit(rect, cx, cy);
                if d.field.focused {
                    d.field.click(&self.gui.atlas, cx, false, false)
                }
            }
            return true;
        }
        if hit([w / 2. - 70., h - 30., 140., 20.], cx, cy) {
            let id = self.atlas_ports.visual.ui.id.clone();
            self.native_mod_screen_close(&id);
            return true;
        }
        let s = &mut self.atlas_ports.visual.ui;
        let max = ((h - 94.) / 26.).max(1.) as usize;
        let Some(obj) = s.value.as_object() else {
            return true;
        };
        let chosen =
            obj.iter()
                .enumerate()
                .skip(s.scroll)
                .take(max)
                .find_map(|(index, (key, value))| {
                    let y = 42. + (index - s.scroll) as f32 * 26.;
                    hit([x + width * 0.48, y, width * 0.52, 22.], cx, cy)
                        .then(|| (key.clone(), value.clone()))
                });
        if let Some((key, value)) = chosen {
            if let Some(b) = value.as_bool() {
                let id = s.id.clone();
                let mut changed = s.value.clone();
                changed
                    .as_object_mut()
                    .unwrap()
                    .insert(key, Value::Bool(!b));
                match self.atlas_editor_set(&id, changed.clone()) {
                    Ok(()) => self.atlas_ports.visual.ui.value = changed,
                    Err(e) => self.atlas_ports.visual.ui.error = Some(e),
                }
            } else {
                let string = value.is_string();
                let text = if string {
                    value.as_str().unwrap().into()
                } else {
                    value.to_string()
                };
                let mut field = EditBox::new(x + 4., 78., width - 8., 20.);
                field.max_length = 65536;
                field.set_value(&text);
                field.focused = true;
                s.dialog = Some(Dialog { key, field, string });
                s.error = None
            }
        }
        true
    }
    pub(in crate::game) fn atlas_visual_screen_key(
        &mut self,
        key: KeyCode,
        pressed: bool,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        if !self.atlas_visual_editor_active() {
            return false;
        }
        if !pressed {
            return true;
        }
        if key == KeyCode::Escape {
            if self.atlas_ports.visual.ui.dialog.take().is_none() {
                let id = self.atlas_ports.visual.ui.id.clone();
                self.native_mod_screen_close(&id)
            }
            return true;
        }
        if self.atlas_ports.visual.ui.dialog.is_some() {
            if key == KeyCode::Enter || key == KeyCode::NumpadEnter {
                self.atlas_visual_dialog_save()
            } else if let Some(d) = self.atlas_ports.visual.ui.dialog.as_mut() {
                d.field.key(key, ctrl, shift);
            }
            return true;
        }
        if key == KeyCode::ArrowDown {
            self.atlas_visual_screen_scroll(-1.);
        } else if key == KeyCode::ArrowUp {
            self.atlas_visual_screen_scroll(1.);
        }
        true
    }
    pub(in crate::game) fn atlas_visual_screen_shortcut(&mut self, shortcut: EditShortcut) -> bool {
        if !self.atlas_visual_editor_active() {
            return false;
        }
        if let Some(d) = self.atlas_ports.visual.ui.dialog.as_mut() {
            d.field.shortcut(shortcut, &mut self.clipboard);
        }
        true
    }
    pub(in crate::game) fn atlas_visual_screen_char(&mut self, text: &str) -> bool {
        if !self.atlas_visual_editor_active() {
            return false;
        }
        if let Some(d) = self.atlas_ports.visual.ui.dialog.as_mut() {
            d.field.insert_text(text);
        }
        true
    }
    pub(in crate::game) fn atlas_visual_screen_scroll(&mut self, delta: f32) -> bool {
        if !self.atlas_visual_editor_active() {
            return false;
        }
        if self.atlas_ports.visual.ui.dialog.is_none() {
            let Some(r) = &self.renderer else { return true };
            let batch = HudBatch::new([r.config.width, r.config.height]);
            let max_rows = ((batch.height - 94.) / 26.).max(1.) as usize;
            let s = &mut self.atlas_ports.visual.ui;
            let max = s
                .value
                .as_object()
                .map_or(0, |v| v.len())
                .saturating_sub(max_rows);
            s.scroll = if delta > 0.0 {
                s.scroll.saturating_sub(1)
            } else {
                (s.scroll + 1).min(max)
            }
        }
        true
    }
}
