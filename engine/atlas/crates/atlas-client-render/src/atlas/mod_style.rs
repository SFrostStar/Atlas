//! Shared controls for Atlas mod screens, in Minecraft GUI coordinates.
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    text::Text,
    ui_theme::{Face, Ui},
    widgets,
};

pub fn panel(batch: &mut HudBatch, atlas: &HudAtlas, [x, y, w, h]: [f32; 4], radius: f32) {
    let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
    ui.rounded(x, y, w, h, radius, rgba(15, 19, 25, 238));
    ui.border(x, y, w, h, radius, rgba(255, 255, 255, 29));
    ui.finish();
}

#[allow(clippy::too_many_arguments)]
pub fn button(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &Text,
    active: bool,
    hover: bool,
) {
    let label = label.plain();
    let rect = [x, y, w, h];
    widgets::record(&label, rect, active);
    widgets::narratable(|| widgets::Narratable::button(label.clone(), rect, active).hovered(hover));
    widgets::widget_cursor(hover, active);
    if active {
        widgets::target(rect);
    }
    let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
    let radius = (h / 2.0).min(11.0);
    let fill = if active && hover {
        rgba(49, 63, 83, 255)
    } else {
        rgba(29, 36, 47, 242)
    };
    let border = if active && hover {
        rgba(157, 193, 255, 205)
    } else {
        rgba(255, 255, 255, 28)
    };
    ui.rounded(x, y, w, h, radius, fill);
    ui.border(x, y, w, h, radius, border);
    let start = ui.batch.vertices.len();
    let size = 9.0;
    let text_w = ui.measure(Face::Medium, size, &label);
    let ink = if active {
        u32::MAX
    } else {
        rgba(163, 174, 189, 255)
    };
    ui.text_line(
        Face::Medium,
        size,
        h,
        x + ((w - text_w) / 2.0).max(5.0),
        y,
        ink,
        &label,
    );
    ui.batch.clip_since(start, x + 4.0, y, x + w - 4.0, y + h);
    ui.finish();
}

#[allow(clippy::too_many_arguments)]
pub fn slider(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    label: &Text,
    value: f32,
    hover: bool,
    active: bool,
    dragging: bool,
) {
    let rect = [x, y, w, 20.0];
    let track = [x + 4.0, (w - 8.0).max(0.0)];
    let kind = widgets::Kind::Slider { value, track };
    let label = label.plain();
    widgets::record_as(&label, rect, active, kind);
    widgets::narratable(|| widgets::Narratable::slider(label.clone(), rect, active).hovered(hover));
    if active {
        widgets::target_as(rect, kind);
    }
    panel(batch, atlas, rect, 9.0);
    let mut ui = Ui::at_gui_scale(atlas, batch, (0.0, 0.0));
    let has_label = !label.is_empty();
    let line_y = y + if has_label { 14.5 } else { 9.0 };
    let done = track[1] * value.clamp(0.0, 1.0);
    let accent = if active {
        rgba(146, 186, 247, 255)
    } else {
        rgba(146, 154, 166, 255)
    };
    ui.rounded(track[0], line_y, track[1], 2.0, 1.0, rgba(77, 88, 104, 255));
    ui.rounded(track[0], line_y, done, 2.0, 1.0, accent);
    let knob = if active && (hover || dragging) {
        3.5
    } else {
        2.5
    };
    ui.rounded(
        track[0] + done - knob,
        line_y + 1.0 - knob,
        2.0 * knob,
        2.0 * knob,
        knob,
        accent,
    );
    if has_label {
        let start = ui.batch.vertices.len();
        let size = 8.0;
        let text_w = ui.measure(Face::Medium, size, &label);
        ui.text_line(
            Face::Medium,
            size,
            13.0,
            x + ((w - text_w) / 2.0).max(5.0),
            y,
            u32::MAX,
            &label,
        );
        ui.batch
            .clip_since(start, x + 4.0, y, x + w - 4.0, y + 14.0);
    }
    ui.finish();
}
