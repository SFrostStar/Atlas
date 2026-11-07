//! ViewModel supplied 1.21.4: CC0-1.0 (EvoVisuals).
//! Native Rust adaptation of the supplied hand transforms, 13 swing styles,
//! duration and interactive chat controls. See THIRD_PARTY_NOTICES.md.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PoseOp {
    Translate([f32; 3]),
    Scale([f32; 3]),
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
}
#[derive(Clone, Debug)]
pub(super) struct HandSettings {
    pub position: [f32; 3],
    pub scale: f32,
    pub rotation: [f32; 3],
}
impl Default for HandSettings {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            scale: 1.0,
            rotation: [0.0; 3],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum Swing {
    Standard,
    Break,
    Jelly,
    Lunge,
    Spear,
    Slap,
    Down,
    Bonk,
    Circle,
    #[default]
    Swing,
    Swing2,
    Realism,
    Shift,
}
impl Swing {
    pub(super) fn display_name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Break => "Break",
            Self::Jelly => "Jelly",
            Self::Lunge => "Lunge",
            Self::Spear => "Spear",
            Self::Slap => "Slap",
            Self::Down => "Down",
            Self::Bonk => "Bonk",
            Self::Circle => "Circle",
            Self::Swing => "Swing",
            Self::Swing2 => "Swing 2",
            Self::Realism => "Realism",
            Self::Shift => "Shift",
        }
    }
    pub(super) const ALL: [Self; 13] = [
        Self::Standard,
        Self::Break,
        Self::Jelly,
        Self::Lunge,
        Self::Spear,
        Self::Slap,
        Self::Down,
        Self::Bonk,
        Self::Circle,
        Self::Swing,
        Self::Swing2,
        Self::Realism,
        Self::Shift,
    ];
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Standard => "STANDARD",
            Self::Break => "BREAK",
            Self::Jelly => "JELLY",
            Self::Lunge => "LUNGE",
            Self::Spear => "SPEAR",
            Self::Slap => "SLAP",
            Self::Down => "DOWN",
            Self::Bonk => "BONK",
            Self::Circle => "CIRCLE",
            Self::Swing => "SWING",
            Self::Swing2 => "SWING2",
            Self::Realism => "REALISM",
            Self::Shift => "SHIFT",
        }
    }
}
#[derive(Clone, Debug)]
pub(super) struct Config {
    pub main: HandSettings,
    pub off: HandSettings,
    pub swing: Swing,
    pub strength: f32,
    pub duration: f32,
    pub only_target: bool,
    pub auto_swing: bool,
    pub interactive: bool,
    pub outline: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            main: HandSettings::default(),
            off: HandSettings::default(),
            swing: Swing::Swing,
            strength: 1.0,
            duration: 1.0,
            only_target: false,
            auto_swing: false,
            interactive: true,
            outline: true,
        }
    }
}
impl Config {
    pub(super) fn reset_hands(&mut self) {
        self.main = HandSettings::default();
        self.off = HandSettings::default();
    }
    pub(super) fn reset_swing(&mut self) {
        self.swing = Swing::Swing;
        self.strength = 1.0;
        self.duration = 1.0;
        self.only_target = false;
        self.auto_swing = false;
    }
    pub(super) fn hand(&self, main: bool) -> &HandSettings {
        if main { &self.main } else { &self.off }
    }
    pub(super) fn hand_mut(&mut self, main: bool) -> &mut HandSettings {
        if main { &mut self.main } else { &mut self.off }
    }
    pub(super) fn swing_duration(&self, original: i32) -> i32 {
        if self.duration == 1.0 {
            original
        } else {
            (original as f32 * self.duration + 0.5).floor().max(1.0) as i32
        }
    }
    pub(super) fn hand_pose(
        &self,
        main: bool,
        left_dominant: bool,
        selected: bool,
        now_ms: u64,
    ) -> Vec<PoseOp> {
        let hand = self.hand(main);
        let side = if main != left_dominant { 1.0 } else { -1.0 };
        let pivot = [side * 0.56, -0.52, -0.72];
        let mut ops = vec![PoseOp::Translate(pivot), PoseOp::Translate(hand.position)];
        if hand.rotation[0] != 0.0 {
            ops.push(PoseOp::RotateX(hand.rotation[0]));
        }
        if hand.rotation[1] != 0.0 {
            ops.push(PoseOp::RotateY(hand.rotation[1]));
        }
        if hand.rotation[2] != 0.0 {
            ops.push(PoseOp::RotateZ(hand.rotation[2]));
        }
        let scale = hand.scale
            * if self.outline && selected {
                1.0 + 0.04 * (now_ms as f64 * 0.007).sin() as f32
            } else {
                1.0
            };
        if scale != 1.0 {
            ops.push(PoseOp::Scale([scale; 3]));
        }
        ops.push(PoseOp::Translate(pivot.map(|v| -v)));
        ops
    }
    pub(super) fn swing_pose(
        &self,
        right_arm: bool,
        p: f32,
        entity_target: bool,
    ) -> Option<Vec<PoseOp>> {
        if (self.swing == Swing::Standard && self.strength == 1.0)
            || (self.only_target && !entity_target)
        {
            return None;
        }
        Some(swing_pose(self.swing, right_arm, p, self.strength))
    }
}
// Vanilla's 65536-entry sine lookup, including its f32 index multiplication.
fn mth_sin(v: f32) -> f32 {
    let i = ((v * 10430.378) as i32 & 65535) as f64;
    (i * std::f64::consts::TAU / 65536.0).sin() as f32
}
pub(super) fn swing_pose(kind: Swing, right: bool, p: f32, s: f32) -> Vec<PoseOp> {
    use PoseOp::{RotateX as X, RotateY as Y, RotateZ as Z, Scale as S, Translate as T};
    let side = if right { 1.0 } else { -1.0 };
    let h = mth_sin(p.sqrt() * std::f32::consts::PI);
    let g = mth_sin(p * p * std::f32::consts::PI);
    let start = Y(side * 45.0);
    let end = Y(side * -45.0);
    match kind {
        Swing::Standard => vec![
            T([
                side * (-0.4 * h * s),
                0.2 * mth_sin(p.sqrt() * std::f32::consts::TAU) * s,
                -0.2 * mth_sin(p * std::f32::consts::PI) * s,
            ]),
            Y(side * (45.0 + g * -20.0 * s)),
            Z(side * h * -20.0 * s),
            X(h * -80.0 * s),
            end,
        ],
        Swing::Swing => vec![
            T([side * 0.12 * h * s, -0.14 * g * s, -0.12 * h * s]),
            start,
            X(-h * 75.0 * s),
            Z(side * h * -30.0 * s),
            Y(side * (g * -25.0 * s - 45.0)),
        ],
        Swing::Swing2 => vec![
            T([side * 0.15 * h * s, -0.18 * h * s, -0.15 * h * s]),
            start,
            X(-h * 85.0 * s),
            Y(side * mth_sin(p * std::f32::consts::PI) * -35.0 * s),
            Z(side * h * -35.0 * s),
            end,
        ],
        Swing::Break => vec![
            T([0.0, -h * 0.35 * s, -h * 0.2 * s]),
            start,
            X(-h * 90.0 * s),
            Z(side * h * -15.0 * s),
            end,
        ],
        Swing::Lunge => vec![
            T([side * -h * 0.1 * s, h * 0.04 * s, -h * 0.65 * s]),
            start,
            X(-h * 25.0 * s),
            end,
        ],
        Swing::Spear => vec![
            T([side * -h * 0.08 * s, 0.0, -h * 0.75 * s]),
            start,
            Z(side * h * 75.0 * s),
            X(-h * 20.0 * s),
            end,
        ],
        Swing::Slap => vec![
            T([side * -h * 0.4 * s, -h * 0.06 * s, -h * 0.15 * s]),
            start,
            Y(side * h * 80.0 * s),
            Z(side * h * -30.0 * s),
            X(-h * 25.0 * s),
            end,
        ],
        Swing::Down => vec![
            T([0.0, -h * 0.45 * s, -h * 0.25 * s]),
            start,
            X(-h * 105.0 * s),
            Z(side * h * -10.0 * s),
            end,
        ],
        Swing::Bonk => {
            let snap = (f64::from(p).powf(0.35) * std::f64::consts::PI).sin() as f32;
            let bounce = (f64::from(p) * std::f64::consts::PI).sin() as f32;
            vec![
                T([
                    side * bounce * 0.05 * s,
                    -snap * 0.35 * s,
                    -bounce * 0.2 * s,
                ]),
                start,
                X(-snap * 80.0 * s),
                Z(side * bounce * 20.0 * s),
                end,
            ]
        }
        Swing::Circle => {
            let ease = p * p * (3.0 - 2.0 * p);
            let h = mth_sin(p * std::f32::consts::PI);
            vec![
                T([side * h * 0.1 * s, -h * 0.14 * s, -h * 0.14 * s]),
                start,
                Z(side * ease * 360.0 * s),
                X(-h * 30.0 * s),
                end,
            ]
        }
        Swing::Jelly => {
            let w = ((f64::from(p) * std::f64::consts::PI * 2.5).sin() * f64::from(1.0 - p)) as f32;
            vec![
                S([1.0 + w * 0.22 * s, 1.0 - w * 0.22 * s, 1.0]),
                T([side * w * 0.12 * s, -h * 0.22 * s, -h * 0.15 * s]),
                start,
                X(-h * 65.0 * s),
                Z(side * w * 35.0 * s),
                end,
            ]
        }
        Swing::Realism => {
            let h = (f64::from(p).powf(0.7) * std::f64::consts::PI).sin() as f32;
            let w = (f64::from(p).powf(1.4) * std::f64::consts::PI).sin() as f32;
            vec![
                T([side * 0.12 * h * s, -h * 0.2 * s, -h * 0.25 * s]),
                start,
                X(-h * 70.0 * s),
                Y(side * w * -30.0 * s),
                Z(side * h * -25.0 * s),
                end,
            ]
        }
        Swing::Shift => {
            let step = (f64::from(p) * std::f64::consts::PI).sin() as f32;
            let snap = (f64::from(p).powf(0.5) * std::f64::consts::PI).sin() as f32;
            vec![
                T([side * 0.18 * step * s, -snap * 0.25 * s, -step * 0.28 * s]),
                start,
                X(-snap * 80.0 * s),
                Y(side * step * 25.0 * s),
                Z(side * step * -20.0 * s),
                end,
            ]
        }
    }
}
#[derive(Default)]
pub(super) struct ChatEditor {
    pub selected: Option<bool>,
    pub last_click: Option<(u64, f64, f64)>,
    pub indicator_until: u64,
    pub indicator_rotation: bool,
    pub indicator_scale: f32,
    pub indicator_angles: [f32; 2],
}
impl ChatEditor {
    pub(super) fn click(
        &mut self,
        cfg: &Config,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        button: i32,
        now: u64,
    ) -> bool {
        if !cfg.interactive || y > height - 24.0 {
            return false;
        }
        if button == 0
            && self.last_click.is_some_and(|(t, px, py)| {
                now.saturating_sub(t) < 400 && (x - px).hypot(y - py) < 40.0
            })
            && y > height * 0.35
        {
            let hand = x > width * 0.45;
            self.selected = if self.selected == Some(hand) {
                None
            } else {
                Some(hand)
            };
            self.last_click = None;
            true
        } else {
            self.last_click = Some((now, x, y));
            self.selected.is_some()
        }
    }
    pub(super) fn drag(
        &mut self,
        cfg: &mut Config,
        button: i32,
        dx: f64,
        dy: f64,
        shift: bool,
        now: u64,
    ) -> bool {
        let Some(main) = self.selected.filter(|_| cfg.interactive) else {
            return false;
        };
        let h = cfg.hand_mut(main);
        match button {
            0 => {
                h.position[0] += (dx * 0.0035) as f32;
                h.position[1] -= (dy * 0.0035) as f32;
            }
            1 if !shift => {
                h.rotation[1] = wrap(h.rotation[1] + (dx * 0.7) as f32);
                h.rotation[0] = wrap(h.rotation[0] + (dy * 0.7) as f32);
                self.indicator_until = now + 2200;
                self.indicator_rotation = true;
                self.indicator_angles = [h.rotation[0], h.rotation[1]];
            }
            1 | 2 => {
                h.rotation[2] = wrap(h.rotation[2] + (dx * 0.7) as f32);
                self.indicator_until = now + 2200;
                self.indicator_rotation = true;
                self.indicator_angles = [h.rotation[0], h.rotation[2]];
            }
            _ => {}
        }
        true
    }
    pub(super) fn scroll(&mut self, cfg: &mut Config, delta: f64, shift: bool, now: u64) -> bool {
        let Some(main) = self.selected.filter(|_| cfg.interactive) else {
            return false;
        };
        let h = cfg.hand_mut(main);
        h.scale = (h.scale + (delta * if shift { 0.02 } else { 0.05 }) as f32).clamp(0.05, 3.0);
        self.indicator_until = now + 2200;
        self.indicator_rotation = false;
        self.indicator_scale = h.scale;
        true
    }
}
fn wrap(mut degrees: f32) -> f32 {
    while degrees > 180.0 {
        degrees -= 360.0;
    }
    while degrees < -180.0 {
        degrees += 360.0;
    }
    degrees
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_swing_and_standard_gate_match_reference() {
        let mut c = Config::default();
        assert!(c.swing_pose(true, 0.5, false).is_some());
        c.swing = Swing::Standard;
        assert!(c.swing_pose(true, 0.5, false).is_none());
        c.strength = 0.5;
        c.only_target = true;
        assert!(c.swing_pose(true, 0.5, false).is_none());
        assert!(c.swing_pose(true, 0.5, true).is_some());
    }
    #[test]
    fn default_hand_pose_cancels_pivot() {
        let c = Config::default();
        assert_eq!(
            c.hand_pose(true, false, false, 0),
            vec![
                PoseOp::Translate([0.56, -0.52, -0.72]),
                PoseOp::Translate([0.0; 3]),
                PoseOp::Translate([-0.56, 0.52, 0.72])
            ]
        );
    }
    #[test]
    fn double_click_then_drag_and_scroll_preserve_other_hand() {
        let mut c = Config::default();
        let mut e = ChatEditor::default();
        assert!(!e.click(&c, 300.0, 160.0, 400.0, 300.0, 0, 100));
        assert!(e.click(&c, 300.0, 160.0, 400.0, 300.0, 0, 200));
        assert_eq!(e.selected, Some(true));
        e.drag(&mut c, 0, 20.0, -10.0, false, 210);
        e.scroll(&mut c, 2.0, true, 220);
        assert_eq!(c.main.position, [0.07, 0.035, 0.0]);
        assert_eq!(c.main.scale, 1.04);
        assert_eq!(c.off.scale, 1.0);
        assert_eq!(e.indicator_until, 2420);
        assert_eq!(e.indicator_scale, 1.04);
        assert!(!e.indicator_rotation);
    }
    #[test]
    fn rotation_badge_preserves_the_displayed_axis_and_duration() {
        let mut c = Config::default();
        let mut e = ChatEditor {
            selected: Some(true),
            ..Default::default()
        };
        e.drag(&mut c, 1, 10.0, 20.0, false, 100);
        assert_eq!(e.indicator_angles, [14.0, 7.0]);
        assert_eq!(e.indicator_until, 2300);
        e.drag(&mut c, 1, 30.0, 0.0, true, 200);
        assert_eq!(e.indicator_angles, [14.0, 21.0]);
        assert_eq!(e.indicator_until, 2400);
    }
}
