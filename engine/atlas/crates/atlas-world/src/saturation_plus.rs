//! Saturation Plus 1.3.1-1.21.2, MrKirbychu, CC0-1.0.
//! Adapted from the supplied HungerMixin, ExhaustionMixin, Utilities and
//! SaturationStatusEffectMixin. See MOD-PORT-NOTICES.md for provenance.
use crate::food::FoodData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub hunger_limits_saturation: bool,
    pub max_saturation: i32,
    pub always_hungry: bool,
    pub max_exhaustion: i32,
    pub saturation_effect_decay_rate: i32,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            hunger_limits_saturation: false,
            max_saturation: -1,
            always_hungry: false,
            max_exhaustion: 4,
            saturation_effect_decay_rate: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub saturation: f32,
    pub exhaustion: f32,
    pub threshold: f32,
}

/// `HungerManager.addInternal` converts excess nutrition to extra saturation.
/// The gamerule caps are applied at the start of the next food tick.
pub fn add_food(food: &mut FoodData, nutrition: i32, mut saturation: f32) {
    let total = food.food.wrapping_add(nutrition);
    if total > 20 {
        saturation += (total - 20) as f32;
    }
    food.food = total.min(20);
    food.saturation += saturation;
}

impl Rules {
    fn saturation_cap(self, food: i32) -> f32 {
        let cap = if self.max_saturation < 0 {
            f32::MAX
        } else {
            self.max_saturation as f32
        };
        if self.hunger_limits_saturation {
            cap.min(food as f32)
        } else {
            cap
        }
    }

    /// The commands report the capped value before the next food tick applies it.
    pub fn clamp_saturation(self, food: i32, saturation: f32) -> f32 {
        saturation.min(self.saturation_cap(food)).max(0.0)
    }

    /// Run before the usual food tick. Use the returned threshold for both
    /// comparisons and subtraction in the vanilla exhaustion branch.
    pub fn before_tick(self, food: &mut FoodData) -> Snapshot {
        let threshold = if self.max_exhaustion <= 0 {
            food.exhaustion = 0.0;
            i32::MAX as f32
        } else {
            self.max_exhaustion as f32
        };
        food.saturation = food.saturation.min(self.saturation_cap(food.food));
        if food.exhaustion > threshold && food.saturation >= 16_777_216.0 {
            food.exhaustion -= threshold;
            // At this magnitude subtracting 1 can round away. The Java mod
            // subtracts one representable step, including at powers of two.
            let is_power_of_two =
                food.saturation.is_finite() && food.saturation.to_bits() & 0x007f_ffff == 0;
            let divisor = if is_power_of_two {
                16_777_216.0
            } else {
                8_388_608.0
            };
            let loss = (f64::from(food.saturation) / divisor) as f32;
            food.saturation = (food.saturation - loss).max(0.0);
        }
        Snapshot {
            saturation: food.saturation,
            exhaustion: food.exhaustion,
            threshold,
        }
    }

    /// The Saturation status effect's nutrition is its amplifier plus one.
    pub fn apply_effect(self, food: &mut FoodData, nutrition: i32) {
        let saturation = food.saturation;
        let gain = nutrition as f32 * 2.0;
        if saturation <= 20.0 {
            add_food(food, nutrition, gain);
            return;
        }
        if food.food < 20 {
            add_food(food, nutrition, 0.0);
        }
        let decay_base = saturation - if saturation < 100.0 { 19.0 } else { 0.0 };
        let divisor = atlas_core::jmath::pow(
            f64::from(decay_base),
            f64::from(self.saturation_effect_decay_rate.wrapping_abs()),
        ) as f32;
        food.saturation = saturation + gain / divisor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excess_nutrition_can_keep_saturation_above_hunger() {
        let mut food = FoodData {
            food: 19,
            saturation: 18.0,
            ..FoodData::default()
        };
        add_food(&mut food, 4, 2.0);
        assert_eq!((food.food, food.saturation), (20, 23.0));
        assert_eq!(Rules::default().before_tick(&mut food).saturation, 23.0);
        let rules = Rules {
            hunger_limits_saturation: true,
            max_saturation: 12,
            ..Rules::default()
        };
        assert_eq!(rules.clamp_saturation(food.food, 23.0), 12.0);
        assert_eq!(rules.before_tick(&mut food).saturation, 12.0);
    }

    #[test]
    fn exhaustion_zero_and_large_float_loss_follow_java() {
        let mut food = FoodData {
            saturation: 16_777_216.0,
            exhaustion: 10.0,
            ..FoodData::default()
        };
        let rules = Rules {
            max_exhaustion: 6,
            ..Rules::default()
        };
        assert_eq!(
            rules.before_tick(&mut food),
            Snapshot {
                saturation: 16_777_215.0,
                exhaustion: 4.0,
                threshold: 6.0
            }
        );
        food.saturation = 16_777_218.0;
        food.exhaustion = 7.0;
        assert_eq!(rules.before_tick(&mut food).saturation, 16_777_216.0);
        let disabled = Rules {
            max_exhaustion: 0,
            ..rules
        };
        assert_eq!(
            disabled.before_tick(&mut food),
            Snapshot {
                saturation: 16_777_216.0,
                exhaustion: 0.0,
                threshold: 2_147_483_648.0
            }
        );
    }

    #[test]
    fn effect_decay_changes_at_twenty_and_one_hundred() {
        let mut food = FoodData {
            saturation: 20.0,
            ..FoodData::default()
        };
        Rules::default().apply_effect(&mut food, 1);
        assert_eq!(food.saturation, 23.0);
        food.saturation = 21.0;
        food.food = 19;
        Rules::default().apply_effect(&mut food, 2);
        assert_eq!((food.food, food.saturation), (20, 22.0));
        food.saturation = 100.0;
        Rules::default().apply_effect(&mut food, 1);
        assert_eq!(food.saturation, 100.0002);
    }

    #[test]
    fn client_food_prediction_uses_the_same_uncapped_addition() {
        let food = FoodData {
            food: 20,
            saturation: 27.5,
            ..FoodData::default()
        };
        let mut vanilla = food;
        vanilla.eat_with_rules(4, 2.0, None);
        assert_eq!((vanilla.food, vanilla.saturation), (20, 20.0));
        let mut modded = food;
        modded.eat_with_rules(4, 2.0, Some(Rules::default()));
        assert_eq!((modded.food, modded.saturation), (20, 33.5));
    }

    #[test]
    fn original_jar_float_bits_match_at_decay_and_precision_boundaries() {
        // Golden results from the supplied JAR's mixins, before vanilla's food tick.
        let mut food = FoodData {
            saturation: f32::MAX,
            exhaustion: 10.0,
            ..FoodData::default()
        };
        let snapshot = Rules::default().before_tick(&mut food);
        assert_eq!(snapshot.saturation.to_bits(), 0x7f7f_fffd);
        assert_eq!(snapshot.exhaustion, 6.0);
        for (saturation, nutrition, decay, bits) in [
            (21.0, 3, 2, 0x41b4_0000),
            (99.0, 1, 2, 0x42c6_0029),
            (100.0, 1, 2, 0x42c8_001a),
            (100.0, 4, 0, 0x42d8_0000),
        ] {
            food.food = 18;
            food.saturation = saturation;
            Rules {
                saturation_effect_decay_rate: decay,
                ..Default::default()
            }
            .apply_effect(&mut food, nutrition);
            assert_eq!(food.saturation.to_bits(), bits);
        }
    }
}
