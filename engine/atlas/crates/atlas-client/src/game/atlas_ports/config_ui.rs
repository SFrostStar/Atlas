//! Native implementation of the original Cloth and YACL configuration screens.
use super::{
    Game,
    config_model::{self, Definition, Field, Frame, Kind},
    inventory_hud::PositionEditor,
};
use atlas_client::{
    clipboard::EditShortcut,
    hud::{HudAtlas, HudBatch, rgba},
    lang,
    text::Text,
    ui::{self, EditBox},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use winit::keyboard::KeyCode;

pub(super) struct State {
    id: String,
    definition: Option<Definition>,
    original: Value,
    draft: Value,
    defaults: Value,
    category: usize,
    scroll: f32,
    search: EditBox,
    editors: BTreeMap<String, EditBox>,
    invalid: BTreeSet<String>,
    expanded: BTreeSet<String>,
    focus: Option<String>,
    dragging: Option<String>,
    discard_dialog: bool,
    error: Option<String>,
    exit_warning: bool,
    position_editor: Option<PositionEditor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: f32, height: f32, categories: usize, style: Frame) -> Layout {
        let mut batch = HudBatch::new([1280, 720]);
        batch.width = width;
        batch.height = height;
        Layout::new(&batch, categories, style)
    }

    #[test]
    fn cloth_frame_matches_original_java_widget_captures_at_three_gui_sizes() {
        // Recorded from the supplied Chunk Animator screen through Cloth17.
        for (width, height, search, cancel, save) in [
            (
                640.0,
                360.0,
                [72.0, 40.0, 500.0, 18.0],
                [125.0, 334.0, 192.0, 20.0],
                [323.0, 334.0, 192.0, 20.0],
            ),
            (
                426.0,
                240.0,
                [47.0, 40.0, 336.0, 18.0],
                [89.0, 214.0, 121.0, 20.0],
                [216.0, 214.0, 121.0, 20.0],
            ),
            (
                320.0,
                240.0,
                [47.0, 40.0, 230.0, 18.0],
                [71.0, 214.0, 86.0, 20.0],
                [163.0, 214.0, 86.0, 20.0],
            ),
        ] {
            let f = frame(width, height, 1, Frame::Cloth);
            assert_eq!(f.search, search);
            assert_eq!(f.footer(), [cancel, save]);
            assert_eq!(f.top, 68.0);
        }
        let gamma = frame(640.0, 360.0, 3, Frame::Cloth);
        assert_eq!(gamma.search, [72.0, 80.0, 500.0, 18.0]);
        assert_eq!(gamma.top, 108.0);
        assert_eq!(
            cloth_tab_rects(&[30.0, 56.0, 28.0]),
            vec![
                [24.0, 43.0, 38.0, 20.0],
                [64.0, 43.0, 64.0, 20.0],
                [130.0, 43.0, 36.0, 20.0],
            ]
        );
    }

    #[test]
    fn yacl_frame_matches_original_zoomify_list_and_sidebar() {
        let f = frame(640.0, 360.0, 4, Frame::Yacl);
        assert_eq!(f.pane, 426.0);
        assert_eq!((f.clip_top, f.top, f.bottom), (23.0, 27.0, 360.0));
        assert_eq!(f.search, [437.0, 286.0, 191.0, 18.0]);
        assert_eq!(
            f.footer(),
            [[436.0, 308.0, 95.0, 20.0], [436.0, 330.0, 193.0, 20.0]]
        );
    }

    #[test]
    fn color_editor_keeps_invalid_text_out_of_the_draft_and_preserves_original_until_save() {
        let field = Field::new("/color", "color", Kind::Color { alpha: false });
        let value = serde_json::json!({"color":43520});
        let mut state = State {
            definition: Some(Definition {
                title: String::new(),
                frame: Frame::Cloth,
                categories: vec![config_model::Category {
                    label: String::new(),
                    fields: vec![field.clone()],
                }],
            }),
            original: value.clone(),
            draft: value.clone(),
            defaults: value.clone(),
            ..Default::default()
        };
        state
            .editor(&field, [0.0, 0.0, 110.0, 18.0])
            .set_value("#ffaa00");
        state.sync_editor("/color");
        assert_eq!(state.draft["color"], 16755200);
        assert_eq!(state.original, value);
        assert!(state.dirty());
        for bad in ["ffaa00", "#zz0000", "#1000000"] {
            state.editor(&field, [0.0; 4]).set_value(bad);
            state.sync_editor("/color");
            assert!(state.invalid.contains("/color"));
            assert_eq!(state.draft["color"], 16755200);
        }
        state.set("/color", state.defaults["color"].clone());
        assert!(state.invalid.is_empty());
        assert!(!state.dirty());
    }

    #[test]
    fn discard_confirmation_keeps_java_line_wrap_and_button_positions() {
        let wide = frame(640.0, 360.0, 1, Frame::Cloth);
        assert_eq!(
            discard_geometry(wide, 1),
            (
                80.0,
                100.0,
                [[165.0, 156.0, 150.0, 20.0], [325.0, 156.0, 150.0, 20.0]]
            )
        );
        let narrow = frame(320.0, 240.0, 1, Frame::Cloth);
        assert_eq!(
            discard_geometry(narrow, 2),
            (
                80.0,
                100.0,
                [[5.0, 138.0, 150.0, 20.0], [165.0, 138.0, 150.0, 20.0]]
            )
        );
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            id: String::new(),
            definition: None,
            original: Value::Null,
            draft: Value::Null,
            defaults: Value::Null,
            category: 0,
            scroll: 0.0,
            search: EditBox::new(0.0, 0.0, 200.0, 20.0),
            editors: BTreeMap::new(),
            invalid: BTreeSet::new(),
            expanded: BTreeSet::new(),
            focus: None,
            dragging: None,
            discard_dialog: false,
            error: None,
            exit_warning: false,
            position_editor: None,
        }
    }
}

impl State {
    fn finish_position_editor(&mut self) {
        if let Some(editor) = self.position_editor.take() {
            self.draft = editor.finish();
            self.editors.clear();
        }
    }
}

