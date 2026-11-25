use super::{Action, Game, ID, Macro, Mode, Modifier, label};
use atlas_client::{
    atlas::mod_style,
    clipboard::EditShortcut,
    hud::HudBatch,
    keybinds,
    text::Text,
    ui::{self, EditBox},
};
use winit::{event::MouseButton, keyboard::KeyCode};

type Rect = [f32; 4];

#[derive(Default)]
pub(super) enum Page {
    #[default]
    Home,
    List(List),
    Edit(Edit),
    Delete(List, usize),
}

pub(super) struct List {
    server: Option<String>,
    search: EditBox,
    offset: usize,
}

pub(super) struct Edit {
    list: List,
    index: Option<usize>,
    binding: Macro,
    name: EditBox,
    text: EditBox,
    time: EditBox,
    key_capture: bool,
}

impl List {
    fn new(server: Option<String>) -> Self {
        Self {
            server,
            search: input("search", "", 64),
            offset: 0,
        }
    }

    fn matching(&self, entries: &[Macro]) -> Vec<usize> {
        let search = self.search.value.to_lowercase();
        entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (entry.name.to_lowercase().contains(&search)
                    || entry.text.to_lowercase().contains(&search))
                .then_some(index)
            })
            .collect()
    }
}

impl Edit {
    fn new(list: List, index: Option<usize>, binding: Macro) -> Self {
        let name = input("name", &binding.name, 64);
        let text = input("action_text", &binding.text, 256);
        let time = input("milliseconds", &binding.milliseconds.to_string(), 10);
        Self {
            list,
            index,
            binding,
            name,
            text,
            time,
            key_capture: false,
        }
    }

    fn valid(&self) -> bool {
        !self.name.value.trim().is_empty()
            && !self.text.value.trim().is_empty()
            && !self.binding.key.is_empty()
            && self.time.value.parse::<u32>().is_ok()
    }
}

fn input(prompt: &str, value: &str, max: usize) -> EditBox {
    let mut box_ = EditBox::new(0.0, 0.0, 1.0, 20.0);
    box_.message = label(prompt);
    box_.max_length = max;
    box_.set_value(value);
    box_
}

fn key_label(binding: &Macro) -> String {
    let key = if binding.key.is_empty() {
        label("unassigned")
    } else {
        keybinds::display_name(&binding.key).plain()
    };
    match binding.modifier {
        Modifier::None => key,
        Modifier::Shift => format!("Shift + {key}"),
        Modifier::Alt => format!("Alt + {key}"),
        Modifier::Control => format!("Ctrl + {key}"),
    }
}

impl Page {
    fn title(&self) -> String {
        label(match self {
            Self::Home => "title",
            Self::List(list) if list.server.is_some() => "server",
            Self::List(_) => "global",
            Self::Edit(edit) if edit.index.is_some() => "edit",
            Self::Edit(_) => "create",
            Self::Delete(..) => "delete_title",
        })
    }

    fn back(self) -> Option<Self> {
        match self {
            Self::Home => None,
            Self::List(_) => Some(Self::Home),
            Self::Edit(edit) => Some(Self::List(edit.list)),
            Self::Delete(list, _) => Some(Self::List(list)),
        }
    }

    fn fields(&mut self, layout: Layout) -> Vec<(&mut EditBox, Rect)> {
        if let Self::List(list) = self {
            vec![(&mut list.search, layout.full(30.0))]
        } else if let Self::Edit(edit) = self {
            let time_width = layout.inner() * 0.28;
            let mut fields = vec![
                (&mut edit.name, layout.full(30.0)),
                (&mut edit.text, layout.full(58.0)),
            ];
            if edit.binding.mode != Mode::Simple {
                fields.push((
                    &mut edit.time,
                    [
                        layout.x + layout.width - time_width - 10.0,
                        layout.y + 86.0,
                        time_width,
                        20.0,
                    ],
                ));
            }
            fields
        } else {
            Vec::new()
        }
    }

