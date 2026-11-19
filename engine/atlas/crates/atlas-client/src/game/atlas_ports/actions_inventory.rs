//! Inventory Layouts 1.01: native storage, exact click planner and left-side panel.
//! MIT, Inventory Layouts contributors; see THIRD_PARTY_NOTICES.md.
use super::{Game, IDS, Screen, now_ms};
use atlas_client::{
    hud::{HudBatch, rgba},
    items::ItemStack,
    lang,
    menu::Input,
    scoreboard::{json_to_tag, tag_to_json},
    text::Text,
    ui::{self, EditBox},
};
use atlas_items::item_nbt::{item_from_nbt, item_to_nbt};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use winit::keyboard::KeyCode;
#[path = "actions_inventory_planner.rs"]
mod planner;
struct Layout {
    id: String,
    name: String,
    slots: Vec<Value>,
}
#[derive(Default)]
pub(super) struct State {
    file: PathBuf,
    layouts: Vec<Layout>,
    unreadable: bool,
    first: usize,
    selected: Option<usize>,
    editing: Option<(usize, EditBox)>,
    messages: Vec<String>,
    plan: Option<planner::Plan>,
    index: usize,
    next: u64,
    expected: Vec<planner::Stack>,
    cursor: planner::Stack,
}
impl State {
    pub(super) fn load(home: &Path) -> Self {
        let file = home.join("config/inventory-layouts.json");
        let mut s = Self {
            file,
            ..Self::default()
        };
        if !s.file.exists() {
            return s;
        }
        let read = || -> Option<Vec<Layout>> {
            let meta = fs::metadata(&s.file).ok()?;
            if meta.len() > 4 * 1024 * 1024 {
                return None;
            }
            let v: Value = serde_json::from_slice(&fs::read(&s.file).ok()?).ok()?;
            if v["version"] != 1 {
                return None;
            }
            let mut ids = BTreeSet::new();
            let mut result = vec![];
            for v in v["layouts"].as_array()? {
                let id = v["id"].as_str()?.to_owned();
                let name = v["name"].as_str()?.to_owned();
                let slots = v["slots"].as_array()?.clone();
                if id.trim().is_empty()
                    || name.trim().is_empty()
                    || name.encode_utf16().count() > 48
                    || slots.len() != 41
                    || slots.iter().any(Value::is_null)
                    || !ids.insert(id.clone())
                {
                    return None;
                }
                result.push(Layout { id, name, slots });
            }
            Some(result)
        };
        match read() {
            Some(layouts) => s.layouts = layouts,
            None => {
                s.unreadable = true;
                s.messages = vec![lang::get("inventorylayouts.load_error")];
            }
        }
        s
    }
    fn save(&self) -> bool {
        if self.unreadable {
            return false;
        }
        let value = json!({"version":1,"layouts":self.layouts.iter().map(|l|json!({"id":l.id,"name":l.name,"slots":l.slots})).collect::<Vec<_>>()});
        let Some(parent) = self.file.parent() else {
            return false;
        };
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
        let tmp = self.file.with_extension("json.tmp");
        let Ok(bytes) = serde_json::to_vec_pretty(&value) else {
            return false;
        };
        fs::write(&tmp, bytes)
            .and_then(|_| fs::rename(tmp, &self.file))
            .is_ok()
    }
    fn message(&mut self, key: &str) {
        self.messages = vec![lang::get(key)];
    }
}
fn game_slot(logical: usize) -> usize {
    match logical {
        0..36 => logical,
        36 => 504,
        _ => 500 + logical - 37,
    }
}
fn encode(stack: &Option<ItemStack>) -> Value {
    stack.as_ref().map_or(json!({}), |s| {
        let mut s = s.clone();
        s.count = 1;
        tag_to_json(&item_to_nbt(&s))
    })
}
fn snapshot(stack: &Option<ItemStack>, max: u32) -> planner::Stack {
    stack
        .as_ref()
        .map_or_else(planner::Stack::default, |s| planner::Stack {
            key: s.item.to_string(),
            identity: encode(stack).to_string(),
            count: s.count,
            max,
            split_safe: &*s.item != "minecraft:bundle",
        })
}
fn inside(r: [f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}
fn argb(color: i32) -> u32 {
    let c = color as u32;
    rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8)
}
fn draw_button(
    batch: &mut HudBatch,
    atlas: &atlas_client::hud::HudAtlas,
    r: [f32; 4],
    hover: bool,
    busy: bool,
    selected: bool,
) {
    let [x, y, w, h] = r;
    let (fill, border) = if busy {
        (rgba(31, 35, 42, 235), rgba(255, 255, 255, 24))
    } else if hover || selected {
        (rgba(47, 62, 83, 245), rgba(147, 181, 231, 210))
    } else {
        (rgba(25, 32, 43, 236), rgba(255, 255, 255, 32))
    };
    let mut ui = atlas_client::ui_theme::Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
    ui.rounded(x, y, w, h, 6.0, fill);
    ui.border(x, y, w, h, 6.0, border);
    ui.finish();
}
impl Game<'_> {
    fn atlas_inventory_visible(&self) -> bool {
        self.atlas_port_on(IDS[3])
            && self.screen == Screen::Inventory
            && !self.creative
            && !self.spectator()
            && self.craft_size == 2
            && self.remote_menu().1 == "inventory"
            && self.container.is_none()
            && !self.level.recipe_book.open[0]
    }
    fn atlas_inventory_rect(&self, batch: &HudBatch) -> [f32; 4] {
        let (x, y) = self.inventory_origin(batch);
        let width = (x - 10.0).max(48.0).min(132.0);
        [x - width - 8.0, y, width, self.screen_height()]
    }
    fn atlas_inventory_snapshot(&self) -> Vec<planner::Stack> {
        (0..41)
            .map(|i| {
                let s = match i {
                    0..36 => self.inventory[i].clone(),
                    36 => self.offhand.clone(),
                    _ => self.armor[i - 37].clone(),
                };
                let max = s
                    .as_ref()
                    .and_then(|s| self.items().get(&s.item))
                    .map_or(64, |i| i.max_stack);
                snapshot(&s, max)
            })
            .collect()
    }
    fn atlas_inventory_add(&mut self) {
        if self.carried.is_some() {
            self.atlas_ports
                .actions
                .inventory
                .message("inventorylayouts.cursor_busy");
            return;
        }
        let slots: Vec<_> = (0..41)
            .map(|i| match i {
                0..36 => encode(&self.inventory[i]),
                36 => encode(&self.offhand),
                _ => encode(&self.armor[i - 37]),
            })
            .collect();
        let s = &mut self.atlas_ports.actions.inventory;
        if s.unreadable {
            return;
        }
        let mut n = 1;
        let name = loop {
            let name = Text::tr("inventorylayouts.default_name", &[&n]).plain();
            if !s.layouts.iter().any(|l| l.name == name) {
                break name;
            }
            n += 1;
        };
        s.layouts.push(Layout {
            id: format!("atlas-{}-{}", now_ms(), s.layouts.len()),
            name,
            slots,
        });
        s.selected = Some(s.layouts.len() - 1);
        s.first = s.layouts.len().saturating_sub(5);
        s.message(if s.save() {
            "inventorylayouts.saved"
        } else {
            "inventorylayouts.save_error"
        });
    }
    fn atlas_inventory_apply(&mut self, index: usize) {
        if self.carried.is_some() {
            self.atlas_ports
                .actions
                .inventory
                .message("inventorylayouts.cursor_busy");
            return;
        }
        let Some(layout) = self.atlas_ports.actions.inventory.layouts.get(index) else {
            return;
        };
        let wanted = layout
            .slots
            .iter()
            .map(|v| item_from_nbt(&json_to_tag(v)).map(|s| s.item.to_string()))
            .collect();
        let stacks = self.atlas_inventory_snapshot();
        let expected = stacks.clone();
        let mut prototypes = BTreeMap::<String, ItemStack>::new();
        for i in 0..41 {
            let stack = match i {
                0..36 => self.inventory[i].clone(),
                36 => self.offhand.clone(),
                _ => self.armor[i - 37].clone(),
            };
            if let Some(stack) = stack {
                prototypes.insert(encode(&Some(stack.clone())).to_string(), stack);
            }
        }
        let mut plan = None;
        self.menu_session(|menu, env, to_menu| {
            let slots: Option<Vec<_>> = (0..41).map(|i| to_menu(game_slot(i))).collect();
            let slots = slots?;
            plan = planner::plan(
                stacks,
                wanted,
                |i| menu.may_pickup(env, slots[i]),
                |i, s| {
                    prototypes.get(&s.identity).is_some_and(|prototype| {
                        let mut stack = prototype.clone();
                        stack.count = s.count;
                        menu.may_place(env, slots[i], &stack)
                            && menu.slot_max(env, slots[i], &stack) >= s.count
                    })
                },
            );
            Some(false)
        });
        let s = &mut self.atlas_ports.actions.inventory;
        s.selected = Some(index);
        s.plan = plan;
        s.index = 0;
        s.next = now_ms() + 150;
        s.expected = expected;
        s.cursor = planner::Stack::default();
        if s.plan.as_ref().is_some_and(|p| p.clicks.is_empty()) {
            self.atlas_inventory_finish();
        }
    }
    fn atlas_inventory_finish(&mut self) {
        let s = &mut self.atlas_ports.actions.inventory;
        if let Some(p) = s.plan.take() {
            s.message(if p.missing.is_empty() && p.blocked.is_empty() {
                "inventorylayouts.applied"
            } else {
                "inventorylayouts.partial"
            });
            for (key, count) in p.missing {
                s.messages.push(format!("{}: {count}", key));
            }
            for (key, count) in p.blocked {
                s.messages.push(format!("{}: {count}", key));
            }
        }
    }
    pub(in crate::game) fn atlas_inventory_tick(&mut self) {
        if self.atlas_ports.actions.inventory.plan.is_none() {
            return;
        }
        if !self.atlas_inventory_visible() {
            let s = &mut self.atlas_ports.actions.inventory;
            s.plan = None;
            s.message("inventorylayouts.cancelled");
            return;
        }
        if now_ms() < self.atlas_ports.actions.inventory.next {
            return;
        }
        let stacks = self.atlas_inventory_snapshot();
        let cursor = snapshot(
            &self.carried,
            self.carried
                .as_ref()
                .and_then(|s| self.items().get(&s.item))
                .map_or(64, |i| i.max_stack),
        );
        let s = &mut self.atlas_ports.actions.inventory;
        if stacks != s.expected || cursor != s.cursor {
            s.plan = None;
            s.message("inventorylayouts.inventory_changed");
            return;
        }
        let Some(click) = s.plan.as_ref().and_then(|p| p.clicks.get(s.index)).cloned() else {
            self.atlas_inventory_finish();
            return;
        };
        self.menu_ops(&[(
            Some(game_slot(click.slot)),
            click.button,
            match click.action {
                planner::Action::Pickup => Input::Pickup,
                planner::Action::Swap => Input::Swap,
            },
        )]);
        let s = &mut self.atlas_ports.actions.inventory;
        s.expected = click.expected;
        s.cursor = click.cursor;
        s.index += 1;
        s.next = now_ms() + 150;
        let total = s.plan.as_ref().map_or(0, |p| p.clicks.len());
        s.messages = vec![Text::tr("inventorylayouts.progress", &[&s.index, &total]).plain()];
    }
    pub(in crate::game) fn atlas_inventory_draw(&mut self, batch: &mut HudBatch) {
        if !self.atlas_inventory_visible() {
            return;
        }
        let [x, y, w, h] = self.atlas_inventory_rect(batch);
        let rows = ((h - 38.0) / 22.0).floor().max(1.0) as usize;
        let s = &mut self.atlas_ports.actions.inventory;
        s.first = s.first.min(s.layouts.len().saturating_sub(rows));
        let busy = s.plan.is_some();
        let atlas = &self.gui.atlas;
        let mut tooltip = None;
        for row in 0..rows {
            let i = s.first + row;
            let Some(layout) = s.layouts.get(i) else {
                break;
            };
            let top = y + row as f32 * 22.0;
            let rect = [x, top, w - 40.0, 18.0];
            draw_button(
                batch,
                atlas,
                rect,
                inside(rect, self.cursor.0, self.cursor.1),
                busy,
                s.selected == Some(i),
            );
            if let Some((editing, field)) = &mut s.editing
                && *editing == i
            {
                field.x = x + 2.0;
                field.y = top + 1.0;
                field.width = (w - 44.0).max(4.0);
                field.draw(batch, atlas, None);
            } else {
                let len = atlas.font.fit(&layout.name, (w - 48.0).max(1.0));
                let name: String = layout.name.chars().take(len).collect();
                batch.text(
                    atlas,
                    &name,
                    x + 4.0,
                    top + 5.0,
                    if busy { argb(-6645094) } else { u32::MAX },
                    true,
                );
            }
            for dx in [36.0, 16.0] {
                let rect = [x + w - dx, top, 16.0, 18.0];
                draw_button(
                    batch,
                    atlas,
                    rect,
                    inside(rect, self.cursor.0, self.cursor.1),
                    busy,
                    false,
                );
            }
            let rename_x = x + w - 36.0;
            let delete_x = x + w - 16.0;
            for pixel in 0..7 {
                let p = pixel as f32;
                batch.fill(
                    atlas,
                    rename_x + 4.0 + p,
                    top + 12.0 - p,
                    2.0,
                    2.0,
                    argb(-1447447),
                );
                batch.fill(
                    atlas,
                    delete_x + 4.0 + p,
                    top + 5.0 + p,
                    1.0,
                    1.0,
                    argb(-21846),
                );
                batch.fill(
                    atlas,
                    delete_x + 10.0 - p,
                    top + 5.0 + p,
                    1.0,
                    1.0,
                    argb(-21846),
                );
            }
            if inside(rect, self.cursor.0, self.cursor.1) {
                tooltip = Some(vec![
                    Text::literal(&layout.name),
                    Text::tr("inventorylayouts.apply_hint", &[]),
                ]);
            } else if inside([rename_x, top, 16.0, 18.0], self.cursor.0, self.cursor.1) {
                tooltip = Some(vec![Text::tr(
                    if s.editing.as_ref().is_some_and(|(editing, _)| *editing == i) {
                        "inventorylayouts.save_name"
                    } else {
                        "inventorylayouts.rename"
                    },
                    &[],
                )]);
            } else if inside([delete_x, top, 16.0, 18.0], self.cursor.0, self.cursor.1) {
                tooltip = Some(vec![Text::tr("inventorylayouts.delete", &[])]);
            }
        }
        let add_y = y + if s.layouts.len() > rows {
            rows as f32 * 22.0 + 16.0
        } else {
            s.layouts.len() as f32 * 22.0
        };
        let r = [x, add_y, w, 18.0];
        draw_button(
            batch,
            atlas,
            r,
            inside(r, self.cursor.0, self.cursor.1),
            busy || s.unreadable,
            false,
        );
        let text = lang::get("inventorylayouts.add");
        let text = text
            .chars()
            .take(atlas.font.fit(&text, w - 8.0))
            .collect::<String>();
        batch.text(
            atlas,
            &text,
            x + (w - HudBatch::text_width(atlas, &text)) / 2.0,
            add_y + 5.0,
            u32::MAX,
            true,
        );
        if inside(r, self.cursor.0, self.cursor.1) {
            tooltip = Some(vec![
                Text::tr("inventorylayouts.add", &[]),
                Text::tr("inventorylayouts.empty", &[]),
            ]);
        }
        if s.layouts.len() > rows {
            batch.text(
                atlas,
                "<",
                x + 8.0,
                y + rows as f32 * 22.0 + 3.0,
                argb(-1118482),
                true,
            );
            batch.text(
                atlas,
                ">",
                x + w - 14.0,
                y + rows as f32 * 22.0 + 3.0,
                argb(-1118482),
                true,
            );
            let text = format!(
                "{}–{} / {}",
                s.first + 1,
                (s.first + rows).min(s.layouts.len()),
                s.layouts.len()
            );
            batch.text(
                atlas,
                &text,
                x + (w - HudBatch::text_width(atlas, &text)) / 2.0,
                y + rows as f32 * 22.0 + 3.0,
                argb(-5592406),
                true,
            );
        }
        let mut message_tooltip = None;
        if !s.messages.is_empty() {
            let mut message_width = 135.0_f32.min(x - 8.0);
            let mut left = x - message_width - 6.0;
            let mut top = y;
            let mut available_height = h;
            if message_width < 64.0 {
                left = x;
                top = add_y + 24.0;
                message_width = w;
                available_height = 13.0_f32.max(batch.height - top - 3.0);
            }
            let mut lines = Vec::new();
            for message in &s.messages {
                lines.extend(
                    atlas
                        .font
                        .split_lines(&Text::legacy(message), message_width)
                        .into_iter()
                        .map(|(text, _)| text),
                );
            }
            let visible = lines
                .len()
                .min((available_height / 10.0).floor().max(1.0) as usize);
            for (i, text) in lines.iter().take(visible).enumerate() {
                batch.text_styled(
                    atlas,
                    text,
                    left,
                    top + i as f32 * 10.0,
                    argb(if i == 0 { -11145 } else { -990258 }),
                    true,
                );
            }
            if lines.len() > visible {
                batch.text(
                    atlas,
                    "…",
                    left + message_width - 8.0,
                    top + visible as f32 * 10.0 - 10.0,
                    u32::MAX,
                    true,
                );
            }
            if inside(
                [left, top, message_width, visible as f32 * 10.0],
                self.cursor.0,
                self.cursor.1,
            ) {
                message_tooltip = Some(
                    s.messages
                        .iter()
                        .map(|message| Text::legacy(message))
                        .collect::<Vec<_>>(),
                );
            }
        }
        if s.editing.is_none()
            && let Some(lines) = tooltip
        {
            ui::tooltip(batch, atlas, &lines, self.cursor);
        }
        if let Some(lines) = message_tooltip {
            ui::tooltip(batch, atlas, &lines, self.cursor);
        }
    }
    pub(in crate::game) fn atlas_inventory_mouse(&mut self, button: i32) -> bool {
        if !self.atlas_inventory_visible() {
            return false;
        }
        let Some(r) = &self.renderer else {
            return false;
        };
        let batch = HudBatch::new([r.config.width, r.config.height]);
        let [x, y, w, h] = self.atlas_inventory_rect(&batch);
        let rows = ((h - 38.0) / 22.0).floor().max(1.0) as usize;
        let (mx, my) = self.cursor;
        let count = self.atlas_ports.actions.inventory.layouts.len();
        let add_y = y + if count > rows {
            rows as f32 * 22.0 + 16.0
        } else {
            count as f32 * 22.0
        };
        if !inside([x, y, w, add_y + 18.0 - y], mx, my) {
            let s = &mut self.atlas_ports.actions.inventory;
            if s.plan.take().is_some() {
                s.message("inventorylayouts.manual_cancel");
            }
            s.editing = None;
            return false;
        }
        if button != 0 || self.atlas_ports.actions.inventory.plan.is_some() {
            return true;
        }
        for row in 0..rows {
            let i = self.atlas_ports.actions.inventory.first + row;
            if i >= count {
                break;
            }
            let top = y + row as f32 * 22.0;
            if my < top || my >= top + 18.0 {
                continue;
            }
            if inside([x + w - 16.0, top, 16.0, 18.0], mx, my) {
                let s = &mut self.atlas_ports.actions.inventory;
                s.editing = None;
                s.layouts.remove(i);
                s.selected = s.selected.and_then(|selected| {
                    if selected == i {
                        None
                    } else {
                        Some(selected - usize::from(selected > i))
                    }
                });
                s.message(if s.save() {
                    "inventorylayouts.deleted"
                } else {
                    "inventorylayouts.save_error"
                });
                return true;
            }
            if inside([x + w - 36.0, top, 16.0, 18.0], mx, my) {
                if self
                    .atlas_ports
                    .actions
                    .inventory
                    .editing
                    .as_ref()
                    .is_some_and(|(j, _)| *j == i)
                {
                    self.atlas_inventory_save_name();
                } else {
                    let s = &mut self.atlas_ports.actions.inventory;
                    let mut field = EditBox::new(x + 2.0, top + 1.0, (w - 44.0).max(4.0), 16.0);
                    field.max_length = 48;
                    field.set_value(&s.layouts[i].name);
                    field.focused = true;
                    field.highlight = 0;
                    s.editing = Some((i, field));
                    s.message("inventorylayouts.rename_hint");
                }
                return true;
            }
            if mx < x + w - 40.0 {
                if self
                    .atlas_ports
                    .actions
                    .inventory
                    .editing
                    .as_ref()
                    .is_some_and(|(j, _)| *j == i)
                {
                    if let Some((_, f)) = &mut self.atlas_ports.actions.inventory.editing {
                        f.click(&self.gui.atlas, mx, false, false);
                    }
                } else {
                    self.atlas_ports.actions.inventory.editing = None;
                    self.atlas_inventory_apply(i);
                }
                return true;
            }
        }
        if my >= add_y {
            self.atlas_ports.actions.inventory.editing = None;
            self.atlas_inventory_add();
        } else if count > rows && my >= y + rows as f32 * 22.0 {
            let s = &mut self.atlas_ports.actions.inventory;
            s.editing = None;
            s.first = if mx < x + w / 2.0 {
                s.first.saturating_sub(rows)
            } else {
                (s.first + rows).min(count.saturating_sub(rows))
            };
        }
        true
    }
    fn atlas_inventory_save_name(&mut self) {
        let s = &mut self.atlas_ports.actions.inventory;
        // The original keeps the rename field focused when Save/Enter receives
        // an empty name, so the player can correct it instead of losing the edit.
        if s.editing
            .as_ref()
            .is_some_and(|(_, field)| field.value.trim().is_empty())
        {
            return;
        }
        if let Some((i, field)) = s.editing.take() {
            let name = field.value.trim();
            if !name.is_empty() {
                s.layouts[i].name = name.into();
                s.message(if s.save() {
                    "inventorylayouts.renamed"
                } else {
                    "inventorylayouts.save_error"
                });
            }
        }
    }
    pub(in crate::game) fn atlas_inventory_key(
        &mut self,
        key: KeyCode,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        if !self.atlas_inventory_visible() {
            return false;
        }
        let s = &mut self.atlas_ports.actions.inventory;
        if s.editing.is_none() {
            if s.plan.take().is_some() {
                s.message("inventorylayouts.manual_cancel");
            }
            return false;
        }
        match key {
            KeyCode::Enter | KeyCode::NumpadEnter => self.atlas_inventory_save_name(),
            KeyCode::Escape => {
                s.editing = None;
            }
            _ => {
                if let Some((_, f)) = &mut s.editing {
                    f.key(key, ctrl, shift);
                }
            }
        }
        true
    }
    pub(in crate::game) fn atlas_inventory_char(&mut self, text: &str) -> bool {
        if self.atlas_inventory_visible()
            && let Some((_, f)) = &mut self.atlas_ports.actions.inventory.editing
        {
            f.insert_text(text);
            true
        } else {
            false
        }
    }
    pub(in crate::game) fn atlas_inventory_scroll(&mut self, delta: f32) -> bool {
        if !self.atlas_inventory_visible() {
            return false;
        }
        let Some(r) = &self.renderer else {
            return false;
        };
        let batch = HudBatch::new([r.config.width, r.config.height]);
        let [x, y, w, h] = self.atlas_inventory_rect(&batch);
        let rows = ((h - 38.0) / 22.0).floor().max(1.0) as usize;
        let count = self.atlas_ports.actions.inventory.layouts.len();
        let add_y = y + if count > rows {
            rows as f32 * 22.0 + 16.0
        } else {
            count as f32 * 22.0
        };
        let rect = [x, y, w, add_y + 18.0 - y];
        if !inside(rect, self.cursor.0, self.cursor.1) {
            return false;
        }
        let s = &mut self.atlas_ports.actions.inventory;
        s.editing = None;
        s.first = if delta > 0.0 {
            s.first.saturating_sub(1)
        } else if delta < 0.0 {
            s.first + 1
        } else {
            s.first
        };
        true
    }
}
