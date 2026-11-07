//! Independent observable policies of the supplied utility mods.
pub(super) fn auto_sprint(
    playing: bool,
    forward: bool,
    sneaking: bool,
    swimming: bool,
    food: i32,
) -> bool {
    playing && forward && !sneaking && !swimming && food > 6
}
pub(super) fn mentions(message: &str, names: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    let message = message.to_lowercase();
    names
        .into_iter()
        .any(|n| !n.as_ref().trim().is_empty() && message.contains(&n.as_ref().to_lowercase()))
}
pub(super) fn own_prefix(message: &str, own: &str) -> bool {
    let text = message.to_lowercase();
    let own = own.to_lowercase();
    [
        format!("<{own}>"),
        format!("{own}:"),
        format!("{own} »"),
        format!("{own} >"),
    ]
    .iter()
    .any(|p| text.starts_with(p))
}
#[derive(Default)]
pub(super) struct MentionLimiter {
    last: Option<u64>,
}
impl MentionLimiter {
    pub(super) fn take(&mut self, now_ms: u64, cooldown_ms: u64) -> bool {
        if self
            .last
            .is_some_and(|last| now_ms.saturating_sub(last) < cooldown_ms)
        {
            return false;
        }
        self.last = Some(now_ms);
        true
    }
}
pub(super) fn scroll_tier(current: i32, delta: f32, maximum: i32) -> i32 {
    (current
        + if delta > 0.0 {
            1
        } else if delta < 0.0 {
            -1
        } else {
            0
        })
    .clamp(0, maximum.max(0))
}
pub(super) fn zoom_divisor(initial: f64, progress: f64, step_percent: f64, tier: f64) -> f64 {
    let multiplier = 1.0 + (1.0 / initial.max(1.0) - 1.0) * progress;
    ((1.0 / multiplier) * (step_percent / 100.0).powf(tier)).clamp(0.5, 500.0)
}
pub(super) fn transition(kind: &str, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match kind {
        "INSTANT" => {
            if t > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        "EASE_IN_EXP" => {
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else {
                2f64.powf(10.0 * t - 1023f64.log2())
            }
        }
        "EASE_OUT_EXP" => {
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else {
                1.0 - 2f64.powf(10.0 - 1023f64.log2() - 10.0 * t)
            }
        }
        "EASE_IN_SINE" => 1.0 - (t * std::f64::consts::FRAC_PI_2).cos(),
        "EASE_OUT_SINE" => (t * std::f64::consts::FRAC_PI_2).sin(),
        "EASE_IN_OUT_SINE" => -((std::f64::consts::PI * t).cos() - 1.0) / 2.0,
        "EASE_IN_QUAD" => t * t,
        "EASE_OUT_QUAD" => 1.0 - (1.0 - t).powi(2),
        "EASE_IN_OUT_QUAD" => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        "EASE_IN_CUBIC" => t * t * t,
        "EASE_OUT_CUBIC" => 1.0 - (1.0 - t).powi(3),
        "EASE_IN_OUT_CUBIC" => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
        "EASE_IN_OUT_EXP" => {
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else if t < 0.5 {
                2f64.powf(20.0 * t - 1023f64.log2()) / 2.0
            } else {
                (2.0 - 2f64.powf(20.0 - 1023f64.log2() - 20.0 * t)) / 2.0
            }
        }
        _ => t,
    }
}
pub(super) fn phase(kind: &str, t: f64, entering: bool) -> f64 {
    if entering {
        transition(kind, t)
    } else {
        1.0 - transition(kind, 1.0 - t)
    }
}
pub(super) fn inverse_phase(kind: &str, value: f64, entering: bool) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    if value >= 1.0 {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..48 {
        let mid = (lo + hi) / 2.0;
        if phase(kind, mid, entering) < value {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sprint_matches_original_boundary() {
        assert!(!auto_sprint(true, true, false, false, 6));
        assert!(auto_sprint(true, true, false, false, 7));
        assert!(!auto_sprint(true, true, false, true, 20));
        assert!(!auto_sprint(true, true, true, false, 20));
    }
    #[test]
    fn mention_substring_and_cooldown() {
        assert!(mentions("Alexander", ["Alex"]));
        assert!(mentions("Привет игрок!", ["ИГРОК"]));
        assert!(!mentions("hello", [" ", ""]));
        assert!(own_prefix("Alex » hi", "Alex"));
        assert!(!own_prefix(" <Alex> hi", "Alex"));
        let mut l = MentionLimiter::default();
        assert!(l.take(1000, 500));
        assert!(!l.take(1499, 500));
        assert!(l.take(1500, 500));
    }
    #[test]
    fn zoom_steps_and_limits() {
        assert_eq!(scroll_tier(0, 12.0, 10), 1);
        assert_eq!(scroll_tier(10, 1.0, 10), 10);
        assert_eq!(scroll_tier(0, -1.0, 10), 0);
        assert_eq!(zoom_divisor(4.0, 1.0, 150.0, 1.0), 6.0);
        assert_eq!(zoom_divisor(4.0, 1.0, 150.0, 2.0), 9.0);
        assert_eq!(zoom_divisor(4.0, 1.0, 150.0, 10.0), 230.66015625);
        assert_eq!(zoom_divisor(4.0, 1.0, 200.0, 20.0), 500.0);
    }
}