    fn focused(&mut self) -> Option<&mut EditBox> {
        match self {
            Self::List(list) if list.search.focused => Some(&mut list.search),
            Self::Edit(edit) => [&mut edit.name, &mut edit.text, &mut edit.time]
                .into_iter()
                .find(|field| field.focused),
            _ => None,
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
        let width = (batch.width - 16.0).min(410.0).max(12.0);
        let height = (batch.height - 16.0).min(300.0).max(12.0);
        Self {
            x: ((batch.width - width) / 2.0).floor(),
            y: ((batch.height - height) / 2.0).floor(),
            width,
            height,
        }
    }

    fn inner(self) -> f32 {
        self.width - 20.0
    }

    fn full(self, y: f32) -> Rect {
        [self.x + 10.0, self.y + y, self.inner(), 20.0]
    }

    fn pair(self, y: f32, right: bool) -> Rect {
        let width = (self.inner() - 6.0) / 2.0;
        [
            self.x + 10.0 + if right { width + 6.0 } else { 0.0 },
            self.y + y,
            width,
            20.0,
        ]
    }

    fn footer(self, right: bool) -> Rect {
        self.pair(self.height - 28.0, right)
    }

    fn rows(self) -> usize {
        (((self.height - 119.0) / 24.0).floor() as usize).max(1)
    }
}

#[derive(Clone, Copy)]
enum Click {
    Back,
    Global,
    Server,
    Stop,
    New,
    Edit(usize),
    Delete(usize),
    Enable(usize),
    Mode,
    Action,
    Bind,
    Save,
    Confirm,
}

struct Button {
    rect: Rect,
    label: String,
    click: Click,
    active: bool,
}

impl Button {
    fn new(rect: Rect, title: &str, click: Click) -> Self {
        Self {
            rect,
            label: label(title),
            click,
            active: true,
        }
    }
}

fn buttons(page: &Page, state: &super::State, layout: Layout, server: Option<&str>) -> Vec<Button> {
    let mut buttons = vec![Button::new(
        layout.footer(false),
        if matches!(page, Page::Home) {
            "done"
        } else {
            "back"
        },
        Click::Back,
    )];
    match page {
        Page::Home => {
            buttons.push(Button::new(layout.full(43.0), "global", Click::Global));
            let mut choose_server = Button::new(layout.full(69.0), "server", Click::Server);
            choose_server.active = server.is_some();
            buttons.push(choose_server);
            buttons.push(Button::new(layout.full(95.0), "stop", Click::Stop));
        }
        Page::List(list) => {
            buttons.push(Button::new(layout.footer(true), "create", Click::New));
            let entries = state.entries(list.server.as_deref());
            let indices = list.matching(entries);
            let offset = list.offset.min(indices.len().saturating_sub(layout.rows()));
            for (row, index) in indices
                .into_iter()
                .skip(offset)
                .take(layout.rows())
                .enumerate()
            {
                let binding = &entries[index];
                let [x, y, width, h] = layout.full(56.0 + row as f32 * 24.0);
                let enabled_width = 35.0;
                let delete_width = 24.0;
                let edit_width = width - enabled_width - delete_width - 8.0;
                buttons.push(Button {
                    rect: [x, y, edit_width, h],
                    label: format!(
                        "{}  ·  {}{}",
                        binding.name,
                        key_label(binding),
                        if binding.due.is_some() { "  ▶" } else { "" }
                    ),
                    click: Click::Edit(index),
                    active: true,
                });
                buttons.push(Button::new(
                    [x + edit_width + 4.0, y, enabled_width, h],
                    if binding.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    Click::Enable(index),
                ));
                buttons.push(Button::new(
                    [x + edit_width + enabled_width + 8.0, y, delete_width, h],
                    "delete",
                    Click::Delete(index),
                ));
            }
        }
        Page::Edit(edit) => {
            let [x, y, width, height] = layout.full(86.0);
            let w = if edit.binding.mode == Mode::Simple {
                width
            } else {
                width * 0.72 - 6.0
            };
            buttons.push(Button::new(
                [x, y, w, height],
                match edit.binding.mode {
                    Mode::Simple => "simple",
                    Mode::Toggle => "toggle",
                    Mode::Repeat => "repeat",
                    Mode::Delayed => "delayed",
                },
                Click::Mode,
            ));
            buttons.push(Button::new(
                layout.pair(114.0, false),
                match edit.binding.action {
                    Action::Message => "message",
                    Action::Command => "command",
                    Action::FillChat => "fill_chat",
                },
                Click::Action,
            ));
            buttons.push(Button {
                rect: layout.pair(114.0, true),
                label: if edit.key_capture {
                    label("press_key")
                } else {
                    key_label(&edit.binding)
                },
                click: Click::Bind,
                active: true,
            });
            let mut save = Button::new(layout.footer(true), "save", Click::Save);
            save.active = edit.valid() && !state.config_error;
            buttons.push(save);
        }
        Page::Delete(..) => {
            let mut confirm = Button::new(layout.footer(true), "confirm_delete", Click::Confirm);
            confirm.active = !state.config_error;
            buttons.push(confirm);
        }
    }
    buttons
}

fn contains(rect: Rect, point: (f32, f32)) -> bool {
    ui::hovered(point, rect[0], rect[1], rect[2], rect[3])
}

impl Game<'_> {
    pub(in crate::game) fn atlas_macros_open(&mut self) -> bool {
        if !self.atlas_open_port_screen(ID) {
            return false;
        }
        self.atlas_ports.macro_keybinds.screen = Page::Home;
        true
    }

