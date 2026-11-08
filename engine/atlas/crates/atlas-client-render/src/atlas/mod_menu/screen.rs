//! ModsScreen, ModListWidget and DescriptionListWidget from Mod Menu 21.0.0.
//! Layout is in Minecraft GUI pixels, not the modern Atlas settings density.
use super::{
    config::{self, Config},
    model::{self, Entry},
};
use crate::{
    hud::{HudAtlas, HudBatch},
    mod_list::ModList,
    screen_draw::{self as d, Draw, Measure, TextField},
    text::{Style, Text},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use winit::keyboard::KeyCode as K;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    Done,
    Changed,
    Configure(usize),
    OpenLink(String),
    OpenFolder,
    Import(Vec<PathBuf>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Search,
    List,
    Description,
    Filters,
    Sort,
    Libraries,
    Configure,
    Website,
    Issues,
    Folder,
    Done,
    Option(usize),
    Confirm(bool),
}
#[derive(Clone)]
struct Button {
    target: Target,
    rect: [i32; 4],
    label: Text,
    active: bool,
    icon: Option<&'static str>,
}
#[derive(Clone, Debug)]
struct Line {
    text: Text,
    indent: i32,
    link: Option<String>,
}
#[derive(Clone, Copy)]
pub struct Layout {
    pub pane: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub row: i32,
    pub search: [i32; 4],
}
impl Layout {
    pub fn new(w: i32, h: i32, c: &Config) -> Self {
        let pane = w / 2 - 8;
        let filters = if c.config_mode { 0 } else { 22 };
        let max = pane - 32 - filters;
        let sw = if c.config_mode { 200.min(max) } else { max };
        Self {
            pane,
            right: w - pane,
            top: if c.config_mode { 48 } else { 67 },
            bottom: h - 36,
            row: if c.compact_list { 23 } else { 36 },
            search: [pane / 2 - sw / 2 - filters / 2, 22, sw.max(20), 20],
        }
    }
}
pub struct ModMenu {
    pub width: i32,
    pub height: i32,
    config: Config,
    entries: Vec<Entry>,
    rows: Vec<usize>,
    selected: Option<String>,
    search: TextField,
    focus: Target,
    expanded: BTreeSet<String>,
    filters: bool,
    scroll: f64,
    description_scroll: f64,
    options_scroll: f64,
    options: bool,
    last_click: BTreeMap<String, i64>,
    drag: Option<(bool, f64)>,
    pending: Vec<PathBuf>,
    message: Option<Text>,
    blink: std::time::Instant,
    description_height: std::cell::Cell<f64>,
}
impl Default for ModMenu {
    fn default() -> Self {
        let mut search = TextField::new(0, 22, 100, 20);
        search.max_length = 32;
        search.focused = true;
        search.hint = Some(tr("modmenu.search"));
        Self {
            width: 480,
            height: 270,
            config: Config::default(),
            entries: Vec::new(),
            rows: Vec::new(),
            selected: None,
            search,
            focus: Target::Search,
            expanded: BTreeSet::new(),
            filters: false,
            scroll: 0.0,
            description_scroll: 0.0,
            options_scroll: 0.0,
            options: false,
            last_click: BTreeMap::new(),
            drag: None,
            pending: Vec::new(),
            message: None,
            blink: std::time::Instant::now(),
            description_height: std::cell::Cell::new(0.0),
        }
    }
}
fn tr(key: &str) -> Text {
    Text::translate(key, &[])
}
fn translated(key: &str, args: &[Text]) -> Text {
    Text::translate(key, args)
}
fn icon(out: &mut Vec<Draw>, key: &str, x: i32, y: i32, size: i32) {
    out.push(Draw::Blit {
        texture: if key.is_empty() {
            "textures/misc/unknown_pack.png".into()
        } else {
            key.into()
        },
        x,
        y,
        w: size,
        h: size,
        u: 0.0,
        v: 0.0,
        src: [1, 1],
        tex: [1, 1],
        color: -1,
    });
}
fn centered(out: &mut Vec<Draw>, m: &Measure, t: Text, x: i32, y: i32, color: i32) {
    let width = m.width(&t);
    d::text(out, t, x - width / 2, y, color, true);
}
fn option_label(c: &Config, key: &str) -> Text {
    translated(
        "options.generic_value",
        &[
            tr(&format!("option.modmenu.{key}")),
            tr(&format!("option.modmenu.{key}.{}", c.option(key))),
        ],
    )
}
fn scrollbar(out: &mut Vec<Draw>, x: i32, top: i32, bottom: i32, content: f64, scroll: f64) {
    if let Some((y, h)) =
        crate::ui::scrollbar_thumb(top as f32, bottom as f32, content as f32, scroll as f32)
    {
        d::fill(out, x, top, x + 6, bottom, 0xff000000u32 as i32);
        d::fill(
            out,
            x,
            y as i32,
            x + 6,
            (y + h) as i32,
            0xff808080u32 as i32,
        );
        d::fill(
            out,
            x,
            y as i32,
            x + 5,
            (y + h - 1.0) as i32,
            0xffc0c0c0u32 as i32,
        );
    }
}
fn truncated(m: &Measure, text: Text, width: i32) -> Text {
    if m.width(&text) > width {
        d::clip(m, &text, width)
    } else {
        text
    }
}

/// ModsScreen.updateFiltersX: narrow panes center the filters and hide the count.
fn filters_position(l: Layout, width: i32, count_widths: [i32; 3], shown: bool) -> (i32, bool) {
    let search_row = l.search[0] + l.search[2] + 22;
    if width + count_widths[0] + 20 >= search_row
        && (width + count_widths[1] + 20 >= search_row
            || width + count_widths[2] + 20 >= search_row)
    {
        (l.pane / 2 - width / 2, !shown)
    } else {
        (search_row - width + 1, true)
    }
}

fn numeric_label(key: &str, counts: &[(usize, usize)]) -> Text {
    let args: Vec<_> = counts
        .iter()
        .map(|&(shown, total)| {
            Text::literal(if shown == total {
                total.to_string()
            } else {
                format!("{shown}/{total}")
            })
        })
        .collect();
    // TranslationUtil tries all exact-number suffixes before their .n fallbacks.
    for flags in (0..1 << counts.len()).rev() {
        let mut full_key = key.to_owned();
        for (i, &(shown, total)) in counts.iter().enumerate() {
            full_key.push('.');
            if flags & (1 << i) != 0 && shown == total {
                full_key.push_str(&total.to_string());
            } else {
                full_key.push('n');
            }
        }
        if crate::lang::has(&full_key) || flags == 0 {
            return translated(&full_key, &args);
        }
    }
    unreachable!()
}

impl ModMenu {
    pub fn sync(&mut self, list: &ModList, size: (f32, f32)) {
        super::publish_count(list);
        self.width = size.0 as i32;
        self.height = size.1 as i32;
        self.config = list.menu.clone();
        self.entries = model::entries(list);
        self.refilter();
        let l = Layout::new(self.width, self.height, &self.config);
        self.search.x = l.search[0];
        self.search.y = l.search[1];
        self.search.width = l.search[2];
        self.search.height = l.search[3];
        self.search.focused = self.focus == Target::Search && !self.options;
        self.search.message = tr("modmenu.search").plain();
        self.search.hint = Some(tr("modmenu.search"));
    }
    fn refilter(&mut self) {
        let found = model::filtered(&self.entries, &self.config, &self.search.value);
        self.rows.clear();
        for &i in &found {
            let entry = &self.entries[i];
            if entry
                .details
                .parent
                .as_ref()
                .is_some_and(|id| self.entries.iter().any(|p| p.details.id == *id))
            {
                continue;
            }
            self.rows.push(i);
            if self.expanded.contains(&entry.details.id) {
                for &child in &found {
                    if self.entries[child].details.parent.as_deref() == Some(&entry.details.id) {
                        self.rows.push(child);
                    }
                }
            }
        }
        // Filtering clears the row highlight, but Java keeps the selected detail pane.
        if self
            .selected
            .as_ref()
            .is_none_or(|id| !self.entries.iter().any(|e| e.details.id == *id))
        {
            self.selected = self
                .rows
                .first()
                .map(|&i| self.entries[i].details.id.clone());
            self.description_scroll = 0.0;
        }
        self.clamp_scroll();
    }
    fn rebuild_rows(&mut self) {
        // ModListWidget.filter creates new entries, whose click clocks start over.
        // Frame-by-frame sync only recomputes rows and must retain these clocks.
        self.last_click.clear();
        self.refilter();
    }
    fn selected(&self) -> Option<&Entry> {
        self.selected
            .as_ref()
            .and_then(|id| self.entries.iter().find(|e| e.details.id == *id))
    }
    fn content_height(&self) -> f64 {
        (self.rows.len() as i32 * Layout::new(self.width, self.height, &self.config).row + 4) as f64
    }
    fn max_scroll(&self) -> f64 {
        let l = Layout::new(self.width, self.height, &self.config);
        (self.content_height() - (l.bottom - l.top - 4) as f64).max(0.0)
    }
    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }
    fn choose(&mut self, row: usize) {
        if let Some(&i) = self.rows.get(row) {
            let id = self.entries[i].details.id.clone();
            if self.selected.as_ref() != Some(&id) {
                self.selected = Some(id);
                self.description_scroll = 0.0;
            }
        }
    }
    fn selected_row(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|&i| self.selected.as_deref() == Some(&self.entries[i].details.id))
    }
    fn ensure_visible(&mut self, row: usize) {
        let l = Layout::new(self.width, self.height, &self.config);
        let top = l.top + 4 + row as i32 * l.row - self.scroll as i32;
        let up = top - l.top - 4 - l.row;
        if up < 0 {
            self.scroll += up as f64;
        }
        let down = l.bottom - top - l.row * 2;
        if down < 0 {
            self.scroll -= down as f64;
        }
        self.clamp_scroll();
    }
    fn row_at(&self, p: (i32, i32)) -> Option<usize> {
        let l = Layout::new(self.width, self.height, &self.config);
        let index = p.1 - l.top + self.scroll as i32 - 4;
        let right = l.pane - if self.max_scroll() > 0.0 { 12 } else { 6 };
        (p.0 >= 6 && p.0 <= right && p.1 >= l.top && p.1 < l.bottom && index >= 0)
            .then_some((index / l.row) as usize)
            .filter(|i| *i < self.rows.len())
    }
    fn count_labels(&self) -> [Text; 3] {
        let counts = |library: bool| {
            let roots: Vec<_> = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    !e.hidden(&self.config)
                        && e.details.library == library
                        && e.details
                            .parent
                            .as_ref()
                            .is_none_or(|id| !self.entries.iter().any(|p| p.details.id == *id))
                        && (!library
                            || !self
                                .config
                                .show_libraries
                                .hide(true, e.has_config(&self.config)))
                })
                .map(|(i, _)| i)
                .collect();
            (
                roots.iter().filter(|i| self.rows.contains(i)).count(),
                roots.len(),
            )
        };
        let mods = counts(false);
        let libraries = counts(true);
        let mods_text = numeric_label("modmenu.showingMods", &[mods]);
        let full = if libraries.1 == 0 {
            mods_text.clone()
        } else {
            numeric_label("modmenu.showingModsLibraries", &[mods, libraries])
        };
        [
            full,
            mods_text,
            numeric_label("modmenu.showingLibraries", &[libraries]),
        ]
    }
    fn focus_targets(&self, buttons: &[Button]) -> Vec<Target> {
        if self.options || self.message.is_some() || !self.pending.is_empty() {
            return buttons
                .iter()
                .filter(|b| b.active)
                .map(|b| b.target.clone())
                .collect();
        }
        [
            Target::Search,
            Target::Filters,
            Target::Sort,
            Target::Libraries,
            Target::List,
            Target::Configure,
            Target::Website,
            Target::Issues,
            Target::Description,
            Target::Folder,
            Target::Done,
        ]
        .into_iter()
        .filter(|t| {
            matches!(t, Target::Search | Target::List | Target::Description)
                || buttons.iter().any(|b| b.active && b.target == *t)
        })
        .collect()
    }
    fn cycle_target(
        &mut self,
        target: &Target,
        list: &mut ModList,
        reverse: bool,
    ) -> Option<Action> {
        let key = match target {
            Target::Sort => "sorting",
            Target::Libraries => "show_libraries",
            Target::Option(i) => *config::OPTIONS.get(*i)?,
            _ => return None,
        };
        self.config.cycle(key, reverse);
        list.menu = self.config.clone();
        self.rebuild_rows();
        Some(Action::Changed)
    }
    fn double_clicked(&self, id: &str, now: i64) -> bool {
        self.last_click.get(id).is_some_and(|ms| now - *ms < 250)
    }
    fn navigate_parent(&mut self, key: K) -> bool {
        let Some(entry) = self.selected().cloned() else {
            return false;
        };
        if key == K::ArrowLeft {
            if let Some(parent) = &entry.details.parent {
                if let Some(row) = self
                    .rows
                    .iter()
                    .position(|&i| self.entries[i].details.id == *parent)
                {
                    self.choose(row);
                    self.ensure_visible(row);
                    return true;
                }
            }
        }
        if !self
            .entries
            .iter()
            .any(|e| e.details.parent.as_deref() == Some(&entry.details.id))
        {
            return false;
        }
        if key == K::ArrowRight && self.expanded.contains(&entry.details.id) {
            if let Some(row) = self.selected_row() {
                let next = (row + 1).min(self.rows.len() - 1);
                self.choose(next);
                self.ensure_visible(next);
            }
        } else {
            if key == K::ArrowLeft {
                if !self.expanded.remove(&entry.details.id) {
                    return true;
                }
            } else if !self.expanded.insert(entry.details.id.clone()) {
                self.expanded.remove(&entry.details.id);
            }
            self.rebuild_rows();
            if let Some(row) = self.selected_row() {
                self.ensure_visible(row);
            }
        }
        true
    }
    fn buttons(&self, m: &Measure) -> Vec<Button> {
        let l = Layout::new(self.width, self.height, &self.config);
        let mut out = Vec::new();
        let b = |target, rect, label, active, icon| Button {
            target,
            rect,
            label,
            active,
            icon,
        };
        if self.message.is_some() {
            out.push(b(
                Target::Done,
                [self.width / 2 - 100, self.height / 2 + 35, 200, 20],
                tr("gui.done"),
                true,
                None,
            ));
            return out;
        }
        if !self.pending.is_empty() {
            out.push(b(
                Target::Confirm(true),
                [self.width / 2 - 155, self.height / 2 + 30, 150, 20],
                tr("gui.yes"),
                true,
                None,
            ));
            out.push(b(
                Target::Confirm(false),
                [self.width / 2 + 5, self.height / 2 + 30, 150, 20],
                tr("gui.no"),
                true,
                None,
            ));
            return out;
        }
        if self.options {
            for (i, key) in config::OPTIONS.iter().enumerate() {
                let y = 36 + (i / 2) as i32 * 24 - self.options_scroll as i32;
                out.push(b(
                    Target::Option(i),
                    [self.width / 2 - 155 + (i % 2) as i32 * 160, y, 150, 20],
                    option_label(&self.config, key),
                    true,
                    None,
                ));
            }
            out.push(b(
                Target::Done,
                [self.width / 2 - 100, self.height - 28, 200, 20],
                tr("gui.done"),
                true,
                None,
            ));
            return out;
        }
        if !self.config.config_mode {
            out.push(b(
                Target::Filters,
                [l.pane / 2 + l.search[2] / 2 - 8, 22, 20, 20],
                tr("modmenu.toggleFilterOptions"),
                true,
                Some("filters"),
            ));
        }
        if self.filters && !self.config.config_mode {
            let sorting = option_label(&self.config, "sorting");
            let libraries = option_label(&self.config, "show_libraries");
            let sw = m.width(&sorting) + 28;
            let lw = m.width(&libraries) + 20;
            let count_widths = self.count_labels().map(|label| m.width(&label));
            let (fx, _) = filters_position(l, sw + lw + 2, count_widths, self.filters);
            out.push(b(Target::Sort, [fx, 45, sw, 20], sorting, true, None));
            out.push(b(
                Target::Libraries,
                [fx + sw + 2, 45, lw, 20],
                libraries,
                true,
                None,
            ));
        }
        let chosen = self.selected();
        let config = chosen.is_some_and(|e| e.has_config(&self.config));
        if !self.config.hide_config_buttons && config {
            out.push(b(
                Target::Configure,
                [self.width - 24, 48, 20, 20],
                tr("modmenu.configure"),
                config,
                Some("configure"),
            ));
        }
        let half = l.pane / 2 - 2;
        let bw = half.min(200);
        out.push(b(
            Target::Website,
            [l.right + half / 2 - bw / 2, 84, bw, 20],
            tr("modmenu.website"),
            chosen.is_some_and(|e| !e.details.homepage.is_empty()),
            None,
        ));
        out.push(b(
            Target::Issues,
            [l.right + half + 4 + half / 2 - bw / 2, 84, bw, 20],
            tr("modmenu.issues"),
            chosen.is_some_and(|e| !e.details.issues.is_empty()),
            None,
        ));
        out.push(b(
            Target::Folder,
            [self.width / 2 - 154, self.height - 28, 150, 20],
            tr("modmenu.modsFolder"),
            true,
            None,
        ));
        out.push(b(
            Target::Done,
            [self.width / 2 + 4, self.height - 28, 150, 20],
            tr("gui.done"),
            true,
            None,
        ));
        out
    }
    fn activate(&mut self, target: Target, list: &mut ModList) -> Action {
        match target {
            Target::Done if self.message.take().is_some() => Action::None,
            Target::Done if self.options => {
                self.options = false;
                self.focus = Target::Search;
                self.rebuild_rows();
                Action::None
            }
            Target::Done => Action::Done,
            Target::Filters => {
                self.filters = !self.filters;
                Action::None
            }
            Target::Sort | Target::Libraries | Target::Option(_) => self
                .cycle_target(&target, list, false)
                .unwrap_or(Action::None),
            Target::Configure => {
                if let Some(entry) = self.selected().filter(|e| e.has_config(&self.config)) {
                    if entry.details.id == "modmenu" {
                        self.options = true;
                        self.options_scroll = 0.0;
                        self.focus = Target::Option(0);
                        Action::None
                    } else {
                        entry
                            .installed_index
                            .map_or(Action::None, Action::Configure)
                    }
                } else {
                    Action::None
                }
            }
            Target::Website => self.selected().map_or(Action::None, |e| {
                Action::OpenLink(e.details.homepage.clone())
            }),
            Target::Issues => self
                .selected()
                .map_or(Action::None, |e| Action::OpenLink(e.details.issues.clone())),
            Target::Folder => Action::OpenFolder,
            Target::Confirm(yes) => {
                let files = std::mem::take(&mut self.pending);
                if yes {
                    Action::Import(files)
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }
    pub fn click(
        &mut self,
        list: &mut ModList,
        atlas: &HudAtlas,
        size: (f32, f32),
        p: (f32, f32),
        drag: bool,
        right: bool,
        shift: bool,
    ) -> Action {
        atlas.font.using_font(crate::font::MOD_MENU, || {
            self.click_inner(list, atlas, size, p, drag, right, shift)
        })
    }
    fn click_inner(
        &mut self,
        list: &mut ModList,
        atlas: &HudAtlas,
        size: (f32, f32),
        p: (f32, f32),
        drag: bool,
        right: bool,
        shift: bool,
    ) -> Action {
        self.sync(list, size);
        let p = (p.0.floor() as i32, p.1.floor() as i32);
        let m = Measure(&atlas.font);
        let l = Layout::new(self.width, self.height, &self.config);
        if drag {
            if let Some((description, last)) = self.drag {
                let height = if description {
                    self.height - 144
                } else {
                    l.bottom - l.top
                };
                let content = if description {
                    self.description_lines(&m).len() as f64 * 10.0 + 4.0
                } else {
                    self.content_height()
                };
                let delta = (p.1 as f64 - last) * content / height.max(1) as f64;
                if description {
                    self.description_scroll = (self.description_scroll + delta)
                        .clamp(0.0, (content - height as f64).max(0.0));
                } else {
                    self.scroll += delta;
                    self.clamp_scroll();
                }
                self.drag = Some((description, p.1 as f64));
            }
            return Action::None;
        }
        self.drag = None;
        if right {
            return Action::None;
        }
        if let Some(button) = self
            .buttons(&m)
            .into_iter()
            .find(|b| b.active && d::inside(p, b.rect[0], b.rect[1], b.rect[2], b.rect[3]))
        {
            self.focus = button.target.clone();
            if shift {
                if let Some(action) = self.cycle_target(&button.target, list, true) {
                    return action;
                }
            }
            return self.activate(button.target, list);
        }
        if self.options || self.message.is_some() || !self.pending.is_empty() {
            return Action::None;
        }
        if self.search.contains(p.0 as f64, p.1 as f64) {
            self.focus = Target::Search;
            self.search.focused = true;
            self.search.click(&m, p.0 as f64, shift);
            self.blink = std::time::Instant::now();
            return Action::None;
        }
        self.search.focused = false;
        if p.0 >= l.pane - 6
            && p.0 < l.pane
            && p.1 >= l.top
            && p.1 < l.bottom
            && self.max_scroll() > 0.0
        {
            self.drag = Some((false, p.1 as f64));
            return Action::None;
        }
        if p.0 >= self.width - 6 && p.1 >= 108 && p.1 < self.height - 36 {
            self.drag = Some((true, p.1 as f64));
            return Action::None;
        }
        if let Some(row) = self.row_at(p) {
            self.focus = Target::List;
            if let Some(entry) = self.rows.get(row).map(|&i| self.entries[i].clone()) {
                let size = if self.config.compact_list { 19 } else { 32 };
                let has_children = self
                    .entries
                    .iter()
                    .any(|e| e.details.parent.as_deref() == Some(&entry.details.id));
                let icon = p.0 - 6 <= size;
                let now = crate::hud_overlays::now_ms();
                let double = self.double_clicked(&entry.details.id, now);
                if has_children && (icon || double && !self.config.quick_configure) {
                    if !self.expanded.remove(&entry.details.id) {
                        self.expanded.insert(entry.details.id);
                    }
                    self.rebuild_rows();
                    return Action::None;
                }
                // Parent icon expansion bypasses ModListEntry.mouseClicked, including
                // both selecting the row and updating that entry's click clock.
                self.choose(row);
                self.last_click.insert(entry.details.id.clone(), now);
                if self.config.quick_configure && entry.has_config(&self.config) && (icon || double)
                {
                    return self.activate(Target::Configure, list);
                }
            }
            return Action::None;
        }
        if p.0 >= l.right && p.1 >= 108 && p.1 < self.height - 36 {
            self.focus = Target::Description;
            let i = ((p.1 - 110 + self.description_scroll as i32) / 10).max(0) as usize;
            if let Some(link) = self
                .description_lines(&m)
                .get(i)
                .and_then(|l| l.link.clone())
            {
                return Action::OpenLink(link);
            }
        }
        Action::None
    }
    pub fn release(&mut self) {
        self.drag = None;
    }
    pub fn scroll(&mut self, cursor: (f32, f32), amount: f32) {
        if self.options {
            let content = ((config::OPTIONS.len() + 1) / 2 * 24 + 8) as f64;
            self.options_scroll = (self.options_scroll - amount as f64 * 13.0)
                .clamp(0.0, (content - (self.height - 64) as f64).max(0.0));
            return;
        }
        let l = Layout::new(self.width, self.height, &self.config);
        if cursor.0 < l.pane as f32 && cursor.1 >= l.top as f32 && cursor.1 < l.bottom as f32 {
            self.scroll -= amount as f64 * (l.row / 2) as f64;
            self.clamp_scroll();
        } else if cursor.0 >= l.right as f32
            && cursor.1 >= 108.0
            && cursor.1 < (self.height - 36) as f32
        {
            let total = self.description_height.get();
            self.description_scroll = (self.description_scroll - amount as f64 * 5.0)
                .clamp(0.0, (total - (self.height - 144) as f64).max(0.0));
        }
    }
    pub fn type_text(&mut self, atlas: &HudAtlas, value: &str) {
        if self.focus == Target::Search && !self.options {
            atlas.font.using_font(crate::font::MOD_MENU, || {
                self.search.insert_text(&Measure(&atlas.font), value);
            });
            self.scroll = 0.0;
            self.rebuild_rows();
        }
    }
    pub fn shortcut(
        &mut self,
        atlas: &HudAtlas,
        shortcut: crate::clipboard::EditShortcut,
        clipboard: &mut dyn crate::clipboard::Clipboard,
    ) -> bool {
        if self.focus != Target::Search || self.options {
            return false;
        }
        atlas.font.using_font(crate::font::MOD_MENU, || {
            self.search
                .shortcut(&Measure(&atlas.font), shortcut, clipboard);
        });
        self.scroll = 0.0;
        self.rebuild_rows();
        true
    }
    pub fn key(
        &mut self,
        list: &mut ModList,
        atlas: &HudAtlas,
        key: K,
        text: Option<&str>,
        shift: bool,
    ) -> Action {
        atlas.font.using_font(crate::font::MOD_MENU, || {
            let m = Measure(&atlas.font);
            if key == K::Escape {
                if !self.pending.is_empty() {
                    self.pending.clear();
                    return Action::None;
                }
                if self.message.take().is_some() {
                    return Action::None;
                }
                if self.options {
                    self.options = false;
                    self.focus = Target::Search;
                    return Action::None;
                }
                return Action::Done;
            }
            if key == K::Tab {
                let targets = self.focus_targets(&self.buttons(&m));
                let n = targets.iter().position(|t| *t == self.focus).unwrap_or(0);
                self.focus = targets
                    [(n + if shift { targets.len() - 1 } else { 1 }) % targets.len()]
                .clone();
                self.search.focused = self.focus == Target::Search;
                return Action::None;
            }
            if self.focus == Target::Search && !self.options {
                if self.search.key(&m, key, shift, false).is_some() {
                    self.scroll = 0.0;
                    self.rebuild_rows();
                    return Action::None;
                }
                if let Some(t) = text {
                    self.search.insert_text(&m, t);
                    self.scroll = 0.0;
                    self.rebuild_rows();
                    return Action::None;
                }
            }
            if self.focus == Target::List
                && matches!(key, K::ArrowUp | K::ArrowDown | K::Home | K::End)
            {
                if !self.rows.is_empty() {
                    let n = match (self.selected_row(), key) {
                        (Some(n), K::ArrowUp) => n.saturating_sub(1),
                        (Some(n), K::ArrowDown) => (n + 1).min(self.rows.len() - 1),
                        (_, K::End) => self.rows.len() - 1,
                        _ => 0,
                    };
                    self.choose(n);
                    self.ensure_visible(n);
                }
                return Action::None;
            }
            if self.focus == Target::List
                && matches!(
                    key,
                    K::ArrowLeft | K::ArrowRight | K::Enter | K::Space | K::NumpadEnter
                )
            {
                if self.selected_row().is_some() && self.navigate_parent(key) {
                    return Action::None;
                }
            }
            if matches!(key, K::Enter | K::NumpadEnter | K::Space) && self.focus != Target::Search {
                if shift {
                    if let Some(action) = self.cycle_target(&self.focus.clone(), list, true) {
                        return action;
                    }
                }
                return self.activate(self.focus.clone(), list);
            }
            Action::None
        })
    }
    pub fn drop_files(&mut self, paths: Vec<PathBuf>) {
        if paths.iter().any(|p| {
            !p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("hmod") || e.eq_ignore_ascii_case("hnmod"))
        }) {
            self.message = Some(tr("atlas.modmenu.unsupported"));
            self.pending.clear();
        } else {
            self.message = None;
            for path in paths {
                if !self.pending.contains(&path) {
                    self.pending.push(path);
                }
            }
        }
        self.focus = Target::Confirm(false);
    }
    pub fn import_result(&mut self, success: bool) {
        self.message = Some(if success {
            tr("modmenu.dropSuccessful.line1")
                .append(Text::literal("\n"))
                .append(tr("modmenu.dropSuccessful.line2"))
        } else {
            tr("atlas.modmenu.import_fail")
        });
        self.focus = Target::Done;
    }
    pub fn typing(&self) -> bool {
        self.focus == Target::Search
            && !self.options
            && self.message.is_none()
            && self.pending.is_empty()
    }
    pub fn debug(&self) -> String {
        format!(
            "modmenu query={:?} selected={:?} entries={} rows={} scroll={} options={}",
            self.search.value,
            self.selected,
            self.entries.len(),
            self.rows.len(),
            self.scroll,
            self.options
        )
    }

    fn description_lines(&self, m: &Measure) -> Vec<Line> {
        let Some(e) = self.selected() else {
            return Vec::new();
        };
        let l = Layout::new(self.width, self.height, &self.config);
        let width = l.pane - 15;
        let mut out = Vec::new();
        let add = |out: &mut Vec<Line>, text: Text, indent: i32, link: Option<String>| {
            let continuation = if indent == 0 { 0 } else { indent + 8 };
            for (i, line) in m
                .split(&text, (width - continuation).max(1))
                .into_iter()
                .enumerate()
            {
                out.push(Line {
                    text: line,
                    indent: if i == 0 { indent } else { continuation },
                    link: link.clone(),
                });
            }
        };
        add(
            &mut out,
            Text::literal(e.description(&self.config)),
            0,
            None,
        );
        if let Some(problem) = &e.problem {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(
                &mut out,
                Text::literal(problem).with_color(0xff5555),
                0,
                None,
            );
        }
        if !e.enabled {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(
                &mut out,
                tr("atlas.modmenu.disabled").with_color(0xffaa00),
                0,
                None,
            );
        }
        if e.update(&self.config) {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(&mut out, tr("modmenu.hasUpdate"), 0, None);
            // These two update paragraphs use a fixed 8px indent on every line.
            for (text, link) in [
                (tr("modmenu.experimental").with_color(0xffaa00), None),
                (
                    tr("modmenu.downloadLink").with_style(&Style {
                        color: Some(0x5555ff),
                        underlined: Some(true),
                        ..Default::default()
                    }),
                    e.details.update_url.clone(),
                ),
            ] {
                out.extend(
                    m.split(&text, (width - 16).max(1))
                        .into_iter()
                        .map(|text| Line {
                            text,
                            indent: 8,
                            link: link.clone(),
                        }),
                );
            }
        }
        if !self.config.hide_mod_links
            && (!e.details.source.is_empty() || !e.details.links.is_empty())
        {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(&mut out, tr("modmenu.links"), 0, None);
            if !e.details.source.is_empty() {
                add(
                    &mut out,
                    tr("modmenu.source").with_style(&Style {
                        color: Some(0x5555ff),
                        underlined: Some(true),
                        ..Default::default()
                    }),
                    8,
                    Some(e.details.source.clone()),
                );
            }
            for (name, url) in &e.details.links {
                add(
                    &mut out,
                    tr(name).with_style(&Style {
                        color: Some(0x5555ff),
                        underlined: Some(true),
                        ..Default::default()
                    }),
                    8,
                    Some(url.clone()),
                );
            }
        }
        if !self.config.hide_mod_license && !e.details.licenses.is_empty() {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(&mut out, tr("modmenu.license"), 0, None);
            for license in &e.details.licenses {
                add(&mut out, Text::literal(license), 8, None);
            }
        }
        if !self.config.hide_mod_credits && !e.details.credits.is_empty() {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(&mut out, tr("modmenu.credits"), 0, None);
            for (role, names) in &e.details.credits {
                add(
                    &mut out,
                    tr(&format!("modmenu.credits.role.{role}")),
                    8,
                    None,
                );
                for name in names {
                    add(&mut out, Text::literal(name), 16, None);
                }
            }
        }
        if !e.details.dependencies.is_empty() {
            out.push(Line {
                text: Text::empty(),
                indent: 0,
                link: None,
            });
            add(&mut out, tr("atlas.modmenu.dependencies"), 0, None);
            for (id, version) in &e.details.dependencies {
                add(&mut out, Text::literal(format!("{id} {version}")), 8, None);
            }
        }
        out
    }
    pub fn draw(
        &mut self,
        list: &ModList,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        in_world: bool,
    ) {
        atlas.font.using_font(crate::font::MOD_MENU, || {
            self.sync(list, (batch.width, batch.height));
            crate::ui::menu_background(batch, atlas, 0.0, 0.0, batch.width, batch.height, in_world);
            let m = Measure(&atlas.font);
            let mouse = (cursor.0.floor() as i32, cursor.1.floor() as i32);
            let now = crate::hud_overlays::now_ms();
            let mut out = Vec::new();
            if self.message.is_some() || !self.pending.is_empty() {
                self.draw_confirmation(&mut out, &m, mouse, now);
            } else if self.options {
                self.draw_options(&mut out, &m, mouse, now, in_world);
            } else {
                self.draw_mods(&mut out, &m, mouse, now, in_world);
            }
            d::paint(batch, atlas, &out);
        });
    }
    fn draw_buttons(&self, out: &mut Vec<Draw>, m: &Measure, mouse: (i32, i32), now: i64) {
        for b in self.buttons(m) {
            if self.options
                && matches!(b.target, Target::Option(_))
                && (b.rect[1] + 20 < 32 || b.rect[1] > self.height - 32)
            {
                continue;
            }
            let hover = d::inside(mouse, b.rect[0], b.rect[1], b.rect[2], b.rect[3]);
            let focused = b.target == self.focus;
            let [x, y, w, h] = b.rect;
            let rect = [x as f32, y as f32, w as f32, h as f32];
            crate::widgets::record(&b.label.plain(), rect, b.active);
            if b.active {
                crate::widgets::target(rect);
            }
            crate::widgets::widget_cursor(hover, b.active);
            if let Some(kind) = b.icon {
                let state = if !b.active {
                    "disabled"
                } else if hover || focused {
                    "focused"
                } else {
                    "enabled"
                };
                d::sprite(out, &format!("modmenu/{kind}/{state}"), x, y, w, h);
                crate::widgets::narratable(|| {
                    crate::widgets::Narratable::button(
                        b.label.plain(),
                        [x as f32, y as f32, w as f32, h as f32],
                        b.active,
                    )
                    .focused(focused)
                });
                if hover {
                    d::tooltip(out, m, &b.label, mouse);
                }
            } else {
                d::button(
                    out,
                    m,
                    &b.label,
                    x,
                    y,
                    w,
                    h,
                    b.active,
                    hover || focused,
                    now,
                );
            }
        }
    }
    fn draw_options(
        &self,
        out: &mut Vec<Draw>,
        m: &Measure,
        mouse: (i32, i32),
        now: i64,
        in_world: bool,
    ) {
        centered(out, m, tr("modmenu.options"), self.width / 2, 12, -1);
        let prefix = if in_world { "inworld_" } else { "" };
        d::blit(
            out,
            &format!("textures/gui/{prefix}menu_list_background.png"),
            0,
            32,
            (0.0, 0.0),
            self.width,
            self.height - 64,
            [32, 32],
        );
        out.push(Draw::Scissor {
            x0: 0,
            y0: 32,
            x1: self.width,
            y1: self.height - 32,
        });
        for b in self
            .buttons(m)
            .into_iter()
            .filter(|b| matches!(b.target, Target::Option(_)))
        {
            let [x, y, w, h] = b.rect;
            if y >= 32 && y + h <= self.height - 32 {
                crate::widgets::record(
                    &b.label.plain(),
                    [x as f32, y as f32, w as f32, h as f32],
                    true,
                );
                crate::widgets::target([x as f32, y as f32, w as f32, h as f32]);
            }
            d::button(
                out,
                m,
                &b.label,
                x,
                y,
                w,
                h,
                true,
                d::inside(mouse, x, y, w, h) || self.focus == b.target,
                now,
            );
        }
        out.push(Draw::NoScissor);
        scrollbar(
            out,
            self.width / 2 + 162,
            32,
            self.height - 32,
            ((config::OPTIONS.len() + 1) / 2 * 24 + 8) as f64,
            self.options_scroll,
        );
        for b in self
            .buttons(m)
            .into_iter()
            .filter(|b| b.target == Target::Done)
        {
            let [x, y, w, h] = b.rect;
            crate::widgets::record(
                &b.label.plain(),
                [x as f32, y as f32, w as f32, h as f32],
                true,
            );
            crate::widgets::target([x as f32, y as f32, w as f32, h as f32]);
            d::button(
                out,
                m,
                &b.label,
                x,
                y,
                w,
                h,
                true,
                d::inside(mouse, x, y, w, h) || self.focus == Target::Done,
                now,
            );
        }
    }
    fn draw_confirmation(&self, out: &mut Vec<Draw>, m: &Measure, mouse: (i32, i32), now: i64) {
        let title = self
            .message
            .clone()
            .unwrap_or_else(|| tr("modmenu.dropConfirm"));
        for (i, line) in m.split(&title, self.width - 50).iter().enumerate() {
            centered(
                out,
                m,
                line.clone(),
                self.width / 2,
                self.height / 2 - 50 + i as i32 * 10,
                -1,
            );
        }
        if self.message.is_none() {
            for (i, path) in self.pending.iter().take(4).enumerate() {
                centered(
                    out,
                    m,
                    Text::literal(path.file_name().unwrap_or_default().to_string_lossy()),
                    self.width / 2,
                    self.height / 2 - 15 + i as i32 * 10,
                    0xffaaaaaau32 as i32,
                );
            }
        }
        self.draw_buttons(out, m, mouse, now);
    }
    fn draw_mods(
        &self,
        out: &mut Vec<Draw>,
        m: &Measure,
        mouse: (i32, i32),
        now: i64,
        in_world: bool,
    ) {
        let l = Layout::new(self.width, self.height, &self.config);
        centered(out, m, tr("modmenu.title"), l.pane / 2, 8, -1);
        if !self.config.disable_drag_and_drop {
            centered(
                out,
                m,
                tr("modmenu.dropInfo.line1"),
                self.width - l.pane / 2,
                14,
                0xffaaaaaau32 as i32,
            );
            centered(
                out,
                m,
                tr("modmenu.dropInfo.line2"),
                self.width - l.pane / 2,
                25,
                0xffaaaaaau32 as i32,
            );
        }
        self.search
            .draw(out, m, self.blink.elapsed().as_millis() as u64, None);
        if !self.config.config_mode {
            let [full, mods, libraries] = self.count_labels();
            let filter_width = m.width(&option_label(&self.config, "sorting"))
                + 28
                + m.width(&option_label(&self.config, "show_libraries"))
                + 20
                + 2;
            let (fx, show_count) = filters_position(
                l,
                filter_width,
                [m.width(&full), m.width(&mods), m.width(&libraries)],
                self.filters,
            );
            if show_count {
                let available = if self.filters { fx - 5 } else { l.pane - 5 };
                if m.width(&full) <= available {
                    d::text(out, full, l.search[0], 52, -1, true);
                } else {
                    d::text(out, mods, l.search[0], 46, -1, true);
                    d::text(out, libraries, l.search[0], 57, -1, true);
                }
            }
        }
        let prefix = if in_world { "inworld_" } else { "" };
        d::blit(
            out,
            &format!("textures/gui/{prefix}menu_list_background.png"),
            0,
            l.top,
            (l.pane as f32, (l.bottom as f64 + self.scroll) as f32),
            l.pane,
            l.bottom - l.top,
            [32, 32],
        );
        out.push(Draw::Scissor {
            x0: 0,
            y0: l.top,
            x1: l.pane,
            y1: l.bottom,
        });
        let row_width = l.pane - if self.max_scroll() > 0.0 { 18 } else { 12 };
        let icon_size = if self.config.compact_list { 19 } else { 32 };
        for (row, &index) in self.rows.iter().enumerate() {
            let entry = &self.entries[index];
            let y = l.top + 4 + row as i32 * l.row - self.scroll as i32;
            if y + l.row < l.top || y > l.bottom {
                continue;
            }
            let child = entry.details.parent.is_some();
            let x = 6 + if child { 12 } else { 0 };
            let selected = self.selected.as_deref() == Some(&entry.details.id);
            if selected {
                d::fill(
                    out,
                    x - 2,
                    y - 2,
                    x + row_width + 2,
                    y + l.row - 2,
                    if self.focus == Target::List {
                        -1
                    } else {
                        0xff808080u32 as i32
                    },
                );
                d::fill(
                    out,
                    x - 1,
                    y - 1,
                    x + row_width + 1,
                    y + l.row - 3,
                    0xff000000u32 as i32,
                );
            }
            icon(out, &entry.details.icon, x, y, icon_size);
            let name = Text::literal(entry.name(&self.config));
            let visible = truncated(m, name.clone(), row_width - icon_size - 3);
            d::text(out, visible, x + icon_size + 3, y + 1, -1, true);
            if !self.config.hide_badges {
                self.badges(
                    out,
                    m,
                    entry,
                    x + icon_size + 3 + m.width(&name) + 2,
                    y,
                    x + row_width,
                );
            }
            let summary = if self.config.compact_list {
                format!("v{}", entry.details.version)
            } else {
                entry.summary(&self.config)
            };
            let sx = x + icon_size + 3 + if self.config.compact_list { 0 } else { 4 };
            let lines = m.split(
                &Text::literal(summary.trim_end_matches('\n')),
                row_width - icon_size - 7,
            );
            for (i, line) in lines.iter().take(2).enumerate() {
                let mut line = line.clone();
                if i == 1 && lines.len() > 2 {
                    line = line.append(Text::literal("..."));
                }
                d::text(
                    out,
                    line,
                    sx,
                    y + 11 + i as i32 * 9,
                    0xff808080u32 as i32,
                    true,
                );
            }
            let hovered = self.row_at(mouse) == Some(row);
            let rect = [x as f32, y as f32, row_width as f32, (l.row - 4) as f32];
            crate::widgets::record_as(
                &entry.name(&self.config),
                rect,
                true,
                crate::widgets::Kind::Row,
            );
            crate::widgets::target_as(rect, crate::widgets::Kind::Row);
            if hovered && entry.has_config(&self.config) && self.config.quick_configure {
                d::fill(out, x, y, x + icon_size, y + icon_size, -1601138544);
                out.push(Draw::Blit {
                    texture: "atlas/modmenu/mod_configuration".into(),
                    x,
                    y,
                    w: icon_size,
                    h: icon_size,
                    u: 0.0,
                    v: if mouse.0 - x < icon_size {
                        icon_size as f32
                    } else {
                        0.0
                    },
                    src: [icon_size, icon_size],
                    tex: if self.config.compact_list {
                        [152, 152]
                    } else {
                        [256, 256]
                    },
                    color: -1,
                });
            }
        }
        out.push(Draw::NoScissor);
        scrollbar(
            out,
            l.pane - 6,
            l.top,
            l.bottom,
            self.content_height(),
            self.scroll,
        );
        if let Some(entry) = self.selected() {
            icon(out, &entry.details.icon, l.right, 48, 32);
            let name = truncated(m, Text::literal(entry.name(&self.config)), l.pane - 36);
            d::text(out, name.clone(), l.right + 36, 49, -1, true);
            if !self.config.hide_badges {
                self.badges(
                    out,
                    m,
                    entry,
                    l.right + 36 + m.width(&name) + 2,
                    48,
                    self.width - 28,
                );
            }
            if !entry.details.version.is_empty() {
                d::text(
                    out,
                    Text::literal(format!("v{}", entry.details.version)),
                    l.right + 36,
                    60,
                    0xffaaaaaau32 as i32,
                    true,
                );
            }
            if !entry.details.authors.is_empty() {
                let authors = translated(
                    "modmenu.authorPrefix",
                    &[Text::literal(entry.details.authors.join(", "))],
                );
                d::text(
                    out,
                    truncated(m, authors, l.pane - 40),
                    l.right + 36,
                    70,
                    0xffaaaaaau32 as i32,
                    true,
                );
            }
            if d::inside(mouse, l.right + 36, 49, m.width(&name), 9) {
                d::tooltip(
                    out,
                    m,
                    &translated("modmenu.modIdToolTip", &[Text::literal(&entry.details.id)]),
                    mouse,
                );
            }
            let lines = self.description_lines(m);
            self.description_height.set((lines.len() * 10 + 4) as f64);
            d::blit(
                out,
                &format!("textures/gui/{prefix}menu_list_background.png"),
                l.right,
                108,
                (
                    self.width as f32,
                    (self.height - 36 + self.description_scroll as i32) as f32,
                ),
                l.pane,
                self.height - 144,
                [32, 32],
            );
            out.push(Draw::Scissor {
                x0: l.right,
                y0: 108,
                x1: self.width,
                y1: self.height - 36,
            });
            for (i, line) in lines.iter().enumerate() {
                let y = 110 + i as i32 * 10 - self.description_scroll as i32;
                d::text(
                    out,
                    line.text.clone(),
                    l.right + 2 + line.indent,
                    y,
                    -1,
                    true,
                );
            }
            out.push(Draw::NoScissor);
            scrollbar(
                out,
                self.width - 6,
                108,
                self.height - 36,
                (lines.len() * 10 + 4) as f64,
                self.description_scroll,
            );
        }
        self.draw_buttons(out, m, mouse, now);
    }
    fn badges(&self, out: &mut Vec<Draw>, m: &Measure, e: &Entry, mut x: i32, y: i32, max: i32) {
        for (shown, key, border, fill) in [
            (e.details.library, "library", -15698860, -16172759),
            (e.details.client, "clientsideOnly", -13939844, -15848875),
            (e.details.deprecated, "deprecated", -8121306, -11334633),
        ] {
            if !shown {
                continue;
            }
            let text = tr(&format!("modmenu.badge.{key}"));
            let width = m.width(&text) + 6;
            if x + width >= max {
                continue;
            }
            d::fill(out, x + 1, y - 1, x + width, y, border);
            d::fill(out, x, y, x + 1, y + 9, border);
            d::fill(out, x + 1, y + 9, x + width, y + 10, border);
            d::fill(out, x + width, y, x + width + 1, y + 9, border);
            d::fill(out, x + 1, y, x + width, y + 9, fill);
            d::text(
                out,
                text.clone(),
                x + 1 + (width - m.width(&text)) / 2,
                y + 1,
                -3487030,
                false,
            );
            x += width + 3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: &str, parent: Option<&str>) -> Entry {
        Entry {
            details: model::Details {
                id: id.into(),
                name: id.into(),
                parent: parent.map(str::to_owned),
                ..Default::default()
            },
            installed_index: None,
            enabled: true,
            configurable: true,
            problem: None,
        }
    }
    fn button(target: Target) -> Button {
        Button {
            target,
            rect: [0; 4],
            label: Text::empty(),
            active: true,
            icon: None,
        }
    }
    #[test]
    fn original_screen_geometry() {
        let c = Config::default();
        let l = Layout::new(480, 270, &c);
        assert_eq!(
            (l.pane, l.right, l.top, l.bottom, l.row, l.search),
            (232, 248, 67, 234, 36, [16, 22, 178, 20])
        );
        let mut c = c;
        c.compact_list = true;
        c.config_mode = true;
        let l = Layout::new(640, 360, &c);
        assert_eq!(l.row, 23);
        assert_eq!(l.top, 48);
        assert_eq!(l.search[2], 200);
    }
    #[test]
    fn filters_center_only_when_counts_cannot_fit() {
        let c = Config::default();
        let narrow = Layout::new(480, 270, &c);
        assert_eq!(
            filters_position(narrow, 192, [120, 70, 60], true),
            (20, false)
        );
        assert_eq!(
            filters_position(narrow, 192, [120, 70, 60], false),
            (20, true)
        );
        let wide = Layout::new(1280, 720, &c);
        assert_eq!(
            filters_position(wide, 192, [120, 70, 60], true),
            (425, true)
        );
        // A combined count may need two lines without forcing centered filters.
        assert_eq!(
            filters_position(narrow, 130, [80, 20, 20], true),
            (87, true)
        );
    }
    #[test]
    fn tab_follows_original_widget_registration_and_skips_inactive_controls() {
        let menu = ModMenu::default();
        let mut buttons: Vec<_> = [
            Target::Filters,
            Target::Sort,
            Target::Libraries,
            Target::Configure,
            Target::Website,
            Target::Issues,
            Target::Folder,
            Target::Done,
        ]
        .into_iter()
        .map(button)
        .collect();
        assert_eq!(
            menu.focus_targets(&buttons),
            [
                Target::Search,
                Target::Filters,
                Target::Sort,
                Target::Libraries,
                Target::List,
                Target::Configure,
                Target::Website,
                Target::Issues,
                Target::Description,
                Target::Folder,
                Target::Done
            ]
        );
        buttons[4].active = false;
        assert!(!menu.focus_targets(&buttons).contains(&Target::Website));
        let mut menu = menu;
        menu.options = true;
        assert_eq!(
            menu.focus_targets(&[button(Target::Option(0)), button(Target::Done)]),
            [Target::Option(0), Target::Done]
        );
    }
    #[test]
    fn shift_activation_cycles_backwards_and_saves_preferences() {
        let mut menu = ModMenu::default();
        let mut list = ModList::default();
        assert_eq!(
            menu.cycle_target(&Target::Sort, &mut list, true),
            Some(Action::Changed)
        );
        assert_eq!(menu.config.sorting, config::Sorting::HasUpdate);
        assert_eq!(list.menu, menu.config);
        assert_eq!(
            menu.cycle_target(&Target::Sort, &mut list, false),
            Some(Action::Changed)
        );
        assert_eq!(menu.config.sorting, config::Sorting::Ascending);
        assert_eq!(
            menu.cycle_target(&Target::Libraries, &mut list, true),
            Some(Action::Changed)
        );
        assert_eq!(menu.config.show_libraries, config::Libraries::False);
        let i = config::OPTIONS
            .iter()
            .position(|key| *key == "sorting")
            .unwrap();
        menu.cycle_target(&Target::Option(i), &mut list, true);
        assert_eq!(list.menu.sorting, config::Sorting::HasUpdate);
    }
    #[test]
    fn double_click_uses_each_entry_clock_and_strict_250ms_limit() {
        let mut menu = ModMenu::default();
        menu.last_click.insert("a".into(), 10_000);
        menu.last_click.insert("b".into(), 10_080);
        assert!(menu.double_clicked("a", 10_150));
        assert!(menu.double_clicked("b", 10_329));
        assert!(!menu.double_clicked("b", 10_330));
        assert!(!menu.double_clicked("unknown", 10_150));
        menu.refilter();
        assert!(menu.double_clicked("a", 10_150));
        menu.rebuild_rows();
        assert!(!menu.double_clicked("a", 10_150));
    }
    #[test]
    fn filtered_out_selection_keeps_details_until_mod_is_removed() {
        let mut menu = ModMenu::default();
        menu.entries = vec![entry("alpha", None), entry("bravo", None)];
        menu.refilter();
        assert_eq!(menu.selected.as_deref(), Some("alpha"));
        menu.description_scroll = 20.0;
        menu.search.value = "bravo".into();
        menu.refilter();
        assert_eq!(menu.selected.as_deref(), Some("alpha"));
        assert_eq!(menu.selected_row(), None);
        assert_eq!(menu.description_scroll, 20.0);
        menu.search.value = "no matches".into();
        menu.refilter();
        assert!(menu.rows.is_empty());
        assert_eq!(menu.selected().unwrap().details.id, "alpha");
        menu.search.value.clear();
        menu.refilter();
        assert_eq!(menu.selected_row(), Some(0));
        menu.entries.remove(0);
        menu.refilter();
        assert_eq!(menu.selected.as_deref(), Some("bravo"));
        assert_eq!(menu.description_scroll, 0.0);
    }
    #[test]
    fn right_enters_expanded_parent_and_left_returns_from_child() {
        let mut menu = ModMenu::default();
        menu.entries = vec![entry("parent", None), entry("child", Some("parent"))];
        menu.refilter();
        assert_eq!(menu.rows, [0]);
        assert!(menu.navigate_parent(K::ArrowRight));
        assert_eq!(menu.rows, [0, 1]);
        assert_eq!(menu.selected.as_deref(), Some("parent"));
        assert!(menu.navigate_parent(K::ArrowRight));
        assert_eq!(menu.selected.as_deref(), Some("child"));
        assert!(menu.navigate_parent(K::ArrowLeft));
        assert_eq!(menu.selected.as_deref(), Some("parent"));
        assert!(menu.navigate_parent(K::ArrowLeft));
        assert_eq!(menu.rows, [0]);
        assert!(!menu.expanded.contains("parent"));
    }
}
