//! The platform's public API: what the Atlas layer, mods and ports may use of the engine
//! (docs/layers.md#platform). The Atlas layer depends on the platform only, never on an engine
//! crate, so it can build from another repository; when it needs something the engine has, it
//! comes through here.
//!
//! Today this re-exports engine modules whole. Each one gets its own stable types as the split
//! goes on (docs/layers.md#plan, M6). The parts that pull in large engine crates sit behind
//! features (`world`, `protocol`, `prepare`, `server-files`), so a crate builds only what it uses.

/// The machine the game runs on (`atlas-system`): OS, CPU, memory, drive, power, GPU maker and
/// memory.
pub use atlas_system as system;

/// The benchmark report format and its hardware probes (`atlas-bench`).
pub use atlas_bench as bench;

/// The `.hzperf` capture contract and its files (`atlas-perf-format`).
pub use atlas_perf_format as perf_format;

/// The pinned Minecraft version: its id, protocol and data version.
pub use atlas_core::version;

/// The player's "no AI" switch, which the launcher, the game and their agent tools share.
pub use atlas_core::no_ai;

/// The prepared folder's file of files (`prepared_files::FILE_NAME`).
pub use atlas_core::prepared_files;

/// Vanilla's world files, as tools outside the game read and write them.
#[cfg(feature = "world")]
pub mod world {
    pub use atlas_core::position::ChunkPos;
    pub use atlas_world::{compression, nbt, region};
}

/// What a server list and a profile head need of the network protocol.
#[cfg(feature = "protocol")]
pub mod protocol {
    pub use atlas_protocol::crypto::offline_uuid;
    pub use atlas_protocol::server_redirect;
}

/// The preparer: the game's files from the player's own Minecraft install.
#[cfg(feature = "prepare")]
pub use atlas_prepare as prepare;

/// A server's own files: `server.properties`, the op, white and ban lists, the user cache.
#[cfg(feature = "server-files")]
pub use atlas_server_files as server_files;

/// Plugging into the game (`atlas-hooks`): implement [`hooks::Hooks`] and
/// [`hooks::register`] it before the game starts (docs/layers.md, "How a Atlas crate plugs
/// in"). The engine's dispatch side is not part of the API.
pub mod hooks {
    /// The players the game shows (`atlas_hooks::players`): the views [`Hooks::frame`] and
    /// [`Hooks::tick`] get, the pose a crate lays over a player's body, what goes by their names,
    /// the crate's sprites, the camera, and what a crate draws on them.
    pub use atlas_hooks::players::{
        BODY_PARTS, Badge, BodyPose, Bone, CameraOverride, CameraView, Frame, Gear, Kind, Light,
        Look, Mark, Mesh, Motion, NameDecoration, PartPoses, Particle, Pass, PlayerDraw,
        PlayerModel, PlayerModels, Quad, SpriteStore, Sprites, View,
    };
    pub use atlas_hooks::{
        Activity, Client, Dimension, GameMode, GetTheGame, Hooks, Play, Started, TooLate, perf,
        register, seat, tunnel,
    };
}

/// The settings store (`atlas_hooks::settings`): a crate's own `key:value` lines in the game
/// folder, loaded and saved with the player's settings (docs/layers.md, "Platform registries").
pub mod settings {
    pub use atlas_hooks::settings::{Store, register};
}

/// Settings pages a crate adds to the options screens (docs/layers.md, "Platform registries").
pub mod pages {
    pub use atlas_hooks::pages::{Act, Label, Page, Place, Row, connect, register};
}

/// The lang files with a crate's text, loaded with the game's own (docs/layers.md, "Platform
/// registries").
pub mod lang {
    pub use atlas_hooks::lang::{Files, register};
}

/// Drawing on the HUD (docs/layers.md, "Platform registries").
pub mod hud {
    pub use atlas_hooks::hud::{Canvas, Corner, Layer, register};
}

/// The multiplayer server list as vanilla stores it (`servers.dat`): read, write, add, remove,
/// and the lock held while it is read and written back.
#[cfg(feature = "protocol")]
pub mod servers {
    pub use atlas_protocol::servers::{
        ServerEntry, load, lock, parse_address, prepend, remove_where, save,
    };
}
