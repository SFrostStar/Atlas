//! Account list and popups follow the IAS 9.0.7 layout.
use super::*;
use atlas_client::{
    clipboard::{self, Clipboard, EditShortcut},
    hud::{HudAtlas, HudBatch, rgba},
    text::Text,
    ui::{self, button},
    widgets,
};
use winit::keyboard::KeyCode;

type Rect = [f32; 4];
const ROW: f32 = 12.0;

#[derive(Clone, Copy)]
enum Click {
    Login(bool),
    Edit,
    Delete,
    Add,
    Close,
    Microsoft(bool),
    Offline(bool),
    Save(bool),
    Confirm,
    Dismiss,
}

#[derive(Clone, Copy)]
struct Layout {
    center: [f32; 2],
    list: Rect,
    search: Rect,
    nickname: Rect,
    shown: usize,
}

impl Layout {
    fn new(w: f32, h: f32) -> Self {
        let [cx, cy] = [(w / 2.0).floor(), (h / 2.0).floor()];
        let width = (w - 190.0).clamp(60.0, 220.0);
        let shown = ((h - 90.0).max(ROW) / ROW).floor() as usize;
        Self {
            center: [cx, cy],
            list: [cx - width / 2.0, 38.0, width, shown as f32 * ROW],
            search: [cx - 75.0, 11.0, 150.0, 20.0],
            nickname: [cx - 75.0, cy - 5.0, 148.0, 20.0],
            shown,
        }
    }

    fn row(self, index: usize) -> Rect {
        let [x, y, w, _] = self.list;
        [x, y + index as f32 * ROW, w, ROW]
    }

