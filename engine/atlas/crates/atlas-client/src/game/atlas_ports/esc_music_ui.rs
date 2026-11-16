//! Pause menu bar based on ESC Music Controller 1.0.0, AntiJOPA (MIT).
use super::{
    Game,
    esc_music::{Command, ID, Track},
};
use crate::game::Screen;
use atlas_client::{
    font::Patch,
    hud::{HudAtlas, HudBatch, MEDIA_COVER_SIZE, MEDIA_COVER_SPRITE, rgba},
    lang, widgets,
};
use std::{sync::Arc, time::Instant};
use winit::event::MouseButton;

type Bounds = [f32; 4];

#[derive(Default)]
pub(super) struct State {
    seek_fraction: Option<f64>,
    uploaded_cover: Option<(u64, Arc<[u8]>)>,
}

struct Layout {
    panel: Bounds,
    cover: Bounds,
    buttons: [Bounds; 3],
    time_x: f32,
    duration_x: f32,
    rail: Bounds,
}

impl Layout {
    fn new(size: (f32, f32), time_width: f32, duration_width: f32) -> Self {
        let width = (size.0 - 24.0).min(380.0).max(0.0);
        let x = ((size.0 - width) / 2.0).floor();
        let y = size.1 - 56.0;
        let time_x = x + 118.0;
        let duration_x = x + width - 8.0 - duration_width;
        let rail_x = time_x + time_width + 6.0;
        Self {
            panel: [x, y, width, 48.0],
            cover: [x + 4.0, y + 4.0, 40.0, 40.0],
            buttons: std::array::from_fn(|i| [x + 51.0 + i as f32 * 21.0, y + 23.0, 18.0, 18.0]),
            time_x,
            duration_x,
            rail: [rail_x, y + 30.0, duration_x - 6.0 - rail_x, 4.0],
        }
    }

    fn seek_fraction(&self, x: f32) -> f64 {
        f64::from(((x - self.rail[0]) / self.rail[2]).clamp(0.0, 1.0))
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
            sampled_at: now - Duration::from_millis(1500),
            cover_rgba: None,
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
}

fn contains([x, y, w, h]: Bounds, point: (f32, f32)) -> bool {
    point.0 >= x && point.0 <= x + w && point.1 >= y && point.1 <= y + h
}

fn color(argb: u32) -> u32 {
    let [a, r, g, b] = argb.to_be_bytes();
    rgba(r, g, b, a)
}

fn fill(batch: &mut HudBatch, atlas: &HudAtlas, [x, y, w, h]: Bounds, argb: u32) {
    batch.fill(atlas, x, y, w, h, color(argb));
}

fn outline(batch: &mut HudBatch, atlas: &HudAtlas, [x, y, w, h]: Bounds, argb: u32) {
    for rect in [
        [x, y, w, 1.0],
        [x, y + h - 1.0, w, 1.0],
        [x, y + 1.0, 1.0, h - 2.0],
        [x + w - 1.0, y + 1.0, 1.0, h - 2.0],
    ] {
        fill(batch, atlas, rect, argb);
    }
}

fn title(track: Option<&Track>) -> String {
    let Some(track) = track else {
        return lang::get("atlas.esc_music.no_track");
    };
    let title = if track.title.is_empty() {
        lang::get("atlas.esc_music.unknown_track")
    } else {
        track.title.clone()
    };
    let artist = if track.artist.is_empty() {
        lang::get("atlas.esc_music.unknown_artist")
    } else {
        track.artist.clone()
    };
    format!("{title} • {artist}")
}

impl Game<'_> {
    fn atlas_esc_music_visible(&self) -> bool {
        self.screen == Screen::Paused && self.atlas_port_on(ID) && !self.prompt_open()
    }