    fn macros_layout(&self) -> Option<Layout> {
        let r = self.renderer.as_ref()?;
        Some(Layout::new(&HudBatch::new([
            r.config.width,
            r.config.height,
        ])))
    }

    fn macros_save_change(
        &mut self,
        server: Option<&str>,
        update: impl FnOnce(&mut Vec<Macro>),
    ) -> bool {
        let state = &mut self.atlas_ports.macro_keybinds;
        let old = state.entries(server).to_vec();
        update(state.entries_mut(server));
        if let Err(error) = state.save() {
            *state.entries_mut(server) = old;
            log::warn!("Macro Keybinds: {error}");
            self.native_chat(&label("save_error"));
            return false;
        }
        true
    }

    fn macros_back(&mut self) {
        let page = std::mem::take(&mut self.atlas_ports.macro_keybinds.screen);
        if let Some(page) = page.back() {
            self.atlas_ports.macro_keybinds.screen = page;
        } else {
            self.native_mod_screen_close(ID);
        }
    }

    fn macros_click(&mut self, click: Click) {
        self.audio.play_button_click();
        match click {
            Click::Back => return self.macros_back(),
            Click::Stop => {
                let server = self.macro_server().map(str::to_owned);
                for binding in self
                    .atlas_ports
                    .macro_keybinds
                    .active_mut(server.as_deref())
                {
                    if matches!(binding.mode, Mode::Repeat | Mode::Delayed) {
                        binding.due = None;
                    }
                }
                return;
            }
            _ => {}
        }
        let state = &mut self.atlas_ports.macro_keybinds;
        let page = std::mem::take(&mut state.screen);
        let page = match (page, click) {
            (Page::Home, Click::Global) => Page::List(List::new(None)),
            (Page::Home, Click::Server) => {
                Page::List(List::new(self.macro_server().map(str::to_owned)))
            }
            (Page::List(list), Click::New) => Page::Edit(Edit::new(list, None, Macro::default())),
            (Page::List(list), Click::Edit(index)) => {
                let binding = state.entries(list.server.as_deref())[index].clone();
                Page::Edit(Edit::new(list, Some(index), binding))
            }
            (Page::List(list), Click::Delete(index)) => Page::Delete(list, index),
            (Page::List(list), Click::Enable(index)) => {
                self.macros_save_change(list.server.as_deref(), |entries| {
                    entries[index].enabled = !entries[index].enabled;
                    if !entries[index].enabled {
                        entries[index].due = None;
                    }
                });
                Page::List(list)
            }
            (Page::Delete(list, index), Click::Confirm) => {
                if self.macros_save_change(list.server.as_deref(), |entries| {
                    entries.remove(index);
                }) {
                    Page::List(list)
                } else {
                    Page::Delete(list, index)
                }
            }
            (Page::Edit(mut edit), Click::Mode) => {
                edit.binding.mode = match edit.binding.mode {
                    Mode::Simple => Mode::Toggle,
                    Mode::Toggle => Mode::Repeat,
                    Mode::Repeat => Mode::Delayed,
                    Mode::Delayed => Mode::Simple,
                };
                Page::Edit(edit)
            }
            (Page::Edit(mut edit), Click::Action) => {
                edit.binding.action = match edit.binding.action {
                    Action::Message => Action::Command,
                    Action::Command => Action::FillChat,
                    Action::FillChat => Action::Message,
                };
                Page::Edit(edit)
            }
            (Page::Edit(mut edit), Click::Bind) => {
                edit.key_capture = true;
                Page::Edit(edit)
            }
            (Page::Edit(edit), Click::Save) if edit.valid() => {
                let binding = Macro {
                    name: edit.name.value.trim().into(),
                    text: edit.text.value.clone(),
                    milliseconds: edit.time.value.parse().unwrap_or(0),
                    due: None,
                    last_action: None,
                    ..edit.binding.clone()
                };
                let saved = self.macros_save_change(edit.list.server.as_deref(), |entries| {
                    if let Some(index) = edit.index {
                        entries[index] = binding;
                    } else {
                        entries.push(binding);
                    }
                });
                if saved {
                    Page::List(edit.list)
                } else {
                    Page::Edit(edit)
                }
            }
            (page, _) => page,
        };
        self.atlas_ports.macro_keybinds.screen = page;
    }

