//! Faithful screens from ArmorIndicator 1.0.0 (ArmorIndicator Dev, CC0/MIT)
//! and Radar 1.0.0 (slmnk, MIT). Original authorship: MOD-PORT-NOTICES.md.
//! Controls and persistence are adapted from the supplied JARs.
use super::super::{
    RadarRow,
    logic::{self, RadarTurn},
};
use super::{Game, hit};
use atlas_client::{
    atlas::mod_style,
    hud::{HudAtlas, HudBatch, rgba},
    hud_kit::motion::Clock,
    lang,
    text::Text,
    ui,
    ui_theme::{Face, Ui, color},
};
use serde_json::json;
use std::{collections::BTreeMap, sync::OnceLock};
use winit::keyboard::KeyCode;

const ARMOR: &str = "atlas-armorindicator";
const RADAR: &str = "atlas-radarmod";
const RADAR_CONTROLS: usize = 9;
#[derive(Default)]
pub(super) struct State {
    id: String,
    preview: Vec<RadarRow>,
    dragging: Option<Drag>,
    focus: Option<usize>,
    sliders: [f64; 4],
}
#[derive(Clone, Copy)]
enum Drag {
    Hud(f32, f32),
    Slider(usize),
}
pub(super) fn supports(id: &str) -> bool {
    matches!(id, ARMOR | RADAR)
}
fn label(key: &str) -> String {
    static LABELS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    let defaults = LABELS.get_or_init(|| {
        serde_json::from_str(include_str!("visual_original_labels.json"))
            .expect("original screen labels")
    });
    lang::get_or(key, defaults.get(key).map(String::as_str).unwrap_or(key))
}
fn format_label(key: &str, values: &[(&str, String)]) -> String {
    let mut text = label(key);
    for (key, value) in values {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}
fn value_label(key: &str, value: impl ToString) -> String {
    format_label(key, &[("value", value.to_string())])
}
fn packed(value: i32) -> u32 {
    let c = value as u32;
    rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8)
}
fn center(batch: &mut HudBatch, atlas: &HudAtlas, text: &str, x: f32, y: f32, color: u32) {
    batch.text(
        atlas,
        text,
        x - atlas.font.width(text) / 2.0,
        y,
        color,
        true,
    );
}
fn armor_rect(index: usize, w: f32, h: f32) -> [f32; 4] {
    [
        (w as i32 / 2 - 100) as f32,
        if index == 6 {
            h - 32.0
        } else {
            40.0 + index as f32 * 24.0
        },
        200.0,
        20.0,
    ]
}
fn radar_panel(w: f32, h: f32) -> [f32; 4] {
    [
        ((w as i32 - 240) / 2) as f32,
        ((h as i32 - 204) / 2) as f32,
        240.0,
        204.0,
    ]
}
fn radar_rect(index: usize, w: f32, h: f32) -> [f32; 4] {
    let [x, y, _, _] = radar_panel(w, h);
    if index < 5 {
        [x + 10.0, y + 28.0 + index as f32 * 24.0, 220.0, 20.0]
    } else {
        let row = (index - 5) / 2;
        [
            x + 10.0 + ((index - 5) % 2) as f32 * 112.0,
            y + 148.0 + row as f32 * 26.0,
            108.0,
            20.0,
        ]
    }
}
fn arrow_focus(rects: &[[f32; 4]], current: usize, key: KeyCode) -> usize {
    let [x, y, w, h] = rects[current];
    let mut best = None;
    for (index, &[tx, ty, tw, th]) in rects.iter().enumerate() {
        let (forward, gap, overlap, sideways) = match key {
            KeyCode::ArrowDown => (
                ty >= y + h,
                ty - y - h,
                tx < x + w && tx + tw > x,
                (tx + tw / 2.0 - x - w / 2.0).abs(),
            ),
            KeyCode::ArrowUp => (
                ty + th <= y,
                y - ty - th,
                tx < x + w && tx + tw > x,
                (tx + tw / 2.0 - x - w / 2.0).abs(),
            ),
            KeyCode::ArrowRight => (
                tx >= x + w,
                tx - x - w,
                ty < y + h && ty + th > y,
                (ty + th / 2.0 - y - h / 2.0).abs(),
            ),
            KeyCode::ArrowLeft => (
                tx + tw <= x,
                x - tx - tw,
                ty < y + h && ty + th > y,
                (ty + th / 2.0 - y - h / 2.0).abs(),
            ),
            _ => return current,
        };
        if forward {
            let rank = (!overlap, gap, sideways, index);
            if best.is_none_or(|previous| rank < previous) {
                best = Some(rank);
            }
        }
    }
    best.map_or(current, |(_, _, _, index)| index)
}
fn samples(count: usize) -> Vec<RadarRow> {
    let clock = Clock::new(std::time::Instant::now(), true);
    let presets = [
        ("Japanese_jenning", 20, 1, 0),
        ("AZAMAT785665", 20, 1, 6),
        ("Fo4zir", 20, 1, 4),
        ("POPKACHETIMBA", 15, 1, 6),
        ("Itz_Dims2", 20, 4, 4),
        ("B1G_TagMagYT", 20, 4, 7),
        ("P0zornik", 20, 6, 6),
        ("angelsdeko", 20, 6, 7),
        ("Steve_Pro", 18, 8, 1),
        ("Alex_Warrior", 12, 10, 2),
        ("Shadow_Hunter", 20, 14, 3),
        ("CraftMaster", 6, 17, 5),
    ];
    (0..count)
        .map(|i| {
            let (name, hp, distance, octant) =
                presets
                    .get(i)
                    .copied()
                    .unwrap_or(("", 20, (20 + i * 3) as i32, 0));
            RadarRow {
                name: if name.is_empty() {
                    format!("Player_{}", i + 1)
                } else {
                    name.into()
                },
                hp,
                distance,
                arrow: RadarTurn::new(octant as f32 * std::f32::consts::FRAC_PI_4, clock),
                skin: "entity/player/wide/steve".into(),
                uuid: 0,
            }
        })
        .collect()
}

