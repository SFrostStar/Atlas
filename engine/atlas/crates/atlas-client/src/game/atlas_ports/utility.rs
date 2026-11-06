//! Small, original Atlas equivalents of utility-mod behavior described by the user's JAR metadata.
//! The Java classes and mod source are not used. Every hook is gated by installation and its switch.
use super::super::{Game, native_hooks::NativeZoom};
use atlas_client::{chat::ChatSource, keybinds};
use winit::keyboard::KeyCode;

#[path = "utility_logic.rs"]
mod logic;

pub(super) const AUTO_SPRINT: &str = "atlas-autosprint";
pub(super) const NO_SHAKE_HAND: &str = "atlas-noshakehand";
pub(super) const ZOOMIFY: &str = "atlas-zoomify";
pub(super) const MENTION_SOUND: &str = "atlas-mentionsound";

#[derive(Default)]
pub(super) struct UtilityState {
    mentions: logic::MentionLimiter,
}

impl Game<'_> {
    /// Equivalent to holding the ordinary sprint key, without bypassing food or motion checks.
    /// Compute this before taking a mutable borrow of `self.player` in the local movement path.
    pub(in crate::game) fn atlas_port_auto_sprint(&self) -> bool {
        self.atlas_port_on(AUTO_SPRINT)
            && logic::auto_sprint(
                self.focused && self.native_playing(),
                self.keys.contains(&KeyCode::KeyW),
                self.keys.contains(&KeyCode::KeyS),
                self.keys.contains(&KeyCode::ShiftLeft),
            )
    }

    /// Only removes the inertial hand sway when the camera turns. Walking, hurt, attack,
    /// item-use, equip and throwing animations keep the ordinary transforms.
    pub(in crate::game) fn atlas_port_hand_sway(&self, partial: f32) -> [f32; 2] {
        if self.atlas_port_on(NO_SHAKE_HAND) {
            [0.0, 0.0]
        } else {
            self.sway.rotation(partial, self.view.x_rot, self.view.y_rot)
        }
    }

    /// The existing native zoom adapter handles FOV easing, turning scale and spyglass priority.
    /// Read physical key state, avoiding a stuck zoom after focus or GUI changes.
    pub(in crate::game) fn atlas_port_zoom(&self, zoom: &mut Option<NativeZoom>) {
        if zoom.is_some()
            || !self.atlas_port_on(ZOOMIFY)
            || !self.focused
            || !self.native_playing()
            || self.player.is_none()
        {
            return;
        }
        let key = self.native_mod_key(ZOOMIFY, "zoom", "key.keyboard.c");
        let held = keybinds::key_code(&key).is_some_and(|key| self.physical_keys.contains(&key));
        if held {
            *zoom = Some(NativeZoom {
                factor: 4.0,
                smooth: true,
                turn: 0.25,
            });
        }
    }

    /// Called once when a visible incoming chat line is appended, for local and remote paths.
    /// Client-origin informational messages never ping. No sound is played without a player.
    pub(in crate::game) fn atlas_port_chat(&mut self, message: &str, source: ChatSource) {
        if !self.atlas_port_on(MENTION_SOUND)
            || !self.native_window()
            || self.player.is_none()
            || source == ChatSource::Client
        {
            return;
        }
        let nickname = self.options.player_name();
        if self
            .atlas_ports
            .utility
            .mentions
            .take(message, &nickname, self.ticks)
        {
            self.audio
                .play("minecraft:block.note_block.pling", None, 0.65, 1.25);
        }
    }
}
