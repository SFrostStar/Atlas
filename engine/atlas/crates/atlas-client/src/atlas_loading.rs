//! Atlas transitions use a tiny code-drawn scene, independent of pack artwork.
use crate::hud::{HudAtlas, HudBatch, rgba};
use std::time::Instant;

const FADE: f32 = 0.25;
const REVEAL_DELAY: f32 = 0.12;

#[derive(Default)]
pub struct Overlay {
    started: Option<Instant>,
    finished: Option<Instant>,
    progress: f32,
    label: String,
}

impl Overlay {
    pub fn update(&mut self, operation: Option<(&str, Option<f32>)>, now: Instant) {
        if let Some((label, progress)) = operation {
            if self.started.is_none() || self.finished.is_some() {
                self.started = Some(now);
                self.finished = None;
                self.progress = 0.0;
            }
            if self.label != label {
                label.clone_into(&mut self.label);
            }
            if let Some(progress) = progress.filter(|p| p.is_finite()) {
                self.progress = self.progress.max(progress.clamp(0.0, 0.99));
            }
        } else if self.started.is_some() && self.finished.is_none() {
            self.finished = Some(now);
            self.progress = 1.0;
        }
        if self
            .finished
            .is_some_and(|at| now.duration_since(at).as_secs_f32() >= FADE)
        {
            *self = Self::default();
        }
    }

    pub fn draw(&self, batch: &mut HudBatch, atlas: &HudAtlas, now: Instant) {
        let Some(started) = self.started else { return };
        let elapsed = now.duration_since(started).as_secs_f32();
        let alpha = self.opacity(now);
        if alpha <= 0.0 {
            return;
        }
        draw_scene(batch, atlas, &self.label, self.progress, elapsed, alpha);
    }

    fn opacity(&self, now: Instant) -> f32 {
        let Some(started) = self.started else { return 0.0 };
        let entering = self
            .finished
            .unwrap_or(now)
            .duration_since(started)
            .as_secs_f32();
        ((entering - REVEAL_DELAY) / FADE).clamp(0.0, 1.0)
            * self.finished.map_or(1.0, |at| {
                1.0 - (now.duration_since(at).as_secs_f32() / FADE).min(1.0)
            })
    }
}

