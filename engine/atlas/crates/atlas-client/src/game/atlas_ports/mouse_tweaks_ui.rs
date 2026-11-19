//! ConfigScreen's original two-column controls and four full-width cycle rows.
use super::{Config, Game, ID};
use atlas_client::{hud::HudBatch, lang, text::Text, ui};
use winit::keyboard::KeyCode;

fn layout(width: f32, height: f32) -> [[f32; 4]; 9] {
    let (x, y) = ((width as i32 / 2 - 155) as f32, (height as i32 / 6) as f32);
    [
        [x, y, 150.0, 20.0],
        [x, y + 24.0, 150.0, 20.0],
        [x + 160.0, y, 150.0, 20.0],
        [x + 160.0, y + 24.0, 150.0, 20.0],
        [x, y + 48.0, 310.0, 20.0],
        [x, y + 72.0, 310.0, 20.0],
        [x, y + 96.0, 310.0, 20.0],
        [x, y + 120.0, 310.0, 20.0],
        [(width as i32 / 2 - 100) as f32, height - 27.0, 200.0, 20.0],
    ]
}

fn labels(config: Config) -> Vec<Text> {
    let on_off = |value| Text::tr(if value { "options.on" } else { "options.off" }, &[]);
    let values = [
        ("rmb", on_off(config.rmb)),
        ("wheel", on_off(config.wheel)),
        ("lmb_with_item", on_off(config.lmb_with_item)),
        ("lmb_without_item", on_off(config.lmb_without_item)),
        (
            "search_order",
            Text::tr(
                if config.search_backwards {
                    "atlas.mouse_tweaks.last_to_first"
                } else {
                    "atlas.mouse_tweaks.first_to_last"
                },
                &[],
            ),
        ),
        (
            "direction",
            Text::tr(
                match config.direction {
                    0 => "atlas.mouse_tweaks.normal",
                    1 => "atlas.mouse_tweaks.inverted",
                    2 => "atlas.mouse_tweaks.position",
                    _ => "atlas.mouse_tweaks.position_inverted",
                },
                &[],
            ),
        ),
        (
            "scaling",
            Text::tr(
                if config.always_one {
                    "atlas.mouse_tweaks.always_one"
                } else {
                    "atlas.mouse_tweaks.proportional"
                },
                &[],
            ),
        ),
        ("debug", on_off(config.debug)),
    ];
    let mut labels: Vec<_> = values
        .into_iter()
        .map(|(key, value)| {
            Text::translate(
                "options.generic_value",
                &[Text::tr(&format!("atlas.mouse_tweaks.{key}"), &[]), value],
            )
        })
        .collect();
    labels.push(Text::tr("gui.done", &[]));
    labels
}

impl Game<'_> {
    pub(in crate::game) fn atlas_mouse_tweaks_open(&mut self) -> bool {
        if !self.atlas_open_port_screen(ID) {
            return false;
        }
        self.atlas_ports.mouse_tweaks.config = Config::load(&Self::game_dir());
        self.atlas_ports.mouse_tweaks.gui_focus = None;
        true
    }

    pub(in crate::game) fn atlas_mouse_tweaks_closed(&mut self) {
        if self.native_mod_screen() == Some(ID)
            && let Err(error) = self.atlas_ports.mouse_tweaks.config.save(&Self::game_dir())
        {
            log::warn!("Mouse Tweaks: cannot save configuration: {error}");
        }
    }

    pub(in crate::game) fn atlas_mouse_tweaks_render(&self, batch: &mut HudBatch) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        let atlas = &self.gui.atlas;
        let title = lang::get("atlas.mouse_tweaks.title");
        batch.text(
            atlas,
            &title,
            ((batch.width - HudBatch::text_width(atlas, &title)) / 2.0).floor(),
            15.0,
            u32::MAX,
            true,
        );
        let state = &self.atlas_ports.mouse_tweaks;
        for (index, ([x, y, w, h], label)) in layout(batch.width, batch.height)
            .into_iter()
            .zip(labels(state.config))
            .enumerate()
        {
            atlas_client::atlas::mod_style::button(
                batch,
                atlas,
                x,
                y,
                w,
                h,
                &label,
                true,
                ui::hovered(self.cursor, x, y, w, h) || state.gui_focus == Some(index),
            );
        }
        true
    }

    fn mouse_tweaks_activate(&mut self, index: usize, reverse: bool) {
        let config = &mut self.atlas_ports.mouse_tweaks.config;
        match index {
            0 => config.rmb = !config.rmb,
            1 => config.wheel = !config.wheel,
            2 => config.lmb_with_item = !config.lmb_with_item,
            3 => config.lmb_without_item = !config.lmb_without_item,
            4 => config.search_backwards = !config.search_backwards,
            5 => config.direction = (config.direction + if reverse { 3 } else { 1 }) % 4,
            6 => config.always_one = !config.always_one,
            7 => config.debug = !config.debug,
            _ => self.native_mod_screen_close(ID),
        }
        self.audio.play_button_click();
    }

    pub(in crate::game) fn atlas_mouse_tweaks_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        if !pressed || !matches!(button, 0 | 1) {
            return true;
        }
        let Some(renderer) = &self.renderer else {
            return true;
        };
        let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
        if let Some(index) = layout(batch.width, batch.height)
            .into_iter()
            .position(|[x, y, w, h]| ui::hovered(self.cursor, x, y, w, h))
        {
            if index == 8 && button != 0 {
                return true;
            }
            self.atlas_ports.mouse_tweaks.gui_focus = Some(index);
            self.mouse_tweaks_activate(index, button == 1 || self.has_shift_down());
        }
        true
    }

    pub(in crate::game) fn atlas_mouse_tweaks_key(&mut self, key: KeyCode) -> bool {
        if self.native_mod_screen() != Some(ID) {
            return false;
        }
        match key {
            KeyCode::Escape => self.native_mod_screen_close(ID),
            KeyCode::Tab => {
                let reverse = self.has_shift_down();
                let focus = &mut self.atlas_ports.mouse_tweaks.gui_focus;
                *focus = Some(match *focus {
                    None => {
                        if reverse {
                            8
                        } else {
                            0
                        }
                    }
                    Some(index) => (index + if reverse { 8 } else { 1 }) % 9,
                });
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(index) = self.atlas_ports.mouse_tweaks.gui_focus {
                    self.mouse_tweaks_activate(index, self.has_shift_down());
                }
            }
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_button_dimensions_and_order_fit_two_columns() {
        let rows = layout(427.0, 240.0);
        assert_eq!(rows[0], [58.0, 40.0, 150.0, 20.0]);
        assert_eq!(rows[2], [218.0, 40.0, 150.0, 20.0]);
        assert_eq!(rows[4], [58.0, 88.0, 310.0, 20.0]);
        assert_eq!(rows[7], [58.0, 160.0, 310.0, 20.0]);
        assert_eq!(rows[8], [113.0, 213.0, 200.0, 20.0]);
    }
}
