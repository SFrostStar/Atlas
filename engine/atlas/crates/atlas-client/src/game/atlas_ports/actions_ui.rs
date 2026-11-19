//! Original screen geometry and input behavior for HotCommands and ViewModel.
//! HotCommands (MIT, supplied 1.0.0); ViewModel (CC0-1.0, supplied 1.21.4).
use super::{Game, HotCommand, IDS, Screen, now_ms, viewmodel};
use atlas_client::{
    atlas::mod_style,
    hud::{HudAtlas, HudBatch, rgba},
    items::ItemStack,
    lang,
    text::Text,
    ui::{self, EditBox},
    ui_theme::{Face, Ui, color},
    widgets,
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
    view_pick: Option<usize>,
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
            view_pick: None,
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
const SWING_ROW_HEIGHT: f32 = 14.0;

fn view_panel(width: f32, height: f32) -> [f32; 4] {
    let panel_height = (height - 8.0).min(270.0);
    [
        (width - 380.0) / 2.0,
        (height - panel_height) / 2.0,
        380.0,
        panel_height,
    ]
}

fn hand_scroll_max(panel_height: f32) -> f32 {
    // The last row ends at 54 + 250 + 20 px, with another 6 px below it.
    330.0 - panel_height
}

fn swing_selector(w: f32, h: f32) -> [f32; 4] {
    let [x, y, ..] = view_panel(w, h);
    [x + 18.0, y + 54.0, 344.0, 24.0]
}

fn swing_picker(w: f32, h: f32) -> [f32; 4] {
    let [x, y, width, height] = swing_selector(w, h);
    let menu_height = SWING_ROW_HEIGHT * viewmodel::Swing::ALL.len() as f32 + 8.0;
    [
        x + width - 132.0,
        (y + height + 3.0).min(h - menu_height - 2.0).max(2.0),
        132.0,
        menu_height,
    ]
}

fn swing_picker_row([x, y, w, _]: [f32; 4], i: usize) -> [f32; 4] {
    [
        x + 4.0,
        y + 4.0 + SWING_ROW_HEIGHT * i as f32,
        w - 8.0,
        SWING_ROW_HEIGHT,
    ]
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
fn trim_legacy(atlas: &HudAtlas, text: &str, width: f32) -> String {
    let mut result = String::new();
    let mut formatting = false;
    for ch in text.chars() {
        let mut next = result.clone();
        next.push(ch);
        if !formatting && ch != '§' && HudBatch::text_width(atlas, &next) > width {
            break;
        }
        result = next;
        formatting = ch == '§' && !formatting;
    }
    result
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
            let cx = batch.width / 2.0;
            let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
            let width = (view.measure(Face::Medium, 9.0, &tip) + 16.0).min(view.w - 8.0);
            let x = cx - width / 2.0;
            view.rounded(
                x,
                9.0,
                width,
                17.0,
                8.5,
                color::with_alpha(color::bg(), 0.91),
            );
            let start = view.batch.vertices.len();
            view.text_centered(Face::Medium, 9.0, cx, 11.0, color::text(), &tip);
            view.batch
                .clip_since(start, x + 5.0, 9.0, x + width - 5.0, 26.0);
            view.finish();
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
            // Hand Y points up; keep the frame between the header and chat input.
            let pixels_per_unit = batch.height * 0.6;
            let dx = (hand.position[0] * pixels_per_unit).clamp(-x1, batch.width - x2);
            let dy = (-hand.position[1] * pixels_per_unit)
                .clamp((28.0 - y1).min(0.0), batch.height - 14.0 - y2);
            let (x1, y1, x2, y2) = (x1 + dx, y1 + dy, x2 + dx, y2 + dy);
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
        let rotation = editor.indicator_rotation;
        let main = if rotation {
            format!(
                "P: {:.0}°  Y: {:.0}°",
                editor.indicator_angles[0], editor.indicator_angles[1]
            )
        } else {
            format!("×{:.1}", editor.indicator_scale)
        };
        let (cx, y) = (batch.width / 2.0, batch.height - 85.0);
        let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
        let width = view.measure(Face::Mono, 10.0, &main) + 20.0;
        let x = cx - width / 2.0;
        view.rounded(
            x,
            y,
            width,
            20.0,
            10.0,
            color::with_alpha(color::bg(), alpha * 0.93),
        );
        view.border(
            x,
            y,
            width,
            20.0,
            10.0,
            color::with_alpha(color::text(), alpha * 0.23),
        );
        view.text_line(
            Face::Mono,
            10.0,
            20.0,
            x + 10.0,
            y,
            color::with_alpha(color::text(), alpha),
            &main,
        );
        view.finish();
    }
    fn actions_view_sliders(
        &self,
        batch: &HudBatch,
    ) -> Vec<([f32; 4], usize, &'static str, f32, f32, f32)> {
        let s = &self.atlas_ports.actions;
        let [x, y, ..] = view_panel(batch.width, batch.height);
        let x = x + 18.0;
        let y = y + 54.0 - s.ui.view_scroll;
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
        let [_, top, _, height] = view_panel(batch.width, batch.height);
        for (r, id, _, _, _, _) in self.actions_view_sliders(batch) {
            if hit(r, x, y) && y >= top + 54.0 && y < top + height - 6.0 {
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
                let norm = ((self.cursor.0 - (r[0] + 5.0)) / (r[2] - 10.0)).clamp(0.0, 1.0);
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
        let state = &self.atlas_ports.actions;
        let [x, y, width, height] = view_panel(batch.width, batch.height);
        let clip_bottom = y + height - 6.0;
        batch.fill(
            atlas,
            0.0,
            0.0,
            batch.width,
            batch.height,
            rgba(6, 8, 13, 165),
        );
        mod_style::panel(batch, atlas, [x, y, width, height], 17.0);
        let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
        view.text_line(
            Face::SemiBold,
            15.0,
            25.0,
            x + 15.0,
            y + 2.0,
            color::text(),
            &lang::get("atlas.viewmodel.title"),
        );
        view.rounded(x + 10.0, y + 26.0, 294.0, 24.0, 12.0, color::raised());
        if state.ui.view_tab == 0 {
            let track = height - 60.0;
            let max_scroll = hand_scroll_max(height);
            let thumb = track * track / (track + max_scroll);
            let fraction = (state.ui.view_scroll / max_scroll).clamp(0.0, 1.0);
            view.rounded(x + 373.0, y + 54.0, 2.0, track, 1.0, color::raised());
            view.rounded(
                x + 373.0,
                y + 54.0 + (track - thumb) * fraction,
                2.0,
                thumb,
                1.0,
                color::hover(),
            );
        }
        view.finish();

        let picking = state.ui.view_pick.is_some();
        for (rect, _, key, value, min, max) in self.actions_view_sliders(batch) {
            let [sx, sy, w, h] = rect;
            let track = [sx + 5.0, w - 10.0];
            let norm = ((value - min) / (max - min)).clamp(0.0, 1.0);
            let label = lang::get(key);
            let number = format!("{value:.2}");
            let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
            let start = view.batch.vertices.len();
            let extent = track[1] * norm;
            let thumb = track[0] + extent;
            view.rounded(sx, sy, w, h, 7.0, color::raised());
            view.rounded(track[0], sy + 14.0, track[1], 1.5, 1.0, color::hover());
            view.rounded(track[0], sy + 14.0, extent, 1.5, 1.0, color::accent());
            view.rounded(thumb - 2.0, sy + 13.0, 4.0, 4.0, 2.0, color::text());
            view.text_line(Face::Medium, 9.0, 13.0, sx + 9.0, sy, color::text(), &label);
            let number_w = view.measure(Face::Mono, 9.0, &number);
            view.text_line(
                Face::Mono,
                9.0,
                13.0,
                sx + w - number_w - 9.0,
                sy,
                color::muted(),
                &number,
            );
            view.batch
                .clip_since(start, x + 4.0, y + 54.0, x + 376.0, clip_bottom);
            view.finish();

            let top = sy.max(y + 54.0);
            let bottom = (sy + h).min(clip_bottom);
            if !picking && bottom > top {
                let hit_rect = [sx, top, w, bottom - top];
                let kind = widgets::Kind::Slider { value: norm, track };
                widgets::record_as(&format!("{label}: {number}"), hit_rect, true, kind);
                widgets::target_as(hit_rect, kind);
                widgets::narratable(|| {
                    widgets::Narratable::slider(label, hit_rect, true).hovered(hit(
                        hit_rect,
                        self.cursor.0,
                        self.cursor.1,
                    ))
                });
            }
        }

        let mut show_reset_hint = false;
        for control in self.actions_buttons(batch) {
            let [bx, by, bw, bh] = control.rect;
            let start = batch.vertices.len();
            let [top, bottom] = if view_content(&control.action) {
                [by.max(y + 54.0), (by + bh).min(clip_bottom)]
            } else {
                [by, by + bh]
            };
            if bottom <= top {
                continue;
            }
            let hit_rect = [bx, top, bw, bottom - top];
            let hover = !picking && hit(hit_rect, self.cursor.0, self.cursor.1);
            let mut label = control.label.plain();
            if matches!(control.action, Action::ViewSwing) {
                label = format!("{} {}", lang::get("atlas.viewmodel.swing_type"), label);
            }
            if !picking {
                widgets::record(&label, hit_rect, true);
                widgets::target(hit_rect);
                widgets::widget_cursor(hover, true);
                widgets::narratable(|| {
                    widgets::Narratable::button(label.clone(), hit_rect, true).hovered(hover)
                });
            }
            let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
            let on = match control.action {
                Action::ViewTab(i) => state.ui.view_tab == i,
                Action::ViewHand(main) => state.ui.view_main == main,
                Action::ViewToggle(i) => match i {
                    0 => state.viewmodel.interactive,
                    1 => state.viewmodel.outline,
                    2 => state.viewmodel.only_target,
                    _ => state.viewmodel.auto_swing,
                },
                _ => false,
            };
            let radius = (bh / 2.0).min(11.0);
            view.rounded(
                bx,
                by,
                bw,
                bh,
                radius,
                if hover || (on && !matches!(control.action, Action::ViewToggle(_))) {
                    color::hover()
                } else {
                    color::raised()
                },
            );
            if hover || (on && !matches!(control.action, Action::ViewToggle(_))) {
                view.border(
                    bx,
                    by,
                    bw,
                    bh,
                    radius,
                    if hover {
                        color::accent()
                    } else {
                        color::border()
                    },
                );
            }
            match control.action {
                Action::ViewToggle(_) => {
                    view.text_line(Face::Medium, 9.0, bh, bx + 9.0, by, color::text(), &label);
                    let sx = bx + bw - 29.0;
                    let sy = by + (bh - 11.0) / 2.0;
                    view.rounded(
                        sx,
                        sy,
                        22.0,
                        11.0,
                        5.5,
                        if on { color::accent() } else { color::hover() },
                    );
                    view.rounded(
                        sx + if on { 12.0 } else { 2.0 },
                        sy + 1.5,
                        8.0,
                        8.0,
                        4.0,
                        if on {
                            color::on_accent()
                        } else {
                            color::text()
                        },
                    );
                }
                Action::ViewSwing => {
                    let heading = lang::get("atlas.viewmodel.swing_type");
                    view.text_line(
                        Face::Medium,
                        10.0,
                        bh,
                        bx + 10.0,
                        by,
                        color::text(),
                        &heading,
                    );
                    let name = state.viewmodel.swing.display_name();
                    let name_w = view.measure(Face::SemiBold, 10.0, name);
                    let chip_x = bx + bw - name_w - 33.0;
                    view.rounded(
                        chip_x,
                        by + 4.0,
                        name_w + 26.0,
                        bh - 8.0,
                        8.0,
                        color::raised(),
                    );
                    view.text_line(
                        Face::SemiBold,
                        10.0,
                        bh,
                        chip_x + 7.0,
                        by,
                        color::text(),
                        name,
                    );
                    let cx = bx + bw - 15.0;
                    let cy = by + bh / 2.0;
                    view.batch.fill_triangle(
                        atlas,
                        [[cx - 2.5, cy - 1.0], [cx + 2.5, cy - 1.0], [cx, cy + 2.0]],
                        color::muted(),
                    );
                    show_reset_hint = hover;
                }
                _ => {
                    let size = if matches!(control.action, Action::Close) {
                        12.0
                    } else {
                        9.5
                    };
                    let width = view.measure(Face::Medium, size, &label);
                    view.text_line(
                        Face::Medium,
                        size,
                        bh,
                        bx + (bw - width) / 2.0,
                        by,
                        color::text(),
                        &label,
                    );
                }
            }
            view.batch.clip_since(start, bx, top, bx + bw, bottom);
            view.finish();
        }

        if let Some(focused) = state.ui.view_pick {
            let rect = swing_picker(batch.width, batch.height);
            let [mx, my, mw, mh] = rect;
            let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
            view.rounded(mx, my, mw, mh, 12.0, rgba(15, 19, 25, 255));
            view.border(mx, my, mw, mh, 12.0, color::hover());
            for (i, swing) in viewmodel::Swing::ALL.into_iter().enumerate() {
                let r @ [rx, ry, rw, rh] = swing_picker_row(rect, i);
                let hover = hit(r, self.cursor.0, self.cursor.1);
                let selected = swing == state.viewmodel.swing;
                let name = swing.display_name();
                widgets::record(name, r, true);
                widgets::target(r);
                widgets::widget_cursor(hover, true);
                widgets::narratable(|| widgets::Narratable::button(name, r, true).hovered(hover));
                if hover || selected || i == focused {
                    view.rounded(rx, ry, rw, rh, 6.0, color::hover());
                }
                view.text_line(Face::Medium, 9.5, rh, rx + 8.0, ry, color::text(), name);
                if selected {
                    view.rounded(rx + rw - 12.0, ry + 5.0, 4.0, 4.0, 2.0, color::accent());
                }
            }
            view.finish();
        } else if show_reset_hint {
            ui::tooltip(
                batch,
                atlas,
                &[tr("atlas.viewmodel.swing_reset_hint")],
                self.cursor,
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
                let [x, y, ..] = view_panel(w, h);
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
                    Text::literal("×"),
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
                            [x + 18.0, y + dy, 344.0, 16.0],
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
                        swing_selector(w, h),
                        Text::literal(s.viewmodel.swing.display_name()),
                        Action::ViewSwing,
                    ));
                    out.push(button(
                        [x + 18.0, y + 84.0, 344.0, 16.0],
                        tr("atlas.viewmodel.onlytarget"),
                        Action::ViewToggle(2),
                    ));
                    out.push(button(
                        [x + 18.0, y + 106.0, 344.0, 16.0],
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
        let raw = item.icon.trim().to_lowercase();
        let empty = raw.is_empty();
        let icon = raw;
        let icon = if icon.contains(':') {
            icon
        } else {
            format!("minecraft:{icon}")
        };
        let fallback = if item.children.is_empty() {
            if empty {
                "minecraft:command_block"
            } else {
                "minecraft:paper"
            }
        } else {
            "minecraft:chest"
        };
        let icon = if icon != "minecraft:air" && self.items().get(&icon).is_some() {
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
        self.actions_draw_viewmodel_chat(batch);
        let screen = self.native_mod_screen().map(str::to_owned);
        if screen.as_deref() == Some(IDS[7]) {
            self.actions_draw_viewmodel(batch);
            return;
        }
        self.atlas_inventory_draw(batch);
        let atlas = &self.gui.atlas;
        if screen.as_deref() == Some(IDS[2]) {
            let s = &self.atlas_ports.actions;
            let title = lang::get(if let Some(dialog) = &s.ui.hot_dialog {
                if dialog.index.is_none() {
                    "gui.hotcommands.editor.dialog_title_add"
                } else {
                    "gui.hotcommands.editor.dialog_title_edit"
                }
            } else if s.ui.hot_edit {
                "gui.hotcommands.editor.title"
            } else {
                "gui.hotcommands.title"
            });
            let category = s.hot_title();
            let title = if s.ui.hot_dialog.is_some() {
                title
            } else if category.is_empty() {
                format!("§{}§l{title}", if s.ui.hot_edit { 'e' } else { '6' })
            } else {
                format!("§7{title} §8▶ §f§l{category}")
            };
            center(
                batch,
                atlas,
                &title,
                batch.width / 2.0,
                if s.ui.hot_dialog.is_some() {
                    20.0
                } else if s.ui.hot_edit {
                    16.0
                } else {
                    14.0
                },
                u32::MAX,
            );
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
                    mod_style::panel(batch, atlas, [x, y, lw, 26.0], 8.0);
                    self.actions_icon(batch, item, x + 4.0, y + 5.0);
                    let name = if item.name.is_empty() {
                        lang::get("atlas.hotcommands.editor.unnamed")
                    } else {
                        item.name.clone()
                    };
                    let name = trim_legacy(atlas, &format!("§f{name}"), lw - 160.0);
                    batch.text(atlas, &name, x + 26.0, y + 3.0, u32::MAX, true);
                    let sub = if item.children.is_empty() {
                        if item.command.is_empty() {
                            lang::get("atlas.hotcommands.editor.empty")
                        } else {
                            format!("§b{}", item.command)
                        }
                    } else {
                        lang::get("atlas.hotcommands.editor.submenu")
                            .replace("%s", &item.children.len().to_string())
                    };
                    let sub = trim_legacy(atlas, &sub, lw - 160.0);
                    batch.text(atlas, &sub, x + 26.0, y + 14.0, 0xffaaaaaa, true);
                }
                let pages = s.current_commands().len().div_ceil(6).max(1);
                if pages > 1 {
                    center(
                        batch,
                        atlas,
                        &format!("{} / {pages}", s.ui.hot_page + 1),
                        batch.width / 2.0,
                        batch.height - 22.0,
                        0xffdddddd,
                    );
                }
            }
            if s.ui.hot_dialog.is_none() && s.current_commands().is_empty() {
                center(
                    batch,
                    atlas,
                    &lang::get("gui.hotcommands.empty"),
                    batch.width / 2.0,
                    batch.height / 2.0 - if s.ui.hot_edit { 0.0 } else { 10.0 },
                    if s.ui.hot_edit {
                        0xff888888
                    } else {
                        0xffaaaaaa
                    },
                );
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
        let mut tooltip: Option<Vec<Text>> = None;
        for b in self.actions_buttons(batch) {
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
                    Action::HotDelete(_) => {
                        tooltip = Some(vec![tr("gui.hotcommands.editor.delete")])
                    }
                    Action::HotEdit(_) => tooltip = Some(vec![tr("gui.hotcommands.editor.edit")]),
                    Action::HotChildren(i) => {
                        tooltip = Some(vec![Text::tr(
                            "gui.hotcommands.editor.subcommands",
                            &[&self.atlas_ports.actions.current_commands()[i]
                                .children
                                .len()],
                        )])
                    }
                    _ => {}
                }
            }
            let [x, y, w, h] = b.rect;
            if let Some(item) = b.custom {
                let atlas = &self.gui.atlas;
                let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
                view.rounded(
                    x,
                    y,
                    w,
                    h,
                    8.0,
                    if hover {
                        color::hover()
                    } else {
                        color::raised()
                    },
                );
                view.border(
                    x,
                    y,
                    w,
                    h,
                    8.0,
                    if hover {
                        color::accent()
                    } else {
                        color::border()
                    },
                );
                view.finish();
                self.actions_icon(batch, &item, x + 4.0, y + (h - 16.0) / 2.0);
                let atlas = &self.gui.atlas;
                let max = w - 36.0 - if item.children.is_empty() { 0.0 } else { 12.0 };
                let trimmed = trim_legacy(atlas, &item.name, max);
                let text = if trimmed != item.name {
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
            } else {
                mod_style::button(
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
                    true,
                );
                dialog.fields[i].x = x;
                dialog.fields[i].y = y;
                dialog.fields[i].draw(batch, &self.gui.atlas, None);
            }
        }
        if screen.as_deref() == Some(IDS[2])
            && let Some(dialog) = self.atlas_ports.actions.ui.hot_dialog.as_ref()
        {
            let x = (batch.width - 260.0) / 2.0;
            let raw = dialog.fields[2].value.trim().to_lowercase();
            let has_children = dialog
                .index
                .and_then(|i| self.atlas_ports.actions.current_commands().get(i))
                .is_some_and(|item| !item.children.is_empty());
            let icon = if raw.is_empty() {
                if has_children {
                    "minecraft:chest".into()
                } else {
                    "minecraft:command_block".into()
                }
            } else {
                let qualified = if raw.contains(':') {
                    raw
                } else {
                    format!("minecraft:{raw}")
                };
                if qualified != "minecraft:air" && self.items().get(&qualified).is_some() {
                    qualified
                } else {
                    "minecraft:barrier".into()
                }
            };
            let item = HotCommand::new("", "", &icon);
            batch.fill(
                &self.gui.atlas,
                x + 236.0,
                136.0,
                20.0,
                20.0,
                packed(-2011028958),
            );
            border(
                batch,
                &self.gui.atlas,
                [x + 236.0, 136.0, 20.0, 20.0],
                packed(-5592406),
            );
            self.actions_icon(batch, &item, x + 238.0, 138.0);
            batch.text(
                &self.gui.atlas,
                &lang::get("atlas.hotcommands.dialog.colors"),
                x,
                160.0,
                packed(-7829368),
                true,
            );
            batch.text(
                &self.gui.atlas,
                &lang::get("atlas.hotcommands.dialog.icons"),
                x,
                170.0,
                packed(-10066330),
                true,
            );
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
                self.atlas_ports.actions.ui.view_pick = None;
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
                s.ui.view_pick = None;
            }
            Action::ViewHand(main) => self.atlas_ports.actions.ui.view_main = main,
            Action::ViewSwing => {
                let s = &mut self.atlas_ports.actions;
                let i = viewmodel::Swing::ALL
                    .iter()
                    .position(|v| *v == s.viewmodel.swing)
                    .unwrap_or(0);
                s.ui.view_pick = Some(i);
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
                    1 => s.viewmodel.reset_hands(),
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
    fn actions_select_swing(&mut self, swing: viewmodel::Swing) {
        let state = &mut self.atlas_ports.actions;
        state.ui.view_pick = None;
        state.viewmodel.swing = swing;
        state.save_viewmodel();
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
        if self.native_mod_screen() == Some(IDS[7]) {
            let swing_tab = self.atlas_ports.actions.ui.view_tab == 1;
            if swing_tab && button == 1 && hit(swing_selector(batch.width, batch.height), x, y) {
                self.actions_select_swing(viewmodel::Swing::Standard);
                return true;
            }
            if self.atlas_ports.actions.ui.view_pick.is_some() {
                if button == 0 {
                    let menu = swing_picker(batch.width, batch.height);
                    self.atlas_ports.actions.ui.view_pick = None;
                    for (i, swing) in viewmodel::Swing::ALL.into_iter().enumerate() {
                        if hit(swing_picker_row(menu, i), x, y) {
                            self.actions_select_swing(swing);
                            break;
                        }
                    }
                }
                return true;
            }
            if button == 0 && self.actions_view_slider(&batch, x, y) {
                return true;
            }
        }
        let [_, panel_y, _, panel_h] = view_panel(batch.width, batch.height);
        for b in self.actions_buttons(&batch) {
            if self.native_mod_screen() == Some(IDS[7])
                && view_content(&b.action)
                && (y < panel_y + 54.0 || y >= panel_y + panel_h - 6.0)
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
            if !self.atlas_ports.actions.ui.hot_edit
                && atlas_client::keybinds::key_code(&self.native_mod_key(
                    IDS[2],
                    "open",
                    "key.keyboard.h",
                )) == Some(key)
            {
                self.actions_press(Action::Close);
            }
            return true;
        }
        if self.native_mod_screen() != Some(IDS[7]) {
            return false;
        }
        let state = &mut self.atlas_ports.actions;
        if let Some(i) = state.ui.view_pick {
            let count = viewmodel::Swing::ALL.len();
            match key {
                KeyCode::Escape => state.ui.view_pick = None,
                KeyCode::Enter | KeyCode::Space | KeyCode::NumpadEnter => {
                    self.actions_select_swing(viewmodel::Swing::ALL[i]);
                }
                KeyCode::ArrowUp => state.ui.view_pick = Some((i + count - 1) % count),
                KeyCode::ArrowDown => state.ui.view_pick = Some((i + 1) % count),
                KeyCode::Home => state.ui.view_pick = Some(0),
                KeyCode::End => state.ui.view_pick = Some(count - 1),
                _ => {}
            }
            return true;
        }
        if state.ui.view_tab == 1
            && matches!(key, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space)
        {
            self.actions_press(Action::ViewSwing);
            return true;
        }
        key != KeyCode::Escape
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
            let Some(renderer) = &self.renderer else {
                return true;
            };
            let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
            let [_, _, _, panel_height] = view_panel(batch.width, batch.height);
            let s = &mut self.atlas_ports.actions;
            let max = if s.ui.view_tab == 0 {
                hand_scroll_max(panel_height)
            } else {
                0.0
            };
            s.ui.view_scroll = (s.ui.view_scroll - (delta * 16.0).trunc()).clamp(0.0, max);
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