#[derive(Clone)]
struct Row {
    field: Field,
    depth: usize,
}
#[derive(Clone, Copy)]
struct Layout {
    width: f32,
    height: f32,
    top: f32,
    clip_top: f32,
    bottom: f32,
    search: [f32; 4],
    right: f32,
    pane: f32,
    frame: Frame,
}
impl Layout {
    fn new(batch: &HudBatch, categories: usize, frame: Frame) -> Self {
        if frame == Frame::Yacl {
            let pane = (batch.width * 2.0 / 3.0).floor();
            let panel_width = batch.width - pane;
            return Self {
                width: batch.width,
                height: batch.height,
                top: 27.0,
                clip_top: 23.0,
                bottom: batch.height,
                search: [pane + 11.0, batch.height - 74.0, panel_width - 23.0, 18.0],
                right: pane - 16.0,
                pane,
                frame,
            };
        }
        let tabs = if categories > 1 { 40.0 } else { 0.0 };
        let search_width = (batch.width - 90.0).min(500.0).max(80.0);
        Self {
            width: batch.width,
            height: batch.height,
            top: 68.0 + tabs,
            clip_top: 32.0 + tabs,
            bottom: batch.height - 31.0,
            search: [
                ((batch.width - search_width) / 2.0).floor() + 2.0,
                40.0 + tabs,
                search_width,
                18.0,
            ],
            right: batch.width - 38.0,
            pane: batch.width,
            frame,
        }
    }
    fn footer(self) -> [[f32; 4]; 2] {
        if self.frame == Frame::Yacl {
            return [
                [
                    self.pane + 10.0,
                    self.height - 52.0,
                    (self.width - self.pane - 24.0) / 2.0,
                    20.0,
                ],
                [
                    self.pane + 10.0,
                    self.height - 30.0,
                    self.width - self.pane - 21.0,
                    20.0,
                ],
            ];
        }
        let width = ((self.width * 0.3333333).floor() - 20.0)
            .min(192.0)
            .max(60.0);
        [
            [
                self.width / 2.0 - width - 3.0,
                self.height - 26.0,
                width,
                20.0,
            ],
            [self.width / 2.0 + 3.0, self.height - 26.0, width, 20.0],
        ]
    }
    fn row_height(self, row: &Row) -> f32 {
        if self.frame == Frame::Cloth {
            24.0
        } else if matches!(row.field.kind, Kind::Group { .. }) {
            21.0
        } else if matches!(row.field.kind, Kind::Label) {
            26.0
        } else {
            22.0
        }
    }
}
fn hit([x, y, w, h]: [f32; 4], mx: f32, my: f32) -> bool {
    mx >= x && mx < x + w && my >= y && my < y + h
}
fn cloth_tab_rects(label_widths: &[f32]) -> Vec<[f32; 4]> {
    let mut x = 24.0;
    label_widths
        .iter()
        .map(|width| {
            let rect = [x, 43.0, width + 8.0, 20.0];
            x += width + 10.0;
            rect
        })
        .collect()
}
fn tr(key: &str) -> Text {
    Text::tr(key, &[])
}
fn centered(batch: &mut HudBatch, atlas: &HudAtlas, text: &str, x: f32, y: f32, color: u32) {
    batch.text(
        atlas,
        text,
        (x - HudBatch::text_width(atlas, text) / 2.0).floor(),
        y,
        color,
        true,
    );
}
fn discard_geometry(layout: Layout, lines: usize) -> (f32, f32, [[f32; 4]; 2]) {
    let message_height = lines as f32 * 9.0;
    let title_y = (((layout.height - message_height) / 2.0).floor() - 29.0).clamp(10.0, 80.0);
    let message_y = title_y + 20.0;
    let y = (message_y + message_height + 20.0)
        .max((layout.height / 6.0).floor() + 96.0)
        .min(layout.height - 24.0);
    (
        title_y,
        message_y,
        [
            [layout.width / 2.0 - 155.0, y, 150.0, 20.0],
            [layout.width / 2.0 + 5.0, y, 150.0, 20.0],
        ],
    )
}
fn draw_discard(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    layout: Layout,
    cursor: (f32, f32),
    focus: Option<&str>,
) {
    let lines = atlas.font.split_lines(
        &tr("text.cloth-config.quit_config_sure"),
        layout.width - 50.0,
    );
    let (title_y, message_y, buttons) = discard_geometry(layout, lines.len());
    centered(
        batch,
        atlas,
        &lang::get("text.cloth-config.quit_config"),
        layout.width / 2.0,
        title_y,
        u32::MAX,
    );
    for (i, (line, _)) in lines.iter().enumerate() {
        centered(
            batch,
            atlas,
            &line.plain(),
            layout.width / 2.0,
            message_y + i as f32 * 9.0,
            u32::MAX,
        );
    }
    for (i, (rect, key)) in buttons
        .into_iter()
        .zip(["text.cloth-config.quit_discard", "gui.cancel"])
        .enumerate()
    {
        atlas_client::atlas::mod_style::button(
            batch,
            atlas,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            &tr(key),
            true,
            hit(rect, cursor.0, cursor.1) || focus == Some(if i == 0 { "discard" } else { "keep" }),
        );
    }
}