impl Game<'_> {
    pub(in crate::game) fn atlas_original_open_screen(&mut self, id: &str) -> bool {
        if !supports(id) || !self.atlas_open_port_screen(id) {
            return false;
        }
        let preview = if id == RADAR {
            let rows = &self.atlas_ports.visual.radar_rows;
            if rows.is_empty() {
                samples(self.atlas_ports.visual.radar.max_players)
            } else {
                rows.clone()
            }
        } else {
            Vec::new()
        };
        self.atlas_ports.visual.ui.original = State {
            id: id.into(),
            preview,
            ..Default::default()
        };
        self.original_radar_reset_sliders();
        self.atlas_ports.visual.ui.id.clear();
        if let Some(r) = self.renderer.as_ref() {
            let batch = HudBatch::new([r.config.width, r.config.height]);
            self.original_radar_clamp(batch.width, batch.height);
        }
        true
    }
    pub(in crate::game) fn atlas_original_screen_active(&self) -> bool {
        let state = &self.atlas_ports.visual.ui.original;
        !state.id.is_empty() && self.native_mod_screen() == Some(state.id.as_str())
    }
    fn original_radar_clamp(&mut self, w: f32, h: f32) {
        if self.atlas_ports.visual.ui.original.id != RADAR {
            return;
        }
        let count = self.atlas_ports.visual.ui.original.preview.len();
        let c = &mut self.atlas_ports.visual.radar;
        let width = (c.hud_width as f32 * c.scale) as i32;
        let height = ((if count == 0 { 40 } else { 26 + count * 24 }) as f32 * c.scale) as i32;
        c.x = c.x.clamp(0, (w as i32 - width).max(0));
        c.y = c.y.clamp(0, (h as i32 - height).max(0));
    }
    fn original_radar_bounds(&self) -> [f32; 4] {
        let c = &self.atlas_ports.visual.radar;
        let count = self.atlas_ports.visual.ui.original.preview.len();
        [
            c.x as f32,
            c.y as f32,
            (c.hud_width as f32 * c.scale) as i32 as f32,
            ((if count == 0 { 40 } else { 26 + count * 24 }) as f32 * c.scale) as i32 as f32,
        ]
    }
    fn original_radar_reset_sliders(&mut self) {
        let c = &self.atlas_ports.visual.radar;
        self.atlas_ports.visual.ui.original.sliders = [
            ((c.scale - 0.5) / 1.5) as f64,
            (c.max_players as f64 - 1.0) / 19.0,
            (c.max_distance - 10.0) / 190.0,
            (c.background_opacity as f64 - 0.1) / 0.9,
        ];
    }
    fn original_radar_slider(&self, index: usize) -> f64 {
        self.atlas_ports.visual.ui.original.sliders[index - 1]
    }
    fn original_radar_set_slider(&mut self, index: usize, value: f64, w: f32, h: f32) {
        let value = value.clamp(0.0, 1.0);
        self.atlas_ports.visual.ui.original.sliders[index - 1] = value;
        let c = &mut self.atlas_ports.visual.radar;
        match index {
            1 => c.scale = ((0.5 + value as f32 * 1.5) * 20.0).round() / 20.0,
            2 => {
                c.max_players = 1 + (value * 19.0).round() as usize;
                if self.atlas_ports.visual.ui.original.preview.len() != c.max_players {
                    self.atlas_ports.visual.ui.original.preview = samples(c.max_players);
                }
            }
            3 => c.max_distance = 10.0 + (value * 190.0).round(),
            4 => c.background_opacity = (0.1 + value * 0.9) as f32,
            _ => return,
        }
        self.original_radar_clamp(w, h);
    }
    fn original_update_drag(&mut self, w: f32, h: f32) {
        let (x, y) = self.cursor;
        match self.atlas_ports.visual.ui.original.dragging {
            Some(Drag::Hud(dx, dy)) => {
                let c = &mut self.atlas_ports.visual.radar;
                c.x = x as i32 - dx as i32;
                c.y = y as i32 - dy as i32;
                self.original_radar_clamp(w, h);
            }
            Some(Drag::Slider(index)) => {
                let [sx, _, sw, _] = radar_rect(index, w, h);
                self.original_radar_set_slider(
                    index,
                    ((x as f64 - (sx as f64 + 4.0)) / (sw as f64 - 8.0)).clamp(0.0, 1.0),
                    w,
                    h,
                );
            }
            None => {}
        }
    }
    fn original_close_screen(&mut self) {
        let id = self.atlas_ports.visual.ui.original.id.clone();
        if let Some(value) = self.atlas_editor_config(&id) {
            let _ = self.atlas_editor_set(&id, value);
        }
        self.atlas_ports.visual.ui.original = State::default();
        self.native_mod_screen_close(&id);
    }
    pub(in crate::game) fn atlas_original_screen_render(&mut self, batch: &mut HudBatch) -> bool {
        if !self.atlas_original_screen_active() {
            return false;
        }
        self.original_update_drag(batch.width, batch.height);
        let atlas = &self.gui.atlas;
        batch.fill_gradient(
            atlas,
            (0.0, 0.0, batch.width, batch.height),
            packed(-1072689136),
            packed(-804253680),
        );
        if self.atlas_ports.visual.ui.original.id == ARMOR {
            self.original_armor_render(batch);
        } else {
            self.original_radar_render(batch);
        }
        true
    }
    fn original_armor_render(&self, batch: &mut HudBatch) {
        let atlas = &self.gui.atlas;
        let config = self.atlas_armor_config(ARMOR).unwrap_or_default();
        let on = |key: &str| {
            label(if config[key].as_bool().unwrap_or(false) {
                "atlas.original.armor.on"
            } else {
                "atlas.original.armor.off"
            })
        };
        let mode = match (
            config["showDurabilityNumbers"].as_bool() == Some(true),
            config["showDurabilityPercentage"].as_bool() == Some(true),
        ) {
            (true, true) => "atlas.original.armor.format_both",
            (_, true) => "atlas.original.armor.format_percentage",
            _ => "atlas.original.armor.format_numbers",
        };
        let labels = [
            value_label(
                "atlas.original.armor.status",
                label(if config["enabled"].as_bool() == Some(true) {
                    "atlas.original.armor.enabled"
                } else {
                    "atlas.original.armor.disabled"
                }),
            ),
            value_label("atlas.original.armor.tint", on("visualDamageTint")),
            value_label(
                "atlas.original.armor.intensity",
                (config["maxOverlayAlpha"].as_f64().unwrap_or(0.55) * 100.0).round() as i32,
            ),
            value_label("atlas.original.armor.hud", on("targetHudEnabled")),
            value_label("atlas.original.armor.format", label(mode)),
            value_label(
                "atlas.original.armor.reach",
                config["targetRange"].as_f64().unwrap_or(24.0) as i32,
            ),
            lang::get("gui.done"),
        ];
        center(
            batch,
            atlas,
            &label("atlas.original.armor.title"),
            (batch.width as i32 / 2) as f32,
            16.0,
            u32::MAX,
        );
        for (i, text) in labels.iter().enumerate() {
            let [x, y, w, h] = armor_rect(i, batch.width, batch.height);
            let focused = self.atlas_ports.visual.ui.original.focus == Some(i);
            atlas_client::atlas::mod_style::button(
                batch,
                atlas,
                x,
                y,
                w,
                h,
                &Text::legacy(text),
                true,
                hit([x, y, w, h], self.cursor.0, self.cursor.1) || focused,
            );
        }
        center(
            batch,
            atlas,
            &label("atlas.original.armor.hint"),
            (batch.width as i32 / 2) as f32,
            batch.height - 50.0,
            packed(-5592406),
        );
        for (i, key) in ["status", "tint", "intensity", "hud", "format", "reach"]
            .into_iter()
            .enumerate()
        {
            if hit(
                armor_rect(i, batch.width, batch.height),
                self.cursor.0,
                self.cursor.1,
            ) {
                let tooltip = Text::literal(label(&format!("atlas.original.armor.tooltip.{key}")));
                let lines = atlas.font.split_formatted(&tooltip, 170.0);
                ui::tooltip(batch, atlas, &lines, self.cursor);
            }
        }
    }
    fn original_radar_render(&self, batch: &mut HudBatch) {
        let atlas = &self.gui.atlas;
        let state = &self.atlas_ports.visual.ui.original;
        let c = &self.atlas_ports.visual.radar;
        self.original_radar_preview(batch);
        let [x, y, w, h] = self.original_radar_bounds();
        let hovered = hit([x, y, w, h], self.cursor.0, self.cursor.1);
        let dragging = matches!(state.dragging, Some(Drag::Hud(_, _)));
        let [px, py, pw, ph] = radar_panel(batch.width, batch.height);
        let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
        if hovered || dragging {
            view.border(x, y, w, h, 10.0 * c.scale, color::accent());
        }
        let hint = label(if dragging {
            "atlas.original.radar.dragging"
        } else {
            "atlas.original.radar.hover"
        });
        if hovered || dragging {
            let width = view.measure(Face::Medium, 9.0, &hint) + 12.0;
            let top = (y - 20.0).max(1.0);
            view.rounded(x, top, width, 16.0, 8.0, color::bg());
            view.text_line(Face::Medium, 9.0, 16.0, x + 6.0, top, color::text(), &hint);
        }
        view.finish();
        mod_style::panel(batch, atlas, [px, py, pw, ph], 15.0);
        let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
        view.text_line(
            Face::SemiBold,
            13.0,
            28.0,
            px + 13.0,
            py,
            color::text(),
            &label("atlas.original.radar.title"),
        );
        let coordinates = format_label(
            "atlas.original.radar.coordinates",
            &[("x", c.x.to_string()), ("y", c.y.to_string())],
        );
        let width = view.measure(Face::Mono, 9.0, &coordinates);
        view.text_line(
            Face::Mono,
            9.0,
            27.0,
            px + pw - width - 12.0,
            py,
            color::muted(),
            &coordinates,
        );
        view.finish();
        let boolean = |value| {
            label(if value {
                "atlas.original.radar.yes"
            } else {
                "atlas.original.radar.no"
            })
        };
        let labels = [
            label(if c.enabled {
                "atlas.original.radar.enabled"
            } else {
                "atlas.original.radar.disabled"
            }),
            format_label(
                "atlas.original.radar.scale",
                &[
                    ("percent", ((c.scale * 100.0) as i32).to_string()),
                    (
                        "scale",
                        format!("{:.2}", c.scale)
                            .replace('.', atlas_threads::locale::decimal_separator()),
                    ),
                ],
            ),
            value_label("atlas.original.radar.players", c.max_players),
            value_label("atlas.original.radar.distance", c.max_distance as i32),
            value_label(
                "atlas.original.radar.opacity",
                (c.background_opacity * 100.0) as i32,
            ),
            value_label("atlas.original.radar.health", boolean(c.show_health)),
            value_label("atlas.original.radar.arrows", boolean(c.show_arrows)),
            label("atlas.original.radar.reset"),
            label("atlas.original.radar.done"),
        ];
        for (i, text) in labels.into_iter().enumerate() {
            let [x, y, w, h] = radar_rect(i, batch.width, batch.height);
            let hover = hit([x, y, w, h], self.cursor.0, self.cursor.1);
            if (1..=4).contains(&i) {
                mod_style::slider(
                    batch,
                    atlas,
                    x,
                    y,
                    w,
                    &Text::legacy(&text),
                    self.original_radar_slider(i) as f32,
                    hover || state.focus == Some(i),
                    true,
                    matches!(state.dragging, Some(Drag::Slider(index)) if index == i),
                );
            } else {
                mod_style::button(
                    batch,
                    atlas,
                    x,
                    y,
                    w,
                    h,
                    &Text::legacy(&text),
                    true,
                    hover || state.focus == Some(i),
                );
            }
        }
    }
    fn original_radar_preview(&self, batch: &mut HudBatch) {
        let v = &self.atlas_ports.visual;
        let rows = &v.ui.original.preview;
        let count = if rows.iter().all(|r| r.uuid == 0) {
            rows.len()
        } else {
            v.radar_count
        };
        self.draw_radar(batch, rows, count);
    }

    pub(in crate::game) fn atlas_visual_radar_draw(&self, batch: &mut HudBatch) {
        let v = &self.atlas_ports.visual;
        self.draw_radar(batch, &v.radar_rows, v.radar_count);
    }

    fn draw_radar(&self, batch: &mut HudBatch, rows: &[RadarRow], count: usize) {
        let atlas = &self.gui.atlas;
        let c = &self.atlas_ports.visual.radar;
        let (x, y, w) = (c.x as f32, c.y as f32, c.hud_width as f32);
        let h = if rows.is_empty() {
            40.0
        } else {
            26.0 + 24.0 * rows.len() as f32
        };
        let start = batch.vertices.len();
        let clock = self.hud_clock();
        let mut view = Ui::at_gui_scale(atlas, batch, self.cursor);
        view.rounded(
            x,
            y,
            w,
            h,
            10.0,
            color::with_alpha(color::bg(), c.background_opacity),
        );
        view.border(
            x,
            y,
            w,
            h,
            10.0,
            color::with_alpha(color::text(), c.background_opacity * 0.23),
        );
        let accent = rgba(146, 186, 247, 255);
        let count = count.to_string();
        let count_width = (view.measure(Face::Mono, 9.0, &count) + 10.0).max(16.0);
        let count_x = x + w - count_width - 9.0;
        let title_start = view.batch.vertices.len();
        view.text_line(
            Face::SemiBold,
            10.5,
            22.0,
            x + 10.0,
            y,
            color::text(),
            &label("atlas.original.radar.nearby"),
        );
        view.batch
            .clip_since(title_start, x + 9.0, y, count_x - 4.0, y + 22.0);
        view.rounded(count_x, y + 4.0, count_width, 14.0, 7.0, color::raised());
        view.text_centered(
            Face::Mono,
            9.0,
            count_x + count_width / 2.0,
            y + 4.0,
            color::text(),
            &count,
        );
        view.rounded(
            x + 8.0,
            y + 22.0,
            w - 16.0,
            1.0,
            0.0,
            color::with_alpha(color::text(), c.background_opacity * 0.18),
        );

        if rows.is_empty() {
            view.text_line(
                Face::Regular,
                9.5,
                16.0,
                x + 10.0,
                y + 23.0,
                color::muted(),
                &label("atlas.original.radar.empty"),
            );
        }
        for (i, row) in rows.iter().enumerate() {
            let ry = y + 24.0 + 24.0 * i as f32;
            view.rounded(x + 7.0, ry + 3.0, 18.0, 18.0, 5.0, color::raised());
            if let Some(skin) = atlas.sprites.get(&row.skin).copied() {
                for (u, v) in [(8.0, 8.0), (40.0, 8.0)] {
                    let mut face = skin;
                    let du = (skin.u1 - skin.u0) / 64.0;
                    let dv = (skin.v1 - skin.v0) / 64.0;
                    face.u0 = skin.u0 + du * u;
                    face.v0 = skin.v0 + dv * v;
                    face.u1 = face.u0 + du * 8.0;
                    face.v1 = face.v0 + dv * 8.0;
                    view.batch
                        .rect_uv(x + 8.0, ry + 4.0, 16.0, 16.0, face, u32::MAX);
                }
            } else {
                let initial = row
                    .name
                    .chars()
                    .next()
                    .unwrap_or(' ')
                    .to_uppercase()
                    .to_string();
                view.text_centered(Face::SemiBold, 9.0, x + 16.0, ry + 5.0, accent, &initial);
            }

            let mut trailing = x + w - 9.0;
            if c.show_arrows {
                let centre = [trailing - 5.0, ry + 12.0];
                let width = HudBatch::text_width(atlas, "↑");
                let arrow_start = view.batch.vertices.len();
                self.native_text(
                    view.batch,
                    "↑",
                    [centre[0] - width / 2.0, ry + 7.0],
                    0xa855f7,
                    true,
                );
                let (sin, cos) = row.arrow.angle(clock).sin_cos();
                for v in &mut view.batch.vertices[arrow_start..] {
                    let dx = v.pos[0] - centre[0];
                    let dy = v.pos[1] - centre[1];
                    v.pos = [
                        centre[0] + dx * cos - dy * sin,
                        centre[1] + dx * sin + dy * cos,
                    ];
                }
                trailing -= 22.0;
            }
            if c.show_distance {
                let distance = value_label("atlas.original.radar.distance_value", row.distance);
                trailing -= view.measure(Face::Mono, 9.0, &distance);
                view.text_line(
                    Face::Mono,
                    9.0,
                    24.0,
                    trailing,
                    ry,
                    color::muted(),
                    &distance,
                );
                trailing -= 7.0;
            }

            let name_x = x + 32.0;
            let mut name = row.name.clone();
            let available = trailing - name_x;
            if view.measure(Face::Medium, 10.0, &name) > available {
                let target = available - view.measure(Face::Medium, 10.0, "…");
                while !name.is_empty() && view.measure(Face::Medium, 10.0, &name) > target {
                    name.pop();
                }
                if target >= 0.0 {
                    name.push('…');
                }
            }
            if c.show_health {
                view.text_line(Face::Medium, 10.0, 13.0, name_x, ry, color::text(), &name);
                let hp = format!("{}{}", label("atlas.original.radar.health_label"), row.hp);
                view.text_line(
                    Face::Regular,
                    8.5,
                    10.0,
                    name_x,
                    ry + 12.0,
                    color::hex(logic::health_color(row.hp), 255),
                    &hp,
                );
            } else {
                view.text_line(Face::Medium, 10.0, 24.0, name_x, ry, color::text(), &name);
            }
        }
        view.finish();
        batch.scale_since(start, x, y, c.scale, c.scale);
    }
    fn original_activate(&mut self, index: usize, w: f32, h: f32) {
        if self.atlas_ports.visual.ui.original.id == ARMOR {
            if index == 6 {
                self.original_close_screen();
                return;
            }
            let Some(mut c) = self.atlas_armor_config(ARMOR) else {
                return;
            };
            match index {
                0 | 1 | 3 => {
                    let key = match index {
                        0 => "enabled",
                        1 => "visualDamageTint",
                        _ => "targetHudEnabled",
                    };
                    c[key] = json!(!c[key].as_bool().unwrap_or(false));
                }
                2 => {
                    let alpha = c["maxOverlayAlpha"].as_f64().unwrap_or(0.55);
                    c["maxOverlayAlpha"] = json!(if alpha <= 0.35 {
                        0.55
                    } else if alpha <= 0.6 {
                        0.85
                    } else if alpha <= 0.9 {
                        1.0
                    } else {
                        0.35
                    });
                }
                4 => {
                    let numbers = c["showDurabilityNumbers"].as_bool() == Some(true);
                    let pct = c["showDurabilityPercentage"].as_bool() == Some(true);
                    let (numbers, pct) = if numbers && pct {
                        (false, true)
                    } else if !numbers && pct {
                        (true, false)
                    } else {
                        (true, true)
                    };
                    c["showDurabilityNumbers"] = json!(numbers);
                    c["showDurabilityPercentage"] = json!(pct);
                }
                5 => {
                    c["targetRange"] =
                        json!(match c["targetRange"].as_f64().unwrap_or(24.0) as i32 {
                            16 => 24,
                            24 => 32,
                            32 => 48,
                            _ => 16,
                        });
                }
                _ => return,
            }
            let _ = self.atlas_armor_set_config(ARMOR, c);
        } else {
            let c = &mut self.atlas_ports.visual.radar;
            match index {
                0 => c.enabled = !c.enabled,
                5 => c.show_health = !c.show_health,
                6 => c.show_arrows = !c.show_arrows,
                7 => {
                    *c = super::super::config::Radar::default();
                    self.atlas_ports.visual.ui.original.preview = samples(c.max_players);
                    self.atlas_ports.visual.ui.original.dragging = None;
                    self.original_radar_reset_sliders();
                    self.original_radar_clamp(w, h);
                }
                8 => self.original_close_screen(),
                _ => {}
            }
        }
    }
    pub(in crate::game) fn atlas_original_screen_mouse(
        &mut self,
        button: i32,
        pressed: bool,
    ) -> bool {
        if !self.atlas_original_screen_active() {
            return false;
        }
        if button != 0 {
            return true;
        }
        let Some(renderer) = self.renderer.as_ref() else {
            return true;
        };
        let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
        let (w, h) = (batch.width, batch.height);
        if !pressed {
            self.original_update_drag(w, h);
            self.atlas_ports.visual.ui.original.dragging = None;
            return true;
        }
        let (x, y) = self.cursor;
        let armor = self.atlas_ports.visual.ui.original.id == ARMOR;
        let chosen = (0..if armor { 7 } else { RADAR_CONTROLS }).find(|i| {
            hit(
                if armor {
                    armor_rect(*i, w, h)
                } else {
                    radar_rect(*i, w, h)
                },
                x,
                y,
            )
        });
        self.atlas_ports.visual.ui.original.focus = chosen;
        if let Some(index) = chosen {
            if !armor && (1..=4).contains(&index) {
                self.atlas_ports.visual.ui.original.dragging = Some(Drag::Slider(index));
                self.original_update_drag(w, h);
            } else {
                self.original_activate(index, w, h)
            }
        } else if !armor && hit(self.original_radar_bounds(), x, y) {
            let c = &self.atlas_ports.visual.radar;
            self.atlas_ports.visual.ui.original.dragging =
                Some(Drag::Hud((x as i32 - c.x) as f32, (y as i32 - c.y) as f32));
        }
        true
    }
    pub(in crate::game) fn atlas_original_screen_key(
        &mut self,
        key: KeyCode,
        pressed: bool,
        shift: bool,
    ) -> bool {
        if !self.atlas_original_screen_active() {
            return false;
        }
        if !pressed {
            return true;
        }
        if key == KeyCode::Escape {
            self.original_close_screen();
            return true;
        }
        let armor = self.atlas_ports.visual.ui.original.id == ARMOR;
        let count = if armor { 7 } else { RADAR_CONTROLS };
        if key == KeyCode::Tab {
            let backwards = shift;
            let s = &mut self.atlas_ports.visual.ui.original;
            s.focus = Some(match s.focus {
                None => {
                    if backwards {
                        count - 1
                    } else {
                        0
                    }
                }
                Some(index) => {
                    if backwards {
                        (index + count - 1) % count
                    } else {
                        (index + 1) % count
                    }
                }
            });
            return true;
        }
        let index = self.atlas_ports.visual.ui.original.focus.unwrap_or(0);
        let Some(renderer) = self.renderer.as_ref() else {
            return true;
        };
        let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
        let (w, h) = (batch.width, batch.height);
        if !armor
            && (1..=4).contains(&index)
            && matches!(key, KeyCode::ArrowLeft | KeyCode::ArrowRight)
        {
            let value = self.original_radar_slider(index)
                + if key == KeyCode::ArrowRight {
                    1.0 / 212.0
                } else {
                    -1.0 / 212.0
                };
            self.original_radar_set_slider(index, value, w, h);
        } else if matches!(
            key,
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
        ) {
            let rects = (0..count)
                .map(|i| {
                    if armor {
                        armor_rect(i, w, h)
                    } else {
                        radar_rect(i, w, h)
                    }
                })
                .collect::<Vec<_>>();
            self.atlas_ports.visual.ui.original.focus =
                Some(if self.atlas_ports.visual.ui.original.focus.is_some() {
                    arrow_focus(&rects, index, key)
                } else {
                    0
                });
        } else if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
            if self.atlas_ports.visual.ui.original.focus.is_some() {
                self.original_activate(index, w, h)
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_java_layout_and_sample_rows() {
        assert_eq!(armor_rect(0, 854.0, 480.0), [327.0, 40.0, 200.0, 20.0]);
        assert_eq!(armor_rect(5, 854.0, 480.0), [327.0, 160.0, 200.0, 20.0]);
        assert_eq!(armor_rect(6, 854.0, 480.0), [327.0, 448.0, 200.0, 20.0]);
        assert_eq!(radar_panel(854.0, 480.0), [307.0, 138.0, 240.0, 204.0]);
        assert_eq!(radar_rect(7, 854.0, 480.0), [317.0, 312.0, 108.0, 20.0]);
        assert_eq!(radar_rect(8, 854.0, 480.0), [429.0, 312.0, 108.0, 20.0]);
        let rows = samples(14);
        assert_eq!(rows.len(), 14);
        assert_eq!(rows[0].name, "Japanese_jenning");
        assert_eq!(rows[3].hp, 15);
        assert_eq!(rows[12].name, "Player_13");
        assert_eq!(rows[12].distance, 56);
    }
    #[test]
    fn original_spatial_arrow_navigation() {
        let radar = (0..RADAR_CONTROLS)
            .map(|i| radar_rect(i, 480.0, 300.0))
            .collect::<Vec<_>>();
        for (from, key, expected) in [
            (5, KeyCode::ArrowDown, 7),
            (7, KeyCode::ArrowDown, 7),
            (8, KeyCode::ArrowUp, 6),
            (5, KeyCode::ArrowRight, 6),
            (6, KeyCode::ArrowLeft, 5),
            (0, KeyCode::ArrowUp, 0),
            (8, KeyCode::ArrowDown, 8),
        ] {
            assert_eq!(arrow_focus(&radar, from, key), expected);
        }
        let armor = (0..7)
            .map(|i| armor_rect(i, 480.0, 300.0))
            .collect::<Vec<_>>();
        assert_eq!(arrow_focus(&armor, 5, KeyCode::ArrowDown), 6);
        assert_eq!(arrow_focus(&armor, 0, KeyCode::ArrowLeft), 0);
    }
}
