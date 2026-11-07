//! AppleSkin 3.0.10 (squeek502), Unlicense; adapted to Rust for Atlas.
//! Reference: the user's appleskin-fabric-mc26.3-3.0.10.jar.
//! Keep Java's f32 arithmetic and strictly-greater exhaustion boundary.

pub fn estimated_health(mut food: i32, mut saturation: f32, mut exhaustion: f32) -> f32 {
    if !saturation.is_finite() || !exhaustion.is_finite() {
        return 0.0;
    }
    let mut health = 0.0;
    while food >= 18 {
        while exhaustion > 4.0 {
            exhaustion -= 4.0;
            if saturation > 0.0 {
                saturation = (saturation - 1.0).max(0.0);
            } else {
                food -= 1;
            }
        }
        if food >= 20 && saturation > f32::MIN_POSITIVE {
            let limited = saturation.min(6.0);
            let iterations = ((4.0_f32.next_up() - exhaustion) / limited).ceil().max(1.0) as i32;
            health += limited / 6.0 * iterations as f32;
            exhaustion += limited * iterations as f32;
        } else if food >= 18 {
            health += 1.0;
            exhaustion += 6.0;
        }
    }
    health
}

/// The four original saturation-outline sprites, including quarter units.
pub fn saturation_sprite(value_in_half_icons: f32) -> f32 {
    if value_in_half_icons >= 1.0 {
        27.0
    } else if value_in_half_icons > 0.5 {
        18.0
    } else if value_in_half_icons > 0.25 {
        9.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Default)]
pub struct Flash {
    pub value: f32,
    pub falling: bool,
}
impl Flash {
    pub fn tick(&mut self) {
        self.value += if self.falling { -0.125 } else { 0.125 };
        if self.value >= 1.5 {
            self.falling = true;
        } else if self.value <= -0.5 {
            self.falling = false;
        }
    }
    pub fn alpha(self, maximum: f32) -> f32 {
        self.value.clamp(0.0, 1.0) * maximum.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarters_use_original_strict_boundaries() {
        assert_eq!(
            [0.1, 0.25, 0.5, 0.75, 1.0].map(saturation_sprite),
            [0.0, 0.0, 9.0, 18.0, 27.0]
        );
    }
    #[test]
    fn flash_preserves_bright_and_dark_holds() {
        let mut flash = Flash::default();
        let values: Vec<_> = (0..32)
            .map(|_| {
                flash.tick();
                flash.alpha(0.65)
            })
            .collect();
        assert_eq!(values.iter().filter(|v| **v == 0.65).count(), 9);
        assert_eq!(values.iter().filter(|v| **v == 0.0).count(), 9);
        assert_eq!(flash.value, 0.0);
    }
    #[test]
    fn regen_respects_food_threshold_and_nonfinite_values() {
        assert_eq!(estimated_health(17, 0.0, 0.0), 0.0);
        assert_eq!(estimated_health(18, 0.0, 0.0), 1.0);
        assert_eq!(estimated_health(18, 0.0, 4.0), 1.0);
        assert_eq!(estimated_health(18, 0.0, 4.000001), 0.0);
        assert_eq!(estimated_health(20, 5.0, f32::NAN), 0.0);
    }
}