impl State {
    fn positioned_rows(&self, layout: Layout) -> Vec<(Row, f32)> {
        let mut y = layout.top - self.scroll;
        self.rows()
            .into_iter()
            .map(|row| {
                let top = y;
                y += layout.row_height(&row);
                (row, top)
            })
            .collect()
    }
    fn content_height(&self, layout: Layout) -> f32 {
        self.rows().iter().map(|row| layout.row_height(row)).sum()
    }
    fn rows(&self) -> Vec<Row> {
        fn add(
            fields: &[Field],
            depth: usize,
            query: &str,
            expanded: &BTreeSet<String>,
            out: &mut Vec<Row>,
        ) {
            for field in fields {
                if let Kind::Group { fields } = &field.kind {
                    let mut children = vec![];
                    add(fields, depth + 1, query, expanded, &mut children);
                    if query.is_empty()
                        || !children.is_empty()
                        || field.label.to_lowercase().contains(query)
                    {
                        out.push(Row {
                            field: field.clone(),
                            depth,
                        });
                        if !query.is_empty() || expanded.contains(&field.path) {
                            out.extend(children)
                        }
                    }
                } else if query.is_empty()
                    || field.label.to_lowercase().contains(query)
                    || field
                        .tooltip
                        .iter()
                        .any(|line| line.to_lowercase().contains(query))
                {
                    out.push(Row {
                        field: field.clone(),
                        depth,
                    });
                }
            }
        }
        let mut rows = vec![];
        if let Some(category) = self
            .definition
            .as_ref()
            .and_then(|d| d.categories.get(self.category))
        {
            add(
                &category.fields,
                0,
                &self.search.value.to_lowercase(),
                &self.expanded,
                &mut rows,
            );
        }
        rows
    }
    fn dirty(&self) -> bool {
        self.draft != self.original || !self.invalid.is_empty()
    }
    fn set(&mut self, path: &str, value: Value) {
        if let Some(slot) = self.draft.pointer_mut(path) {
            *slot = value;
            self.error = None;
            self.invalid.remove(path);
            self.editors.remove(path);
        }
    }
    fn value_label(&self, field: &Field) -> String {
        let value = self.draft.pointer(&field.path).unwrap_or(&Value::Null);
        if let Some(display) = field.display {
            return display(value);
        }
        match &field.kind {
            Kind::Bool => lang::get(if value.as_bool().unwrap_or(false) {
                "text.cloth-config.boolean.value.true"
            } else {
                "text.cloth-config.boolean.value.false"
            }),
            Kind::Enum { values, labels } => values
                .iter()
                .position(|v| v == value)
                .and_then(|i| labels.get(i))
                .cloned()
                .unwrap_or_else(|| value.to_string()),
            Kind::Color { alpha } => format!(
                "#{:0width$x}",
                value.as_i64().unwrap_or(0) as u32,
                width = if *alpha { 8 } else { 6 }
            ),
            Kind::Action { button, .. } => button.clone(),
            _ => value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string()),
        }
    }
    fn control_rects(&self, layout: Layout, y: f32, atlas: &HudAtlas) -> ([f32; 4], [f32; 4]) {
        if layout.frame == Frame::Yacl {
            return (
                [9.0, y, layout.right - 31.0, 20.0],
                [layout.right - 20.0, y, 20.0, 20.0],
            );
        }
        let reset_width =
            HudBatch::text_width(atlas, &lang::get("text.cloth-config.reset_value")) + 8.0;
        (
            [layout.right - 150.0, y, 148.0 - reset_width, 20.0],
            [layout.right - reset_width, y, reset_width, 20.0],
        )
    }
    fn editor(&mut self, field: &Field, rect: [f32; 4]) -> &mut EditBox {
        let value = self.draft.pointer(&field.path).unwrap_or(&Value::Null);
        let text = if matches!(field.kind, Kind::Color { .. }) {
            self.value_label(field)
        } else {
            value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string())
        };
        let editor = self.editors.entry(field.path.clone()).or_insert_with(|| {
            let mut e = EditBox::new(rect[0], rect[1], rect[2], rect[3]);
            e.max_length = 256;
            e.set_value(&text);
            e
        });
        editor.x = rect[0];
        editor.y = rect[1];
        editor.width = rect[2];
        editor.height = rect[3];
        editor
    }
    fn sync_editor(&mut self, path: &str) {
        let Some(field) = self
            .rows()
            .into_iter()
            .find(|row| row.field.path == path)
            .map(|row| row.field)
        else {
            return;
        };
        let Some(text) = self.editors.get(path).map(|e| e.value.clone()) else {
            return;
        };
        let value = match field.kind {
            Kind::Color { alpha } => text
                .trim()
                .strip_prefix('#')
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .filter(|v| alpha || *v <= 0xFFFFFF)
                .map(Value::from),
            _ => field.numeric(&text),
        };
        if let Some(value) = value {
            if let Some(slot) = self.draft.pointer_mut(path) {
                *slot = value;
            }
            self.invalid.remove(path);
        } else {
            self.invalid.insert(path.into());
        }
    }
}

