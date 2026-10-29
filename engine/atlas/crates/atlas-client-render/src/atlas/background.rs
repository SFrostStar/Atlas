//! Atlas night photograph with independently twinkling stars and rare meteors.
use crate::hud::{HudAtlas, HudBatch, rgba};
pub fn background(batch: &mut HudBatch, atlas: &HudAtlas) {
    use std::{sync::OnceLock, time::Instant};
    static START: OnceLock<Instant> = OnceLock::new();
    background_at(
        batch,
        atlas,
        START.get_or_init(Instant::now).elapsed().as_secs_f32(),
    );
}

fn background_at(batch: &mut HudBatch, atlas: &HudAtlas, seconds: f32) {
    let (w, h) = (batch.width, batch.height);
    if let Some(mut photo) = atlas.sprites.get("atlas/menu-night").copied() {
        // Centre-cover without stretching. The coverage-filter UV range also
        // gives the photograph smooth sampling, independently of pixel sprites.
        let aspect = photo.w as f32 / photo.h as f32;
        let target = w / h;
        if target < aspect {
            let crop = (1.0 - target / aspect) * 0.5;
            let span = photo.u1 - photo.u0;
            photo.u0 += span * crop;
            photo.u1 -= span * crop;
        } else {
            let crop = (1.0 - aspect / target) * 0.5;
            let span = photo.v1 - photo.v0;
            photo.v0 += span * crop;
            photo.v1 -= span * crop;
        }
        photo.u0 += 4.0;
        photo.u1 += 4.0;
        batch.rect_uv(0.0, 0.0, w, h, photo, u32::MAX);
    } else {
        batch.fill_gradient(
            atlas,
            (0.0, 0.0, w, h),
            rgba(10, 15, 26, 255),
            rgba(24, 40, 48, 255),
        );
    }
    // Different phases and periods keep the stars from pulsing in unison.
    for i in 0..58_u32 {
        let x = ((i * 127 + 31) % 997) as f32 / 997.0 * w;
        let y = ((i * 71 + 13) % 503) as f32 / 503.0 * h * 0.48;
        let pulse = ((seconds * (0.65 + (i % 7) as f32 * 0.12) + i as f32 * 2.399).sin() * 0.5
            + 0.5)
            .powi(3);
        let alpha = (35.0 + pulse * 155.0) as u8;
        let size = if i % 9 == 0 { 0.7 } else { 0.45 };
        if pulse > 0.65 {
            batch.fill(
                atlas,
                x - 0.6,
                y - 0.6,
                size + 1.2,
                size + 1.2,
                rgba(155, 198, 223, (pulse * 20.0) as u8),
            );
        }
        batch.fill(atlas, x, y, size, size, rgba(188, 216, 237, alpha));
    }
    // A short, fading trail crosses the sky occasionally; the landscape stays still.
    for lane in 0..2 {
        let period = 18.0 + lane as f32 * 11.0;
        let clock = seconds + period - 3.0 - lane as f32 * 6.0;
        let phase = clock.rem_euclid(period);
        if phase < 2.0 {
            let cycle = (clock / period).floor();
            let p = phase / 2.0;
            let fade = (p * std::f32::consts::PI).sin();
            let x = w * (-0.12 + p * 1.15);
            let y = h * (0.04 + (cycle * 0.173 + lane as f32 * 0.11).rem_euclid(0.12) + p * 0.23);
            for tail in (0..32).rev() {
                let t = tail as f32 / 32.0;
                let tx = x - t * w * 0.16;
                let ty = y - t * h * 0.032;
                let a = (fade * (1.0 - t).powi(2) * 180.0) as u8;
                batch.fill(
                    atlas,
                    tx - 0.8,
                    ty - 0.8,
                    w * 0.16 / 32.0 + 1.6,
                    2.1,
                    rgba(132, 191, 220, a / 7),
                );
                batch.fill(
                    atlas,
                    tx,
                    ty,
                    w * 0.16 / 32.0 + 0.12,
                    0.6,
                    rgba(208, 232, 248, a),
                );
            }
        }
    }
}

/// Paint the same scene in list/header regions, preserving their clipping.
pub fn region(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, w: f32, h: f32) {
    let start = batch.vertices.len();
    background(batch, atlas);
    batch.clip_since(start, x, y, x + w, y + h);
}
