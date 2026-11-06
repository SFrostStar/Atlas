//! Atlas's own food HUD: current hunger/saturation and held-food restoration.
//! Uses the same exported FoodProperties as the game's consumable path. No Java
//! implementation or artwork is included; nutrition never changes simulation.

mod model {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(super) struct Food {
        pub hunger: i32,
        pub saturation: f32,
    }

    impl Food {
        pub(super) fn new(hunger: i32, saturation: f32) -> Self {
            let hunger = hunger.clamp(0, 20);
            let saturation = if saturation.is_finite() {
                saturation.clamp(0.0, hunger as f32)
            } else {
                0.0
            };
            Self { hunger, saturation }
        }

        /// FoodData.eat_properties: saturation is already an absolute gain,
        /// not the modifier multiplied by nutrition in the JSON component.
        pub(super) fn after(self, nutrition: u32, saturation: f32) -> Self {
            let hunger = (self.hunger as u32).saturating_add(nutrition).min(20) as i32;
            let gain = if saturation.is_finite() {
                saturation.max(0.0)
            } else {
                0.0
            };
            Self::new(hunger, self.saturation + gain)
        }

        pub(super) fn fraction(self) -> f32 {
            self.saturation / 20.0
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Food;

        #[test]
        fn held_food_preview_caps_both_values_as_eating_does() {
            let food = Food::new(18, 4.0);
            assert_eq!(food.after(8, 12.8), Food::new(20, 16.8));
            assert_eq!(Food::new(10, 1.0).after(4, 2.4), Food::new(14, 3.4));
            assert_eq!(Food::new(20, 19.0).after(4, 10.0), Food::new(20, 20.0));
        }

        #[test]
        fn invalid_server_values_never_make_invalid_hud_geometry() {
            assert_eq!(Food::new(-5, 10.0), Food::new(0, 0.0));
            assert_eq!(Food::new(50, f32::INFINITY), Food::new(20, 0.0));
            assert_eq!(Food::new(10, f32::NAN).fraction(), 0.0);
            assert_eq!(Food::new(1, 0.0).after(u32::MAX, -5.0), Food::new(20, 0.0));
        }
    }
}

// Engine adapter.
use super::super::Game;
use atlas_client::hud::{HudBatch, rgba};
use model::Food;

/// The caller owns install/enable checks and F1 visibility, like native_hud.
pub(super) fn draw(game: &Game<'_>, batch: &mut HudBatch) {
    let Some(player) = game.player.as_ref() else {
        return;
    };
    if game.spectator() {
        return;
    }
    let food = Food::new(player.health.food.food, player.health.food.saturation);
    let held = [game.held(), game.offhand.as_ref()]
        .into_iter()
        .flatten()
        .find_map(|stack| game.res.items().get(&stack.item).and_then(|info| info.food));
    let russian = atlas_client::lang::code().starts_with("ru");
    let hunger = if russian { "Голод" } else { "Hunger" };
    let saturation = if russian { "Насыщ." } else { "Saturation" };
    let mut lines = vec![
        format!("{hunger} {}/20", food.hunger),
        format!("{saturation} {:.1}/20", food.saturation),
    ];
    if let Some((nutrition, gain)) = held {
        let next = food.after(nutrition, gain);
        let prefix = if russian { "Еда" } else { "Food" };
        lines.push(format!(
            "{prefix} +{} / +{:.1}",
            next.hunger - food.hunger,
            next.saturation - food.saturation
        ));
    }
    let atlas = &game.gui.atlas;
    let width = lines
        .iter()
        .map(|line| HudBatch::text_width(atlas, line))
        .fold(80.0, f32::max)
        + 6.0;
    let height = lines.len() as f32 * 10.0 + 7.0;
    let x = (batch.width - width - 3.0).max(3.0);
    let armor_rows = if game.atlas_port_on("atlas-armorindicator") {
        game.armor.iter().filter(|s| s.is_some()).count()
    } else {
        0
    };
    let armor_height = if armor_rows == 0 {
        0.0
    } else {
        16.0 + armor_rows as f32 * 18.0
    };
    let y = (batch.height - 25.0 - armor_height - height).max(3.0);
    batch.fill(atlas, x, y, width, height, rgba(10, 14, 19, 155));
    for (i, line) in lines.iter().enumerate() {
        let color = if i == 2 {
            rgba(255, 190, 114, 255)
        } else {
            rgba(230, 235, 239, 255)
        };
        super::compact(
            batch,
            atlas,
            line,
            x + 3.0,
            y + 3.0 + i as f32 * 10.0,
            color,
        );
    }
    let by = y + height - 3.0;
    batch.fill(atlas, x + 3.0, by, width - 6.0, 1.0, rgba(66, 57, 41, 255));
    if let Some((nutrition, gain)) = held {
        batch.fill(
            atlas,
            x + 3.0,
            by,
            (width - 6.0) * food.after(nutrition, gain).fraction(),
            1.0,
            rgba(125, 87, 43, 255),
        );
    }
    batch.fill(
        atlas,
        x + 3.0,
        by,
        (width - 6.0) * food.fraction(),
        1.0,
        rgba(255, 176, 76, 255),
    );
}
