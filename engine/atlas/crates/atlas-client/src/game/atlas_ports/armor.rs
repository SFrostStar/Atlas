//! Atlas's own equipped-armour durability HUD, using vanilla item data and icons.
//! This initial adapter shows the local player's gear, not other entities' gear.

mod model {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(super) struct Durability {
        pub remaining: u32,
        pub maximum: u32,
        pub fraction: f32,
    }

    pub(super) fn durability(maximum: u32, damage: u32) -> Option<Durability> {
        if maximum == 0 {
            return None;
        }
        let remaining = maximum.saturating_sub(damage);
        Some(Durability {
            remaining,
            maximum,
            fraction: remaining as f32 / maximum as f32,
        })
    }

    pub(super) fn color(fraction: f32) -> u32 {
        if fraction <= 0.2 {
            0xFF_5C_59
        } else if fraction <= 0.5 {
            0xFF_C6_62
        } else {
            0x9A_D9_A2
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{color, durability};

        #[test]
        fn damaged_and_undamageable_items_have_defined_readings() {
            assert!(durability(0, 0).is_none());
            assert_eq!(durability(363, 0).unwrap().remaining, 363);
            let broken = durability(363, 900).unwrap();
            assert_eq!(broken.remaining, 0);
            assert_eq!(broken.fraction, 0.0);
            assert_eq!(durability(100, 25).unwrap().fraction, 0.75);
        }

        #[test]
        fn low_durability_has_a_distinct_warning() {
            assert_eq!(color(0.0), color(0.2));
            assert_ne!(color(0.2), color(0.21));
            assert_ne!(color(0.5), color(0.51));
        }
    }
}

// Engine adapter.
use super::super::Game;
use atlas_client::hud::{HudBatch, rgba};

pub(super) fn draw(game: &Game<'_>, batch: &mut HudBatch) {
    if game.player.is_none() || game.spectator() {
        return;
    }
    let equipped: Vec<_> = game
        .armor
        .iter()
        .enumerate()
        .filter_map(|(slot, stack)| stack.as_ref().map(|stack| (slot, stack)))
        .collect();
    if equipped.is_empty() {
        return;
    }
    let atlas = &game.gui.atlas;
    let russian = atlas_client::lang::code().starts_with("ru");
    let title = if russian { "Броня" } else { "Armour" };
    let width = 96.0;
    let height = 12.0 + equipped.len() as f32 * 18.0;
    let (x, y) = (
        (batch.width - width - 3.0).max(3.0),
        (batch.height - 25.0 - height).max(3.0),
    );
    batch.fill(atlas, x, y, width, height, rgba(10, 14, 19, 155));
    super::compact(
        batch,
        atlas,
        title,
        x + 3.0,
        y + 3.0,
        rgba(230, 235, 239, 255),
    );
    for (row, (_, stack)) in equipped.iter().enumerate() {
        let py = y + 12.0 + row as f32 * 18.0;
        game.draw_item_model(batch, stack, x + 2.0, py);
        let unbreakable = stack
            .data()
            .and_then(|data| data.other("minecraft:unbreakable"))
            .is_some();
        let maximum = game
            .res
            .items()
            .get(&stack.item)
            .map_or(0, |info| info.max_damage);
        let reading = model::durability(maximum, stack.damage);
        let (label, fraction, rgb) = if unbreakable {
            ("∞".to_owned(), 1.0, 0x9C_CE_F5)
        } else if let Some(d) = reading {
            (
                format!("{}/{}", d.remaining, d.maximum),
                d.fraction,
                model::color(d.fraction),
            )
        } else {
            ("—".to_owned(), 0.0, 0xBA_C4_CD)
        };
        let color = rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255);
        super::compact(batch, atlas, &label, x + 21.0, py + 1.0, color);
        batch.fill(
            atlas,
            x + 21.0,
            py + 12.0,
            width - 26.0,
            2.0,
            rgba(45, 51, 58, 255),
        );
        batch.fill(
            atlas,
            x + 21.0,
            py + 12.0,
            (width - 26.0) * fraction,
            2.0,
            color,
        );
    }
}