    pub(in crate::game) fn atlas_macros_render(&mut self, batch: &mut HudBatch) {
        if self.native_mod_screen() != Some(ID) {
            return;
        }
        let server = self.macro_server().map(str::to_owned);
        let atlas = &self.gui.atlas;
        let state = &mut self.atlas_ports.macro_keybinds;
        let layout = Layout::new(batch);
        mod_style::panel(
            batch,
            atlas,
            [layout.x, layout.y, layout.width, layout.height],
            8.0,
        );
        let title = state.screen.title();
        batch.text(
            atlas,
            &title,
            layout.x + 10.0,
            layout.y + 11.0,
            u32::MAX,
            true,
        );
        let note = match &state.screen {
            _ if state.config_error => Some(label("load_error")),
            Page::Home if server.is_none() => Some(label("no_server")),
            Page::Delete(list, index) => {
                Some(state.entries(list.server.as_deref())[*index].name.clone())
            }
            Page::Edit(edit) => {
                let duplicate = |macro_: &Macro| {
                    edit.binding.key == macro_.key && edit.binding.modifier == macro_.modifier
                };
                let current = state
                    .entries(edit.list.server.as_deref())
                    .iter()
                    .enumerate()
                    .any(|(i, m)| Some(i) != edit.index && duplicate(m));
                let other = edit.list.server.as_ref().map_or_else(
                    || {
                        server
                            .as_deref()
                            .is_some_and(|server| state.entries(Some(server)).iter().any(duplicate))
                    },
                    |_| state.entries(None).iter().any(duplicate),
                );
                (current || other).then(|| label("duplicate"))
            }
            _ => None,
        };
        if let Some(note) = note {
            let start = batch.vertices.len();
            let [x, y, w, _] = layout.full(match state.screen {
                Page::Home => 126.0,
                Page::Delete(..) => 48.0,
                _ => layout.height - 43.0,
            });
            batch.text(atlas, &note, x, y, 0xffeeb778, true);
            batch.clip_since(start, x, y, x + w, y + 12.0);
        }
        for button in buttons(&state.screen, state, layout, server.as_deref()) {
            let [x, y, w, h] = button.rect;
            mod_style::button(
                batch,
                atlas,
                x,
                y,
                w,
                h,
                &Text::literal(button.label),
                button.active,
                contains(button.rect, self.cursor),
            );
        }
        for (field, [x, y, w, h]) in state.screen.fields(layout) {
            (field.x, field.y, field.width, field.height) = (x, y, w, h);
            field.draw(batch, atlas, None);
            if field.value.is_empty() && !field.focused {
                batch.text(atlas, &field.message, x + 5.0, y + 6.0, 0xff9099a7, false);
            }
        }
    }