    fn footer(self) -> [(Click, Rect, &'static str); 6] {
        let [cx, cy] = self.center;
        let y = 2.0 * cy - 48.0;
        [
            (
                Click::Login(false),
                [cx - 154.0, y, 100.0, 20.0],
                "ias.accounts.login",
            ),
            (
                Click::Login(true),
                [cx - 154.0, y + 24.0, 100.0, 20.0],
                "ias.accounts.offlineLogin",
            ),
            (
                Click::Edit,
                [cx - 50.0, y, 100.0, 20.0],
                "ias.accounts.edit",
            ),
            (
                Click::Delete,
                [cx - 50.0, y + 24.0, 100.0, 20.0],
                "ias.accounts.delete",
            ),
            (Click::Add, [cx + 54.0, y, 100.0, 20.0], "ias.accounts.add"),
            (
                Click::Close,
                [cx + 54.0, y + 24.0, 100.0, 20.0],
                "gui.cancel",
            ),
        ]
    }

    fn popup_buttons(self, popup: Popup) -> Vec<(Click, Rect, &'static str)> {
        let [cx, cy] = self.center;
        let left = [cx - 75.0, cy + 27.0, 74.0, 20.0];
        let right = [cx + 1.0, cy + 27.0, 74.0, 20.0];
        let all = [cx - 75.0, cy + 27.0, 150.0, 20.0];
        match popup {
            Popup::Provider(edit) => vec![
                (
                    Click::Microsoft(edit),
                    [cx - 75.0, cy - 24.0, 150.0, 20.0],
                    "ias.add.microsoft",
                ),
                (
                    Click::Offline(edit),
                    [cx - 75.0, cy, 150.0, 20.0],
                    "ias.add.offline",
                ),
                (Click::Dismiss, all, "gui.cancel"),
            ],
            Popup::Offline(edit) => vec![
                (Click::Save(edit), left, "gui.done"),
                (Click::Dismiss, right, "gui.cancel"),
            ],
            Popup::Delete => vec![
                (Click::Confirm, left, "ias.delete"),
                (Click::Dismiss, right, "gui.cancel"),
            ],
            Popup::Microsoft | Popup::Login => vec![(Click::Dismiss, all, "gui.cancel")],
        }
    }
}

fn hit([x, y, w, h]: Rect, cursor: (f32, f32)) -> bool {
    cursor.0 >= x && cursor.0 < x + w && cursor.1 >= y && cursor.1 < y + h
}

fn arrow_rect([x, y, w, h]: Rect, direction: i8) -> Rect {
    let inset = if direction < 0 { 28.0 } else { 15.0 };
    [x + w - inset, y, 11.0, h]
}

fn badge(mode: &str, w: f32, h: f32) -> Rect {
    let x = w / 2.0;
    if mode == "title" {
        [
            (x + 104.0).min(w - 22.0).floor(),
            (h / 4.0 + 72.0).floor(),
            20.0,
            20.0,
        ]
    } else {
        [(x + 158.0).min(w - 22.0).floor(), h - 30.0, 20.0, 20.0]
    }
}

fn set_rect(field: &mut EditBox, [x, y, w, h]: Rect) {
    (field.x, field.y, field.width, field.height) = (x, y, w, h);
}

fn center(batch: &mut HudBatch, atlas: &HudAtlas, label: &Text, x: f32, y: f32, tint: u32) {
    batch.text_styled(
        atlas,
        label,
        (x - atlas.font.width_text(label) / 2.0).floor(),
        y,
        tint,
        true,
    );
}

fn skin_piece(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    name: &str,
    [x, y, w, h]: Rect,
    [u, v, sw, sh]: Rect,
) {
    if let Some(mut tex) = atlas.sprites.get(name).copied() {
        let du = (tex.u1 - tex.u0) / 64.0;
        let dv = (tex.v1 - tex.v0) / 64.0;
        tex.u0 += u * du;
        tex.v0 += v * dv;
        tex.u1 = tex.u0 + sw * du;
        tex.v1 = tex.v0 + sh * dv;
        batch.rect_uv(x, y, w, h, tex, u32::MAX);
    }
}

fn draw_model(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    resources: &super::super::super::Resources,
    skin: &str,
    cy: f32,
    cursor: (f32, f32),
) {
    use super::super::super::{
        Mob, Vec3,
        perspective::{emit_preview, player_preview_quads},
    };
    let Some(kind) = resources.player_type().clone() else {
        return;
    };
    let cx = 47.5;
    let yaw = ((cx - cursor.0) / 40.0).atan();
    let pitch = ((cy - cursor.1) / 40.0).atan();
    let mut model = Mob::new(0, kind, Vec3::ZERO, 180.0);
    model.skin = Some(skin.to_owned());
    model.slim = skin.contains("/slim/");
    model.body_yaw = 180.0 + yaw * 20.0;
    model.previous_body_yaw = model.body_yaw;
    model.head_yaw = model.body_yaw + yaw * 20.0;
    model.previous_head_yaw = model.head_yaw;
    model.head_pitch = -pitch * 20.0;
    model.previous_head_pitch = model.head_pitch;
    let lookup = |name: &str| atlas.sprites.get(name).map(|r| [r.u0, r.v0, r.u1, r.v1]);
    let mut vertices = Vec::new();
    atlas_client::mobs::render::render_with(
        &model,
        1.0,
        glam::DVec3::ZERO,
        &lookup,
        240 | 240 << 8,
        resources.mob_data(),
        &[],
        &mut vertices,
    );
    let size = 54.0;
    let projection = glam::Mat4::from_translation(glam::Vec3::new(cx, cy, 0.0))
        * glam::Mat4::from_scale(glam::Vec3::new(size, size, -size))
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.9, 0.0))
        * glam::Mat4::from_rotation_z(std::f32::consts::PI)
        * glam::Mat4::from_rotation_x((pitch * 20.0).to_radians());
    let quads = player_preview_quads(vertices, projection, None, &atlas.sprites);
    emit_preview(batch, quads, Vec::new(), [5.0, cy - 60.0, 90.0, cy + 60.0]);
}

