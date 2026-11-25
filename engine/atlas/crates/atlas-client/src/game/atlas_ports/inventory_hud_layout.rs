use super::{Config, EQUIPMENT, Game, Horizontal, Place, Vertical, label};
use atlas_client::{
    atlas::mod_style,
    hud::{HudBatch, rgba},
    items::ItemStack,
    text::Text,
    widgets,
};
use serde_json::Value;
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Inventory,
    Effects,
    Equipment,
    Slot(usize),
}

#[derive(Clone, Copy)]
struct Region {
    part: Part,
    rect: [f32; 4],
}

#[derive(Clone, Copy)]
struct Drag {
    part: Part,
    offset: [f32; 2],
    size: [f32; 2],
}

pub(in crate::game::atlas_ports) struct PositionEditor {
    config: Config,
    focused: Part,
    drag: Option<Drag>,
    window: [f32; 2],
}

const ICONS: [&str; 8] = [
    "minecraft:iron_helmet",
    "minecraft:iron_chestplate",
    "minecraft:iron_leggings",
    "minecraft:iron_boots",
    "minecraft:shield",
    "minecraft:iron_sword",
    "minecraft:chest",
    "minecraft:arrow",
];

fn hit([x, y, w, h]: [f32; 4], [mx, my]: [f32; 2]) -> bool {
    x <= mx && mx < x + w && y <= my && my < y + h
}

impl PositionEditor {
    pub(in crate::game::atlas_ports) fn new(draft: &Value) -> Option<Self> {
        Some(Self {
            config: serde_json::from_value(draft.clone()).ok()?,
            focused: Part::Inventory,
            drag: None,
            window: [0.0; 2],
        })
    }

    pub(in crate::game::atlas_ports) fn finish(self) -> Value {
        serde_json::to_value(self.config).expect("inventory HUD layout is serializable")
    }

    fn regions(&self) -> Vec<Region> {
        let width = if self.config.inventory.vertical {
            52.0
        } else {
            160.0
        };
        let height = if self.config.inventory.vertical {
            160.0
        } else {
            52.0
        };
        let scale = if self.config.inventory.compact {
            0.75
        } else {
            1.0
        };
        let size = [width * scale, height * scale];
        let [ix, iy] = self.config.inventory.position.clamped(self.window, size);
        let [ew, eh] = self.config.effects.dimensions();
        let ([ex, ey], _) = self.config.effects.origin(self.window, 1);
        let mut regions = vec![
            Region {
                part: Part::Inventory,
                rect: [ix, iy, size[0], size[1]],
            },
            Region {
                part: Part::Effects,
                rect: [ex, ey, ew, eh],
            },
        ];
        let equipment = &self.config.equipment;
        let scale = f32::from(equipment.scale) / 100.0;
        if equipment.move_together {
            let [w, h] = [90.0 * scale, 70.0 * scale];
            let [x, y] = equipment.position.clamped(self.window, [w, h]);
            regions.push(Region {
                part: Part::Equipment,
                rect: [x, y, w, h],
            });
        } else {
            let size = 16.0 * scale;
            regions.extend(equipment.slots.iter().enumerate().map(|(slot, place)| {
                let [x, y] = place.clamped(self.window, [size; 2]);
                Region {
                    part: Part::Slot(slot),
                    rect: [x, y, size, size],
                }
            }));
        }
        regions
    }

    fn done(&self) -> [f32; 4] {
        [self.window[0] - 86.0, 8.0, 78.0, 20.0]
    }

