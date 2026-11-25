//! Inventory controls for AutoCraft, DropAll, and Chest Sorter.
use super::{Game, IDS, Screen};
use atlas_client::{atlas::mod_style, hud::HudBatch, lang, text::Text, ui};

enum Action {
    Craft(usize),
    Drop,
    Sort,
}

struct Button {
    rect: [f32; 4],
    label: Text,
    action: Action,
}

fn hit(rect: [f32; 4], x: f32, y: f32) -> bool {
    x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3]
}

fn button(rect: [f32; 4], label: Text, action: Action) -> Button {
    Button {
        rect,
        label,
        action,
    }
}

impl Game<'_> {
    fn actions_buttons(&self, batch: &HudBatch) -> Vec<Button> {
        let mut out = Vec::new();
        let h = batch.height;
        if self.screen == Screen::Inventory {
            let (x, y) = self.inventory_origin(batch);
            if self.atlas_port_on(IDS[0]) && self.atlas_crafting_table() {
                for (i, dx, bw, key) in [
                    (0, 0.0, 57.0, "atlas.autocraft.crystal"),
                    (1, 59.0, 58.0, "atlas.autocraft.eye"),
                    (2, 119.0, 57.0, "atlas.autocraft.arrows"),
                ] {
                    let mut label = lang::get(key);
                    if self.atlas_ports.actions.autocraft == Some(i) {
                        label = format!("§a▶ {label}");
                    }
                    out.push(button(
                        [x + dx, (y - 22.0).max(2.0), bw, 20.0],
                        Text::legacy(&label),
                        Action::Craft(i),
                    ));
                }
            }
            let chest = self.atlas_actions_chest();
            if self.atlas_port_on(IDS[1])
                && (chest
                    || (self.container.is_none()
                        && self.remote_menu().1 == "inventory"
                        && self.craft_size == 2))
            {
                out.push(button(
                    if chest {
                        [x + 114.0, y + 4.0, 56.0, 14.0]
                    } else {
                        [x + 180.0, h / 2.0 - 22.0, 56.0, 20.0]
                    },
                    Text::tr("atlas.dropall.button", &[]),
                    Action::Drop,
                ));
            }
            if chest && self.atlas_port_on(IDS[3]) {
                out.push(button(
                    [x - 39.0, y + 6.0, 36.0, 16.0],
                    Text::tr("atlas.sortmod.button", &[]),
                    Action::Sort,
                ));
            }
        }
        out
    }

    pub(in crate::game) fn atlas_actions_draw(&mut self, batch: &mut HudBatch) {
        let mut tooltip: Option<Vec<Text>> = None;
        for b in self.actions_buttons(batch) {
            let hover = hit(b.rect, self.cursor.0, self.cursor.1);
            if hover && let Action::Craft(recipe) = b.action {
                let key = match recipe {
                    0 => "atlas.autocraft.crystal.tip",
                    1 => "atlas.autocraft.eye.tip",
                    _ => "atlas.autocraft.arrows.tip",
                };
                tooltip = Some(lang::get(key).lines().map(Text::legacy).collect());
            }
            let [x, y, w, h] = b.rect;
            mod_style::button(batch, &self.gui.atlas, x, y, w, h, &b.label, true, hover);
        }
        if let Some(lines) = tooltip {
            ui::tooltip(batch, &self.gui.atlas, &lines, self.cursor);
        }
    }

    pub(in crate::game) fn atlas_actions_mouse(&mut self, button: i32, pressed: bool) -> bool {
        if !pressed || button != 0 || self.screen != Screen::Inventory {
            return false;
        }
        let Some(renderer) = &self.renderer else {
            return false;
        };
        let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
        let (x, y) = self.cursor;
        let Some(button) = self
            .actions_buttons(&batch)
            .into_iter()
            .find(|b| hit(b.rect, x, y))
        else {
            return false;
        };
        match button.action {
            Action::Craft(recipe) => {
                self.atlas_actions_craft_toggle(recipe);
            }
            Action::Drop => {
                self.atlas_actions_drop_all();
            }
            Action::Sort => {
                self.atlas_actions_start_sort();
            }
        }
        true
    }
}
