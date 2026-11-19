//! Aspect Ratio 1.0.0 by usaryy, recreated from the supplied Java mod's behavior.
use super::Game;
use atlas_client::{hud::HudBatch, text::Text, ui};
use winit::keyboard::KeyCode;

pub(super) const ID: &str = "atlas-aspectratio";
const KEY_NAME: &str = "open_menu";
const DEFAULT_KEY: &str = "key.keyboard.h";
const MIN_RATIO: f32 = 0.5;
const MAX_RATIO: f32 = 4.0;
const PRESETS: [(f32, &str); 5] = [
    (4.0 / 3.0, "4:3"),
    (16.0 / 10.0, "16:10"),
    (16.0 / 9.0, "16:9"),
    (21.0 / 9.0, "21:9"),
    (32.0 / 9.0, "32:9"),
];

pub(super) fn keys() -> Vec<[String; 3]> {
    vec![[
        KEY_NAME.into(),
        atlas_client::lang::get("key.aspectratio.open_menu"),
        DEFAULT_KEY.into(),
    ]]
}

#[derive(Default)]
pub(super) struct State {
    // Java keeps the override for this game session and Reset uses the window again.
    ratio: Option<f32>,
    dragging: bool,
    slider_focused: bool,
}

struct Layout {
    slider: [f32; 4],
    presets: [[f32; 4]; 5],
    reset: [f32; 4],
}

impl Layout {
    fn new(batch: &HudBatch) -> Self {
        let x = (batch.width / 2.0).floor() - 140.0;
        let y = (batch.height / 2.0).floor() - 20.0;
        Self {
            slider: [x, y, 280.0, 20.0],
            presets: std::array::from_fn(|i| [x + i as f32 * 56.0, y + 28.0, 52.0, 16.0]),
            reset: [x, y + 50.0, 50.0, 16.0],
        }
    }
}

fn ratio_label(ratio: f32) -> String {
    for (preset, label) in PRESETS {
        if (ratio - preset).abs() < 0.02 {
            return label.into();
        }
    }
    format!("{ratio:.2}:1").replace('.', atlas_threads::locale::decimal_separator())
}

impl Game<'_> {
    pub(in crate::game) fn atlas_aspect_ratio(&self) -> Option<f32> {
        if self.atlas_port_on(ID) {
            self.atlas_ports.aspect_ratio.ratio
        } else {
            None
        }
    }

    pub(in crate::game) fn atlas_aspect_open_key(&mut self, key: &str) {
        if self.native_playing()
            && self.player.is_some()
            && self.atlas_port_on(ID)
            && self.native_mod_key(ID, KEY_NAME, DEFAULT_KEY) == key
            && self.atlas_open_port_screen(ID)
        {
            self.atlas_ports.aspect_ratio.dragging = false;
            self.atlas_ports.aspect_ratio.slider_focused = false;
        }
    }

    pub(in crate::game) fn atlas_aspect_screen_key(&mut self, key: KeyCode, repeat: bool) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        if key == KeyCode::Escape || (!repeat && key == KeyCode::KeyH) {
            self.atlas_ports.aspect_ratio.dragging = false;
            self.native_mod_screen_close(ID);
            return true;
        }
        false
    }

    pub(in crate::game) fn atlas_aspect_render(&self, batch: &mut HudBatch) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        let atlas = &self.gui.atlas;
        let state = &self.atlas_ports.aspect_ratio;
        let layout = Layout::new(batch);
        let [x, y, w, h] = layout.slider;
        let ratio = state.ratio.unwrap_or(batch.width / batch.height.max(1.0));
        let label = ratio_label(ratio);
        let label_x = ((batch.width - HudBatch::text_width(atlas, &label)) / 2.0).floor();
        batch.text(atlas, &label, label_x, y - 14.0, u32::MAX, true);
        atlas_client::atlas::mod_style::slider(
            batch,
            atlas,
            x,
            y,
            w,
            &Text::empty(),
            (ratio - MIN_RATIO) / (MAX_RATIO - MIN_RATIO),
            ui::hovered(self.cursor, x, y, w, h) || state.slider_focused,
            true,
            state.dragging,
        );
        for ((_, caption), [x, y, w, h]) in PRESETS.into_iter().zip(layout.presets) {
            atlas_client::atlas::mod_style::button(
                batch,
                atlas,
                x,
                y,
                w,
                h,
                &Text::literal(caption),
                true,
                ui::hovered(self.cursor, x, y, w, h),
            );
        }
        let [x, y, w, h] = layout.reset;
        atlas_client::atlas::mod_style::button(
            batch,
            atlas,
            x,
            y,
            w,
            h,
            &Text::tr("atlas.aspect_ratio.reset", &[]),
            true,
            ui::hovered(self.cursor, x, y, w, h),
        );
        true
    }

    pub(in crate::game) fn atlas_aspect_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        if button != 0 {
            return true;
        }
        if !pressed {
            if std::mem::take(&mut self.atlas_ports.aspect_ratio.dragging) {
                self.audio.play_button_click();
            }
            return true;
        }
        let Some(layout) = self.atlas_aspect_layout() else {
            return true;
        };
        let [x, y, w, h] = layout.slider;
        let state = &mut self.atlas_ports.aspect_ratio;
        state.slider_focused = ui::hovered(self.cursor, x, y, w, h);
        if state.slider_focused {
            state.dragging = true;
            self.atlas_aspect_drag();
        } else if let Some((ratio, _)) =
            PRESETS
                .into_iter()
                .zip(layout.presets)
                .find_map(|(preset, [x, y, w, h])| {
                    ui::hovered(self.cursor, x, y, w, h).then_some(preset)
                })
        {
            state.ratio = Some(ratio);
            self.audio.play_button_click();
        } else {
            let [x, y, w, h] = layout.reset;
            if ui::hovered(self.cursor, x, y, w, h) {
                state.ratio = None;
                self.audio.play_button_click();
            }
        }
        true
    }

    pub(in crate::game) fn atlas_aspect_drag(&mut self) -> bool {
        if self.native_mod_screen() != Some(ID) || !self.atlas_ports.aspect_ratio.dragging {
            return false;
        }
        if let Some(layout) = self.atlas_aspect_layout() {
            let [x, _, w, _] = layout.slider;
            self.atlas_ports.aspect_ratio.ratio =
                Some(MIN_RATIO + ui::slider_value(x, w, self.cursor.0) * (MAX_RATIO - MIN_RATIO));
        }
        true
    }

    fn atlas_aspect_layout(&self) -> Option<Layout> {
        let renderer = self.renderer.as_ref()?;
        Some(Layout::new(&HudBatch::new([
            renderer.config.width,
            renderer.config.height,
        ])))
    }
}
