//! Pause menu bar based on ESC Music Controller 1.0.0, AntiJOPA (MIT).
use super::{
    Game,
    esc_music::{Command, ID, Track},
};
use crate::game::Screen;
use atlas_client::{
    hud::HudBatch,
    lang,
    ui_theme::{Face, Ui, color},
    widgets,
};
use std::{borrow::Cow, sync::Arc, time::Instant};
use winit::{event::MouseButton, keyboard::KeyCode};

type Bounds = [f32; 4];

const PILL_H: f32 = 24.0;
const MIN_WIDTH: f32 = 168.0;
const TITLE_SIZE: f32 = 10.5;
const PLAYBACK: [(u8, Command); 4] = [
    (20, Command::Previous),
    (21, Command::Toggle),
    (22, Command::Next),
    (25, Command::RepeatOne),
];
const SEEK_ID: u8 = 23;
const PILL_ID: u8 = 24;
const VOLUME_ID: u8 = 26;

struct VolumeDrag {
    id: Arc<str>,
    level: f32,
}

#[derive(Default)]
pub(super) struct State {
    seek_fraction: Option<f64>,
    volume_drag: Option<VolumeDrag>,
    title: String,
    title_since: Option<Instant>,
    expansion: f32,
    last_frame: Option<Instant>,
    pinned: bool,
    keyboard: bool,
}

struct Layout {
    pill: Bounds,
    card: Bounds,
    cover: Bounds,
    buttons: [Bounds; 4],
    caption: Bounds,
    rail: Bounds,
    seek_end: f32,
    volume: Bounds,
}

impl Layout {
    fn new(size: (f32, f32), pause_bottom: f32, duration: u64) -> Self {
        let width = (size.0 - 16.0).clamp(0.0, 228.0);
        let x = ((size.0 - width) / 2.0).floor();
        let bottom = size.1 - 8.0;
        let room = bottom - pause_bottom - 5.0;
        let top = room < 42.0;
        let tall = room >= 78.0;
        let height = if tall {
            86.0_f32.min(room)
        } else if top {
            38.0
        } else {
            42.0
        };
        let y = if top { 1.0 } else { bottom - height };
        let pill_x = ((size.0 - 124.0) / 2.0).floor();
        let pill_y = if top { y } else { bottom - PILL_H };
        let text_x = x + if tall { 51.0 } else { 39.0 };
        let buttons_x = if tall {
            x + (width - 103.0) / 2.0
        } else {
            x + width - 102.0
        };
        let button_y = y + if tall { height - 38.0 } else { 5.0 };
        let text_edge = if tall {
            x + width - 10.0
        } else {
            buttons_x - 3.0
        };
        let time_width = if duration >= 3_600_000 { 41.0 } else { 33.0 };
        let shared = !tall && duration > 0;
        let seek_span = if shared { width * 0.60 } else { width };
        let volume_left = if shared { seek_span } else { 0.0 };
        Self {
            pill: [pill_x, pill_y, 124.0, PILL_H],
            card: [x, y, width, height],
            cover: if tall {
                [x + 10.0, y + 7.0, 32.0, 32.0]
            } else if top {
                [x + 10.0, y + 6.0, 20.0, 20.0]
            } else {
                [x + 10.0, y + 6.0, 24.0, 24.0]
            },
            buttons: std::array::from_fn(|i| {
                [
                    buttons_x + i as f32 * 23.0 + if i == 3 { 5.0 } else { 0.0 },
                    button_y,
                    22.0,
                    21.0,
                ]
            }),
            caption: [
                text_x,
                y + if tall { 9.0 } else { 5.0 },
                text_edge - text_x,
                11.5,
            ],
            rail: [
                x + time_width,
                y + if tall { height - 42.0 } else { height - 8.0 },
                seek_span - time_width * 2.0,
                2.0,
            ],
            seek_end: x + seek_span,
            volume: [
                x + volume_left + 22.0,
                y + height - 8.0,
                width - volume_left - 50.0,
                2.0,
            ],
        }
    }

