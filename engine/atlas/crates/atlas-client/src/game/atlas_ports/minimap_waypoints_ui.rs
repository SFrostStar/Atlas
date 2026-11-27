use super::{COLORS, Game, ID, MAX_NAME, MAX_POINTS, MAX_SYMBOL, Waypoint, label};
use atlas_client::{
    atlas::mod_style,
    clipboard::EditShortcut,
    hud::HudBatch,
    lang,
    text::Text,
    ui::{self, EditBox},
    ui_theme::{Ui, color},
    widgets,
};
use winit::{event::MouseButton, keyboard::KeyCode};

type Rect = [f32; 4];

pub(super) struct Screen {
    scope: String,
    page: Page,
    problem: Option<String>,
}

enum Page {
    List(List),
    Edit(Box<Editor>),
    Delete(List, usize),
}

struct List {
    search: EditBox,
    selected: Option<usize>,
    offset: usize,
}

impl List {
    fn new() -> Self {
        Self {
            search: field("search", "", MAX_NAME),
            selected: None,
            offset: 0,
        }
    }

    fn matching(&self, points: &[Waypoint]) -> Vec<usize> {
        let search = self.search.value.trim().to_lowercase();
        points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| {
                (point.display_name().to_lowercase().contains(&search)
                    || point.symbol.to_lowercase().contains(&search))
                .then_some(index)
            })
            .collect()
    }
}

struct Editor {
    return_to: Option<List>,
    index: Option<usize>,
    draft: Waypoint,
    fields: [EditBox; 5],
}

impl Editor {
    fn new(return_to: Option<List>, index: Option<usize>, draft: Waypoint) -> Self {
        let mut fields = [
            field("name", &draft.display_name(), MAX_NAME),
            field("symbol", &draft.symbol, MAX_SYMBOL),
            field("x", &draft.position[0].to_string(), 9),
            field("y", &draft.position[1].to_string(), 9),
            field("z", &draft.position[2].to_string(), 9),
        ];
        fields[0].editable = !draft.death;
        fields[1].editable = !draft.death;
        fields[if draft.death { 2 } else { 0 }].focused = true;
        Self {
            return_to,
            index,
            draft,
            fields,
        }
    }

    fn candidate(&self) -> Option<Waypoint> {
        let [name, symbol, x, y, z] = &self.fields;
        let name = name.value.trim();
        let symbol = symbol.value.trim();
        let marker = Waypoint {
            name: name.into(),
            symbol: if symbol.is_empty() {
                name.chars()
                    .next()
                    .map(|ch| ch.to_string())
                    .unwrap_or_default()
            } else {
                symbol.into()
            },
            position: [
                x.value.parse().ok()?,
                y.value.parse().ok()?,
                z.value.parse().ok()?,
            ],
            ..self.draft.clone()
        };
        marker.valid().then_some(marker)
    }
}

fn field(key: &str, value: &str, max_length: usize) -> EditBox {
    let mut field = EditBox::new(0.0, 0.0, 1.0, 20.0);
    field.max_length = max_length;
    field.message = label(key);
    field.set_value(value);
    field.focused = false;
    field
}

impl Page {
    fn title(&self) -> String {
        label(match self {
            Self::List(_) => "title",
            Self::Edit(editor) if editor.index.is_some() => "edit",
            Self::Edit(_) => "create",
            Self::Delete(..) => "confirm_title",
        })
    }

    fn focused(&mut self) -> Option<&mut EditBox> {
        match self {
            Self::List(list) if list.search.focused => Some(&mut list.search),
            Self::Edit(editor) => editor.fields.iter_mut().find(|field| field.focused),
            _ => None,
        }
    }

