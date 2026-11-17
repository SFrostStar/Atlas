//! Gameplay and startup, above the reusable client library.
// No `#![recursion_limit]` or other inner attribute atlas-mixins doesn't know here: the
// launcher's atlas-mixins then rejects every native mod targeting this crate (#8169).
#![forbid(unsafe_code)]
extern crate self as atlas_client;
pub use atlas_client_lib::*;
pub mod entry;
pub mod game;
mod atlas_look;
pub mod mod_api;

#[cfg(test)]
mod brewing_speed_load_tests;
#[cfg(test)]
mod living_death_tests;