/// Kept allocation-free apart from the batch's reused vertex buffer: no images,
/// videos, particle simulation or pack-dependent logo are needed.
pub fn draw_scene(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    label: &str,
    progress: f32,
    time: f32,
    alpha: f32,
) {
    let (w, h) = (batch.width, batch.height);
    let cx = w / 2.0;
    let cy = h * 0.43;
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    batch.fill_gradient(
        atlas,
        (0.0, 0.0, w, h),
        rgba(15, 18, 34, a),
        rgba(91, 46, 43, a),
    );
    let sun = 0.86 + 0.08 * (time * std::f32::consts::TAU / 12.0).sin();
    // A stepped, pixel-art disk with soft bands around it.
    for (radius, opacity) in [(62.0_f32, 0.025_f32), (52.0, 0.035), (43.0, 0.07)] {
        for row in -10_i32..10 {
            let y = row as f32 * radius / 10.0;
            let half = ((radius * radius - y * y).max(0.0).sqrt() / 3.0).floor() * 3.0;
            batch.fill(
                atlas,
                cx - half,
                cy + y,
                half * 2.0,
                radius / 10.0 + 0.5,
                rgba(255, 146, 83, (a as f32 * opacity) as u8),
            );
        }
    }
    for row in -10_i32..11 {
        let y = row as f32 * 3.0;
        let half = ((33.0_f32 * 33.0 - y * y).max(0.0).sqrt() / 3.0).floor() * 3.0;
        batch.fill(
            atlas,
            cx - half,
            cy + y,
            half * 2.0,
            3.0,
            rgba(
                (204.0 * sun) as u8,
                (103.0 * sun) as u8,
                (65.0 * sun) as u8,
                a,
            ),
        );
    }
    // Distant ridges: fixed geometry, deliberately still and quiet.
    for (base, tint, seed) in [(0.75, [53, 37, 48], 0.8), (0.85, [28, 27, 39], 1.9)] {
        for x in (0..w.ceil() as usize).step_by(8) {
            let xf = x as f32;
            let ridge = (xf / 49.0 + seed).sin() * 9.0 + (xf / 23.0).cos() * 4.0;
            let top = h * base + ridge;
            batch.fill(
                atlas,
                xf,
                top,
                8.0,
                h - top,
                rgba(tint[0], tint[1], tint[2], a),
            );
        }
    }
    let pulse = 0.5 + 0.5 * (time * std::f32::consts::TAU / 3.5).sin();
    let orange = rgba(255, (139.0 + pulse * 25.0) as u8, 64, a);
    // The exact transparent launcher application image, never a font glyph or
    // a second drawing of the mark. A faint larger copy provides the pulse.
    batch.sprite_scaled(
        atlas,
        "atlas/loading-logo",
        cx - 38.0,
        cy - 38.0,
        76.0,
        76.0,
        rgba(255, 255, 255, (a as f32 * pulse * 0.065) as u8),
    );
    batch.sprite_scaled(
        atlas,
        "atlas/loading-logo",
        cx - 36.0,
        cy - 36.0,
        72.0,
        72.0,
        rgba(255, 255, 255, (a as f32 * (0.88 + 0.12 * pulse)) as u8),
    );
    let bar_width = 150.0_f32.min(w - 40.0).max(1.0);
    let left = cx - bar_width / 2.0;
    let top = cy + 54.0;
    batch.fill(
        atlas,
        left,
        top,
        bar_width,
        3.0,
        rgba(255, 218, 192, (a as f32 * 0.12) as u8),
    );
    let done = progress.clamp(0.0, 1.0) * bar_width;
    batch.fill(atlas, left, top, done, 3.0, orange);
    if progress == 0.0 {
        let segment = bar_width * 0.18;
        let offset = (time * 35.0) % (bar_width + segment) - segment;
        let x = offset.max(0.0);
        let end = (offset + segment).min(bar_width);
        if end > x {
            batch.fill(atlas, left + x, top, end - x, 3.0, orange);
        }
    }
    let width = HudBatch::text_width(atlas, label);
    batch.text(
        atlas,
        label,
        cx - width / 2.0,
        top + 13.0,
        rgba(226, 212, 208, a),
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn progress_is_monotonic_and_completion_has_a_fade() {
        let now = Instant::now();
        let mut overlay = Overlay::default();
        overlay.update(Some(("Resources", Some(0.6))), now);
        overlay.update(Some(("Apply", Some(0.2))), now + Duration::from_millis(100));
        assert_eq!(overlay.progress, 0.6);
        overlay.update(None, now + Duration::from_millis(200));
        assert_eq!(overlay.progress, 1.0);
        assert!(overlay.started.is_some());
        overlay.update(None, now + Duration::from_millis(451));
        assert!(overlay.started.is_none());
    }
    #[test]
    fn new_operation_during_fade_starts_fresh() {
        let now = Instant::now();
        let mut overlay = Overlay::default();
        overlay.update(Some(("Open", Some(0.5))), now);
        overlay.update(None, now);
        overlay.update(Some(("Save", None)), now);
        assert_eq!(overlay.progress, 0.0);
        assert!(overlay.finished.is_none());
    }
    #[test]
    fn short_operations_do_not_flash_and_long_ones_fade_both_ways() {
        let now = Instant::now();
        let mut overlay = Overlay::default();
        overlay.update(Some(("Open", None)), now);
        overlay.update(None, now + Duration::from_millis(50));
        assert_eq!(overlay.opacity(now + Duration::from_millis(60)), 0.0);
        overlay.update(Some(("Resources", None)), now + Duration::from_secs(1));
        assert_eq!(overlay.opacity(now + Duration::from_millis(1100)), 0.0);
        assert!((overlay.opacity(now + Duration::from_millis(1245)) - 0.5).abs() < 0.001);
        overlay.update(None, now + Duration::from_secs(2));
        assert!((overlay.opacity(now + Duration::from_millis(2125)) - 0.5).abs() < 0.001);
    }
}