    fn fields(&mut self, layout: Layout) -> Vec<(&mut EditBox, Rect)> {
        match self {
            Self::List(list) => vec![(&mut list.search, layout.cell(29.0, 0, 1))],
            Self::Edit(editor) => {
                let mut upper = layout.cell(35.0, 0, 1);
                upper[2] = layout.width * 0.66 - 10.0;
                let name_right = upper[0] + upper[2] + 5.0;
                let symbol_width = layout.x + layout.width - 10.0 - name_right;
                let [name, symbol, x, y, z] = &mut editor.fields;
                let mut fields = vec![
                    (name, upper),
                    (symbol, [name_right, upper[1], symbol_width, upper[3]]),
                    (x, layout.cell(65.0, 0, 3)),
                    (y, layout.cell(65.0, 1, 3)),
                    (z, layout.cell(65.0, 2, 3)),
                ];
                if editor.draft.death {
                    fields.remove(1);
                    fields[0].1 = layout.cell(35.0, 0, 1);
                }
                fields
            }
            Self::Delete(..) => Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct Layout {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Layout {
    fn new(batch: &HudBatch) -> Self {
        let width = (batch.width - 16.0).clamp(12.0, 400.0);
        let height = (batch.height - 16.0).clamp(12.0, 248.0);
        Self {
            x: ((batch.width - width) * 0.5).floor(),
            y: ((batch.height - height) * 0.5).floor(),
            width,
            height,
        }
    }

    fn cell(self, y: f32, i: usize, n: usize) -> Rect {
        let w = ((self.width - 20.0) - (n - 1) as f32 * 5.0) / n as f32;
        [
            self.x + 10.0 + i as f32 * (w + 5.0),
            self.y + y,
            w.max(1.0),
            20.0,
        ]
    }

    fn rows(self) -> usize {
        (((self.height - 117.0) / 23.0).floor() as usize).max(1)
    }

    fn footer(self, i: usize, n: usize) -> Rect {
        self.cell(self.height - 28.0, i, n)
    }
}

#[derive(Clone, Copy)]
enum Action {
    Back,
    New,
    Select(usize),
    Edit(usize),
    Remove(usize),
    Enable(usize),
    Color(usize),
    Visibility,
    Range,
    Temporary,
    Save,
    Confirm,
}

struct Button(Action, Rect, String, bool);

fn buttons(page: &Page, points: &[Waypoint], layout: Layout) -> Vec<Button> {
    let mut buttons = Vec::new();
    let mut add = |action, rect, title, active| buttons.push(Button(action, rect, title, active));
    match page {
        Page::List(list) => {
            let matching = list.matching(points);
            let selected = list.selected.filter(|index| matching.contains(index));
            let offset = list
                .offset
                .min(matching.len().saturating_sub(layout.rows()));
            for (row, index) in matching
                .into_iter()
                .skip(offset)
                .take(layout.rows())
                .enumerate()
            {
                let point = &points[index];
                let [x, y, z] = point.position;
                let title = format!("{}   ·   {x}, {y}, {z}", point.display_name());
                add(
                    Action::Select(index),
                    layout.cell(55.0 + row as f32 * 23.0, 0, 1),
                    title,
                    true,
                );
            }
            let y = layout.height - 53.0;
            let index = selected.unwrap_or(0);
            let selected = selected.and_then(|index| points.get(index));
            add(
                Action::Edit(index),
                layout.cell(y, 0, 3),
                label("edit"),
                selected.is_some(),
            );
            add(
                Action::Enable(index),
                layout.cell(y, 1, 3),
                label(if selected.is_none_or(|point| point.enabled) {
                    "enabled"
                } else {
                    "disabled"
                }),
                selected.is_some(),
            );
            add(
                Action::Remove(index),
                layout.cell(y, 2, 3),
                label("remove"),
                selected.is_some(),
            );
            add(
                Action::New,
                layout.footer(0, 2),
                label("create"),
                points.len() < MAX_POINTS,
            );
            add(
                Action::Back,
                layout.footer(1, 2),
                lang::get("gui.done"),
                true,
            );
        }
        Page::Edit(edit) => {
            let draft = &edit.draft;
            let color = label("color");
            for i in 0..COLORS.len() {
                add(
                    Action::Color(i),
                    layout.cell(102.0 + (i / 8) as f32 * 23.0, i % 8, 8),
                    format!("{color} {}", i + 1),
                    true,
                );
            }
            for (i, (action, key)) in [
                (
                    Action::Visibility,
                    if draft.enabled { "enabled" } else { "disabled" },
                ),
                (Action::Range, if draft.global { "global" } else { "local" }),
                (
                    Action::Temporary,
                    if draft.temporary {
                        "temporary"
                    } else {
                        "permanent"
                    },
                ),
            ]
            .into_iter()
            .enumerate()
            {
                add(action, layout.cell(152.0, i, 3), label(key), true);
            }
            add(
                Action::Save,
                layout.footer(0, 2),
                label("save"),
                edit.candidate().is_some(),
            );
            add(
                Action::Back,
                layout.footer(1, 2),
                lang::get("gui.cancel"),
                true,
            );
        }
        Page::Delete(..) => {
            add(Action::Confirm, layout.footer(0, 2), label("remove"), true);
            add(
                Action::Back,
                layout.footer(1, 2),
                lang::get("gui.cancel"),
                true,
            );
        }
    }
    buttons
}

fn hovered(cursor: (f32, f32), [x, y, w, h]: Rect) -> bool {
    ui::hovered(cursor, x, y, w, h)
}

impl Game<'_> {
    fn minimap_waypoints_position(&self) -> Option<[i32; 3]> {
        self.player.as_ref().map(|p| {
            let pos = &p.position;
            [
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            ]
        })
    }

    pub(in crate::game) fn atlas_minimap_waypoints_open(&mut self, create: bool) -> bool {
        let Some(scope) = self.atlas_minimap_world_key() else {
            return false;
        };
        let Some(position) = self.minimap_waypoints_position() else {
            return false;
        };
        if !self.atlas_port_on(ID)
            || self.native_mod_screen().is_some()
            || !self.atlas_open_port_screen(ID)
        {
            return false;
        }
        let state = &mut self.atlas_ports.minimap_waypoints;
        state.refresh(&Self::game_dir());
        let page = if create {
            Page::Edit(Box::new(Editor::new(None, None, Waypoint::draft(position))))
        } else {
            Page::List(List::new())
        };
        state.screen = Some(Screen {
            scope,
            page,
            problem: state.unreadable.then(|| label("load_error")),
        });
        true
    }

    pub(in crate::game) fn atlas_minimap_temporary_waypoint_open(&mut self) -> bool {
        if !self.atlas_minimap_waypoints_open(true) {
            return false;
        }
        if let Some(Screen {
            page: Page::Edit(edit),
            ..
        }) = &mut self.atlas_ports.minimap_waypoints.screen
        {
            edit.draft.temporary = true;
        }
        true
    }

    pub(in crate::game) fn atlas_minimap_waypoints_active(&self) -> bool {
        self.native_mod_screen() == Some(ID) && self.atlas_ports.minimap_waypoints.screen.is_some()
    }

    pub(in crate::game) fn atlas_minimap_waypoints_closed(&mut self) {
        self.atlas_ports.minimap_waypoints.screen = None;
    }

    fn minimap_waypoints_layout(&self) -> Option<Layout> {
        let renderer = self.renderer.as_ref()?;
        Some(Layout::new(&HudBatch::new([
            renderer.config.width,
            renderer.config.height,
        ])))
    }

    fn minimap_waypoints_action(&mut self, action: Action) {
        let position = self.minimap_waypoints_position();
        let state = &mut self.atlas_ports.minimap_waypoints;
        let Some(Screen {
            scope,
            page,
            mut problem,
        }) = state.screen.take()
        else {
            return;
        };
        self.audio.play_button_click();
        let mut edit_points = |change: &mut dyn FnMut(&mut Vec<Waypoint>)| match state.edit(
            &Self::game_dir(),
            &scope,
            change,
        ) {
            Ok(()) => {
                problem = None;
                true
            }
            Err(error) => {
                problem = Some(error.message());
                false
            }
        };
        let page = match (page, action) {
            (Page::List(_), Action::Back) => None,
            (Page::Edit(edit), Action::Back) => edit.return_to.map(Page::List),
            (Page::Delete(list, _), Action::Back) => Some(Page::List(list)),
            (Page::List(mut list), Action::Select(index)) => {
                list.selected = Some(index);
                Some(Page::List(list))
            }
            (Page::List(list), Action::New) => Some(match position {
                Some(position) => Page::Edit(Box::new(Editor::new(
                    Some(list),
                    None,
                    Waypoint::draft(position),
                ))),
                None => Page::List(list),
            }),
            (Page::List(list), Action::Edit(index)) => {
                Some(match state.entries(&scope).get(index).cloned() {
                    Some(point) => {
                        Page::Edit(Box::new(Editor::new(Some(list), Some(index), point)))
                    }
                    None => Page::List(list),
                })
            }
            (Page::List(list), Action::Remove(index)) => Some(Page::Delete(list, index)),
            (Page::List(list), Action::Enable(index)) => {
                edit_points(&mut |points| {
                    if let Some(point) = points.get_mut(index) {
                        point.enabled = !point.enabled;
                    }
                });
                Some(Page::List(list))
            }
            (Page::Delete(mut list, index), Action::Confirm) => {
                if edit_points(&mut |points| {
                    if index < points.len() {
                        points.remove(index);
                    }
                }) {
                    list.selected = None;
                    Some(Page::List(list))
                } else {
                    Some(Page::Delete(list, index))
                }
            }
            (Page::Edit(mut edit), Action::Color(i)) => {
                edit.draft.color = COLORS[i];
                Some(Page::Edit(edit))
            }
            (Page::Edit(mut edit), Action::Visibility) => {
                edit.draft.enabled = !edit.draft.enabled;
                Some(Page::Edit(edit))
            }
            (Page::Edit(mut edit), Action::Range) => {
                edit.draft.global = !edit.draft.global;
                Some(Page::Edit(edit))
            }
            (Page::Edit(mut edit), Action::Temporary) => {
                edit.draft.temporary = !edit.draft.temporary;
                Some(Page::Edit(edit))
            }
            (Page::Edit(edit), Action::Save) => {
                let saved = edit.candidate().is_some_and(|point| {
                    edit_points(&mut |points| {
                        if let Some(index) = edit.index {
                            if let Some(existing) = points.get_mut(index) {
                                *existing = point.clone();
                            }
                        } else {
                            points.push(point.clone());
                        }
                    })
                });
                if saved {
                    edit.return_to.map(Page::List)
                } else {
                    Some(Page::Edit(edit))
                }
            }
            (page, _) => Some(page),
        };
        if let Some(page) = page {
            state.screen = Some(Screen {
                scope,
                page,
                problem,
            });
        } else {
            self.native_mod_screen_close(ID);
        }
    }

    pub(in crate::game) fn atlas_minimap_waypoints_render(&mut self, batch: &mut HudBatch) {
        if !self.atlas_minimap_waypoints_active() {
            return;
        }
        let atlas = &self.gui.atlas;
        let state = &mut self.atlas_ports.minimap_waypoints;
        let Some(screen) = &mut state.screen else {
            return;
        };
        let points = state
            .worlds
            .get(&screen.scope)
            .map_or(&[][..], Vec::as_slice);
        let layout = Layout::new(batch);
        mod_style::panel(
            batch,
            atlas,
            [layout.x, layout.y, layout.width, layout.height],
            8.0,
        );
        batch.text(
            atlas,
            &screen.page.title(),
            layout.x + 10.0,
            layout.y + 10.0,
            u32::MAX,
            true,
        );
        let selected = match &screen.page {
            Page::List(list) => list.selected,
            _ => None,
        };
        for Button(action, rect, name, active) in buttons(&screen.page, points, layout) {
            let [x, y, w, h] = rect;
            let over = hovered(self.cursor, rect);
            if let Action::Color(i) = action {
                let current =
                    matches!(&screen.page, Page::Edit(editor) if editor.draft.color == COLORS[i]);
                widgets::record(&name, rect, active);
                widgets::narratable(|| {
                    widgets::Narratable::button(name, rect, active).hovered(over)
                });
                widgets::widget_cursor(over, active);
                widgets::target(rect);
                let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
                ui.rounded(
                    x + 1.0,
                    y + 1.0,
                    w - 2.0,
                    h - 2.0,
                    5.0,
                    color::hex(COLORS[i], 255),
                );
                if current || over {
                    ui.border(x, y, w, h, 6.0, u32::MAX);
                }
                ui.finish();
            } else {
                let chosen = matches!(action, Action::Select(index) if Some(index) == selected);
                mod_style::button(batch, atlas, x, y, w, h, &Text::literal(name), active, over);
                if let Action::Select(index) = action {
                    let point = &points[index];
                    point.draw_badge(batch, atlas, [x + 13.0, y + h / 2.0], 11.0);
                    if chosen {
                        let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
                        ui.border(x, y, w, h, 9.0, color::hex(point.color, 255));
                        ui.finish();
                    }
                }
            }
        }
        let note = match &screen.page {
            Page::Edit(_) => Some((label("color"), 90.0)),
            Page::List(list) if list.matching(points).is_empty() => Some((label("empty"), 64.0)),
            Page::Delete(_, i) => points
                .get(*i)
                .map(|point| (point.display_name().into_owned(), 45.0)),
            _ => None,
        };
        if let Some((note, y)) = note {
            let [x, y, w, _] = layout.cell(y, 0, 1);
            let start = batch.vertices.len();
            batch.text(atlas, &note, x, y, 0xffb8c3d0, true);
            batch.clip_since(start, x, y, x + w, y + 12.0);
        }
        if let Some(problem) = &screen.problem {
            let y = match &screen.page {
                Page::List(_) => layout.height - 65.0,
                _ => layout.height - 42.0,
            };
            let [x, y, w, _] = layout.cell(y, 0, 1);
            let start = batch.vertices.len();
            batch.text(atlas, problem, x, y, 0xffeeb778, true);
            batch.clip_since(start, x, y, x + w, y + 10.0);
        }
        let editing = matches!(screen.page, Page::Edit(_));
        for (field, [x, y, w, h]) in screen.page.fields(layout) {
            (field.x, field.y, field.width, field.height) = (x, y, w, h);
            field.draw(batch, atlas, None);
            if editing {
                batch.text(atlas, &field.message, x + 1.0, y - 10.0, 0xffb8c3d0, false);
            } else if field.value.is_empty() && !field.focused {
                batch.text(atlas, &field.message, x + 4.0, y + 6.0, 0xff9099a7, false);
            }
        }
    }

    pub(in crate::game) fn atlas_minimap_waypoints_mouse(
        &mut self,
        button: MouseButton,
        pressed: bool,
    ) -> bool {
        if !self.atlas_minimap_waypoints_active() {
            return false;
        }
        if button != MouseButton::Left || !pressed {
            return true;
        }
        let Some(layout) = self.minimap_waypoints_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.minimap_waypoints;
        let Some(screen) = &mut state.screen else {
            return true;
        };
        for (field, rect) in screen.page.fields(layout) {
            field.focused = hovered(self.cursor, rect);
            if field.focused {
                field.click(
                    &self.gui.atlas,
                    self.cursor.0,
                    self.modifiers.shift_key(),
                    false,
                );
            }
        }
        let points = state
            .worlds
            .get(&screen.scope)
            .map_or(&[][..], Vec::as_slice);
        if let Some(Button(action, ..)) = buttons(&screen.page, points, layout)
            .into_iter()
            .find(|button| button.3 && hovered(self.cursor, button.1))
        {
            self.minimap_waypoints_action(action);
        }
        true
    }

    pub(in crate::game) fn atlas_minimap_waypoints_key(&mut self, key: KeyCode) -> bool {
        if !self.atlas_minimap_waypoints_active() {
            return false;
        }
        if key == KeyCode::Escape {
            self.minimap_waypoints_action(Action::Back);
            return true;
        }
        let Some(screen) = &mut self.atlas_ports.minimap_waypoints.screen else {
            return true;
        };
        if key == KeyCode::Enter && matches!(screen.page, Page::Edit(_)) {
            self.minimap_waypoints_action(Action::Save);
        } else if key == KeyCode::Tab
            && let Page::Edit(edit) = &mut screen.page
        {
            let count = edit.fields.len();
            let current = edit
                .fields
                .iter()
                .position(|field| field.focused)
                .unwrap_or(count - 1);
            let step = if self.modifiers.shift_key() {
                count - 1
            } else {
                1
            };
            let next = (1..=count)
                .map(|offset| (current + offset * step) % count)
                .find(|&i| edit.fields[i].editable);
            for (index, field) in edit.fields.iter_mut().enumerate() {
                field.focused = Some(index) == next;
                if field.focused {
                    field.reset_blink();
                }
            }
        } else if let Some(field) = screen.page.focused() {
            field.key(
                key,
                self.modifiers.control_key(),
                self.modifiers.shift_key(),
            );
        }
        true
    }

    pub(in crate::game) fn atlas_minimap_waypoints_text(&mut self, text: &str) -> bool {
        if !self.atlas_minimap_waypoints_active() {
            return false;
        }
        if let Some(screen) = &mut self.atlas_ports.minimap_waypoints.screen
            && let Some(field) = screen.page.focused()
        {
            field.insert_text(text);
            if let Page::List(list) = &mut screen.page {
                list.offset = 0;
                list.selected = None;
            }
        }
        true
    }

    pub(in crate::game) fn atlas_minimap_waypoints_shortcut(
        &mut self,
        shortcut: EditShortcut,
    ) -> bool {
        if !self.atlas_minimap_waypoints_active() {
            return false;
        }
        if let Some(screen) = &mut self.atlas_ports.minimap_waypoints.screen
            && let Some(field) = screen.page.focused()
        {
            field.shortcut(shortcut, &mut self.clipboard);
        }
        true
    }

    pub(in crate::game) fn atlas_minimap_waypoints_scroll(&mut self, delta: f32) -> bool {
        if !self.atlas_minimap_waypoints_active() {
            return false;
        }
        let Some(layout) = self.minimap_waypoints_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.minimap_waypoints;
        if let Some(Screen {
            page: Page::List(list),
            scope,
            ..
        }) = &mut state.screen
        {
            let points = state.worlds.get(scope).map_or(&[][..], Vec::as_slice);
            let max = list.matching(points).len().saturating_sub(layout.rows());
            list.offset = list
                .offset
                .min(max)
                .saturating_add_signed(-delta.round() as isize)
                .min(max);
        }
        true
    }
}