    fn visible(&self) -> bool {
        self.card[2] >= MIN_WIDTH
    }

    fn panel(&self, expansion: f32) -> Bounds {
        std::array::from_fn(|i| self.pill[i] + (self.card[i] - self.pill[i]) * expansion)
    }

    fn seek_target(&self) -> Bounds {
        let [x, y, w, h] = self.rail;
        [x - 3.0, y - 4.0, w + 6.0, h + 8.0]
    }

    fn seek_fraction(&self, x: f32) -> f64 {
        f64::from(((x - self.rail[0]) / self.rail[2]).clamp(0.0, 1.0))
    }

    fn volume_target(&self) -> Bounds {
        let [x, y, w, h] = self.volume;
        [x - 3.0, y - 4.0, w + 6.0, h + 8.0]
    }

    fn volume_level(&self, x: f32) -> f32 {
        ((x - self.volume[0]) / self.volume[2]).clamp(0.0, 1.0)
    }
}

fn position(track: &Track, now: Instant) -> u64 {
    let elapsed = if track.playing {
        now.saturating_duration_since(track.sampled_at).as_millis() as u64
    } else {
        0
    };
    track
        .position_ms
        .saturating_add(elapsed)
        .min(track.duration_ms)
}

fn time_label(ms: u64) -> String {
    let seconds = ms / 1000;
    let hours = seconds / 3600;
    let minutes = seconds / 60 % 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn seek_position(fraction: f64, duration: u64) -> u64 {
    // The reference bridge accepts seconds rounded to two decimal places.
    let ten_ms_steps = (fraction * duration as f64 / 10.0).round() as u64;
    ten_ms_steps.saturating_mul(10).min(duration)
}

fn marquee_offset(overflow: f32, elapsed: f32) -> f32 {
    if overflow <= 0.0 {
        return 0.0;
    }
    let forward = overflow / 22.0;
    let back = (overflow / 44.0).max(0.2);
    let moving = elapsed % (2.0 + forward + 0.75 + back) - 2.0;
    if moving < forward {
        moving.max(0.0) * 22.0
    } else {
        let t = ((moving - forward - 0.75) / back).clamp(0.0, 1.0);
        overflow * (1.0 - t * t * (3.0 - 2.0 * t))
    }
}

fn contains([x, y, w, h]: Bounds, point: (f32, f32)) -> bool {
    point.0 >= x && point.0 <= x + w && point.1 >= y && point.1 <= y + h
}

fn clipped_line(
    ui: &mut Ui<'_>,
    face: Face,
    size: f32,
    [x, y, w, h]: Bounds,
    text: &str,
    color: u32,
    scroll: f32,
) {
    let start = ui.batch.vertices.len();
    ui.text_line(face, size, h, x - scroll, y, color, text);
    ui.batch.clip_since(start, x, y, x + w, y + h);
}

fn media_icon(ui: &mut Ui<'_>, rect: Bounds, control: usize, playing: bool, ink: u32) {
    let [x, y, w, h] = rect;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    if control == 3 {
        for (lead, direction) in [(cy - 3.5, 1.0), (cy + 2.5, -1.0)] {
            let tip = cx + 5.0 * direction;
            let base = cx + 1.0 * direction;
            ui.rounded(cx - 4.5, lead, 9.0, 1.2, 0.6, ink);
            let centre = [tip, lead + 0.6];
            ui.batch.fill_quad(
                ui.atlas,
                [[base, lead - 1.6], centre, centre, [base, lead + 2.8]],
                ink,
            );
        }
        return;
    }
    if control == 1 && playing {
        ui.rounded(cx - 3.75, cy - 4.75, 2.5, 9.5, 0.8, ink);
        ui.rounded(cx + 1.25, cy - 4.75, 2.5, 9.5, 0.8, ink);
        return;
    }
    let direction = if control == 0 { -1.0 } else { 1.0 };
    let tip = cx + 4.0 * direction;
    let base = cx - 3.0 * direction;
    let center = [tip, cy];
    ui.batch.fill_quad(
        ui.atlas,
        [[base, cy - 4.5], center, center, [base, cy + 4.5]],
        ink,
    );
    if control != 1 {
        ui.rounded(tip + 0.8 * direction - 0.7, cy - 4.3, 1.4, 8.6, 0.7, ink);
    }
}

fn draw_cover(ui: &mut Ui<'_>, rect: Bounds, track: Option<&Track>, alpha: f32) {
    let [x, y, w, h] = rect;
    ui.rounded(
        x,
        y,
        w,
        h,
        w * (7.0 / 24.0),
        color::with_alpha(color::raised(), alpha),
    );
    if let Some(cover) = track.and_then(|track| track.cover.clone()) {
        ui.batch
            .draw_picture(cover, rect, color::with_alpha(u32::MAX, alpha));
        return;
    }
    let ink = color::with_alpha(color::muted(), alpha);
    ui.rounded(
        x + w * 0.28,
        y + h * 0.54,
        w * 0.30,
        h * 0.22,
        h * 0.11,
        ink,
    );
    ui.rounded(x + w * 0.50, y + h * 0.27, w * 0.08, h * 0.37, 0.7, ink);
    ui.rounded(x + w * 0.50, y + h * 0.27, w * 0.23, h * 0.09, 0.7, ink);
}

fn media_focus(id: u8) -> bool {
    matches!(id, PILL_ID | SEEK_ID | VOLUME_ID) || PLAYBACK.iter().any(|(other, _)| *other == id)
}

fn draw_volume(
    ui: &mut Ui<'_>,
    layout: &Layout,
    level: Option<f32>,
    dragging: bool,
    focused: bool,
    alpha: f32,
    cursor: (f32, f32),
) {
    let [x, y, w, h] = layout.volume;
    let target = layout.volume_target();
    let ready = alpha >= 0.52 && level.is_some();
    let hovered = contains(target, cursor);
    let fraction = level.unwrap_or(0.0);
    if alpha >= 0.52 {
        let label = lang::get(if ready {
            "atlas.esc_music.volume"
        } else {
            "atlas.esc_music.volume_unavailable"
        });
        let kind = widgets::Kind::Slider {
            value: fraction,
            track: [x, w],
        };
        widgets::record_as(&label, target, ready, kind);
        if ready {
            widgets::target_as(target, kind);
            widgets::slider_cursor(hovered || dragging, true, dragging);
        }
        widgets::narratable(|| widgets::Narratable::slider(label, target, ready).hovered(hovered));
    }
    let dim = color::with_alpha(color::muted(), alpha);
    let ink = color::with_alpha(color::text(), alpha);
    let cy = y + h / 2.0;
    ui.rounded(x - 16.0, cy - 1.5, 3.0, 3.0, 0.5, dim);
    ui.batch.fill_quad(
        ui.atlas,
        [
            [x - 14.0, cy - 1.5],
            [x - 10.0, cy - 3.8],
            [x - 10.0, cy + 3.8],
            [x - 14.0, cy + 1.5],
        ],
        dim,
    );
    ui.rounded(
        x,
        y,
        w,
        h,
        h / 2.0,
        color::with_alpha(color::hover(), alpha),
    );
    if ready {
        ui.rounded(x, y, w * fraction, h, h / 2.0, ink);
        if hovered || dragging || focused {
            ui.rounded(x + w * fraction - 2.5, cy - 2.5, 5.0, 5.0, 2.5, ink);
        }
        if focused {
            ui.border(
                target[0],
                target[1] + 0.5,
                target[2],
                target[3] - 1.0,
                4.0,
                color::accent(),
            );
        }
    }
    let value = level.map_or_else(|| "–".to_owned(), |level| format!("{:.0}%", level * 100.0));
    ui.text_line(Face::Mono, 7.0, 9.0, x + w + 4.0, cy - 4.5, dim, &value);
}

fn transition(expansion: f32, open: bool, seconds: f32) -> f32 {
    let target = if open { 1.0 } else { 0.0 };
    let next = target + (expansion - target) * (-seconds * if open { 19.0 } else { 15.0 }).exp();
    if (next - target).abs() < 0.003 {
        target
    } else {
        next
    }
}

impl Game<'_> {
    fn atlas_esc_music_visible(&self) -> bool {
        self.screen == Screen::Paused && self.atlas_port_on(ID) && !self.prompt_open()
    }

    pub(in crate::game) fn atlas_esc_music_render(&mut self, batch: &mut HudBatch) {
        if !self.atlas_esc_music_visible() {
            let state = &mut self.atlas_ports.esc_music_ui;
            state.title_since = None;
            state.last_frame = None;
            state.expansion = 0.0;
            state.pinned = false;
            state.keyboard = false;
            state.volume_drag = None;
            return;
        }
        self.atlas_ports.esc_music.poll();
        let (layout, duration) = self.atlas_esc_music_layout((batch.width, batch.height));
        if !layout.visible() {
            return;
        }
        let playing = self.atlas_ports.esc_music.playing();
        let track = self.atlas_ports.esc_music.track();
        let volume = track.and_then(|track| track.volume.as_ref());
        let state = &mut self.atlas_ports.esc_music_ui;
        if self.drag_button != Some(MouseButton::Left) {
            state.seek_fraction = None;
            state.volume_drag = None;
        } else if state
            .volume_drag
            .as_ref()
            .is_some_and(|drag| volume.is_none_or(|volume| volume.id != drag.id))
        {
            state.volume_drag = None;
        }
        let now = Instant::now();
        let elapsed = state
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32().min(0.05));
        let hovered = contains(layout.panel(state.expansion), self.cursor);
        let opened = hovered
            || state.pinned
            || state.seek_fraction.is_some()
            || state.volume_drag.is_some()
            || (state.keyboard && self.pause_focus.is_some_and(media_focus));
        state.expansion = transition(state.expansion, opened, elapsed);
        let current = state.seek_fraction.map_or_else(
            || track.map_or(0, |track| position(track, now)),
            |fraction| (fraction * duration as f64) as u64,
        );
        let mut ui = Ui::at_gui_scale(&self.gui.atlas, batch, self.cursor);
        let [x, y, width, height] = layout.panel(state.expansion);
        let radius = PILL_H / 2.0 + state.expansion * 3.0;
        ui.rounded(x, y, width, height, radius, color::hex(0x090a0c, 252));
        ui.border(x, y, width, height, radius, color::hex(0xffffff, 38));

        let title = track.map_or_else(
            || Cow::Owned(lang::get("atlas.esc_music.no_track")),
            |track| {
                if track.title.trim().is_empty() {
                    Cow::Owned(lang::get("atlas.esc_music.unknown_track"))
                } else {
                    Cow::Borrowed(track.title.as_str())
                }
            },
        );
        if state.title != title.as_ref() {
            state.title = title.into_owned();
            state.title_since = Some(now);
        }
        let since = *state.title_since.get_or_insert(now);
        let alpha = ((state.expansion - 0.78) / 0.22).clamp(0.0, 1.0);
        if alpha < 1.0 {
            let compact_alpha = 1.0 - alpha;
            let centre_y = y + height / 2.0;
            draw_cover(
                &mut ui,
                [x + 6.0, centre_y - 8.0, 16.0, 16.0],
                track,
                compact_alpha,
            );
            let text_bounds = [x + 27.0, centre_y - 6.0, width - 55.0, 12.0];
            let overflow = ui.measure(Face::Medium, 10.0, &state.title) - text_bounds[2];
            clipped_line(
                &mut ui,
                Face::Medium,
                10.0,
                text_bounds,
                &state.title,
                color::with_alpha(color::text(), compact_alpha),
                marquee_offset(overflow, now.duration_since(since).as_secs_f32()),
            );
            let phase = now.duration_since(since).as_secs_f32() * 9.0;
            for (i, max) in [5.0, 9.0, 12.0, 7.0].into_iter().enumerate() {
                let h = if playing {
                    3.0 + max * (0.52 + 0.32 * (phase + i as f32 * 1.27).sin())
                } else {
                    max * 0.46
                };
                ui.rounded(
                    x + width - 21.0 + i as f32 * 3.4,
                    centre_y - h / 2.0,
                    1.9,
                    h,
                    1.0,
                    color::with_alpha(color::muted(), compact_alpha),
                );
            }
        }
        if alpha < 0.52 {
            let label = lang::get("atlas.esc_music.controls");
            let target = layout.panel(state.expansion);
            widgets::record(&label, target, true);
            widgets::target(target);
            widgets::narratable(|| {
                widgets::Narratable::button(label, target, true).hovered(hovered)
            });
            widgets::widget_cursor(hovered, true);
        }
        if alpha < 0.12 {
            ui.finish();
            return;
        }
        draw_cover(&mut ui, layout.cover, track, alpha);
        let overflow = ui.measure(Face::SemiBold, TITLE_SIZE, &state.title) - layout.caption[2];
        clipped_line(
            &mut ui,
            Face::SemiBold,
            TITLE_SIZE,
            layout.caption,
            &state.title,
            color::with_alpha(color::text(), alpha),
            marquee_offset(overflow, now.duration_since(since).as_secs_f32()),
        );
        if let Some(track) = track {
            let artist = if track.artist.trim().is_empty() {
                Cow::Owned(lang::get("atlas.esc_music.unknown_artist"))
            } else {
                Cow::Borrowed(track.artist.as_str())
            };
            let [x, y, w, h] = layout.caption;
            clipped_line(
                &mut ui,
                Face::Regular,
                8.75,
                [x, y + h, w, 10.5],
                &artist,
                color::muted(),
                0.0,
            );
        }

        let names = [
            "atlas.esc_music.previous",
            if playing {
                "atlas.esc_music.pause"
            } else {
                "atlas.esc_music.play"
            },
            "atlas.esc_music.next",
            if track.is_some_and(|track| track.repeat_one) {
                "atlas.esc_music.repeat_one_on"
            } else {
                "atlas.esc_music.repeat_one"
            },
        ];
        for (i, (rect, name)) in layout.buttons.into_iter().zip(names).enumerate() {
            let active = alpha >= 0.52 && track.is_some_and(|track| i != 3 || track.can_repeat);
            let hovered = active && contains(rect, self.cursor);
            let focused = active && self.pause_focus == Some(PLAYBACK[i].0);
            let label = lang::get(name);
            if alpha >= 0.52 {
                widgets::record(&label, rect, active);
            }
            if active {
                widgets::target(rect);
                widgets::widget_cursor(hovered, true);
            }
            widgets::narratable(|| {
                widgets::Narratable::button(label, rect, active)
                    .hovered(hovered)
                    .focused(focused)
            });
            let [bx, by, bw, bh] = rect;
            if active && (hovered || focused || i == 1) {
                ui.rounded(
                    bx + 1.0,
                    by + 2.0,
                    bw - 2.0,
                    bh - 4.0,
                    10.0,
                    if hovered {
                        color::hover()
                    } else {
                        color::raised()
                    },
                );
            }
            if focused {
                ui.border(
                    bx + 1.0,
                    by + 2.0,
                    bw - 2.0,
                    bh - 4.0,
                    10.0,
                    color::accent(),
                );
            }
            let ink = if i == 3 && track.is_some_and(|track| track.repeat_one) {
                color::accent()
            } else if track.is_some_and(|track| i != 3 || track.can_repeat) {
                color::text()
            } else {
                color::with_alpha(color::muted(), 0.46)
            };
            media_icon(&mut ui, rect, i, playing, color::with_alpha(ink, alpha));
        }

        let [rx, ry, rw, rh] = layout.rail;
        if duration > 0 && rw >= 16.0 {
            let time = time_label(current);
            let total = time_label(duration);
            let text_y = ry - 3.6;
            let time_ink = color::with_alpha(color::muted(), alpha);
            ui.text_line(
                Face::Mono,
                7.2,
                9.6,
                layout.card[0] + 10.0,
                text_y,
                time_ink,
                &time,
            );
            let total_x = layout.seek_end - 10.0 - ui.measure(Face::Mono, 7.2, &total);
            ui.text_line(Face::Mono, 7.2, 9.6, total_x, text_y, time_ink, &total);
            let fraction = (current as f64 / duration as f64).clamp(0.0, 1.0) as f32;
            let target = layout.seek_target();
            let kind = widgets::Kind::Slider {
                value: fraction,
                track: [rx, rw],
            };
            let can_seek = alpha >= 0.52 && track.is_some_and(|track| track.can_seek);
            if alpha >= 0.52 {
                widgets::record_as(&lang::get("atlas.esc_music.seek"), target, can_seek, kind);
            }
            if can_seek {
                widgets::target_as(target, kind);
            }
            let hot = can_seek && (contains(target, self.cursor) || state.seek_fraction.is_some());
            let focused = can_seek && self.pause_focus == Some(SEEK_ID);
            widgets::slider_cursor(hot, true, state.seek_fraction.is_some());
            ui.rounded(
                rx,
                ry,
                rw,
                rh,
                rh / 2.0,
                color::with_alpha(color::hover(), alpha),
            );
            let filled = rw * fraction;
            ui.rounded(
                rx,
                ry,
                filled,
                rh,
                rh / 2.0,
                color::with_alpha(color::text(), alpha),
            );
            if focused {
                ui.border(
                    rx - 3.0,
                    ry - 2.75,
                    rw + 6.0,
                    rh + 5.5,
                    4.0,
                    color::accent(),
                );
            }
            if hot || focused {
                ui.rounded(
                    rx + filled - 2.75,
                    ry + rh / 2.0 - 2.75,
                    5.5,
                    5.5,
                    2.75,
                    color::text(),
                );
            }
        }
        if track.is_some() {
            let level = state
                .volume_drag
                .as_ref()
                .map(|drag| drag.level)
                .or_else(|| volume.map(|volume| volume.level));
            draw_volume(
                &mut ui,
                &layout,
                level,
                state.volume_drag.is_some(),
                self.pause_focus == Some(VOLUME_ID),
                alpha,
                self.cursor,
            );
        }
        ui.finish();
    }

    fn atlas_esc_music_layout(&self, size: (f32, f32)) -> (Layout, u64) {
        let duration = self
            .atlas_ports
            .esc_music
            .track()
            .map_or(0, |track| track.duration_ms);
        (
            Layout::new(size, self.pause_menu_bottom(size), duration),
            duration,
        )
    }

    pub(in crate::game) fn atlas_esc_music_targets(&self, size: (f32, f32)) -> Vec<(u8, Bounds)> {
        if !self.atlas_esc_music_visible() {
            return Vec::new();
        }
        let (layout, duration) = self.atlas_esc_music_layout(size);
        if !layout.visible() {
            return Vec::new();
        }
        let mut targets = vec![(
            PILL_ID,
            layout.panel(self.atlas_ports.esc_music_ui.expansion),
        )];
        if let Some(track) = self.atlas_ports.esc_music.track()
            && (self.atlas_ports.esc_music_ui.expansion >= 0.895
                || self.pause_focus == Some(PILL_ID))
        {
            targets.extend(
                PLAYBACK
                    .into_iter()
                    .zip(layout.buttons)
                    .enumerate()
                    .filter(|(i, _)| *i != 3 || track.can_repeat)
                    .map(|(_, ((id, _), rect))| (id, rect)),
            );
            if track.can_seek && duration > 0 && layout.rail[2] >= 16.0 {
                targets.push((SEEK_ID, layout.seek_target()));
            }
            if track.volume.is_some() {
                targets.push((VOLUME_ID, layout.volume_target()));
            }
        }
        targets
    }

    pub(in crate::game) fn atlas_esc_music_key(&mut self, key: KeyCode) -> bool {
        if !self.atlas_esc_music_visible() {
            return false;
        }
        if matches!(
            key,
            KeyCode::Tab
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
        ) {
            self.atlas_ports.esc_music_ui.keyboard = true;
        }
        let Some(id) = self.pause_focus else {
            return false;
        };
        if id == PILL_ID && matches!(key, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
            let state = &mut self.atlas_ports.esc_music_ui;
            state.pinned = !state.pinned;
            state.keyboard = state.pinned;
            return true;
        }
        let Some(track) = self.atlas_ports.esc_music.track() else {
            return false;
        };
        if id == VOLUME_ID
            && matches!(key, KeyCode::ArrowLeft | KeyCode::ArrowRight)
            && let Some(volume) = &track.volume
        {
            let id = volume.id.clone();
            let step = if key == KeyCode::ArrowLeft {
                -0.05
            } else {
                0.05
            };
            let level = (volume.level + step).clamp(0.0, 1.0);
            self.atlas_ports.esc_music.set_volume(&id, level);
            return true;
        }
        let action = match key {
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                PLAYBACK.iter().find_map(|(control, command)| {
                    (*control == id && (!matches!(command, Command::RepeatOne) || track.can_repeat))
                        .then_some(*command)
                })
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight
                if id == SEEK_ID && track.can_seek && track.duration_ms > 0 =>
            {
                let now = position(track, Instant::now());
                let ms = if key == KeyCode::ArrowLeft {
                    now.saturating_sub(5_000)
                } else {
                    now.saturating_add(5_000).min(track.duration_ms)
                };
                Some(Command::Seek(ms))
            }
            _ => None,
        };
        if let Some(command) = action {
            self.atlas_ports.esc_music.command(command);
            self.audio.play_button_click();
        }
        action.is_some()
    }

    pub(in crate::game) fn atlas_esc_music_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if !self.atlas_esc_music_visible() {
            self.atlas_ports.esc_music_ui.seek_fraction = None;
            self.atlas_ports.esc_music_ui.volume_drag = None;
            return false;
        }
        if button != 0 {
            return false;
        }
        let (layout, duration) = self.atlas_esc_music_layout(self.gui_size());
        if !pressed {
            if let Some(drag) = self.atlas_ports.esc_music_ui.volume_drag.take() {
                self.atlas_ports.esc_music.set_volume(&drag.id, drag.level);
                return true;
            }
            let Some(fraction) = self.atlas_ports.esc_music_ui.seek_fraction.take() else {
                return false;
            };
            if duration > 0 {
                self.atlas_ports
                    .esc_music
                    .command(Command::Seek(seek_position(fraction, duration)));
            }
            return true;
        }
        let state = &mut self.atlas_ports.esc_music_ui;
        if !layout.visible() || !contains(layout.panel(state.expansion), self.cursor) {
            return false;
        }
        state.keyboard = false;
        if state.expansion < 0.895 {
            state.pinned = !state.pinned;
            self.pause_focus = Some(PILL_ID);
            return true;
        }
        if let Some(track) = self.atlas_ports.esc_music.track() {
            for (rect, (id, command)) in layout.buttons.into_iter().zip(PLAYBACK) {
                if contains(rect, self.cursor)
                    && (!matches!(command, Command::RepeatOne) || track.can_repeat)
                {
                    self.pause_focus = Some(id);
                    self.atlas_ports.esc_music.command(command);
                    self.audio.play_button_click();
                    return true;
                }
            }
            if track.can_seek
                && duration > 0
                && layout.rail[2] >= 16.0
                && contains(layout.seek_target(), self.cursor)
            {
                self.pause_focus = Some(SEEK_ID);
                state.seek_fraction = Some(layout.seek_fraction(self.cursor.0));
                self.audio.play_button_click();
                return true;
            }
            if let Some(volume) = &track.volume
                && contains(layout.volume_target(), self.cursor)
            {
                self.pause_focus = Some(VOLUME_ID);
                state.volume_drag = Some(VolumeDrag {
                    id: volume.id.clone(),
                    level: layout.volume_level(self.cursor.0),
                });
                return true;
            }
        }
        state.pinned = false;
        true
    }

    pub(in crate::game) fn atlas_esc_music_drag(&mut self) -> bool {
        if self.drag_button != Some(MouseButton::Left) || !self.atlas_esc_music_visible() {
            self.atlas_ports.esc_music_ui.seek_fraction = None;
            self.atlas_ports.esc_music_ui.volume_drag = None;
            return false;
        }
        let state = &self.atlas_ports.esc_music_ui;
        if state.seek_fraction.is_none() && state.volume_drag.is_none() {
            return false;
        }

        let (layout, duration) = self.atlas_esc_music_layout(self.gui_size());
        if let Some(drag) = &mut self.atlas_ports.esc_music_ui.volume_drag {
            drag.level = layout.volume_level(self.cursor.0);
            return true;
        }
        if layout.rail[2] >= 16.0 && duration > 0 {
            self.atlas_ports.esc_music_ui.seek_fraction = Some(layout.seek_fraction(self.cursor.0));
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn media_timing_interpolates_only_playing_tracks_and_clamps_to_duration() {
        let now = Instant::now();
        let mut track = Track {
            source_id: String::new(),
            title: "Test track".into(),
            artist: "Test artist".into(),
            duration_ms: 60_000,
            position_ms: 58_250,
            playing: true,
            can_seek: true,
            can_repeat: true,
            repeat_one: false,
            native_repeat: false,
            volume: None,
            sampled_at: now - Duration::from_millis(1500),
            cover: None,
        };
        assert_eq!(position(&track, now), 59_750);
        assert_eq!(time_label(position(&track, now)), "00:59");
        assert_eq!(position(&track, now + Duration::from_secs(4)), 60_000);
        track.playing = false;
        assert_eq!(position(&track, now), 58_250);
        assert_eq!(time_label(3_661_999), "1:01:01");
        assert_eq!(seek_position(0.5, 123_456), 61_730);
        assert_eq!(seek_position(1.0, 123_456), 123_456);
    }

    #[test]
    fn overflowing_title_returns_to_start_then_holds_for_two_seconds() {
        assert_eq!(marquee_offset(-1.0, 20.0), 0.0);
        assert_eq!(marquee_offset(22.0, 1.99), 0.0);
        assert_eq!(marquee_offset(22.0, 2.5), 11.0);
        assert_eq!(marquee_offset(22.0, 3.25), 22.0);
        assert_eq!(marquee_offset(22.0, 4.0), 11.0);
        assert_eq!(marquee_offset(22.0, 4.25), 0.0);
        assert_eq!(marquee_offset(22.0, 6.24), 0.0);
        assert_eq!(marquee_offset(22.0, 6.75), 11.0);
    }

    #[test]
    fn card_eases_open_and_closed_with_elapsed_time() {
        let first = transition(0.0, true, 0.02);
        assert!(first > 0.0 && first < 1.0);
        let later = transition(first, true, 0.25);
        assert!(later > 0.98);
        let shrinking = transition(later, false, 0.02);
        assert!(shrinking < later && shrinking > 0.0);
        let layout = Layout::new((430.0, 300.0), 180.0, 120_000);
        assert_eq!(layout.panel(0.0), layout.pill);
        assert_eq!(layout.panel(1.0), layout.card);
    }

    #[test]
    fn controls_stay_clear_of_pause_buttons_even_in_short_windows() {
        let roomy = Layout::new((430.0, 300.0), 210.0, 1_230_000);
        let compact = Layout::new((320.0, 240.0), 210.0, 3_600_000);
        assert!(roomy.card[1] > 210.0);
        assert!(compact.card[1] + compact.card[3] < 40.0);
        let seek = compact.seek_target();
        let volume = compact.volume_target();
        assert!(seek[2] >= 16.0);
        assert!(seek[0] + seek[2] < volume[0]);
        assert_eq!(
            compact.seek_fraction(compact.rail[0] + compact.rail[2] / 2.0),
            0.5
        );
    }
}