    pub(in crate::game) fn atlas_esc_music_render(&mut self, batch: &mut HudBatch) {
        if !self.atlas_esc_music_visible() {
            return;
        }
        let _font = atlas_client::font::java_mod_font_scope();
        self.atlas_ports.esc_music.poll();
        let track = self.atlas_ports.esc_music.track();
        let state = &mut self.atlas_ports.esc_music_ui;
        if self.drag_button != Some(MouseButton::Left) {
            state.seek_fraction = None;
        }
        let atlas = &self.gui.atlas;
        let duration = track.map_or(0, |track| track.duration_ms);
        let current = state.seek_fraction.map_or_else(
            || track.map_or(0, |track| position(track, Instant::now())),
            |fraction| (fraction * duration as f64) as u64,
        );
        let time = time_label(current);
        let total = time_label(duration);
        let layout = Layout::new(
            (batch.width, batch.height),
            atlas.font.width(&time),
            atlas.font.width(&total),
        );
        let [x, y, width, _] = layout.panel;
        if width < 62.0 {
            return;
        }
        fill(batch, atlas, layout.panel, 0xe6111318);
        outline(
            batch,
            atlas,
            layout.panel,
            if contains(layout.panel, self.cursor) {
                0xff4a5568
            } else {
                0xff2a2e39
            },
        );
        fill(
            batch,
            atlas,
            [x + 1.0, y + 1.0, width - 2.0, 1.0],
            0x331db954,
        );

        let [cx, cy, cw, ch] = layout.cover;
        if let Some(cover) = track.and_then(|track| track.cover_rgba.as_ref())
            && cover.len() == (MEDIA_COVER_SIZE * MEDIA_COVER_SIZE * 4) as usize
            && let Some(rect) = atlas.sprites.get(MEDIA_COVER_SPRITE)
        {
            if state
                .uploaded_cover
                .as_ref()
                .is_none_or(|(revision, previous)| {
                    *revision != atlas.revision || !Arc::ptr_eq(previous, cover)
                })
            {
                self.map_patches.push(Patch {
                    x: (rect.u0 * atlas.width as f32).round() as u32,
                    y: (rect.v0 * atlas.height as f32).round() as u32,
                    width: MEDIA_COVER_SIZE,
                    height: MEDIA_COVER_SIZE,
                    rgba: cover.to_vec(),
                });
                state.uploaded_cover = Some((atlas.revision, Arc::clone(cover)));
            }
            batch.rect_uv(cx, cy, cw, ch, *rect, u32::MAX);
            outline(batch, atlas, layout.cover, 0x40ffffff);
        } else {
            fill(batch, atlas, layout.cover, 0xff191c23);
            outline(batch, atlas, layout.cover, 0xff2f3540);
            let note = "♫";
            batch.text(
                atlas,
                note,
                cx + ((cw - atlas.font.width(note)) / 2.0).floor(),
                cy + 15.0,
                color(0xff718096),
                false,
            );
        }

        let caption = title(track);
        let fit = atlas.font.fit(&caption, width - 59.0);
        let trimmed = fit < caption.chars().count();
        let caption: String = if track.is_some() && trimmed && fit > 3 {
            caption.chars().take(fit - 3).chain("...".chars()).collect()
        } else {
            caption.chars().take(fit).collect()
        };
        batch.text(
            atlas,
            &caption,
            x + 51.0,
            y + 6.0,
            if track.is_some() {
                u32::MAX
            } else {
                color(0xff8a939e)
            },
            true,
        );
        let playing = track.is_some_and(|track| track.playing);
        let names = [
            "atlas.esc_music.previous",
            if playing {
                "atlas.esc_music.pause"
            } else {
                "atlas.esc_music.play"
            },
            "atlas.esc_music.next",
        ];
        let symbols = ["⏮", if playing { "⏸" } else { "▶" }, "⏭"];
        for ((rect, symbol), name) in layout.buttons.into_iter().zip(symbols).zip(names) {
            let hovered = contains(rect, self.cursor);
            let label = lang::get(name);
            widgets::record(&label, rect, true);
            widgets::target(rect);
            widgets::narratable(|| widgets::Narratable::button(label, rect, true).hovered(hovered));
            fill(
                batch,
                atlas,
                rect,
                if hovered { 0xff353c48 } else { 0xff1f232b },
            );
            outline(
                batch,
                atlas,
                rect,
                if hovered { 0xff5865f2 } else { 0xff2f3540 },
            );
            batch.text(
                atlas,
                symbol,
                rect[0] + ((18.0 - atlas.font.width(symbol)) / 2.0).floor(),
                rect[1] + 5.0,
                if hovered { u32::MAX } else { color(0xffd1d5db) },
                false,
            );
        }
        batch.text(
            atlas,
            &time,
            layout.time_x,
            y + 28.0,
            color(0xffa0aab5),
            true,
        );
        batch.text(
            atlas,
            &total,
            layout.duration_x,
            y + 28.0,
            color(0xffa0aab5),
            true,
        );

        let [rx, ry, rw, rh] = layout.rail;
        if rw > 20.0 {
            let active = duration > 0;
            let fraction = if active {
                (current as f64 / duration as f64).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            let target = [rx - 2.0, ry - 6.0, rw + 4.0, rh + 12.0];
            let kind = widgets::Kind::Slider {
                value: fraction,
                track: [rx, rw],
            };
            widgets::record_as(&lang::get("atlas.esc_music.seek"), target, active, kind);
            if active {
                widgets::target_as(target, kind);
            }
            let hot = contains([rx, ry - 3.0, rw, rh + 6.0], self.cursor)
                || state.seek_fraction.is_some();
            fill(batch, atlas, layout.rail, 0xff282c35);
            let filled = (rw * fraction).round();
            if filled > 0.0 {
                fill(
                    batch,
                    atlas,
                    [rx, ry, filled, rh],
                    if hot { 0xff1ed760 } else { 0xff1db954 },
                );
            }
            let radius = if hot { 4.0 } else { 3.0 };
            fill(
                batch,
                atlas,
                [
                    rx + filled - radius,
                    ry + rh / 2.0 - radius,
                    2.0 * radius,
                    2.0 * radius,
                ],
                if hot { 0xffffffff } else { 0xffe0e0e0 },
            );
        }
    }

    fn atlas_esc_music_layout(&self) -> (Layout, u64) {
        let track = self.atlas_ports.esc_music.track();
        let duration = track.map_or(0, |track| track.duration_ms);
        let current = track.map_or(0, |track| position(track, Instant::now()));
        let font = &self.gui.atlas.font;
        (
            Layout::new(
                self.gui_size(),
                font.width(&time_label(current)),
                font.width(&time_label(duration)),
            ),
            duration,
        )
    }

    pub(in crate::game) fn atlas_esc_music_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if !self.atlas_esc_music_visible() {
            self.atlas_ports.esc_music_ui.seek_fraction = None;
            return false;
        }
        if button != 0 {
            return false;
        }
        let _font = atlas_client::font::java_mod_font_scope();
        let (layout, duration) = self.atlas_esc_music_layout();
        if !pressed {
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
        for (rect, command) in
            layout
                .buttons
                .into_iter()
                .zip([Command::Previous, Command::Toggle, Command::Next])
        {
            if contains(rect, self.cursor) {
                self.atlas_ports.esc_music.command(command);
                self.audio.play_button_click();
                return true;
            }
        }
        let [x, y, w, h] = layout.rail;
        if w > 20.0 && duration > 0 && contains([x - 2.0, y - 6.0, w + 4.0, h + 12.0], self.cursor)
        {
            let fraction = layout.seek_fraction(self.cursor.0);
            self.atlas_ports.esc_music_ui.seek_fraction = Some(fraction);
            self.atlas_ports
                .esc_music
                .command(Command::Seek(seek_position(fraction, duration)));
            self.audio.play_button_click();
            return true;
        }
        contains(layout.panel, self.cursor)
    }

    pub(in crate::game) fn atlas_esc_music_drag(&mut self) -> bool {
        if self.drag_button != Some(MouseButton::Left) || !self.atlas_esc_music_visible() {
            self.atlas_ports.esc_music_ui.seek_fraction = None;
            return false;
        }
        if self.atlas_ports.esc_music_ui.seek_fraction.is_none() {
            return false;
        }
        let _font = atlas_client::font::java_mod_font_scope();
        let (layout, duration) = self.atlas_esc_music_layout();
        if layout.rail[2] > 0.0 && duration > 0 {
            self.atlas_ports.esc_music_ui.seek_fraction = Some(layout.seek_fraction(self.cursor.0));
            return true;
        }
        false
    }
}
