//! The game as the platform runs it, for an executable to start (docs/layers.md, "How a Atlas
//! crate plugs in"). The official executable (`atlas-official`, Atlas layer) registers its
//! hooks (`atlas_platform::hooks`) and then calls [`main`]; it never names an engine crate.
//!
//! An executable built on it:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOCATOR: atlas_game::Allocator = atlas_game::ALLOCATOR;
//! static MOD_API_LEVEL: [u8; atlas_game::MOD_API_LEVEL.len()] = atlas_game::MOD_API_LEVEL;
//!
//! fn main() {
//!     std::hint::black_box(&MOD_API_LEVEL);
//!     my_crate::register();
//!     atlas_game::main();
//! }
//! ```
//!
//! The platform's own parts register here, in [`main`], as they leave the engine (layer plan M6).
#![forbid(unsafe_code)]

/// The game's allocator (mimalloc, with its thread tags), for the executable's
/// `#[global_allocator]`.
pub use atlas_threads::{ALLOCATOR, Allocator};

/// The mod API level this game serves. The executable keeps it in the program, where the
/// launcher reads it without running it (`atlas_mods::api_level::MARK`): a `static` its
/// `main` reads, so the linker keeps it.
pub const MOD_API_LEVEL: [u8; atlas_mods::api_level::MARK.len()] = atlas_mods::api_level::MARK;

/// Runs the game with the hooks registered so far, and returns when it ends. Register hooks
/// (`atlas_platform::hooks::register`) before calling it.
pub fn main() {
    atlas_client::entry::main();
}