    pub(in crate::game) fn atlas_macros_mouse(
        &mut self,
        button: MouseButton,
        pressed: bool,
    ) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        if !pressed {
            return true;
        }
        let state = &mut self.atlas_ports.macro_keybinds;
        if let Page::Edit(edit) = &mut state.screen
            && edit.key_capture
        {
            edit.binding.key = keybinds::mouse_name(button);
            edit.binding.modifier = Modifier::None;
            edit.key_capture = false;
            return true;
        }
        if button != MouseButton::Left {
            return true;
        }
        if let Some(layout) = self.macros_layout() {
            let server = self.macro_server().map(str::to_owned);
            let state = &mut self.atlas_ports.macro_keybinds;
            for (field, rect) in state.screen.fields(layout) {
                field.focused = contains(rect, self.cursor);
                if field.focused {
                    field.click(&self.gui.atlas, self.cursor.0, false, false);
                }
            }
            if let Some(button) = buttons(&state.screen, state, layout, server.as_deref())
                .into_iter()
                .find(|button| button.active && contains(button.rect, self.cursor))
            {
                self.macros_click(button.click);
            }
        }
        true
    }

    pub(in crate::game) fn atlas_macros_key(&mut self, key: KeyCode) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        let state = &mut self.atlas_ports.macro_keybinds;
        if let Page::Edit(edit) = &mut state.screen
            && edit.key_capture
        {
            if key == KeyCode::Escape {
                edit.key_capture = false;
            } else if !matches!(
                key,
                KeyCode::ShiftLeft
                    | KeyCode::ShiftRight
                    | KeyCode::AltLeft
                    | KeyCode::AltRight
                    | KeyCode::ControlLeft
                    | KeyCode::ControlRight
            ) && let Some(name) = keybinds::key_name(key)
            {
                edit.binding.key = name.into();
                edit.binding.modifier = Modifier::current(self.modifiers);
                edit.key_capture = false;
            }
            return true;
        }
        if key == KeyCode::Escape {
            self.macros_back();
        } else if let Some(field) = state.screen.focused() {
            field.key(
                key,
                self.modifiers.control_key(),
                self.modifiers.shift_key(),
            );
        }
        true
    }

    pub(in crate::game) fn atlas_macros_binding_key(&self) -> bool {
        self.native_mod_screen() == Some(ID)
            && matches!(&self.atlas_ports.macro_keybinds.screen, Page::Edit(edit) if edit.key_capture)
    }

    pub(in crate::game) fn atlas_macros_text(&mut self, text: &str) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        if let Some(field) = self.atlas_ports.macro_keybinds.screen.focused() {
            field.insert_text(text);
        }
        true
    }

    pub(in crate::game) fn atlas_macros_shortcut(&mut self, shortcut: EditShortcut) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        if let Some(field) = self.atlas_ports.macro_keybinds.screen.focused() {
            field.shortcut(shortcut, &mut self.clipboard);
        }
        true
    }

    pub(in crate::game) fn atlas_macros_scroll(&mut self, delta: f32) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        let Some(layout) = self.macros_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.macro_keybinds;
        let Page::List(list) = &state.screen else {
            return true;
        };
        let max = list
            .matching(state.entries(list.server.as_deref()))
            .len()
            .saturating_sub(layout.rows());
        if let Page::List(list) = &mut state.screen {
            list.offset = list
                .offset
                .min(max)
                .saturating_add_signed(-delta.round() as isize)
                .min(max);
        }
        true
    }
}
