//! Chunk Animator 1.0.0+1.21.4 (MIT, Donyka), adapted to native Rust terrain drawing.
//! See THIRD_PARTY_NOTICES.md. The clock begins on the first visible draw.
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub mode: Mode,
    pub easing: Easing,
    pub duration: u64,
    pub disable_near: bool,
    pub min_y: i32,
    pub max_y: i32,
    pub player: [i32; 2],
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Below,
            easing: Easing::Sine,
            duration: 1000,
            disable_near: false,
            min_y: -64,
            max_y: 320,
            player: [0; 2],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Below,
    Above,
    Hybrid,
    HorizontalSlide,
    HorizontalSlideAlternate,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
    Linear,
    Quad,
    Cubic,
    Quart,
    Quint,
    Expo,
    #[default]
    Sine,
    Circ,
    Back,
    Bounce,
    Elastic,
}
impl Easing {
    pub fn ease(self, elapsed: f32, distance: f32, duration: f32) -> f32 {
        if duration <= 0.0 || elapsed >= duration {
            return distance;
        }
        let t = elapsed.max(0.0) / duration;
        let v = match self {
            Self::Linear => t,
            Self::Quad => -t * (t - 2.0),
            Self::Cubic => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            Self::Quart => {
                let t = t - 1.0;
                -(t * t * t * t - 1.0)
            }
            Self::Quint => {
                let t = t - 1.0;
                t * t * t * t * t + 1.0
            }
            Self::Expo => (1.0 - 2.0_f64.powf(f64::from(-10.0 * t))) as f32,
            Self::Sine => (f64::from(t) * std::f64::consts::FRAC_PI_2).sin() as f32,
            Self::Circ => {
                let t = t - 1.0;
                f64::from(1.0 - t * t).sqrt() as f32
            }
            Self::Back => {
                let t = t - 1.0;
                t * t * ((1.70158 + 1.0) * t + 1.70158) + 1.0
            }
            Self::Bounce => {
                if t < 0.36363637 {
                    7.5625 * t * t
                } else if t < 0.72727275 {
                    let t = t - 0.54545456;
                    7.5625 * t * t + 0.75
                } else if t < 0.90909094 {
                    let t = t - 0.8181818;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 0.95454544;
                    7.5625 * t * t + 0.984375
                }
            }
            Self::Elastic => {
                if t == 0.0 {
                    0.0
                } else {
                    let p = duration * 0.3;
                    (2.0_f64.powf(f64::from(-10.0 * t))
                        * (f64::from(t * duration - p / 4.0) * std::f64::consts::TAU
                            / f64::from(p))
                        .sin()
                        + 1.0) as f32
                }
            }
        };
        distance * v
    }
}
#[derive(Clone, Debug)]
pub struct ChunkAnimation {
    pub started: Option<u64>,
    pub completed: bool,
    pub facing: [i32; 2],
}
#[derive(Default)]
pub struct State {
    chunks: BTreeMap<[i32; 3], ChunkAnimation>,
}
impl State {
    pub fn reset(&mut self) {
        self.chunks.clear();
    }
    pub fn retain(&mut self, origins: &std::collections::BTreeSet<[i32; 3]>) {
        self.chunks.retain(|origin, _| origins.contains(origin));
    }
    pub fn offset(&mut self, origin: [i32; 3], config: Config, now_ms: u64) -> [f32; 3] {
        let Config {
            mode,
            easing,
            duration,
            disable_near,
            min_y,
            max_y,
            player,
        } = config;
        let dx = i64::from(player[0]) - i64::from(origin[0]) - 8;
        let dz = i64::from(player[1]) - i64::from(origin[2]) - 8;
        let data = self.chunks.entry(origin).or_insert_with(|| ChunkAnimation {
            started: None,
            completed: disable_near && dx * dx + dz * dz <= 4096,
            facing: [0; 2],
        });
        if data.completed || duration == 0 {
            data.completed = true;
            return [0.0; 3];
        }
        let started = *data.started.get_or_insert_with(|| {
            data.facing = if dx.abs() > dz.abs() {
                [if dx > 0 { 1 } else { -1 }, 0]
            } else {
                [0, if dz > 0 { 1 } else { -1 }]
            };
            now_ms
        });
        let elapsed = now_ms.saturating_sub(started);
        if elapsed >= duration {
            data.completed = true;
            return [0.0; 3];
        }
        let mode = if mode == Mode::Hybrid {
            if origin[1] < 63 {
                Mode::Below
            } else {
                Mode::Above
            }
        } else {
            mode
        };
        let ease = |d| easing.ease(elapsed as f32, d, duration as f32);
        match mode {
            Mode::Below => {
                let d = (origin[1] - min_y).abs() as f32;
                [0.0, -d + ease(d), 0.0]
            }
            Mode::Above => {
                let d = (max_y - origin[1]).max(0) as f32;
                [0.0, d - ease(d), 0.0]
            }
            Mode::HorizontalSlide | Mode::HorizontalSlideAlternate => {
                let d = -(200.0 - ease(200.0));
                [data.facing[0] as f32 * d, 0.0, data.facing[1] as f32 * d]
            }
            Mode::Hybrid => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starts_on_visible_render_and_never_restarts_on_remesh() {
        let mut state = State::default();
        let c = Config {
            easing: Easing::Linear,
            ..Default::default()
        };
        assert_eq!(state.offset([0, 64, 0], c, 100), [0.0, -128.0, 0.0]);
        assert_eq!(state.offset([0, 64, 0], c, 600), [0.0, -64.0, 0.0]);
        assert_eq!(state.offset([0, 64, 0], c, 1100), [0.0; 3]);
        assert_eq!(state.offset([0, 64, 0], c, 2100), [0.0; 3]);
    }
    #[test]
    fn configured_modes_respect_horizon_world_bounds_and_player_distance() {
        let mut state = State::default();
        assert_eq!(
            state.offset(
                [0, 64, 0],
                Config {
                    disable_near: true,
                    ..Default::default()
                },
                0
            ),
            [0.0; 3]
        );
        assert_eq!(
            state.offset(
                [80, 64, 0],
                Config {
                    mode: Mode::Above,
                    ..Default::default()
                },
                0
            ),
            [0.0, 256.0, 0.0]
        );
        assert_eq!(
            state.offset(
                [96, 64, 0],
                Config {
                    mode: Mode::Hybrid,
                    ..Default::default()
                },
                0
            ),
            [0.0, 256.0, 0.0]
        );
        assert_eq!(
            state.offset(
                [112, 32, 0],
                Config {
                    mode: Mode::Hybrid,
                    ..Default::default()
                },
                0
            ),
            [0.0, -96.0, 0.0]
        );
        assert_eq!(
            state.offset(
                [128, 32, 0],
                Config {
                    mode: Mode::HorizontalSlide,
                    ..Default::default()
                },
                0
            ),
            [200.0, 0.0, 0.0]
        );
    }
    #[test]
    fn easing_endpoints_cover_all_eleven_functions() {
        for e in [
            Easing::Linear,
            Easing::Quad,
            Easing::Cubic,
            Easing::Quart,
            Easing::Quint,
            Easing::Expo,
            Easing::Sine,
            Easing::Circ,
            Easing::Back,
            Easing::Bounce,
            Easing::Elastic,
        ] {
            assert!(e.ease(0.0, 64.0, 1000.0).abs() < 0.00001);
            assert_eq!(e.ease(1000.0, 64.0, 1000.0), 64.0);
            assert_eq!(e.ease(9999.0, 64.0, 1000.0), 64.0);
        }
    }
}