    pub(in crate::game::atlas_ports) fn draw(&mut self, batch: &mut HudBatch, game: &Game<'_>) {
        self.window = [batch.width, batch.height];
        let atlas = &game.gui.atlas;
        batch.fill(
            atlas,
            0.0,
            0.0,
            self.window[0],
            self.window[1],
            rgba(5, 9, 16, 112),
        );
        batch.text(atlas, &label("edit_positions"), 10.0, 9.0, u32::MAX, true);
        batch.text(
            atlas,
            &label("move_instruction"),
            10.0,
            22.0,
            rgba(185, 199, 220, 255),
            true,
        );
        for region in self.regions() {
            let [x, y, w, h] = region.rect;
            let active = region.part == self.focused;
            let name = label(match region.part {
                Part::Inventory => "inventory",
                Part::Effects => "effects",
                Part::Equipment => "equipment",
                Part::Slot(slot) => EQUIPMENT[slot],
            });
            let color = if active {
                rgba(119, 172, 255, 210)
            } else {
                rgba(140, 154, 175, 150)
            };
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, color);
            batch.fill(atlas, x, y, w, h, rgba(25, 34, 48, 244));
            match region.part {
                Part::Slot(slot) => {
                    let first = batch.vertices.len();
                    game.draw_item_model(batch, &ItemStack::new(ICONS[slot], 1), x, y);
                    batch.scale_since(first, x, y, w / 16.0, h / 16.0);
                }
                Part::Effects if w < 32.0 => {
                    batch.sprite_scaled(
                        atlas,
                        "mob_effect/speed",
                        x + (w - 12.0) / 2.0,
                        y + (h - 12.0) / 2.0,
                        12.0,
                        12.0,
                        u32::MAX,
                    );
                }
                _ => {
                    let width = atlas.font.width(&name);
                    let start = batch.vertices.len();
                    batch.text(
                        atlas,
                        &name,
                        x + ((w - width) / 2.0).max(2.0),
                        y + (h - 8.0) / 2.0,
                        u32::MAX,
                        true,
                    );
                    batch.clip_since(start, x, y, x + w, y + h);
                }
            }
            widgets::record(&name, region.rect, true);
            if hit(region.rect, [game.cursor.0, game.cursor.1]) {
                widgets::widget_cursor(true, true);
            }
        }
        let [x, y, w, h] = self.done();
        mod_style::button(
            batch,
            atlas,
            x,
            y,
            w,
            h,
            &Text::tr("gui.done", &[]),
            true,
            hit(self.done(), [game.cursor.0, game.cursor.1]),
        );
    }

    /// Returns true once the player finishes arranging the overlays.
    pub(in crate::game::atlas_ports) fn mouse(
        &mut self,
        mouse: [f32; 2],
        button: i32,
        pressed: bool,
    ) -> bool {
        if button != 0 {
            return false;
        }
        if !pressed {
            self.drag = None;
            return false;
        }
        if hit(self.done(), mouse) {
            return true;
        }
        if let Some(region) = self
            .regions()
            .into_iter()
            .rev()
            .find(|r| hit(r.rect, mouse))
        {
            self.focused = region.part;
            let [x, y, w, h] = region.rect;
            self.drag = Some(Drag {
                part: region.part,
                offset: [mouse[0] - x, mouse[1] - y],
                size: [w, h],
            });
        }
        false
    }

    pub(in crate::game::atlas_ports) fn dragged(&mut self, mouse: [f32; 2]) {
        if let Some(drag) = self.drag {
            self.place_at(
                drag.part,
                [mouse[0] - drag.offset[0], mouse[1] - drag.offset[1]],
                drag.size,
            );
        }
    }

    pub(in crate::game::atlas_ports) fn key(&mut self, key: KeyCode, shift: bool) -> bool {
        if matches!(key, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) {
            return true;
        }
        let regions = self.regions();
        let index = regions
            .iter()
            .position(|region| region.part == self.focused)
            .unwrap_or(0);
        if key == KeyCode::Tab {
            let step = if shift { regions.len() - 1 } else { 1 };
            self.focused = regions[(index + step) % regions.len()].part;
            return false;
        }
        let delta = if shift { 10.0 } else { 1.0 };
        let [dx, dy] = match key {
            KeyCode::ArrowLeft => [-delta, 0.0],
            KeyCode::ArrowRight => [delta, 0.0],
            KeyCode::ArrowUp => [0.0, -delta],
            KeyCode::ArrowDown => [0.0, delta],
            _ => return false,
        };
        let [x, y, w, h] = regions[index].rect;
        self.place_at(regions[index].part, [x + dx, y + dy], [w, h]);
        false
    }

    fn place_at(&mut self, part: Part, origin: [f32; 2], size: [f32; 2]) {
        let [width, height] = self.window;
        let [w, h] = size;
        let x = origin[0].clamp(0.0, (width - w).max(0.0));
        let y = origin[1].clamp(0.0, (height - h).max(0.0));
        let right_padding = if part == Part::Effects {
            if self.config.effects.horizontal {
                self.config.effects.origin(self.window, 1).1
            } else {
                w
            }
        } else {
            0.0
        };
        let (horizontal, ox) = if x + w / 2.0 <= width / 3.0 {
            (Horizontal::Left, x)
        } else if x + w / 2.0 >= width * 2.0 / 3.0 {
            (Horizontal::Right, width - x - right_padding)
        } else {
            (Horizontal::Middle, ((width - w) / 2.0).floor() - x)
        };
        let (vertical, oy) = if y + h / 2.0 <= height / 3.0 {
            (Vertical::Top, y)
        } else if y + h / 2.0 >= height * 2.0 / 3.0 {
            (Vertical::Bottom, height - y)
        } else {
            (Vertical::Center, ((height - h) / 2.0).floor() - y)
        };
        let position = Place {
            horizontal,
            vertical,
            x: ox.round() as i32,
            y: oy.round() as i32,
        };
        match part {
            Part::Inventory => self.config.inventory.position = position,
            Part::Effects => self.config.effects.position = position,
            Part::Equipment => self.config.equipment.position = position,
            Part::Slot(slot) => self.config.equipment.slots[slot] = position,
        }
    }
}
