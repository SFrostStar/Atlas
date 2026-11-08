//! The look of Atlas's graphics settings screen, in one place: sizes,
//! colours and the controls it draws with. The screen (`graphics_ui`) only
//! lays out rows and handles input; a menu theme replaces this module.
//!
//! Vanilla Minecraft widget sprites and square list surfaces. Atlas settings
//! deliberately use the game's controls, independently of the Electron launcher.
use crate::{
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

/// What a setting costs in frame time, shown with its description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    None,
    Low,
    Medium,
    High,
    /// Turning it on makes frames faster (culling, caching, batching).
    Saves,
}
impl Cost {
    pub fn label(self) -> &'static str {
        match self {
            Cost::None => "No cost",
            Cost::Low => "Low cost",
            Cost::Medium => "Medium cost",
            Cost::High => "High cost",
            Cost::Saves => "Saves time",
        }
    }
    /// `0xRRGGBB`: the palette's success, warn and danger.
    pub fn rgb(self) -> u32 {
        match self {
            Cost::None => 0xa7adb5,
            Cost::Low | Cost::Saves => 0x80c49a,
            Cost::Medium => 0xe9b766,
            Cost::High => 0xe78a82,
        }
    }
}

/// `0xRRGGBB` and an alpha in the HUD's colour encoding.
const fn hex(c: u32, a: u8) -> u32 {
    (a as u32) << 24 | (c & 0xff) << 16 | (c >> 8 & 0xff) << 8 | (c >> 16 & 0xff)
}
/// Neutral Minecraft list colours; resource packs supply the widget sprites.
pub const BACKGROUND: u32 = hex(0x000000, 0);
const GLASS: u32 = hex(0x000000, 0);
const GROUP: u32 = hex(0x000000, 35);
pub const HOVER: u32 = hex(0xffffff, 20);
pub const OPEN: u32 = hex(0xffffff, 35);
pub const HEAD_RULE: u32 = hex(0xffffff, 45);
pub const ROW_RULE: u32 = hex(0xffffff, 15);
const WELL: u32 = hex(0x000000, 200);
const TRACK: u32 = hex(0x808080, 255);
pub const ACCENT: u32 = u32::MAX;
pub const WHITE: u32 = u32::MAX;
pub const MUTED: u32 = hex(0xa0a0a0, 255);
pub const SOFT: u32 = hex(0xc0c0c0, 255);
pub const GREY: u32 = SOFT;
pub const VEIL: u32 = hex(0x000000, 0);

/// Sizes in GUI pixels.
/// Shorter screens than this (a 720p window at scale 3) keep a narrower margin on top.
pub const TITLED: f32 = 300.0;
pub const ROW: f32 = 24.0;
/// A group's name over its panel, and a page's over its groups while searching.
pub const GROUP_ROW: f32 = 18.0;
pub const PAGE_ROW: f32 = 16.0;
/// Space under a group's panel, and more before the next page's name.
pub const GROUP_GAP: f32 = 10.0;
pub const PAGE_GAP: f32 = 6.0;
/// The control on the right of a row (a button, a box or a slider), and the
/// left column: the search field, the page list and the buttons.
pub const CONTROL_W: f32 = 112.0;
pub const CONTROL_H: f32 = 20.0;
pub const SEARCH_H: f32 = 20.0;
/// Vanilla's scroller: the column `ui::Scroll::press` takes.
pub const SCROLLER_W: f32 = 6.0;
/// The strip under the plate that describes a setting.
pub const DOCK_H: f32 = 54.0;
pub const BUTTON_H: f32 = 20.0;
/// A row's name and control, from its group panel's edges.
pub const INSET: f32 = 10.0;

