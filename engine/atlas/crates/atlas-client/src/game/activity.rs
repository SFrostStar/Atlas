//! What the player is doing, for the hooks above the engine (`Hooks::activity`: the Atlas
//! layer's Discord presence shows it).
use super::*;
use atlas_client::command::args::GameMode;
use atlas_hooks::{Activity, Client, Dimension, Play};

/// At startup, for a game window (never a dedicated server's): the player's first activity, then
/// `Hooks::client_started`.
pub(super) fn client_started(options: &Options) {
    if options.dedicated.is_some() {
        return;
    }
    let automated = options.benchmark.is_some()
        || options.offscreen
        || options.frames.is_some()
        || options.world_frames.is_some();
    atlas_hooks::engine::activity(Activity::Menus);
    atlas_hooks::engine::client_started(&Client::new(automated));
}

impl Game<'_> {
    /// Each frame: only a change reaches the hooks.
    pub(super) fn report_activity(&self) {
        // One status per computer: P1's, not a splitscreen player's.
        if self.options.split_guest {
            return;
        }
        atlas_hooks::engine::activity(self.activity());
    }
    fn activity(&self) -> Activity {
        if self.title.is_some() {
            return Activity::Menus;
        }
        let max = atlas_client::lan::MAX_PLAYERS;
        let play = if let Some(host) = &self.lan {
            Play::Hosting {
                players: 1 + host.hub.guests.len(),
                max,
            }
        } else if let Some(server) = &self.integrated {
            match (server.published(), &self.remote) {
                (Some(_), Some(remote)) => Play::Hosting {
                    players: remote.player_count(),
                    max,
                },
                _ => Play::Singleplayer,
            }
        } else if self.remote.is_some() {
            Play::Multiplayer
        } else {
            Play::Singleplayer
        };
        Activity::World {
            play,
            dimension: match self.dim {
                Dim::Overworld => Dimension::Overworld,
                Dim::Nether => Dimension::Nether,
                Dim::End => Dimension::End,
            },
            mode: match self.mode {
                GameMode::Survival => atlas_hooks::GameMode::Survival,
                GameMode::Creative => atlas_hooks::GameMode::Creative,
                GameMode::Adventure => atlas_hooks::GameMode::Adventure,
                GameMode::Spectator => atlas_hooks::GameMode::Spectator,
            },
        }
    }
}
