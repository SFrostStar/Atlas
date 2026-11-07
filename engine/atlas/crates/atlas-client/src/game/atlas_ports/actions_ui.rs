//! Original screen geometry and input behavior for HotCommands and ViewModel.
//! HotCommands (MIT, supplied 1.0.0); ViewModel (CC0-1.0, supplied 1.21.4).
use super::{Game, HotCommand, IDS, Screen, now_ms, viewmodel};
use atlas_client::{
    hud::{HudAtlas, HudBatch, rgba},
    items::ItemStack,
    lang,
    text::Text,
    ui::{self, EditBox},
};
use winit::keyboard::KeyCode;

pub(super) struct State {
    hot_scroll: usize,
    hot_page: usize,
    hot_edit: bool,
    hot_dialog: Option<Dialog>,
    view_tab: usize,
    view_main: bool,
    view_scroll: f32,
    view_slider: Option<usize>,
    hot_edit_origin: usize,
}
impl Default for State {
    fn default() -> Self {
        Self {
            hot_scroll: 0,
            hot_page: 0,
            hot_edit: false,
            hot_dialog: None,
            view_tab: 0,
            view_main: true,
            view_scroll: 0.0,
            view_slider: None,
            hot_edit_origin: 0,
        }
    }
}
struct Dialog {
    index: Option<usize>,
    fields: [EditBox; 3],
    return_editor: bool,
}
#[derive(Clone)]
enum Action {
    Close,
    Back,
    Settings,
    HotItem(usize),
    HotUp,
    HotDown,
    HotNew,
    HotEdit(usize),
    HotChildren(usize),
    HotDelete(usize),
    HotMove(usize, bool),
    HotPage(bool),
    HotReset,
    DialogSave,
    DialogCancel,
    ViewTab(usize),
    ViewHand(bool),
    ViewSwing,
    ViewToggle(usize),
    ViewReset(usize),
    Craft(usize),
    Drop,
    Sort,
}
struct Button {
    rect: [f32; 4],
    label: Text,
    action: Action,
    active: bool,
    custom: Option<HotCommand>,
}
fn tr(key: &str) -> Text {
    Text::tr(key, &[])
}
fn hit(rect: [f32; 4], x: f32, y: f32) -> bool {
    x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3]
}
fn button(rect: [f32; 4], label: Text, action: Action) -> Button {
    Button {
        rect,
        label,
        action,
        active: true,
        custom: None,
    }
}
fn view_content(action: &Action) -> bool {
    matches!(
        action,
        Action::ViewHand(_) | Action::ViewSwing | Action::ViewToggle(_) | Action::ViewReset(_)
    )
}
fn packed(argb: i32) -> u32 {
    let c = argb as u32;
    rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8)
}
fn border(batch: &mut HudBatch, atlas: &HudAtlas, r: [f32; 4], c: u32) {
    let [x, y, w, h] = r;
    batch.fill(atlas, x, y, w, 1.0, c);
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, c);
    batch.fill(atlas, x, y, 1.0, h, c);
    batch.fill(atlas, x + w - 1.0, y, 1.0, h, c);
}
fn center(batch: &mut HudBatch, atlas: &HudAtlas, text: &str, cx: f32, y: f32, color: u32) {
    batch.text(
        atlas,
        text,
        (cx - HudBatch::text_width(atlas, text) / 2.0).floor(),
        y,
        color,
        true,
    );
}
impl super::State {
    fn current_commands(&self) -> &[HotCommand] {
        let mut items = self.commands.as_slice();
        for i in &self.command_path {
            let Some(item) = items.get(*i) else {
                break;
            };
            items = &item.children;
        }
        items
    }
    fn current_commands_mut(&mut self) -> &mut Vec<HotCommand> {
        let mut items = &mut self.commands;
        for i in &self.command_path {
            items = &mut items[*i].children;
        }
        items
    }
    fn hot_title(&self) -> String {
        let mut items = self.commands.as_slice();
        let mut name = String::new();
        for i in &self.command_path {
            if let Some(item) = items.get(*i) {
                name = item.name.clone();
                items = &item.children;
            }
        }
        name
    }
}
impl Game<'_> {
    fn actions_draw_viewmodel_chat(&self, batch: &mut HudBatch) {
        if !self.atlas_port_on(IDS[7]) {
            return;
        }
        let state = &self.atlas_ports.actions;
        let editor = &state.viewmodel_editor;
        let atlas = &self.gui.atlas;
        if self.screen == Screen::Chat
            && let Some(main) = editor.selected
        {
            let hand = state.viewmodel.hand(main);
            let hand_name = lang::get(if main {
                "atlas.viewmodel.chat.main"
            } else {
                "atlas.viewmodel.chat.off"
            });
            let pitch = format!("{:.0}", hand.rotation[0]);
            let yaw = format!("{:.0}", hand.rotation[1]);
            let scale = format!("{:.1}", hand.scale);
            let tip = Text::tr(
                "atlas.viewmodel.chat.tip",
                &[&hand_name, &pitch, &yaw, &scale],
            )
            .plain();
            let width = HudBatch::text_width(atlas, &tip);
            let cx = (batch.width - width) / 2.0;
            let cy = 12.0;
            batch.fill(
                atlas,
                cx - 8.0,
                cy - 4.0,
                width + 16.0,
                17.0,
                packed(-300609754),
            );
            batch.fill(
                atlas,
                cx - 8.0,
                cy - 4.0,
                width + 16.0,
                1.0,
                packed(-7643914),
            );
            batch.text(atlas, &tip, cx, cy, u32::MAX, false);
            let (x1, y1, x2, y2) = if main {
                (
                    (batch.width * 0.62).floor(),
                    (batch.height * 0.4).floor(),
                    batch.width - 15.0,
                    batch.height - 30.0,
                )
            } else {
                (
                    15.0,
                    (batch.height * 0.4).floor(),
                    (batch.width * 0.38).floor(),
                    batch.height - 30.0,
                )
            };
            let c = packed(-4160260);
            batch.fill(atlas, x1, y1, x2 - x1, y2 - y1, packed(410794733));
            for (x, y, w, h) in [
                (x1, y1, 22.0, 2.0),
                (x1, y1, 2.0, 22.0),
                (x2 - 22.0, y1, 22.0, 2.0),
                (x2 - 2.0, y1, 2.0, 22.0),
                (x1, y2 - 2.0, 22.0, 2.0),
                (x1, y2 - 22.0, 2.0, 22.0),
                (x2 - 22.0, y2 - 2.0, 22.0, 2.0),
                (x2 - 2.0, y2 - 22.0, 2.0, 22.0),
            ] {
                batch.fill(atlas, x, y, w, h, c);
            }
        }
    }
    pub(in crate::game) fn atlas_actions_viewmodel_hud(&self, batch: &mut HudBatch) {
        if !self.native_window() || !self.atlas_port_on(IDS[7]) {
            return;
        }
        let state = &self.atlas_ports.actions;
        let editor = &state.viewmodel_editor;
        let atlas = &self.gui.atlas;
        let remaining = editor.indicator_until.saturating_sub(now_ms());
        if remaining == 0 {
            return;
        }
        let alpha = if remaining < 500 {
            remaining as f32 / 500.0
        } else {
            1.0
        };
        let alpha = (alpha * 255.0) as u32;
        if alpha == 0 {
            return;
        }
        let rotation = editor.indicator_rotation;
        let main = if rotation {
            format!(
                "P: {:.0}°  Y: {:.0}°",
                editor.indicator_angles[0], editor.indicator_angles[1]
            )
        } else {
            format!("×{:.1}", editor.indicator_scale)
        };
        let full = format!("{}{main} ▼", if rotation { "🔄 " } else { "" });
        let width = HudBatch::text_width(atlas, &full) + 36.0;
        let x = ((batch.width - width) / 2.0).floor();
        let y = batch.height - 85.0;
        let bg = packed(((alpha.min(235) << 24) | 1708592) as i32);
        let border_color = packed(((alpha << 24) | 9133302) as i32);
        batch.fill(atlas, x + 1.0, y, width - 2.0, 22.0, bg);
        batch.fill(atlas, x, y + 1.0, width, 20.0, bg);
        for (dx, dy, w, h) in [
            (2.0, 0.0, width - 4.0, 1.0),
            (2.0, 21.0, width - 4.0, 1.0),
            (0.0, 2.0, 1.0, 18.0),
            (width - 1.0, 2.0, 1.0, 18.0),
        ] {
            batch.fill(atlas, x + dx, y + dy, w, h, border_color);
        }
        let icon_x = x + 8.0;
        let icon_y = y + 5.0;
        let icon = packed(((alpha << 24) | 12616956) as i32);
        if rotation {
            batch.text(atlas, "🔄", icon_x - 1.0, y + 7.0, icon, false);
        } else {
            let c = packed(((alpha.min(180) << 24) | 4988309) as i32);
            batch.fill(atlas, icon_x, icon_y + 1.0, 7.0, 11.0, c);
            batch.fill(atlas, icon_x + 1.0, icon_y, 5.0, 13.0, c);
            batch.fill(atlas, icon_x + 3.0, icon_y + 3.0, 1.0, 3.0, icon);
        }
        let tx = icon_x + if rotation { 13.0 } else { 11.0 };
        batch.text(
            atlas,
            &main,
            tx,
            y + 7.0,
            packed(((alpha << 24) | 16777215) as i32),
            false,
        );
        batch.text(
            atlas,
            "▼",
            tx + HudBatch::text_width(atlas, &main) + 3.0,
            y + 7.0,
            packed(((alpha << 24) | 16007006) as i32),
            false,
        );
    }
    fn actions_view_sliders(
        &self,
        batch: &HudBatch,
    ) -> Vec<([f32; 4], usize, &'static str, f32, f32, f32)> {
        let s = &self.atlas_ports.actions;
        let x = (batch.width - 380.0) / 2.0 + 18.0;
        let y = (batch.height - 270.0) / 2.0 + 54.0 - s.ui.view_scroll;
        if s.ui.view_tab == 0 {
            let h = s.viewmodel.hand(s.ui.view_main);
            [
                ("atlas.viewmodel.posx", h.position[0], -2.0, 2.0),
                ("atlas.viewmodel.posy", h.position[1], -2.0, 2.0),
                ("atlas.viewmodel.posz", h.position[2], -2.0, 2.0),
                ("atlas.viewmodel.scale", h.scale, 0.1, 3.0),
                ("atlas.viewmodel.pitch", h.rotation[0], -180.0, 180.0),
                ("atlas.viewmodel.yaw", h.rotation[1], -180.0, 180.0),
                ("atlas.viewmodel.roll", h.rotation[2], -180.0, 180.0),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (key, value, min, max))| {
                (
                    [x, y + 28.0 + i as f32 * 24.0, 344.0, 18.0],
                    i,
                    key,
                    value,
                    min,
                    max,
                )
            })
            .collect()
        } else {
            vec![
                (
                    [x, y + 32.0, 344.0, 18.0],
                    7,
                    "atlas.viewmodel.strength",
                    s.viewmodel.strength,
                    0.1,
                    3.0,
                ),
                (
                    [x, y + 56.0, 344.0, 18.0],
                    8,
                    "atlas.viewmodel.duration",
                    s.viewmodel.duration,
                    0.1,
                    3.0,
                ),
            ]
        }
    }
    fn actions_view_slider(&mut self, batch: &HudBatch, x: f32, y: f32) -> bool {
        for (r, id, _, _, _, _) in self.actions_view_sliders(batch) {
            if hit(r, x, y)
                && y >= (batch.height - 270.0) / 2.0 + 54.0
                && y < (batch.height + 270.0) / 2.0 - 6.0
            {
                self.atlas_ports.actions.ui.view_slider = Some(id);
                self.actions_view_slide(batch);
                return true;
            }
        }
        false
    }
    fn actions_view_slide(&mut self, batch: &HudBatch) {
        let Some(id) = self.atlas_ports.actions.ui.view_slider else {
            return;
        };
        for (r, i, _, _, min, max) in self.actions_view_sliders(batch) {
            if i == id {
                let norm = ((self.cursor.0 - r[0]) / r[2]).clamp(0.0, 1.0);
                let mut value = min + norm * (max - min);
                if value.abs() < 0.03 && min < 0.0 && max > 0.0 {
                    value = 0.0;
                }
                if (value - 1.0).abs() < 0.03 {
                    value = 1.0;
                }
                let s = &mut self.atlas_ports.actions;
                let hand = s.viewmodel.hand_mut(s.ui.view_main);
                match id {
                    0..=2 => hand.position[id] = value,
                    3 => hand.scale = value,
                    4..=6 => hand.rotation[id - 4] = value,
                    7 => s.viewmodel.strength = value,
                    _ => s.viewmodel.duration = value,
                }
                s.save_viewmodel();
                break;
            }
        }
    }
    fn actions_draw_viewmodel(&mut self, batch: &mut HudBatch) {
        self.actions_view_slide(batch);
        let atlas = &self.gui.atlas;
        let x = (batch.width - 380.0) / 2.0;
        let y = (batch.height - 270.0) / 2.0;
        batch.fill_gradient(
            atlas,
            (0.0, 0.0, batch.width, batch.height),
            packed(-1878325225),
            packed(-1341454313),
        );
        border(
            batch,
            atlas,
            [x - 1.0, y - 1.0, 382.0, 272.0],
            packed(1717050503),
        );
        batch.fill(atlas, x, y, 380.0, 270.0, packed(-300675289));
        batch.fill(atlas, x, y, 380.0, 2.0, packed(-8635667));
        batch.text(
            atlas,
            &lang::get("atlas.viewmodel.title"),
            x + 14.0,
            y + 12.0,
            u32::MAX,
            false,
        );
        for (r, _, key, value, min, max) in self.actions_view_sliders(batch) {
            let [sx, sy, w, h] = r;
            if sy < y + 54.0 || sy + h > y + 264.0 {
                continue;
            }
            let width = ((value - min) / (max - min)).clamp(0.0, 1.0) * (w - 2.0);
            batch.fill(atlas, sx, sy, w, h, packed(1428034616));
            batch.fill(
                atlas,
                sx + 1.0,
                sy + 1.0,
                width,
                h - 2.0,
                packed(-2005124371),
            );
            batch.fill(atlas, sx + width - 1.0, sy, 4.0, h, packed(-4160260));
            batch.text(
                atlas,
                &lang::get(key),
                sx + 6.0,
                sy + 5.0,
                packed(-2039584),
                false,
            );
            let text = format!("{value:.2}");
            batch.text(
                atlas,
                &text,
                sx + w - HudBatch::text_width(atlas, &text) - 6.0,
                sy + 5.0,
                u32::MAX,
                false,
            );
        }
    }
    fn actions_buttons(&self, batch: &HudBatch) -> Vec<Button> {
        let mut out = Vec::new();
        let w = batch.width;
        let h = batch.height;
        if self.screen == Screen::Inventory {
            let (x, y) = self.inventory_origin(batch);
            if self.atlas_port_on(IDS[0]) && self.atlas_crafting_table() {
                for (i, dx, bw, key) in [
                    (0, 0.0, 57.0, "atlas.autocraft.crystal"),
                    (1, 59.0, 58.0, "atlas.autocraft.eye"),
                    (2, 119.0, 57.0, "atlas.autocraft.arrows"),
                ] {
                    let mut label = lang::get(key);
                    if self.atlas_ports.actions.autocraft == Some(i) {
                        label = format!("§a▶ {label}");
                    }
                    out.push(button(
                        [x + dx, (y - 22.0).max(2.0), bw, 20.0],
                        Text::legacy(&label),
                        Action::Craft(i),
                    ));
                }
            }
            let chest = self.atlas_actions_chest();
            if self.atlas_port_on(IDS[1])
                && (chest
                    || (self.container.is_none()
                        && self.remote_menu().1 == "inventory"
                        && self.craft_size == 2))
            {
                out.push(button(
                    if chest {
                        [x + 114.0, y + 4.0, 56.0, 14.0]
                    } else {
                        [x + 180.0, h / 2.0 - 22.0, 56.0, 20.0]
                    },
                    tr("atlas.dropall.button"),
                    Action::Drop,
                ));
            }
            if chest && self.atlas_port_on(IDS[5]) {
                out.push(button(
                    [x - 39.0, y + 6.0, 36.0, 16.0],
                    tr("atlas.sortmod.button"),
                    Action::Sort,
                ));
            }
        }
        match self.native_mod_screen() {
            Some(id) if id == IDS[2] => {
                let s = &self.atlas_ports.actions;
                if s.ui.hot_dialog.is_some() {
                    let x = (w - 260.0) / 2.0;
                    out.push(button(
                        [x, 180.0, 125.0, 20.0],
                        tr("gui.hotcommands.editor.save"),
                        Action::DialogSave,
                    ));
                    out.push(button(
                        [x + 135.0, 180.0, 125.0, 20.0],
                        tr("gui.hotcommands.editor.cancel"),
                        Action::DialogCancel,
                    ));
                } else if s.ui.hot_edit {
                    out.push(button(
                        [12.0, 12.0, 65.0, 20.0],
                        tr("gui.hotcommands.back"),
                        Action::Back,
                    ));
                    out.push(button(
                        [w - 130.0, 12.0, 118.0, 20.0],
                        tr("gui.hotcommands.editor.add"),
                        Action::HotNew,
                    ));
                    let list_w = 380.0_f32.min(w - 24.0);
                    let x = (w - list_w) / 2.0;
                    for (i, item) in s
                        .current_commands()
                        .iter()
                        .enumerate()
                        .skip(s.ui.hot_page * 6)
                        .take(6)
                    {
                        let y = 44.0 + (i - s.ui.hot_page * 6) as f32 * 30.0;
                        let r = x + list_w;
                        for (dx, bw, label, a) in [
                            (24.0, 22.0, "🗑".into(), Action::HotDelete(i)),
                            (50.0, 22.0, "✎".into(), Action::HotEdit(i)),
                            (
                                84.0,
                                32.0,
                                format!("📂{}", item.children.len()),
                                Action::HotChildren(i),
                            ),
                            (108.0, 20.0, "▼".into(), Action::HotMove(i, false)),
                            (130.0, 20.0, "▲".into(), Action::HotMove(i, true)),
                        ] {
                            let mut b = button([r - dx, y, bw, 22.0], Text::literal(label), a);
                            b.active = match b.action {
                                Action::HotMove(_, true) => i > 0,
                                Action::HotMove(_, false) => i + 1 < s.current_commands().len(),
                                _ => true,
                            };
                            out.push(b);
                        }
                    }
                    if s.command_path.is_empty() {
                        out.push(button(
                            [12.0, h - 28.0, 150.0, 20.0],
                            tr("gui.hotcommands.editor.reset_defaults"),
                            Action::HotReset,
                        ));
                    }
                    let pages = s.current_commands().len().div_ceil(6).max(1);
                    if pages > 1 {
                        out.push(button(
                            [w / 2.0 - 55.0, h - 28.0, 24.0, 20.0],
                            tr("gui.hotcommands.prev_page"),
                            Action::HotPage(false),
                        ));
                        out.push(button(
                            [w / 2.0 + 31.0, h - 28.0, 24.0, 20.0],
                            tr("gui.hotcommands.next_page"),
                            Action::HotPage(true),
                        ));
                    }
                    out.push(button(
                        [w - 95.0, h - 28.0, 85.0, 20.0],
                        tr("gui.hotcommands.editor.save"),
                        Action::Back,
                    ));
                } else {
                    if !s.command_path.is_empty() {
                        out.push(button(
                            [12.0, 10.0, 65.0, 20.0],
                            tr("gui.hotcommands.back"),
                            Action::Back,
                        ));
                    }
                    out.push(button(
                        [w - 88.0, 10.0, 76.0, 20.0],
                        tr("gui.hotcommands.settings"),
                        Action::Settings,
                    ));
                    let max_visible = ((h - 66.0) / 25.0).max(3.0) as usize;
                    let visible = s.current_commands().len().min(max_visible);
                    let max_scroll = s.current_commands().len().saturating_sub(max_visible);
                    let scroll = s.ui.hot_scroll.min(max_scroll);
                    let stack = visible as f32 * 25.0 - 3.0;
                    let y = 38.0 + ((h - 66.0) - stack) / 2.0;
                    let x = (w - 210.0) / 2.0;
                    if scroll > 0 {
                        out.push(button(
                            [x + 90.0, y - 18.0, 30.0, 14.0],
                            Text::literal("▲"),
                            Action::HotUp,
                        ));
                    }
                    for (i, item) in s
                        .current_commands()
                        .iter()
                        .enumerate()
                        .skip(scroll)
                        .take(visible)
                    {
                        let mut b = button(
                            [x, y + (i - scroll) as f32 * 25.0, 210.0, 22.0],
                            Text::legacy(&item.name),
                            Action::HotItem(i),
                        );
                        b.custom = Some(item.clone());
                        out.push(b);
                    }
                    if scroll < max_scroll {
                        out.push(button(
                            [x + 90.0, y + stack + 4.0, 30.0, 14.0],
                            Text::literal("▼"),
                            Action::HotDown,
                        ));
                    }
                    out.push(button(
                        [w - 75.0, h - 24.0, 65.0, 18.0],
                        tr("gui.hotcommands.close"),
                        Action::Close,
                    ));
                }
            }
            Some(id) if id == IDS[7] => {
                let s = &self.atlas_ports.actions;
                let x = (w - 380.0) / 2.0;
                let y = (h - 270.0) / 2.0;
                out.push(button(
                    [x + 14.0, y + 28.0, 140.0, 20.0],
                    tr("atlas.viewmodel.tab.hand"),
                    Action::ViewTab(0),
                ));
                out.push(button(
                    [x + 160.0, y + 28.0, 140.0, 20.0],
                    tr("atlas.viewmodel.tab.swing"),
                    Action::ViewTab(1),
                ));
                out.push(button(
                    [x + 356.0, y + 10.0, 14.0, 14.0],
                    Text::literal("✕"),
                    Action::Close,
                ));
                let y = y + 54.0 - s.ui.view_scroll;
                if s.ui.view_tab == 0 {
                    out.push(button(
                        [x + 18.0, y, 160.0, 20.0],
                        tr("atlas.viewmodel.main"),
                        Action::ViewHand(true),
                    ));
                    out.push(button(
                        [x + 192.0, y, 160.0, 20.0],
                        tr("atlas.viewmodel.off"),
                        Action::ViewHand(false),
                    ));
                    for (i, dy, key) in [
                        (0, 200.0, "atlas.viewmodel.interactive"),
                        (1, 222.0, "atlas.viewmodel.outline"),
                    ] {
                        out.push(button(
                            [x + 18.0, y + dy, 344.0, 14.0],
                            tr(key),
                            Action::ViewToggle(i),
                        ));
                    }
                    out.push(button(
                        [x + 18.0, y + 250.0, 160.0, 20.0],
                        tr("atlas.viewmodel.reset.hand"),
                        Action::ViewReset(0),
                    ));
                    out.push(button(
                        [x + 192.0, y + 250.0, 160.0, 20.0],
                        tr("atlas.viewmodel.reset.all"),
                        Action::ViewReset(1),
                    ));
                } else {
                    out.push(button(
                        [x + 18.0, y, 344.0, 24.0],
                        Text::literal(s.viewmodel.swing.name()),
                        Action::ViewSwing,
                    ));
                    out.push(button(
                        [x + 18.0, y + 84.0, 344.0, 14.0],
                        tr("atlas.viewmodel.onlytarget"),
                        Action::ViewToggle(2),
                    ));
                    out.push(button(
                        [x + 18.0, y + 106.0, 344.0, 14.0],
                        tr("atlas.viewmodel.autoswing"),
                        Action::ViewToggle(3),
                    ));
                    out.push(button(
                        [x + 18.0, y + 136.0, 344.0, 20.0],
                        tr("atlas.viewmodel.reset.swing"),
                        Action::ViewReset(2),
                    ));
                }
            }
            _ => {}
        }
        out
    }
    fn actions_icon(&self, batch: &mut HudBatch, item: &HotCommand, x: f32, y: f32) {
        let icon = item.icon.trim().to_lowercase();
        let icon = if icon.contains(':') {
            icon
        } else {
            format!("minecraft:{icon}")
        };
        let fallback = if item.children.is_empty() {
            "minecraft:paper"
        } else {
            "minecraft:chest"
        };
        let icon = if self.items().get(&icon).is_some() {
            icon.as_str()
        } else {
            fallback
        };
        self.draw_item_model(
            batch,
            &ItemStack {
                item: icon.into(),
                count: 1,
                damage: 0,
                extra: None,
            },
            x,
            y,
        );
    }
    pub(in crate::game) fn atlas_actions_draw(&mut self, batch: &mut HudBatch) {
        self.atlas_inventory_draw(batch);
        self.actions_draw_viewmodel_chat(batch);
        let screen = self.native_mod_screen().map(str::to_owned);
        let atlas = &self.gui.atlas;
        if screen.as_deref() == Some(IDS[2]) {
            let s = &self.atlas_ports.actions;
            let title = lang::get(if s.ui.hot_edit {
                "gui.hotcommands.editor.title"
            } else {
                "gui.hotcommands.title"
            });
            let category = s.hot_title();
            let title = if category.is_empty() {
                format!("§6§l{title}")
            } else {
                format!("§7{title} §8▶ §f§l{category}")
            };
            center(batch, atlas, &title, batch.width / 2.0, 14.0, u32::MAX);
            if s.ui.hot_dialog.is_none() && s.ui.hot_edit {
                let lw = 380.0_f32.min(batch.width - 24.0);
                let x = (batch.width - lw) / 2.0;
                for (i, item) in s
                    .current_commands()
                    .iter()
                    .enumerate()
                    .skip(s.ui.hot_page * 6)
                    .take(6)
                {
                    let y = 44.0 + (i - s.ui.hot_page * 6) as f32 * 30.0;
                    batch.fill(atlas, x, y, lw, 26.0, packed(-2011684824));
                    border(batch, atlas, [x, y, lw, 26.0], packed(1430541670));
                    self.actions_icon(batch, item, x + 4.0, y + 5.0);
                    let n = atlas.font.fit(&item.name, lw - 160.0);
                    let name: String = item.name.chars().take(n).collect();
                    batch.text(atlas, &name, x + 26.0, y + 3.0, u32::MAX, true);
                    let sub = if item.children.is_empty() {
                        item.command.clone()
                    } else {
                        format!("📂 {}", item.children.len())
                    };
                    let n = atlas.font.fit(&sub, lw - 160.0);
                    let sub: String = sub.chars().take(n).collect();
                    batch.text(atlas, &sub, x + 26.0, y + 14.0, 0xffaaaaaa, true);
                }
            }
            if !s.ui.hot_edit && s.ui.hot_dialog.is_none() {
                batch.text(
                    atlas,
                    &lang::get("atlas.hotcommands.edit_hint"),
                    12.0,
                    batch.height - 18.0,
                    0xff777777,
                    true,
                );
            }
        }
        if screen.as_deref() == Some(IDS[7]) {
            self.actions_draw_viewmodel(batch);
        }
        let mut tooltip: Option<Vec<Text>> = None;
        for b in self.actions_buttons(batch) {
            // Original ViewModel scrolling clips the content below the tabs.
            if screen.as_deref() == Some(IDS[7])
                && view_content(&b.action)
                && (b.rect[1] < (batch.height - 270.0) / 2.0 + 54.0
                    || b.rect[1] + b.rect[3] > (batch.height + 270.0) / 2.0 - 6.0)
            {
                continue;
            }
            let hover = hit(b.rect, self.cursor.0, self.cursor.1);
            if hover {
                match b.action {
                    Action::Craft(recipe) => {
                        let key = match recipe {
                            0 => "atlas.autocraft.crystal.tip",
                            1 => "atlas.autocraft.eye.tip",
                            _ => "atlas.autocraft.arrows.tip",
                        };
                        tooltip = Some(lang::get(key).lines().map(Text::legacy).collect());
                    }
                    Action::HotItem(_) => {
                        if let Some(item) = b.custom.as_ref() {
                            let text = if !item.children.is_empty() {
                                lang::get("gui.hotcommands.tooltip.submenu")
                                    .replace("%d", &item.children.len().to_string())
                                    .replace("%s", &item.children.len().to_string())
                            } else if !item.command.is_empty() {
                                lang::get("gui.hotcommands.tooltip.command")
                                    .replace("%s", &item.command)
                            } else {
                                String::new()
                            };
                            if !text.is_empty() {
                                tooltip = Some(vec![Text::legacy(&text)]);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let [x, y, w, h] = b.rect;
            if let Some(item) = b.custom {
                let atlas = &self.gui.atlas;
                batch.fill(
                    atlas,
                    x,
                    y,
                    w,
                    h,
                    packed(if hover { -584963014 } else { -1156507614 }),
                );
                border(
                    batch,
                    atlas,
                    b.rect,
                    packed(if hover { -13058568 } else { -1440008891 }),
                );
                self.actions_icon(batch, &item, x + 4.0, y + (h - 16.0) / 2.0);
                let atlas = &self.gui.atlas;
                let max = w - 36.0 - if item.children.is_empty() { 0.0 } else { 12.0 };
                let n = atlas.font.fit(&item.name, max);
                let trimmed: String = item.name.chars().take(n).collect();
                let text = if n < item.name.chars().count() {
                    format!("{trimmed}...")
                } else {
                    trimmed
                };
                let tx = (x + (w - HudBatch::text_width(atlas, &text)) / 2.0).max(x + 22.0);
                batch.text(
                    atlas,
                    &text,
                    tx,
                    y + (h - 9.0) / 2.0,
                    if hover { u32::MAX } else { packed(-1906448) },
                    true,
                );
                if !item.children.is_empty() {
                    batch.text(
                        atlas,
                        "▶",
                        x + w - 11.0,
                        y + (h - 9.0) / 2.0,
                        if hover {
                            packed(-10752)
                        } else {
                            packed(-7035976)
                        },
                        true,
                    );
                }
            } else if screen.as_deref() == Some(IDS[7]) {
                let atlas = &self.gui.atlas;
                let s = &self.atlas_ports.actions;
                match b.action {
                    Action::ViewToggle(i) => {
                        let on = match i {
                            0 => s.viewmodel.interactive,
                            1 => s.viewmodel.outline,
                            2 => s.viewmodel.only_target,
                            _ => s.viewmodel.auto_swing,
                        };
                        batch.text_styled(atlas, &b.label, x, y + 3.0, packed(-2236963), false);
                        let px = x + w - 32.0;
                        batch.fill(
                            atlas,
                            px,
                            y,
                            32.0,
                            14.0,
                            packed(if on { -8635667 } else { 1714626900 }),
                        );
                        batch.fill(
                            atlas,
                            if on { px + 20.0 } else { px + 2.0 },
                            y + 2.0,
                            10.0,
                            10.0,
                            u32::MAX,
                        );
                    }
                    Action::Close => {
                        batch.text(
                            atlas,
                            "✕",
                            x,
                            y,
                            if hover {
                                packed(-43691)
                            } else {
                                packed(-7829368)
                            },
                            false,
                        );
                    }
                    Action::ViewSwing => {
                        batch.fill(
                            atlas,
                            x,
                            y,
                            w,
                            h,
                            packed(if hover { -2006112039 } else { 1145838997 }),
                        );
                        batch.fill(atlas, x, y, 3.0, h, packed(-5745161));
                        batch.text(
                            atlas,
                            &lang::get("atlas.viewmodel.swing_type"),
                            x + 12.0,
                            y + 8.0,
                            packed(-3355444),
                            false,
                        );
                        let name = format!("§d§l{} §7⟳", s.viewmodel.swing.display_name());
                        batch.text(
                            atlas,
                            &name,
                            x + w - HudBatch::text_width(atlas, &name) - 12.0,
                            y + 8.0,
                            u32::MAX,
                            false,
                        );
                    }
                    _ => {
                        let selected = match b.action {
                            Action::ViewTab(i) => s.ui.view_tab == i,
                            Action::ViewHand(main) => s.ui.view_main == main,
                            _ => false,
                        };
                        let reset = matches!(b.action, Action::ViewReset(_));
                        let color = if reset {
                            if hover { -1713625562 } else { 1436097307 }
                        } else if selected {
                            -8635667
                        } else if hover {
                            1716264341
                        } else {
                            858656869
                        };
                        batch.fill(atlas, x, y, w, h, packed(color));
                        center(
                            batch,
                            atlas,
                            &b.label.plain(),
                            x + w / 2.0,
                            y + 6.0,
                            if selected { u32::MAX } else { packed(-7829368) },
                        );
                    }
                }
            } else {
                ui::button(
                    batch,
                    &self.gui.atlas,
                    x,
                    y,
                    w,
                    h,
                    &b.label,
                    b.active,
                    hover,
                );
            }
        }
        if screen.as_deref() == Some(IDS[2])
            && let Some(dialog) = self.atlas_ports.actions.ui.hot_dialog.as_mut()
        {
            let x = (batch.width - 260.0) / 2.0;
            for (i, key) in [
                "gui.hotcommands.editor.name_label",
                "gui.hotcommands.editor.cmd_label",
                "gui.hotcommands.editor.icon_label",
            ]
            .into_iter()
            .enumerate()
            {
                let y = 56.0 + i as f32 * 40.0;
                batch.text(
                    &self.gui.atlas,
                    &lang::get(key),
                    x,
                    y - 12.0,
                    0xffdddddd,
                    false,
                );
                dialog.fields[i].x = x;
                dialog.fields[i].y = y;
                dialog.fields[i].draw(batch, &self.gui.atlas, None);
            }
        }
        if let Some(lines) = tooltip {
            ui::tooltip(batch, &self.gui.atlas, &lines, self.cursor);
        }
    }
    fn actions_open_dialog(&mut self, index: Option<usize>) {
        let s = &mut self.atlas_ports.actions;
        let item = index
            .and_then(|i| s.current_commands().get(i))
            .cloned()
            .unwrap_or_else(|| HotCommand::new("", "", ""));
        let values = [item.name.replace('§', "&"), item.command, item.icon];
        let fields = std::array::from_fn(|i| {
            let mut field = EditBox::new(
                0.0,
                56.0 + i as f32 * 40.0,
                if i == 2 { 230.0 } else { 260.0 },
                20.0,
            );
            field.max_length = if i == 1 { 256 } else { 128 };
            field.set_value(&values[i]);
            field.focused = i == 0;
            field
        });
        s.ui.hot_dialog = Some(Dialog {
            index,
            fields,
            return_editor: s.ui.hot_edit,
        });
    }
    fn actions_press(&mut self, action: Action) {
        match action {
            Action::Close => {
                if let Some(id) = self.native_mod_screen().map(str::to_owned) {
                    self.native_mod_screen_close(&id);
                }
            }
            Action::Back => {
                let s = &mut self.atlas_ports.actions;
                if s.ui.hot_edit {
                    if s.command_path.len() <= s.ui.hot_edit_origin {
                        s.ui.hot_edit = false;
                    } else {
                        s.command_path.pop();
                    }
                    s.save_commands();
                } else {
                    s.command_path.pop();
                }
                s.ui.hot_page = 0;
                s.ui.hot_scroll = 0;
            }
            Action::Settings => {
                let s = &mut self.atlas_ports.actions;
                s.ui.hot_edit = true;
                s.ui.hot_page = 0;
                s.ui.hot_edit_origin = s.command_path.len();
            }
            Action::HotItem(i) => {
                let item = self.atlas_ports.actions.current_commands()[i].clone();
                if !item.children.is_empty() {
                    let s = &mut self.atlas_ports.actions;
                    s.command_path.push(i);
                    s.ui.hot_scroll = 0;
                } else if !item.command.is_empty() {
                    self.native_mod_screen_close(IDS[2]);
                    self.handle_chat_input(item.command.trim());
                }
            }
            Action::HotUp => {
                self.atlas_ports.actions.ui.hot_scroll =
                    self.atlas_ports.actions.ui.hot_scroll.saturating_sub(1)
            }
            Action::HotDown => self.atlas_ports.actions.ui.hot_scroll += 1,
            Action::HotNew => self.actions_open_dialog(None),
            Action::HotEdit(i) => self.actions_open_dialog(Some(i)),
            Action::HotChildren(i) => {
                let s = &mut self.atlas_ports.actions;
                s.command_path.push(i);
                s.ui.hot_page = 0;
            }
            Action::HotDelete(i) => {
                let s = &mut self.atlas_ports.actions;
                s.current_commands_mut().remove(i);
                let max = s.current_commands().len().div_ceil(6).saturating_sub(1);
                s.ui.hot_page = s.ui.hot_page.min(max);
                s.save_commands();
            }
            Action::HotMove(i, up) => {
                let s = &mut self.atlas_ports.actions;
                let n = s.current_commands().len();
                if (up && i > 0) || (!up && i + 1 < n) {
                    s.current_commands_mut()
                        .swap(i, if up { i - 1 } else { i + 1 });
                    s.save_commands();
                }
            }
            Action::HotPage(next) => {
                let s = &mut self.atlas_ports.actions;
                let pages = s.current_commands().len().div_ceil(6).max(1);
                s.ui.hot_page = if next {
                    (s.ui.hot_page + 1).min(pages - 1)
                } else {
                    s.ui.hot_page.saturating_sub(1)
                };
            }
            Action::HotReset => {
                let s = &mut self.atlas_ports.actions;
                s.commands = super::default_commands();
                s.ui.hot_page = 0;
                s.save_commands();
            }
            Action::DialogSave => {
                let s = &mut self.atlas_ports.actions;
                if let Some(dialog) = s.ui.hot_dialog.take() {
                    let mut name = dialog.fields[0].value.trim().to_owned();
                    let command = dialog.fields[1].value.trim().to_owned();
                    if name.is_empty() && !command.is_empty() {
                        name = command.clone();
                    }
                    let icon = dialog.fields[2].value.trim().to_owned();
                    let name = name.replace('&', "§");
                    if let Some(i) = dialog.index {
                        if let Some(item) = s.current_commands_mut().get_mut(i) {
                            item.name = name;
                            item.command = command;
                            item.icon = icon;
                        }
                    } else {
                        s.current_commands_mut()
                            .push(HotCommand::new(&name, &command, &icon));
                    }
                    s.ui.hot_edit = dialog.return_editor;
                    s.save_commands();
                }
            }
            Action::DialogCancel => {
                self.atlas_ports.actions.ui.hot_dialog = None;
            }
            Action::ViewTab(i) => {
                let s = &mut self.atlas_ports.actions;
                s.ui.view_tab = i;
                s.ui.view_scroll = 0.0;
                s.ui.view_slider = None;
            }
            Action::ViewHand(main) => self.atlas_ports.actions.ui.view_main = main,
            Action::ViewSwing => {
                let s = &mut self.atlas_ports.actions;
                let i = viewmodel::Swing::ALL
                    .iter()
                    .position(|v| *v == s.viewmodel.swing)
                    .unwrap_or(0);
                s.viewmodel.swing = viewmodel::Swing::ALL[(i + 1) % 13];
                s.save_viewmodel();
            }
            Action::ViewToggle(i) => {
                let s = &mut self.atlas_ports.actions;
                let c = &mut s.viewmodel;
                match i {
                    0 => c.interactive = !c.interactive,
                    1 => c.outline = !c.outline,
                    2 => c.only_target = !c.only_target,
                    _ => c.auto_swing = !c.auto_swing,
                }
                s.save_viewmodel();
            }
            Action::ViewReset(i) => {
                let s = &mut self.atlas_ports.actions;
                match i {
                    0 => *s.viewmodel.hand_mut(s.ui.view_main) = viewmodel::HandSettings::default(),
                    1 => s.viewmodel = viewmodel::Config::default(),
                    _ => s.viewmodel.reset_swing(),
                }
                s.save_viewmodel();
            }
            Action::Craft(i) => {
                self.atlas_actions_craft_toggle(i);
            }
            Action::Drop => {
                self.atlas_actions_drop_all();
            }
            Action::Sort => {
                self.atlas_actions_start_sort();
            }
        }
    }
    pub(in crate::game) fn atlas_actions_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if !pressed {
            self.atlas_ports.actions.ui.view_slider = None;
            return false;
        }
        if self.atlas_inventory_mouse(button) {
            return true;
        }
        let Some(r) = &self.renderer else {
            return false;
        };
        let batch = HudBatch::new([r.config.width, r.config.height]);
        let (x, y) = self.cursor;
        if self.native_mod_screen() == Some(IDS[2])
            && self.atlas_ports.actions.ui.hot_dialog.is_some()
        {
            let s = &mut self.atlas_ports.actions;
            if let Some(d) = s.ui.hot_dialog.as_mut() {
                for f in &mut d.fields {
                    f.x = (batch.width - 260.0) / 2.0;
                    f.focused = hit([f.x, f.y, f.width, f.height], x, y);
                    if f.focused {
                        f.click(&self.gui.atlas, x, false, false);
                    }
                }
            }
        }
        if self.native_mod_screen() == Some(IDS[7])
            && button == 0
            && self.actions_view_slider(&batch, x, y)
        {
            return true;
        }
        for b in self.actions_buttons(&batch) {
            if self.native_mod_screen() == Some(IDS[7])
                && view_content(&b.action)
                && (b.rect[1] < (batch.height - 270.0) / 2.0 + 54.0
                    || b.rect[1] + b.rect[3] > (batch.height + 270.0) / 2.0 - 6.0)
            {
                continue;
            }
            if b.active && hit(b.rect, x, y) {
                if button == 1
                    && let Action::HotItem(i) = b.action
                {
                    self.actions_open_dialog(Some(i));
                    return true;
                }
                if button == 0 {
                    self.actions_press(b.action);
                    return true;
                }
            }
        }
        if self
            .native_mod_screen()
            .is_some_and(|id| id == IDS[2] || id == IDS[7])
        {
            return true;
        }
        if self.screen == Screen::Chat && self.atlas_port_on(IDS[7]) {
            let s = &mut self.atlas_ports.actions;
            return s.viewmodel_editor.click(
                &s.viewmodel,
                x as f64,
                y as f64,
                batch.width as f64,
                batch.height as f64,
                button,
                now_ms(),
            );
        }
        false
    }
    pub(in crate::game) fn atlas_actions_screen_key(
        &mut self,
        key: KeyCode,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        if self.atlas_inventory_key(key, ctrl, shift) {
            return true;
        }
        if self.native_mod_screen() == Some(IDS[2]) {
            if key == KeyCode::Escape {
                if self.atlas_ports.actions.ui.hot_dialog.is_some() {
                    self.actions_press(Action::DialogCancel);
                } else if self.atlas_ports.actions.ui.hot_edit
                    || !self.atlas_ports.actions.command_path.is_empty()
                {
                    self.actions_press(Action::Back);
                } else {
                    self.actions_press(Action::Close);
                }
                return true;
            }
            if let Some(d) = self.atlas_ports.actions.ui.hot_dialog.as_mut() {
                if key == KeyCode::Tab {
                    let i = d.fields.iter().position(|f| f.focused).unwrap_or(0);
                    for f in &mut d.fields {
                        f.focused = false;
                    }
                    d.fields[(i + if shift { 2 } else { 1 }) % 3].focused = true;
                    return true;
                }
                for f in &mut d.fields {
                    if f.focused {
                        f.key(key, ctrl, shift);
                    }
                }
                return true;
            }
            if key == KeyCode::KeyH {
                self.actions_press(Action::Close);
            }
            return true;
        }
        self.native_mod_screen() == Some(IDS[7]) && key != KeyCode::Escape
    }
    pub(in crate::game) fn atlas_actions_screen_char(&mut self, text: &str) -> bool {
        if self.atlas_inventory_char(text) {
            return true;
        }
        if self.native_mod_screen() == Some(IDS[2])
            && let Some(d) = self.atlas_ports.actions.ui.hot_dialog.as_mut()
        {
            for f in &mut d.fields {
                if f.focused {
                    f.insert_text(text);
                }
            }
            return true;
        }
        false
    }
    pub(in crate::game) fn atlas_actions_scroll(&mut self, delta: f32) -> bool {
        if self.atlas_inventory_scroll(delta) {
            return true;
        }
        if self.native_mod_screen() == Some(IDS[2]) {
            if self.atlas_ports.actions.ui.hot_dialog.is_none()
                && !self.atlas_ports.actions.ui.hot_edit
            {
                let Some(r) = &self.renderer else {
                    return true;
                };
                let batch = HudBatch::new([r.config.width, r.config.height]);
                let s = &mut self.atlas_ports.actions;
                let max = s
                    .current_commands()
                    .len()
                    .saturating_sub(((batch.height - 66.0) / 25.0).max(3.0) as usize);
                s.ui.hot_scroll = if delta > 0.0 {
                    s.ui.hot_scroll.saturating_sub(1)
                } else {
                    (s.ui.hot_scroll + 1).min(max)
                };
            }
            return true;
        }
        if self.native_mod_screen() == Some(IDS[7]) {
            let s = &mut self.atlas_ports.actions;
            let max = if s.ui.view_tab == 0 { 85.0 } else { 0.0 };
            s.ui.view_scroll = (s.ui.view_scroll - delta * 20.0).clamp(0.0, max);
            return true;
        }
        if self.screen == Screen::Chat && self.atlas_port_on(IDS[7]) {
            let shift = self.has_shift_down();
            let s = &mut self.atlas_ports.actions;
            let changed =
                s.viewmodel_editor
                    .scroll(&mut s.viewmodel, delta as f64, shift, now_ms());
            if changed {
                s.save_viewmodel();
            }
            return changed;
        }
        false
    }
    pub(in crate::game) fn atlas_actions_drag(&mut self, button: i32, dx: f32, dy: f32) -> bool {
        if self.screen == Screen::Chat && self.atlas_port_on(IDS[7]) {
            let shift = self.has_shift_down();
            let s = &mut self.atlas_ports.actions;
            let changed = s.viewmodel_editor.drag(
                &mut s.viewmodel,
                button,
                dx as f64,
                dy as f64,
                shift,
                now_ms(),
            );
            if changed {
                s.save_viewmodel();
            }
            changed
        } else {
            false
        }
    }
    pub(in crate::game) fn atlas_actions_command(&mut self, message: &str) -> bool {
        if self.atlas_gamma_command(message) {
            return true;
        }
        if !self.atlas_port_on(IDS[7]) {
            return false;
        }
        let msg = message.trim();
        if msg.eq_ignore_ascii_case(".viewmodel reset")
            || msg.eq_ignore_ascii_case("/viewmodel reset")
            || msg.eq_ignore_ascii_case("viewmodel reset")
        {
            self.atlas_ports.actions.viewmodel.reset_hands();
            self.atlas_ports.actions.save_viewmodel();
            self.native_chat(&lang::get("atlas.viewmodel.reset.done"));
            true
        } else if msg.eq_ignore_ascii_case(".viewmodel")
            || msg.eq_ignore_ascii_case("/viewmodel")
            || msg.eq_ignore_ascii_case("viewmodel")
        {
            self.atlas_open_port_screen(IDS[7])
        } else {
            false
        }
    }
}