pub fn fill(batch: &mut HudBatch, atlas: &HudAtlas, [x, y, w, h]: [f32; 4], color: u32) {
    batch.fill(atlas, x, y, w, h, color);
}
/// Square surfaces, as in Minecraft's option lists.
pub fn rounded(batch: &mut HudBatch, atlas: &HudAtlas, rect: [f32; 4], _radius: f32, color: u32) {
    fill(batch, atlas, rect, color);
}
pub fn ring(batch: &mut HudBatch, atlas: &HudAtlas, [x, y, w, h]: [f32; 4], color: u32) {
    for rect in [
        [x, y, w, 1.0],
        [x, y + h - 1.0, w, 1.0],
        [x, y, 1.0, h],
        [x + w - 1.0, y, 1.0, h],
    ] {
        fill(batch, atlas, rect, color);
    }
}
pub fn panel(batch: &mut HudBatch, atlas: &HudAtlas, rect: [f32; 4]) {
    fill(batch, atlas, rect, GLASS);
}
pub fn group(batch: &mut HudBatch, atlas: &HudAtlas, rect: [f32; 4]) {
    fill(batch, atlas, rect, GROUP);
}
pub fn well(batch: &mut HudBatch, atlas: &HudAtlas, rect: [f32; 4]) {
    fill(batch, atlas, rect, WELL);
    ring(batch, atlas, rect, TRACK);
}
pub fn scroller(batch: &mut HudBatch, atlas: &HudAtlas, track: [f32; 4], thumb: [f32; 4]) {
    fill(batch, atlas, track, hex(0x000000, 255));
    fill(batch, atlas, thumb, TRACK);
    let [x, y, w, h] = thumb;
    fill(batch, atlas, [x, y, w - 1.0, h - 1.0], hex(0xc0c0c0, 255));
}
/// `text` cut to `width` with an ellipsis, as a long name gives way to its control.
pub fn fit(atlas: &HudAtlas, text: &str, width: f32) -> String {
    if atlas.font.width(text) <= width {
        return text.to_owned();
    }
    let mut out = String::new();
    for c in text.chars() {
        if atlas.font.width(&format!("{out}{c}...")) > width {
            break;
        }
        out.push(c);
    }
    format!("{}...", out.trim_end())
}
/// Vanilla option toggles use a regular On/Off button.
pub fn checkbox(batch: &mut HudBatch, atlas: &HudAtlas, right: f32, y: f32, on: bool, live: bool) {
    let label = Text::literal(crate::lang::get(if on { "On" } else { "Off" }));
    button(
        batch,
        atlas,
        [right - 56.0, y + (ROW - CONTROL_H) / 2.0, 56.0, CONTROL_H],
        &label,
        (false, live, false),
    );
}
/// Resource-pack-aware Minecraft AbstractButton, without recording a second widget.
pub fn button(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    rect: [f32; 4],
    label: &Text,
    (hover, live, _primary): (bool, bool, bool),
) {
    let [x, y, w, h] = rect;
    let sprite = if !live {
        "gui/sprites/widget/button_disabled"
    } else if hover {
        "gui/sprites/widget/button_highlighted"
    } else {
        "gui/sprites/widget/button"
    };
    ui::nine_slice(batch, atlas, sprite, x, y, w, h, u32::MAX);
    scrolling_label(
        batch,
        atlas,
        label,
        [x + 2.0, y + ((h - 8.0) / 2.0).floor(), w - 4.0],
        (y, h),
        if live { WHITE } else { MUTED },
    );
}
/// A widget label of `[left, y, width]` inside the widget's `(top, height)`: centred when it fits,
/// else slid (vanilla `AbstractWidget.extractScrollingStringOverContents`) and clipped to the box.
fn scrolling_label(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    text: &Text,
    [left, y, width]: [f32; 3],
    (top, height): (f32, f32),
    color: u32,
) {
    let text = text.resolved();
    let text_width = atlas.font.width_text(&text);
    let x = if text_width <= width {
        left + (width / 2.0).floor() - (text_width / 2.0).floor()
    } else {
        let overflow = text_width - width;
        let period = (f64::from(overflow) * 0.5).max(3.0);
        let seconds = crate::hud_overlays::now_ms() as f64 / 1000.0;
        let phase = (std::f64::consts::TAU * seconds / period).cos();
        let amount = (std::f64::consts::FRAC_PI_2 * phase).sin() / 2.0 + 0.5;
        left - (f64::from(overflow) * amount) as i32 as f32
    };
    let first = batch.vertices.len();
    batch.text_styled(atlas, &text, x, y, color, true);
    batch.clip_since(first, left, top, left + width, top + height);
}
/// Minecraft's textured slider track and eight-pixel handle.
pub fn slider(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    rect: [f32; 4],
    value: &Text,
    position: f32,
    (hover, live): (bool, bool),
) {
    let [x, y, w, h] = rect;
    ui::nine_slice(
        batch,
        atlas,
        "gui/sprites/widget/slider",
        x,
        y,
        w,
        h,
        u32::MAX,
    );
    let handle = if hover && live {
        "gui/sprites/widget/slider_handle_highlighted"
    } else {
        "gui/sprites/widget/slider_handle"
    };
    ui::nine_slice(
        batch,
        atlas,
        handle,
        x + (position.clamp(0.0, 1.0) * (w - 8.0)).floor(),
        y,
        8.0,
        h,
        u32::MAX,
    );
    scrolling_label(
        batch,
        atlas,
        value,
        [x + 2.0, y + ((h - 8.0) / 2.0).floor(), w - 4.0],
        (y, h),
        if live { WHITE } else { MUTED },
    );
}
pub fn cycler(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    rect: [f32; 4],
    value: &Text,
    state: (bool, bool),
) {
    button(batch, atlas, rect, value, (state.0, state.1, false));
}
/// A screen's title, centred at the top (split-screen profiles).
pub fn title(batch: &mut HudBatch, atlas: &HudAtlas, text: &Text) {
    let w = batch.width;
    ui::centered_text(batch, atlas, text, 0.0, 12.0, w, u32::MAX);
}
/// The cost's colour in the HUD's encoding: the square beside its name.
pub fn cost_color(cost: Cost) -> u32 {
    hex(cost.rgb(), 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> HudAtlas {
        HudAtlas::build(std::path::Path::new("/nonexistent/atlas-test-assets")).unwrap()
    }

    fn x_span(batch: &HudBatch, plain: &HudBatch) -> (f32, f32) {
        let label = |v: &&crate::hud::HudVertex| !plain.vertices.iter().any(|p| p.pos == v.pos);
        batch
            .vertices
            .iter()
            .filter(label)
            .map(|v| v.pos[0])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| {
                (lo.min(x), hi.max(x))
            })
    }

    /// A label wider than its widget slides inside the widget's box instead of being cut by
    /// characters; a short one stays centred.
    #[test]
    fn long_labels_stay_in_the_widget_and_short_ones_centred() {
        let atlas = atlas();
        let rect = [10.0, 20.0, 100.0, 20.0];
        let long = Text::literal("Graphics API: Prefer Vulkan (Experimental)");
        assert!(atlas.font.width_text(&long.resolved()) > rect[2]);
        let short = Text::literal("Done");
        let short_w = atlas.font.width_text(&short.resolved());
        let draws: [(&str, fn(&mut HudBatch, &HudAtlas, [f32; 4], &Text)); 3] = [
            ("button", |b, a, r, t| {
                button(b, a, r, t, (false, true, false))
            }),
            ("slider", |b, a, r, t| {
                slider(b, a, r, t, 0.5, (false, true))
            }),
            ("cycler", |b, a, r, t| cycler(b, a, r, t, (false, true))),
        ];
        for (name, draw) in draws {
            let mut plain = HudBatch::new([640, 480]);
            draw(&mut plain, &atlas, rect, &Text::empty());
            let base = plain.vertices.len();
            let mut batch = HudBatch::new([640, 480]);
            draw(&mut batch, &atlas, rect, &long);
            assert!(batch.vertices.len() > base, "{name}: long label drawn");
            // The widget's own fill, track and handle also draw: only the label's vertices count.
            let label = batch
                .vertices
                .iter()
                .filter(|v| !plain.vertices.iter().any(|p| p.pos == v.pos));
            for v in label {
                assert!(
                    (rect[0]..=rect[0] + rect[2]).contains(&v.pos[0])
                        && (rect[1]..=rect[1] + rect[3]).contains(&v.pos[1]),
                    "{name}: {:?} outside the widget",
                    v.pos
                );
            }
            // The label's own glyphs (past the widget's fill) start centred.
            let mut batch = HudBatch::new([640, 480]);
            draw(&mut batch, &atlas, rect, &short);
            let (lo, hi) = x_span(&batch, &plain);
            let middle = (lo + hi) / 2.0;
            let want = rect[0] + rect[2] / 2.0;
            assert!(
                (middle - want).abs() <= short_w / 2.0 + 2.0 && (middle - want).abs() < 3.0,
                "{name}: centre {middle}, want {want}"
            );
        }
    }
}