impl Game<'_> {
    fn atlas_config_active(&self) -> bool {
        let state = &self.atlas_ports.config_ui;
        !state.id.is_empty()
            && state.definition.is_some()
            && self.native_mod_screen() == Some(state.id.as_str())
    }
    pub(in crate::game) fn atlas_config_open(&mut self, id: &str) -> bool {
        let Some(definition) = config_model::definition(id) else {
            return false;
        };
        let Some(value) = self.atlas_config_value(id) else {
            return false;
        };
        let Some(defaults) = self.atlas_config_default(id) else {
            return false;
        };
        if !self.atlas_open_port_screen(id) {
            return false;
        }
        fn expand(fields: &[Field], out: &mut BTreeSet<String>) {
            for field in fields {
                if let Kind::Group { fields } = &field.kind {
                    out.insert(field.path.clone());
                    expand(fields, out)
                }
            }
        }
        let mut expanded = BTreeSet::new();
        if definition.frame == Frame::Yacl {
            for category in &definition.categories {
                expand(&category.fields, &mut expanded);
            }
        }
        self.atlas_ports.config_ui = State {
            id: id.into(),
            definition: Some(definition),
            original: value.clone(),
            draft: value,
            defaults,
            expanded,
            ..Default::default()
        };
        true
    }
    fn atlas_config_close(&mut self) {
        let id = self.atlas_ports.config_ui.id.clone();
        self.atlas_ports.config_ui = State::default();
        self.native_mod_screen_close(&id);
    }
    fn atlas_config_cancel(&mut self) {
        if self
            .atlas_ports
            .config_ui
            .definition
            .as_ref()
            .is_some_and(|d| d.frame == Frame::Yacl)
        {
            if self.atlas_ports.config_ui.dirty() {
                self.atlas_ports.config_ui.exit_warning = true;
            } else {
                self.atlas_config_close();
            }
            return;
        }
        if self.atlas_ports.config_ui.dirty() {
            self.atlas_ports.config_ui.discard_dialog = true;
            self.atlas_ports.config_ui.focus = Some("discard".into());
        } else {
            self.atlas_config_close()
        }
    }
    fn atlas_config_save(&mut self) {
        let state = &self.atlas_ports.config_ui;
        if !state.invalid.is_empty() {
            return;
        }
        let id = state.id.clone();
        let value = state.draft.clone();
        match self.atlas_config_apply(&id, value.clone()) {
            Ok(()) => {
                if self
                    .atlas_ports
                    .config_ui
                    .definition
                    .as_ref()
                    .is_some_and(|d| d.frame == Frame::Yacl)
                {
                    let state = &mut self.atlas_ports.config_ui;
                    state.original = value;
                    state.editors.clear();
                    state.invalid.clear();
                    state.exit_warning = false;
                    state.error = None;
                } else {
                    self.atlas_config_close();
                }
            }
            Err(error) => self.atlas_ports.config_ui.error = Some(error),
        }
    }
    pub(in crate::game) fn atlas_config_render(&mut self, batch: &mut HudBatch) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        if let Some(mut editor) = self.atlas_ports.config_ui.position_editor.take() {
            editor.draw(batch, self);
            self.atlas_ports.config_ui.position_editor = Some(editor);
            return true;
        }
        self.native_apply_screen_blur();
        self.render_transparent_background("atlas_config", batch);
        let atlas = &self.gui.atlas;
        let state = &mut self.atlas_ports.config_ui;
        let definition = state.definition.as_ref().unwrap().clone();
        let layout = Layout::new(batch, definition.categories.len(), definition.frame);
        if state.discard_dialog {
            draw_discard(batch, atlas, layout, self.cursor, state.focus.as_deref());
            return true;
        }
        let yacl = definition.frame == Frame::Yacl;
        if yacl {
            atlas_client::atlas::mod_style::panel(
                batch,
                atlas,
                [
                    layout.pane + 3.0,
                    26.0,
                    layout.width - layout.pane - 6.0,
                    layout.height - 51.0,
                ],
                10.0,
            );
        } else {
            ui::separators(batch, atlas, 28.0, layout.bottom, false);
            centered(
                batch,
                atlas,
                &definition.title,
                layout.width / 2.0,
                18.0,
                u32::MAX,
            );
        }
        let search_start = batch.vertices.len();
        state.search.x = layout.search[0];
        state.search.y = layout.search[1] - if yacl { 0.0 } else { state.scroll };
        state.search.width = layout.search[2];
        state.search.height = layout.search[3];
        state.search.focused = state.focus.as_deref() == Some("search");
        state.search.message = lang::get("gui.recipebook.search_hint");
        state.search.draw(batch, atlas, None);
        if state.search.value.is_empty() && !state.search.focused {
            batch.text_styled(
                atlas,
                &tr("gui.recipebook.search_hint"),
                state.search.text_x(),
                state.search.text_y(),
                rgba(128, 128, 128, 255),
                true,
            );
        }
        if !yacl {
            batch.clip_since(
                search_start,
                0.0,
                layout.clip_top,
                layout.width,
                layout.bottom,
            );
        }
        if definition.categories.len() > 1 {
            let widths: Vec<_> = if yacl {
                vec![
                    ((layout.width - 56.0) / definition.categories.len() as f32).floor();
                    definition.categories.len()
                ]
            } else {
                definition
                    .categories
                    .iter()
                    .map(|c| HudBatch::text_width(atlas, &c.label))
                    .collect()
            };
            let mut x = (layout.width - widths.iter().sum::<f32>()) / 2.0;
            let cloth_tabs = cloth_tab_rects(&widths);
            if !yacl {
                atlas_client::atlas::mod_style::panel(
                    batch,
                    atlas,
                    [20.0, 41.0, layout.width - 40.0, 24.0],
                    10.0,
                );
            }
            for (i, (category, width)) in definition.categories.iter().zip(widths).enumerate() {
                if yacl {
                    let y = if i == state.category { 0.0 } else { 4.0 };
                    let rect = [x, y, width, 24.0 - y];
                    atlas_client::widgets::record_as(
                        &category.label,
                        rect,
                        true,
                        atlas_client::widgets::Kind::Tab(i == state.category),
                    );
                    atlas_client::atlas::mod_style::panel(
                        batch,
                        atlas,
                        [rect[0] + 1.0, rect[1] + 1.0, rect[2] - 2.0, rect[3] - 2.0],
                        8.0,
                    );
                    centered(
                        batch,
                        atlas,
                        &category.label,
                        x + width / 2.0,
                        if i == state.category { 7.0 } else { 10.0 },
                        u32::MAX,
                    );
                    if i == state.category {
                        let underline = HudBatch::text_width(atlas, &category.label);
                        batch.fill(
                            atlas,
                            x + (width - underline) / 2.0,
                            21.0,
                            underline,
                            1.0,
                            u32::MAX,
                        );
                    }
                } else {
                    let [x, y, width, height] = cloth_tabs[i];
                    atlas_client::atlas::mod_style::button(
                        batch,
                        atlas,
                        x,
                        y,
                        width,
                        height,
                        &Text::literal(&category.label),
                        i != state.category,
                        hit(cloth_tabs[i], self.cursor.0, self.cursor.1),
                    );
                }
                x += width;
            }
        }
        state.scroll = state.scroll.clamp(
            0.0,
            (state.content_height(layout) - (layout.bottom - layout.top)).max(0.0),
        );
        let rows = state.positioned_rows(layout);
        let mut tooltip = None;
        for (row, y) in &rows {
            let field = &row.field;
            let y = *y;
            if y + layout.row_height(row) < layout.clip_top || y >= layout.bottom {
                continue;
            }
            let start = batch.vertices.len();
            let label_x = if yacl {
                13.0
            } else {
                40.0 + row.depth as f32 * 14.0
            };
            let label = field.label.clone();
            if !yacl {
                if matches!(field.kind, Kind::Group { .. }) {
                    batch.text(
                        atlas,
                        if state.expanded.contains(&field.path) || !state.search.value.is_empty() {
                            "-"
                        } else {
                            "+"
                        },
                        label_x - 12.0,
                        y + 6.0,
                        u32::MAX,
                        true,
                    );
                    atlas_client::widgets::record_as(
                        &label,
                        [label_x - 12.0, y, layout.right - label_x + 12.0, 20.0],
                        true,
                        atlas_client::widgets::Kind::Button,
                    );
                }
                batch.text(
                    atlas,
                    &label,
                    label_x,
                    y + 6.0,
                    if state.invalid.contains(&field.path) {
                        rgba(255, 85, 85, 255)
                    } else {
                        rgba(160, 160, 160, 255)
                    },
                    true,
                );
            }
            if yacl && matches!(field.kind, Kind::Group { .. }) {
                batch.text(
                    atlas,
                    if state.expanded.contains(&field.path) || !state.search.value.is_empty() {
                        "▼"
                    } else {
                        "▶"
                    },
                    16.0,
                    y + 5.0,
                    u32::MAX,
                    true,
                );
                centered(
                    batch,
                    atlas,
                    &field.label,
                    layout.pane / 2.0 - 4.0,
                    y + 5.0,
                    u32::MAX,
                );
                atlas_client::widgets::record_as(
                    &field.label,
                    [9.0, y, layout.right - 9.0, 21.0],
                    true,
                    atlas_client::widgets::Kind::Button,
                );
            }
            if yacl && matches!(field.kind, Kind::Label) {
                for (i, (line, _)) in atlas
                    .font
                    .split_lines(&Text::legacy(&field.label), layout.right - 16.0)
                    .iter()
                    .enumerate()
                {
                    batch.text_styled(atlas, line, 13.0, y + 4.0 + i as f32 * 9.0, u32::MAX, true);
                }
            }
            if !matches!(field.kind, Kind::Group { .. } | Kind::Label) {
                let (value_rect, reset_rect) = state.control_rects(layout, y, atlas);
                let value = state.draft.pointer(&field.path).unwrap_or(&Value::Null);
                let changed = state.defaults.pointer(&field.path) != Some(value)
                    || state.invalid.contains(&field.path);
                if !matches!(field.kind, Kind::Action { .. }) {
                    atlas_client::atlas::mod_style::button(
                        batch,
                        atlas,
                        reset_rect[0],
                        y,
                        reset_rect[2],
                        20.0,
                        &if yacl {
                            Text::literal("↻")
                        } else {
                            tr("text.cloth-config.reset_value")
                        },
                        changed && field.enabled,
                        hit(reset_rect, self.cursor.0, self.cursor.1),
                    );
                }
                let label = state.value_label(field);
                if yacl {
                    let mut rect = value_rect;
                    if matches!(field.kind, Kind::Action { .. }) {
                        rect[2] += 22.0;
                    }
                    let hover = hit(rect, self.cursor.0, self.cursor.1);
                    ui::blit_sprite(
                        batch,
                        atlas,
                        if !field.enabled {
                            "gui/sprites/widget/button_disabled"
                        } else if hover {
                            "gui/sprites/widget/button_highlighted"
                        } else {
                            "gui/sprites/widget/button"
                        },
                        rect[0],
                        rect[1],
                        rect[2],
                        rect[3],
                        u32::MAX,
                    );
                    atlas_client::widgets::record_as(
                        &field.label,
                        rect,
                        field.enabled,
                        match field.kind {
                            Kind::Int {
                                min,
                                max,
                                slider: true,
                                ..
                            } => atlas_client::widgets::Kind::Slider {
                                value: (value.as_i64().unwrap_or(min) - min) as f32
                                    / (max - min).max(1) as f32,
                                track: [rect[0] + 4.0, rect[2] - 8.0],
                            },
                            Kind::Float {
                                min,
                                max,
                                slider: true,
                                ..
                            } => atlas_client::widgets::Kind::Slider {
                                value: ((value.as_f64().unwrap_or(min) - min) / (max - min)) as f32,
                                track: [rect[0] + 4.0, rect[2] - 8.0],
                            },
                            _ => atlas_client::widgets::Kind::Button,
                        },
                    );
                    atlas_client::widgets::narratable(|| {
                        atlas_client::widgets::Narratable::button(
                            field.label.clone(),
                            rect,
                            field.enabled,
                        )
                        .hovered(hover)
                    });
                    let text_color = if field.enabled {
                        u32::MAX
                    } else {
                        rgba(160, 160, 160, 255)
                    };
                    batch.text(
                        atlas,
                        &field.label,
                        rect[0] + 4.0,
                        y + 6.0,
                        text_color,
                        true,
                    );
                    if matches!(field.kind, Kind::Bool) {
                        let x = rect[0] + rect[2] - 14.0;
                        batch.fill(atlas, x, y + 5.0, 10.0, 10.0, u32::MAX);
                        batch.fill(atlas, x + 1.0, y + 6.0, 8.0, 8.0, rgba(0, 0, 0, 255));
                        if value.as_bool().unwrap_or(false) {
                            batch.fill(atlas, x + 2.0, y + 7.0, 6.0, 6.0, u32::MAX);
                        }
                    } else {
                        batch.text(
                            atlas,
                            &label,
                            rect[0] + rect[2] - HudBatch::text_width(atlas, &label) - 4.0,
                            y + 6.0,
                            text_color,
                            true,
                        );
                    }
                } else {
                    match field.kind {
                        Kind::Int {
                            min,
                            max,
                            slider: true,
                            ..
                        } => {
                            let value = value.as_i64().unwrap_or(min);
                            atlas_client::atlas::mod_style::slider(
                                batch,
                                atlas,
                                value_rect[0],
                                y,
                                value_rect[2],
                                &Text::tr("options.generic_value", &[&value]),
                                (value - min) as f32 / (max - min).max(1) as f32,
                                hit(value_rect, self.cursor.0, self.cursor.1),
                                field.enabled,
                                state.dragging.as_deref() == Some(field.path.as_str()),
                            );
                        }
                        Kind::Float {
                            min,
                            max,
                            slider: true,
                            ..
                        } => {
                            let value = value.as_f64().unwrap_or(min);
                            atlas_client::atlas::mod_style::slider(
                                batch,
                                atlas,
                                value_rect[0],
                                y,
                                value_rect[2],
                                &Text::literal(&label),
                                ((value - min) / (max - min)) as f32,
                                hit(value_rect, self.cursor.0, self.cursor.1),
                                field.enabled,
                                state.dragging.as_deref() == Some(field.path.as_str()),
                            );
                        }
                        Kind::Int { slider: false, .. }
                        | Kind::Float { slider: false, .. }
                        | Kind::Color { .. } => {
                            if matches!(field.kind, Kind::Color { .. }) {
                                let c = state
                                    .draft
                                    .pointer(&field.path)
                                    .and_then(Value::as_u64)
                                    .unwrap_or(0) as u32;
                                batch.fill(
                                    atlas,
                                    value_rect[0] - 21.0,
                                    y,
                                    20.0,
                                    20.0,
                                    rgba(160, 160, 160, 255),
                                );
                                batch.fill(
                                    atlas,
                                    value_rect[0] - 20.0,
                                    y + 1.0,
                                    18.0,
                                    18.0,
                                    rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, 255),
                                );
                            }
                            let focused = state.focus.as_deref() == Some(field.path.as_str());
                            let editor = state.editor(
                                field,
                                [value_rect[0] + 2.0, y + 1.0, value_rect[2] - 4.0, 18.0],
                            );
                            editor.focused = focused;
                            editor.draw(batch, atlas, None);
                        }
                        _ => atlas_client::atlas::mod_style::button(
                            batch,
                            atlas,
                            value_rect[0],
                            y,
                            value_rect[2],
                            20.0,
                            &Text::legacy(&label),
                            field.enabled,
                            hit(value_rect, self.cursor.0, self.cursor.1),
                        ),
                    }
                }
            }
            if hit(
                [label_x, y, (layout.right - label_x).max(0.0), 20.0],
                self.cursor.0,
                self.cursor.1,
            ) && self.cursor.1 >= layout.clip_top
                && self.cursor.1 < layout.bottom
            {
                if state.invalid.contains(&field.path) {
                    let key = match field.kind {
                        Kind::Int { .. } => "text.cloth-config.error.not_valid_number_int",
                        Kind::Color { .. } => {
                            if state
                                .editors
                                .get(&field.path)
                                .is_some_and(|e| !e.value.trim().starts_with('#'))
                            {
                                "text.cloth-config.error.color.no_hash"
                            } else {
                                "text.cloth-config.error.color.invalid_color"
                            }
                        }
                        _ => "text.cloth-config.error.not_valid_number_float",
                    };
                    tooltip = Some(vec![tr(key)]);
                } else if !field.tooltip.is_empty() {
                    tooltip = Some(
                        field
                            .tooltip
                            .iter()
                            .map(|line| Text::legacy(line))
                            .collect::<Vec<_>>(),
                    );
                }
            }
            batch.clip_since(start, 0.0, layout.clip_top, layout.pane, layout.bottom);
        }
        ui::scrollbar(
            batch,
            atlas,
            if yacl {
                layout.pane - 6.0
            } else {
                layout.width - 20.0
            },
            layout.clip_top,
            layout.bottom,
            state.content_height(layout) + (layout.top - layout.clip_top),
            state.scroll,
        );
        let [cancel, save] = layout.footer();
        atlas_client::atlas::mod_style::button(
            batch,
            atlas,
            cancel[0],
            cancel[1],
            cancel[2],
            cancel[3],
            &tr(if yacl && !state.dirty() {
                "controls.reset"
            } else {
                "gui.cancel"
            }),
            true,
            hit(cancel, self.cursor.0, self.cursor.1),
        );
        atlas_client::atlas::mod_style::button(
            batch,
            atlas,
            save[0],
            save[1],
            save[2],
            save[3],
            &tr(if yacl {
                if state.dirty() {
                    if state.exit_warning {
                        "yacl.gui.save_before_exit"
                    } else {
                        "yacl.gui.save"
                    }
                } else {
                    "gui.done"
                }
            } else {
                "text.cloth-config.save_and_done"
            }),
            (!yacl && state.dirty() || yacl) && state.invalid.is_empty(),
            hit(save, self.cursor.0, self.cursor.1),
        );
        if yacl {
            let undo = [cancel[0] + cancel[2] + 4.0, cancel[1], cancel[2], cancel[3]];
            atlas_client::atlas::mod_style::button(
                batch,
                atlas,
                undo[0],
                undo[1],
                undo[2],
                undo[3],
                &tr("yacl.gui.undo"),
                state.dirty(),
                hit(undo, self.cursor.0, self.cursor.1),
            );
            if let Some(lines) = tooltip.as_ref() {
                let mut y = 40.0;
                for line in lines {
                    for (line, _) in atlas
                        .font
                        .split_lines(line, layout.width - layout.pane - 24.0)
                    {
                        batch.text_styled(atlas, &line, layout.pane + 12.0, y, u32::MAX, true);
                        y += 10.0;
                    }
                }
            }
        }
        if let Some(error) = &state.error {
            centered(
                batch,
                atlas,
                error,
                layout.width / 2.0,
                layout.height - 42.0,
                rgba(255, 85, 85, 255),
            );
        }
        if !yacl && let Some(lines) = tooltip {
            ui::tooltip(batch, atlas, &lines, self.cursor);
        }
        true
    }
    fn atlas_config_layout(&self) -> Option<Layout> {
        let r = self.renderer.as_ref()?;
        let d = self.atlas_ports.config_ui.definition.as_ref()?;
        Some(Layout::new(
            &HudBatch::new([r.config.width, r.config.height]),
            d.categories.len(),
            d.frame,
        ))
    }
    fn atlas_config_field_press(&mut self, field: &Field, rect: [f32; 4], button: i32) {
        if !field.enabled {
            return;
        }
        let state = &mut self.atlas_ports.config_ui;
        match &field.kind {
            Kind::Bool => {
                let on = state
                    .draft
                    .pointer(&field.path)
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state.set(&field.path, Value::Bool(!on));
            }
            Kind::Enum { values, .. } => {
                if values.is_empty() {
                    return;
                }
                let current = state.draft.pointer(&field.path);
                let index = values
                    .iter()
                    .position(|value| Some(value) == current)
                    .unwrap_or(0);
                let next = if button == 1 {
                    (index + values.len() - 1) % values.len()
                } else {
                    (index + 1) % values.len()
                };
                state.set(&field.path, values[next].clone());
            }
            Kind::Group { .. } => {
                if !state.expanded.remove(&field.path) {
                    state.expanded.insert(field.path.clone());
                }
            }
            Kind::Int { slider: true, .. } | Kind::Float { slider: true, .. } => {
                state.dragging = Some(field.path.clone());
                self.atlas_config_drag();
            }
            Kind::Int { slider: false, .. }
            | Kind::Float { slider: false, .. }
            | Kind::Color { .. } => {
                state.focus = Some(field.path.clone());
                let field_rect = [rect[0] + 2.0, rect[1] + 1.0, rect[2] - 4.0, 18.0];
                let editor = state.editor(field, field_rect);
                editor.focused = true;
                editor.click(&self.gui.atlas, self.cursor.0, false, false);
            }
            Kind::Action { action, .. } => {
                if action == "inventoryhud.arrange" {
                    if state.invalid.is_empty() {
                        state.position_editor = PositionEditor::new(&state.draft);
                        state.focus = None;
                    }
                    return;
                }
                let mut draft = state.draft.clone();
                match self.atlas_config_action(action, &mut draft) {
                    Ok(()) => {
                        let state = &mut self.atlas_ports.config_ui;
                        if action.starts_with("zoomify.preset.") {
                            state.original = draft.clone();
                            state.scroll = 0.0;
                        }
                        state.draft = draft;
                        state.editors.clear();
                        state.invalid.clear();
                    }
                    Err(error) => self.atlas_ports.config_ui.error = Some(error),
                }
            }
            Kind::Label => {}
        }
    }
    pub(in crate::game) fn atlas_config_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        let state = &mut self.atlas_ports.config_ui;
        if let Some(editor) = &mut state.position_editor {
            if editor.mouse([self.cursor.0, self.cursor.1], button, pressed) {
                state.finish_position_editor();
            }
            return true;
        }
        if !pressed {
            self.atlas_ports.config_ui.dragging = None;
            return true;
        }
        let Some(layout) = self.atlas_config_layout() else {
            return true;
        };
        let (mx, my) = self.cursor;
        if self.atlas_ports.config_ui.discard_dialog {
            let count = self
                .gui
                .atlas
                .font
                .split_lines(
                    &tr("text.cloth-config.quit_config_sure"),
                    layout.width - 50.0,
                )
                .len();
            let (_, _, [discard, keep]) = discard_geometry(layout, count);
            if button == 0 && hit(discard, mx, my) {
                self.atlas_config_close();
            } else if button == 0 && hit(keep, mx, my) {
                self.atlas_ports.config_ui.discard_dialog = false;
            }
            return true;
        }
        if ![0, 1].contains(&button) {
            return true;
        }
        let [cancel, save] = layout.footer();
        if layout.frame == Frame::Cloth {
            if hit(cancel, mx, my) {
                if button == 0 {
                    self.atlas_config_cancel();
                }
                return true;
            }
            if hit(save, mx, my) {
                if button == 0 && self.atlas_ports.config_ui.dirty() {
                    self.atlas_config_save();
                }
                return true;
            }
        } else {
            let undo = [cancel[0] + cancel[2] + 4.0, cancel[1], cancel[2], cancel[3]];
            if hit(cancel, mx, my) {
                if self.atlas_ports.config_ui.dirty() {
                    self.atlas_config_close();
                    return true;
                }
                let state = &mut self.atlas_ports.config_ui;
                state.draft = state.defaults.clone();
                state.editors.clear();
                state.invalid.clear();
                return true;
            }
            if hit(undo, mx, my) {
                let state = &mut self.atlas_ports.config_ui;
                state.draft = state.original.clone();
                state.editors.clear();
                state.invalid.clear();
                return true;
            }
            if hit(save, mx, my) {
                if self.atlas_ports.config_ui.dirty() {
                    self.atlas_config_save()
                } else {
                    self.atlas_config_close()
                }
                return true;
            }
        }
        let state = &mut self.atlas_ports.config_ui;
        let mut search = layout.search;
        if layout.frame == Frame::Cloth {
            search[1] -= state.scroll;
        }
        if hit(search, mx, my) && my >= layout.clip_top {
            state.focus = Some("search".into());
            state.search.focused = true;
            state.search.click(&self.gui.atlas, mx, false, false);
            return true;
        }
        let definition = state.definition.as_ref().unwrap();
        if definition.categories.len() > 1 {
            if layout.frame == Frame::Yacl {
                let width = ((layout.width - 56.0) / definition.categories.len() as f32).floor();
                let x = (layout.width - width * definition.categories.len() as f32) / 2.0;
                for i in 0..definition.categories.len() {
                    if hit([x + i as f32 * width, 0.0, width, 24.0], mx, my) {
                        state.category = i;
                        state.scroll = 0.0;
                        state.focus = None;
                        return true;
                    }
                }
            } else {
                let widths: Vec<_> = definition
                    .categories
                    .iter()
                    .map(|c| HudBatch::text_width(&self.gui.atlas, &c.label))
                    .collect();
                for (i, rect) in cloth_tab_rects(&widths).into_iter().enumerate() {
                    if hit(rect, mx, my) {
                        state.category = i;
                        state.scroll = 0.0;
                        state.focus = None;
                        return true;
                    }
                }
            }
        }
        if my < layout.clip_top || my >= layout.bottom || mx >= layout.pane {
            return true;
        }
        let rows = state.positioned_rows(layout);
        if mx >= layout.pane - 20.0 && state.content_height(layout) > layout.bottom - layout.top {
            state.dragging = Some("scrollbar".into());
            self.atlas_config_drag();
            return true;
        }
        for (row, y) in rows {
            let (value, reset) =
                self.atlas_ports
                    .config_ui
                    .control_rects(layout, y, &self.gui.atlas);
            if !matches!(
                row.field.kind,
                Kind::Group { .. } | Kind::Label | Kind::Action { .. }
            ) && hit(reset, mx, my)
            {
                if let Some(default) = self
                    .atlas_ports
                    .config_ui
                    .defaults
                    .pointer(&row.field.path)
                    .cloned()
                {
                    self.atlas_ports.config_ui.set(&row.field.path, default);
                }
                return true;
            }
            let group = matches!(row.field.kind, Kind::Group { .. });
            let mut control = value;
            if matches!(row.field.kind, Kind::Action { .. }) && layout.frame == Frame::Yacl {
                control[2] += 22.0;
            }
            if hit(control, mx, my)
                || group
                    && hit(
                        [9.0, y, layout.pane - 30.0, layout.row_height(&row)],
                        mx,
                        my,
                    )
            {
                self.atlas_config_field_press(&row.field, control, button);
                return true;
            }
        }
        self.atlas_ports.config_ui.focus = None;
        true
    }
    pub(in crate::game) fn atlas_config_key(
        &mut self,
        key: KeyCode,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        let state = &mut self.atlas_ports.config_ui;
        if let Some(editor) = &mut state.position_editor {
            if editor.key(key, shift) {
                state.finish_position_editor();
            }
            return true;
        }
        let focus = self.atlas_ports.config_ui.focus.clone();
        if key == KeyCode::Escape {
            if self.atlas_ports.config_ui.discard_dialog {
                self.atlas_ports.config_ui.discard_dialog = false;
            } else {
                self.atlas_config_cancel();
            }
            return true;
        }
        if self.atlas_ports.config_ui.discard_dialog {
            if matches!(key, KeyCode::Tab | KeyCode::ArrowLeft | KeyCode::ArrowRight) {
                self.atlas_ports.config_ui.focus = Some(
                    if focus.as_deref() == Some("discard") {
                        "keep"
                    } else {
                        "discard"
                    }
                    .into(),
                );
            } else if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
                if focus.as_deref() == Some("keep") {
                    self.atlas_ports.config_ui.discard_dialog = false;
                } else {
                    self.atlas_config_close();
                }
            }
            return true;
        }
        if ctrl && key == KeyCode::KeyF {
            let state = &mut self.atlas_ports.config_ui;
            state.focus = Some("search".into());
            state.search.focused = true;
            state.scroll = 0.0;
            return true;
        }
        if key == KeyCode::Tab {
            let mut targets = vec!["search".to_owned()];
            for row in self.atlas_ports.config_ui.rows() {
                if row.field.enabled && !matches!(row.field.kind, Kind::Label) {
                    targets.push(row.field.path.clone());
                    let state = &self.atlas_ports.config_ui;
                    if !matches!(row.field.kind, Kind::Group { .. } | Kind::Action { .. })
                        && (state.draft.pointer(&row.field.path)
                            != state.defaults.pointer(&row.field.path)
                            || state.invalid.contains(&row.field.path))
                    {
                        targets.push(format!("reset:{}", row.field.path));
                    }
                }
            }
            targets.push("cancel".into());
            let yacl = self
                .atlas_ports
                .config_ui
                .definition
                .as_ref()
                .is_some_and(|d| d.frame == Frame::Yacl);
            if yacl && self.atlas_ports.config_ui.dirty() {
                targets.push("undo".into());
            }
            if (yacl || self.atlas_ports.config_ui.dirty())
                && self.atlas_ports.config_ui.invalid.is_empty()
            {
                targets.push("save".into());
            }
            let at = targets
                .iter()
                .position(|path| Some(path) == focus.as_ref())
                .unwrap_or(0);
            let next = if shift {
                (at + targets.len() - 1) % targets.len()
            } else {
                (at + 1) % targets.len()
            };
            let target = targets[next].clone();
            if let Some(layout) = self.atlas_config_layout() {
                if target == "search" {
                    self.atlas_ports.config_ui.scroll = 0.0;
                } else {
                    let path = target.strip_prefix("reset:").unwrap_or(&target);
                    let state = &mut self.atlas_ports.config_ui;
                    if let Some((_, y)) = state
                        .positioned_rows(layout)
                        .into_iter()
                        .find(|(row, _)| row.field.path == path)
                    {
                        if y < layout.clip_top {
                            state.scroll = (state.scroll + y - layout.clip_top).max(0.0);
                        } else if y + 20.0 > layout.bottom {
                            state.scroll += y + 20.0 - layout.bottom;
                        }
                    }
                }
            }
            self.atlas_ports.config_ui.focus = Some(target);
            return true;
        }
        if let Some(path) = focus {
            if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
                if path == "cancel" {
                    if self
                        .atlas_ports
                        .config_ui
                        .definition
                        .as_ref()
                        .is_some_and(|d| d.frame == Frame::Yacl)
                    {
                        if self.atlas_ports.config_ui.dirty() {
                            self.atlas_config_close();
                        } else {
                            let state = &mut self.atlas_ports.config_ui;
                            state.draft = state.defaults.clone();
                            state.editors.clear();
                            state.invalid.clear();
                        }
                    } else {
                        self.atlas_config_cancel();
                    }
                    return true;
                }
                if path == "save" {
                    if self.atlas_ports.config_ui.dirty() {
                        self.atlas_config_save();
                    } else {
                        self.atlas_config_close();
                    }
                    return true;
                }
                if path == "undo" {
                    let state = &mut self.atlas_ports.config_ui;
                    state.draft = state.original.clone();
                    state.editors.clear();
                    state.invalid.clear();
                    state.exit_warning = false;
                    return true;
                }
                if let Some(pointer) = path.strip_prefix("reset:") {
                    if let Some(value) = self
                        .atlas_ports
                        .config_ui
                        .defaults
                        .pointer(pointer)
                        .cloned()
                    {
                        self.atlas_ports.config_ui.set(pointer, value);
                    }
                    return true;
                }
            }
            if path == "search" {
                let state = &mut self.atlas_ports.config_ui;
                state.search.key(key, ctrl, shift);
                state.scroll = 0.0;
            } else if let Some(editor) = self.atlas_ports.config_ui.editors.get_mut(&path) {
                editor.key(key, ctrl, shift);
                self.atlas_ports.config_ui.sync_editor(&path);
            } else if matches!(key, KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter) {
                if let Some(row) = self
                    .atlas_ports
                    .config_ui
                    .rows()
                    .into_iter()
                    .find(|row| row.field.path == path)
                {
                    self.atlas_config_field_press(&row.field, [0.0; 4], 0);
                }
            }
        } else if key == KeyCode::ArrowDown {
            self.atlas_config_scroll(-1.0);
        } else if key == KeyCode::ArrowUp {
            self.atlas_config_scroll(1.0);
        }
        true
    }
    pub(in crate::game) fn atlas_config_char(&mut self, text: &str) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        if self.atlas_ports.config_ui.position_editor.is_some() {
            return true;
        }
        let state = &mut self.atlas_ports.config_ui;
        if state.discard_dialog {
            return true;
        }
        if let Some(path) = state.focus.clone() {
            if path == "search" {
                state.search.insert_text(text);
                state.scroll = 0.0;
            } else if let Some(editor) = state.editors.get_mut(&path) {
                editor.insert_text(text);
                state.sync_editor(&path);
            }
        }
        true
    }
    pub(in crate::game) fn atlas_config_shortcut(&mut self, shortcut: EditShortcut) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        if self.atlas_ports.config_ui.position_editor.is_some() {
            return true;
        }
        let state = &mut self.atlas_ports.config_ui;
        if state.discard_dialog {
            return true;
        }
        if let Some(path) = state.focus.clone() {
            if path == "search" {
                state.search.shortcut(shortcut, &mut self.clipboard);
                state.scroll = 0.0;
            } else if let Some(editor) = state.editors.get_mut(&path) {
                editor.shortcut(shortcut, &mut self.clipboard);
                state.sync_editor(&path);
            }
        }
        true
    }
    pub(in crate::game) fn atlas_config_scroll(&mut self, delta: f32) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        if self.atlas_ports.config_ui.position_editor.is_some() {
            return true;
        }
        let Some(layout) = self.atlas_config_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.config_ui;
        if !state.discard_dialog && self.cursor.0 < layout.pane {
            state.scroll = (state.scroll - delta * 24.0).clamp(
                0.0,
                (state.content_height(layout) - (layout.bottom - layout.top)).max(0.0),
            );
        }
        true
    }
    pub(in crate::game) fn atlas_config_drag(&mut self) -> bool {
        if !self.atlas_config_active() {
            return false;
        }
        if let Some(editor) = &mut self.atlas_ports.config_ui.position_editor {
            editor.dragged([self.cursor.0, self.cursor.1]);
            return true;
        }
        let Some(layout) = self.atlas_config_layout() else {
            return true;
        };
        let state = &mut self.atlas_ports.config_ui;
        let Some(path) = state.dragging.clone() else {
            return true;
        };
        if path == "scrollbar" {
            let max = (state.content_height(layout) - (layout.bottom - layout.top)).max(0.0);
            state.scroll = ((self.cursor.1 - layout.clip_top) / (layout.bottom - layout.clip_top))
                .clamp(0.0, 1.0)
                * max;
            return true;
        }
        let Some(row) = state.rows().into_iter().find(|r| r.field.path == path) else {
            return true;
        };
        let (rect, _) = state.control_rects(layout, 0.0, &self.gui.atlas);
        let normal = ui::slider_value(rect[0], rect[2], self.cursor.0) as f64;
        let value = match row.field.kind {
            Kind::Int { min, max, step, .. } => {
                let step = step.max(1);
                Value::from(
                    (min + (((max - min) as f64 * normal) / step as f64).round() as i64 * step)
                        .clamp(min, max),
                )
            }
            Kind::Float { min, max, step, .. } => {
                let mut value = min + (max - min) * normal;
                if step > 0.0 {
                    value = min + ((value - min) / step).round() * step;
                }
                Value::from(value.clamp(min, max))
            }
            _ => return true,
        };
        state.set(&path, value);
        true
    }
}