fn preview_skin<'a>(batch: &mut HudBatch, atlas: &'a HudAtlas, account: &Account) -> &'a str {
    let lookup = format!("name:{}", account.name.to_ascii_lowercase());
    batch
        .inline_profiles
        .push(serde_json::Value::String(account.name.clone()));
    atlas
        .object_skins
        .get(&lookup)
        .map(String::as_str)
        .unwrap_or_else(|| atlas_client::tab_list::default_skin(account.uuid()))
}

impl State {
    fn click_active(&self, click: Click) -> bool {
        if self.pending.is_some() {
            return false;
        }
        match click {
            Click::Login(false) => self.selection().is_some_and(Account::online),
            Click::Login(true) | Click::Edit | Click::Delete | Click::Confirm => {
                self.selection().is_some()
            }
            Click::Save(_) => valid_name(&self.nickname.value),
            Click::Microsoft(_) => {
                self.catalog.microsoft.configured && !self.catalog.microsoft.active
            }
            _ => true,
        }
    }

    fn move_selection(&mut self, direction: i8) {
        if let Some(account) = self.selection() {
            let accounts = &self.catalog.accounts;
            let Some(at) = accounts.iter().position(|item| item.id == account.id) else {
                return;
            };
            if (at as isize + direction as isize) < 0
                || (at as isize + direction as isize) >= accounts.len() as isize
            {
                return;
            }
            let id = account.id.clone();
            self.request(Request {
                action: Some(Action::Move),
                account_id: Some(&id),
                direction: Some(direction),
                ..Default::default()
            });
        }
    }
}

impl Game<'_> {
    fn ias_layout(&self) -> Option<(Layout, f32, f32)> {
        let r = self.renderer.as_ref()?;
        let batch = HudBatch::new([r.config.width, r.config.height]);
        Some((
            Layout::new(batch.width, batch.height),
            batch.width,
            batch.height,
        ))
    }

    fn ias_open(&mut self) {
        if !self.atlas_open_port_screen(ID) {
            return;
        }
        self.native_mod_screen = Some(ACCOUNT_SCREEN.to_owned());
        let state = &mut self.atlas_ports.ias;
        state.refresh();
        state.search.set_value("");
        state.search.focused = false;
        state.selected = None;
        state.scroll = 0;
        state.popup = None;
        state.notice = None;
        state.last_click = None;
    }

    pub(in crate::game) fn atlas_ias_draw_badge(&mut self, batch: &mut HudBatch) {
        let Some(mode) = self.ias_opener() else {
            return;
        };
        self.atlas_ias_poll();
        let state = &self.atlas_ports.ias;
        let atlas = &self.gui.atlas;
        if state.config.button(mode) {
            let rect @ [x, y, w, h] = badge(mode, batch.width, batch.height);
            let hover = hit(rect, self.cursor);
            let message = lang::get("atlas.ias.open");
            widgets::record(&message, rect, true);
            widgets::target(rect);
            widgets::widget_cursor(hover, true);
            widgets::narratable(|| {
                widgets::Narratable::button(message.clone(), rect, true).hovered(hover)
            });
            batch.sprite_scaled(
                atlas,
                if hover {
                    "atlas/ias/button_focus"
                } else {
                    "atlas/ias/button_plain"
                },
                x,
                y,
                w,
                h,
                u32::MAX,
            );
            if hover {
                ui::tooltip(batch, atlas, &[Text::literal(&message)], self.cursor);
            }
        }

        let current = state
            .catalog
            .accounts
            .iter()
            .find(|a| a.id == state.catalog.current_id)
            .map(|a| a.name.as_str())
            .or(self.options.player_name.as_deref());
        if state.config.text(mode)
            && let Some(name) = current
        {
            let ty = if mode == "title" {
                (batch.height / 4.0 + 164.0).floor()
            } else {
                15.0
            };
            center(
                batch,
                atlas,
                &Text::tr("ias.title", &[&name]),
                batch.width / 2.0,
                ty,
                u32::MAX,
            );
        }
    }

    pub(in crate::game) fn atlas_ias_draw(&mut self, batch: &mut HudBatch) {
        if !self.atlas_ias_active() {
            return;
        }
        self.atlas_ias_poll();
        if !self.atlas_ias_active() {
            return;
        }
        self.flush_skin_slots();
        let layout = Layout::new(batch.width, batch.height);
        let state = &mut self.atlas_ports.ias;
        let atlas = &self.gui.atlas;
        let [cx, cy] = layout.center;
        let popup = state.popup;
        let now = Instant::now();
        let label = Text::tr("ias.accounts", &[]);
        ui::menu_background(
            batch,
            atlas,
            0.0,
            0.0,
            batch.width,
            batch.height,
            self.title.is_none(),
        );
        widgets::begin_screen(|| widgets::ScreenInfo::new("mod", label.plain()));
        center(batch, atlas, &label, cx, 1.0, u32::MAX);
        set_rect(&mut state.search, layout.search);
        state.search.message = lang::get("ias.accounts.search");
        state.search.suggestion = state
            .search
            .value
            .is_empty()
            .then(|| state.search.message.clone());
        state.search.draw(batch, atlas, None);

        let matching = state.matching();
        let scroll = state
            .scroll
            .min(matching.len().saturating_sub(layout.shown));
        let rows = &matching[scroll..];
        let mut hover_id = None;
        let mut tooltip = None;
        let [lx, ly, lw, lh] = layout.list;
        batch.fill(atlas, lx, ly, lw, lh, rgba(0, 0, 0, 80));
        for (i, a) in rows.iter().take(layout.shown).enumerate() {
            let rect @ [x, y, w, h] = layout.row(i);
            let hovering = popup.is_none() && hit(rect, self.cursor);
            let selected = state.selected.as_deref() == Some(a.id.as_str());
            if hovering {
                hover_id = Some(a.id.clone());
                if state.hover.as_ref().is_some_and(|(id, at)| {
                    id == &a.id && now.duration_since(*at) >= Duration::from_millis(500)
                }) {
                    let kind = lang::get(if a.online() {
                        "ias.accounts.tip.type.microsoft"
                    } else {
                        "ias.accounts.tip.type.offline"
                    });
                    let uuid = a.uuid_text();
                    tooltip = Some(vec![
                        Text::literal(format!(
                            "{}: {}",
                            lang::get("ias.accounts.tip.nick"),
                            a.name
                        )),
                        Text::literal(format!("{}: {uuid}", lang::get("ias.accounts.tip.uuid"))),
                        Text::literal(format!("{}: {kind}", lang::get("ias.accounts.tip.type"))),
                    ]);
                }
            }
            if selected || hovering {
                batch.fill(
                    atlas,
                    x,
                    y,
                    w,
                    h,
                    rgba(255, 255, 255, if selected { 132 } else { 60 }),
                );
                batch.fill(
                    atlas,
                    x + 1.0,
                    y + 1.0,
                    w - 2.0,
                    h - 2.0,
                    rgba(0, 0, 0, 192),
                );
            }
            if popup.is_none() {
                widgets::record_as(&a.name, rect, true, widgets::Kind::Row);
                widgets::target_as(rect, widgets::Kind::Row);
            }
            let skin = preview_skin(batch, atlas, a);
            skin_piece(
                batch,
                atlas,
                skin,
                [x + 3.0, y + 2.0, 8.0, 8.0],
                [8.0, 8.0, 8.0, 8.0],
            );
            skin_piece(
                batch,
                atlas,
                skin,
                [x + 3.0, y + 2.0, 8.0, 8.0],
                [40.0, 8.0, 8.0, 8.0],
            );
            let start = batch.vertices.len();
            let color = if state.catalog.current_id == a.id {
                rgba(0, 255, 0, 255)
            } else {
                u32::MAX
            };
            batch.text(atlas, &a.name, x + 13.0, y + 2.0, color, false);
            batch.clip_since(start, x + 12.0, y, x + w - 30.0, y + h);
            if hovering || selected {
                let position = state
                    .catalog
                    .accounts
                    .iter()
                    .position(|item| item.id == a.id)
                    .unwrap_or(0);
                for direction in [-1, 1] {
                    let rect @ [ax, ay, aw, _] = arrow_rect(rect, direction);
                    let enabled = if direction < 0 {
                        position != 0
                    } else {
                        position + 1 < state.catalog.accounts.len()
                    };
                    let key = match (direction, enabled, hit(rect, self.cursor)) {
                        (-1, false, _) => "atlas/ias/up_disabled",
                        (-1, true, true) => "atlas/ias/up_focus",
                        (-1, true, false) => "atlas/ias/up_plain",
                        (_, false, _) => "atlas/ias/down_disabled",
                        (_, true, true) => "atlas/ias/down_focus",
                        (_, true, false) => "atlas/ias/down_plain",
                    };
                    batch.sprite_scaled(atlas, key, ax, ay + 2.0, aw, 7.0, u32::MAX);
                }
            }
        }
        let excess = matching.len().saturating_sub(layout.shown);
        if excess > 0 {
            let track_x = lx + lw + 1.0;
            let thumb = (lh * layout.shown as f32 / matching.len() as f32).clamp(8.0, lh);
            let top = ly + (lh - thumb) * (scroll as f32 / excess as f32);
            batch.fill(atlas, track_x, ly, 4.0, lh, rgba(0, 0, 0, 220));
            batch.fill(atlas, track_x, top, 4.0, thumb, rgba(168, 168, 168, 255));
            widgets::scroll_area(layout.list, scroll as f32, excess as f32);
        }
        if let Some(account) = state.selection() {
            let skin = preview_skin(batch, atlas, account);
            draw_model(batch, atlas, &self.res, skin, cy, self.cursor);
        }
        if state.hover.as_ref().map(|(id, _)| id) != hover_id.as_ref() {
            state.hover = hover_id.map(|id| (id, now));
        }
        for (click, rect @ [x, y, w, h], key) in layout.footer() {
            button(
                batch,
                atlas,
                x,
                y,
                w,
                h,
                &Text::tr(key, &[]),
                popup.is_none() && state.click_active(click),
                popup.is_none() && hit(rect, self.cursor),
            );
        }
        if let Some(popup) = popup {
            let title = Text::tr(
                match popup {
                    Popup::Provider(false) => "ias.add",
                    Popup::Provider(true) => "ias.edit",
                    Popup::Offline(_) => "ias.offline",
                    Popup::Delete => "ias.delete",
                    Popup::Microsoft => "ias.add.microsoft",
                    Popup::Login => "ias.login",
                },
                &[],
            );
            widgets::begin_screen(|| widgets::ScreenInfo::new("mod", title.plain()));
            batch.fill(
                atlas,
                0.0,
                0.0,
                batch.width,
                batch.height,
                rgba(0, 0, 0, 128),
            );
            batch.fill(
                atlas,
                cx - 80.0,
                cy - 50.0,
                160.0,
                100.0,
                rgba(32, 32, 32, 248),
            );
            batch.fill(
                atlas,
                cx - 79.0,
                cy - 51.0,
                158.0,
                1.0,
                rgba(32, 32, 32, 248),
            );
            batch.fill(
                atlas,
                cx - 79.0,
                cy + 50.0,
                158.0,
                1.0,
                rgba(32, 32, 32, 248),
            );
            let start = batch.vertices.len();
            center(batch, atlas, &title, cx, cy - 48.0, u32::MAX);
            batch.scale_since(start, cx, cy - 48.0, 2.0, 2.0);
            match popup {
                Popup::Offline(_) => {
                    set_rect(&mut state.nickname, layout.nickname);
                    state.nickname.message = lang::get("ias.offline.nick");
                    state.nickname.suggestion =
                        state.nickname.value.is_empty().then(|| "Steve".to_owned());
                    center(
                        batch,
                        atlas,
                        &Text::tr("ias.offline.nick", &[]),
                        cx,
                        cy - 15.0,
                        u32::MAX,
                    );
                    state.nickname.draw(batch, atlas, None);
                }
                Popup::Delete => {
                    if let Some(a) = state.selection() {
                        let prompt = Text::tr("ias.delete.confirm", &[&a.name]);
                        for (i, line) in atlas.font.wrap_components(&prompt, 150).iter().enumerate()
                        {
                            center(
                                batch,
                                atlas,
                                line,
                                cx,
                                cy - 15.0 + 10.0 * i as f32,
                                u32::MAX,
                            );
                        }
                    }
                }
                Popup::Microsoft => {
                    let text = if let Some(error) = state.catalog.microsoft.error.as_deref() {
                        Text::literal(lang::get_or(error, error))
                    } else if let Some(code) = state.catalog.microsoft.user_code.as_deref() {
                        Text::tr("atlas.ias.microsoft_code", &[&code])
                    } else if state.catalog.microsoft.active || state.pending.is_some() {
                        Text::tr("ias.login.initializing", &[])
                    } else {
                        Text::tr("ias.login.done", &[])
                    };
                    for (i, line) in atlas.font.wrap_components(&text, 150).iter().enumerate() {
                        center(
                            batch,
                            atlas,
                            line,
                            cx,
                            cy - 16.0 + 10.0 * i as f32,
                            u32::MAX,
                        );
                    }
                }
                Popup::Login => {
                    if state.pending.is_some() {
                        center(
                            batch,
                            atlas,
                            &Text::tr("ias.login.initializing", &[]),
                            cx,
                            cy - 6.0,
                            u32::MAX,
                        );
                    }
                }
                Popup::Provider(_) => {}
            }
            for (click, rect @ [x, y, w, h], key) in layout.popup_buttons(popup) {
                let active = state.click_active(click);
                let hover = hit(rect, self.cursor);
                button(batch, atlas, x, y, w, h, &Text::tr(key, &[]), active, hover);
                if hover
                    && matches!(click, Click::Microsoft(_))
                    && !state.catalog.microsoft.configured
                {
                    tooltip = Some(vec![Text::tr("atlas.ias.microsoft_unconfigured", &[])]);
                } else if hover
                    && matches!(click, Click::Save(_))
                    && !valid_name(&state.nickname.value)
                {
                    tooltip = Some(vec![Text::tr("atlas.ias.invalid_name", &[])]);
                }
            }
        }
        if let Some(notice) = &state.notice {
            let color = rgba(255, 164, 164, 255);
            center(
                batch,
                atlas,
                &Text::literal(notice),
                cx,
                batch.height - 60.0,
                color,
            );
        }
        if let Some(lines) = tooltip {
            ui::tooltip(batch, atlas, &lines, self.cursor);
        }
    }

    fn ias_click(&mut self, click: Click) {
        let state = &mut self.atlas_ports.ias;
        if !state.click_active(click) {
            return;
        }
        let selection = state.selection().cloned();
        let selected = selection.as_ref().map(|a| a.id.as_str());
        match click {
            Click::Login(offline) => {
                if state.request(Request {
                    action: Some(Action::Switch),
                    account_id: selected,
                    offline: Some(offline),
                    ..Default::default()
                }) {
                    state.popup = Some(Popup::Login);
                }
            }
            Click::Edit => state.popup = Some(Popup::Provider(true)),
            Click::Add => state.popup = Some(Popup::Provider(false)),
            Click::Delete | Click::Confirm => {
                if matches!(click, Click::Confirm) {
                    state.request(Request {
                        action: Some(Action::Delete),
                        account_id: selected,
                        ..Default::default()
                    });
                } else {
                    state.popup = Some(Popup::Delete);
                }
            }
            Click::Microsoft(edit) => {
                if state.request(Request {
                    action: Some(Action::Microsoft),
                    account_id: if edit { selected } else { None },
                    ..Default::default()
                }) {
                    state.popup = Some(Popup::Microsoft);
                }
            }
            Click::Offline(edit) => {
                let old = if edit {
                    selection.as_ref().map_or("", |a| a.name.as_str())
                } else {
                    ""
                };
                state.nickname.set_value(old);
                state.nickname.focused = true;
                state.popup = Some(Popup::Offline(edit));
            }
            Click::Save(edit) => {
                let name = state.nickname.value.clone();
                state.request(Request {
                    action: Some(if edit { Action::Edit } else { Action::Add }),
                    account_id: if edit { selected } else { None },
                    name: Some(&name),
                    ..Default::default()
                });
            }
            Click::Dismiss => {
                if matches!(state.popup, Some(Popup::Microsoft)) && state.catalog.microsoft.active {
                    state.request(Request {
                        action: Some(Action::CancelMicrosoft),
                        ..Default::default()
                    });
                } else {
                    state.popup = None;
                    state.notice = None;
                }
            }
            Click::Close => self.native_mod_screen_close(ACCOUNT_SCREEN),
        }
    }

    pub(in crate::game) fn atlas_ias_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if !pressed {
            return button == 0 && std::mem::take(&mut self.atlas_ports.ias.swallow_release);
        }
        if !self.atlas_ias_active() {
            let Some(mode) = self.ias_opener() else {
                return false;
            };
            if let Some((_, w, h)) = self.ias_layout()
                && button == 0
                && self.atlas_ports.ias.config.button(mode)
                && hit(badge(mode, w, h), self.cursor)
            {
                self.atlas_ports.ias.swallow_release = true;
                self.ias_open();
                return true;
            }
            return false;
        }
        if button != 0 {
            return true;
        }
        let Some((layout, _, _)) = self.ias_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.ias;
        state.swallow_release = true;
        if let Some(popup) = state.popup {
            if let Some((click, _, _)) = layout
                .popup_buttons(popup)
                .into_iter()
                .find(|(_, rect, _)| hit(*rect, self.cursor))
            {
                self.ias_click(click);
            } else if matches!(popup, Popup::Offline(_)) {
                state.nickname.focused = hit(layout.nickname, self.cursor);
                if state.nickname.focused {
                    state
                        .nickname
                        .click(&self.gui.atlas, self.cursor.0, false, false);
                }
            }
            return true;
        }
        state.search.focused = hit(layout.search, self.cursor);
        if state.search.focused {
            state
                .search
                .click(&self.gui.atlas, self.cursor.0, false, false);
            return true;
        }
        if let Some((click, _, _)) = layout
            .footer()
            .into_iter()
            .find(|(_, rect, _)| hit(*rect, self.cursor))
        {
            self.ias_click(click);
            return true;
        }
        if hit(layout.list, self.cursor) {
            let index = ((self.cursor.1 - layout.list[1]) / ROW) as usize;
            let rows = state.matching();
            let scroll = state.scroll.min(rows.len().saturating_sub(layout.shown));
            if let Some(a) = rows.get(scroll + index) {
                let id = a.id.clone();
                let rect = layout.row(index);
                let now = Instant::now();
                let double_click = state.last_click.as_ref().is_some_and(|(last, at)| {
                    last == &id && now.duration_since(*at) <= Duration::from_millis(250)
                });
                let offline = !a.online() || self.modifiers.shift_key();
                state.selected = Some(id.clone());
                state.last_click = Some((id, now));
                if hit(arrow_rect(rect, -1), self.cursor) {
                    state.move_selection(-1);
                } else if hit(arrow_rect(rect, 1), self.cursor) {
                    state.move_selection(1);
                } else if double_click {
                    self.ias_click(Click::Login(offline));
                }
            }
        }
        true
    }

    pub(in crate::game) fn atlas_ias_key(&mut self, key: KeyCode, repeat: bool) -> bool {
        if !self.atlas_ias_active() {
            if key == KeyCode::F7 && !repeat && self.ias_opener().is_some() {
                self.ias_open();
                return true;
            }
            return false;
        }
        let ctrl = self.has_control_down_with_quirk();
        let shift = self.modifiers.shift_key();
        let layout = self.ias_layout().map(|(layout, _, _)| layout);
        let state = &mut self.atlas_ports.ias;
        if ctrl {
            if key == KeyCode::KeyC
                && state.popup.is_none()
                && let Some(account) = state.selection()
            {
                let value = if shift {
                    account.uuid_text()
                } else {
                    account.name.clone()
                };
                self.clipboard.set(&value);
                return true;
            }
            let shortcut = match key {
                KeyCode::KeyA => Some(EditShortcut::SelectAll),
                KeyCode::KeyC => Some(EditShortcut::Copy),
                KeyCode::KeyV => Some(EditShortcut::Paste),
                KeyCode::KeyX => Some(EditShortcut::Cut),
                _ => None,
            };
            if let Some(shortcut) = shortcut
                && let Some(field) = state.field()
            {
                let changed = clipboard::apply(field, shortcut, &mut self.clipboard);
                if changed && state.popup.is_none() {
                    state.update_search();
                }
                return true;
            }
        }
        if let Some(popup) = state.popup {
            if key == KeyCode::Escape {
                self.ias_click(Click::Dismiss);
            } else if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter) {
                match popup {
                    Popup::Offline(edit) => self.ias_click(Click::Save(edit)),
                    Popup::Delete => self.ias_click(Click::Confirm),
                    _ => {}
                }
            } else if let Some(field) = state.field() {
                field.key(key, ctrl, shift);
            }
            return true;
        }
        if key == KeyCode::Escape {
            self.native_mod_screen_close(ACCOUNT_SCREEN);
            return true;
        }
        if key == KeyCode::Tab {
            state.search.focused = !state.search.focused;
            return true;
        }
        if let Some(edit) = state.field()
            && edit.key(key, ctrl, shift)
        {
            state.update_search();
            return true;
        }
        match key {
            KeyCode::KeyN if ctrl => self.ias_click(Click::Add),
            KeyCode::KeyR if ctrl => self.ias_click(Click::Edit),
            KeyCode::NumpadAdd => self.ias_click(Click::Add),
            KeyCode::NumpadSubtract => self.ias_click(Click::Edit),
            KeyCode::Delete | KeyCode::NumpadDecimal => self.ias_click(Click::Delete),
            KeyCode::PageUp => state.move_selection(-1),
            KeyCode::PageDown => state.move_selection(1),
            KeyCode::ArrowUp | KeyCode::ArrowDown if shift => {
                state.move_selection(if key == KeyCode::ArrowUp { -1 } else { 1 })
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                let rows = state.matching();
                let current = rows
                    .iter()
                    .position(|a| state.selected.as_deref() == Some(a.id.as_str()));
                let index = current.map_or(0, |i| {
                    if key == KeyCode::ArrowUp {
                        i.saturating_sub(1)
                    } else {
                        (i + 1).min(rows.len().saturating_sub(1))
                    }
                });
                state.selected = rows.get(index).map(|a| a.id.clone());
                if let Some(layout) = layout {
                    state.scroll = state
                        .scroll
                        .min(index)
                        .max((index + 1).saturating_sub(layout.shown));
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let offline = shift || state.selection().is_none_or(|a| !a.online());
                self.ias_click(Click::Login(offline));
            }
            _ => {}
        }
        true
    }

    pub(in crate::game) fn atlas_ias_text(&mut self, text: &str) -> bool {
        if !self.atlas_ias_active() {
            return false;
        }
        let state = &mut self.atlas_ports.ias;
        if let Some(field) = state.field() {
            field.insert_text(text);
            if state.popup.is_none() {
                state.update_search();
            }
        }
        true
    }

    pub(in crate::game) fn atlas_ias_scroll(&mut self, delta: f32) -> bool {
        if !self.atlas_ias_active() {
            return false;
        }
        let Some((layout, _, _)) = self.ias_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.ias;
        if state.popup.is_none() && hit(layout.list, self.cursor) {
            let last = state.matching().len().saturating_sub(layout.shown);
            state.scroll = state
                .scroll
                .saturating_add_signed((delta * -2.0).round() as isize)
                .min(last);
        }
        true
    }
}
