//! Atlas resource preparation acceleration, shared by startup and pack changes.
//! Keep marker version and path hashing stable when updating the engine adapter.
mod cache;
mod links;
mod overlay;
use crate::packs::Pack;
pub(crate) use cache::Dependencies;
pub(crate) use links::link;
pub(crate) use overlay::mirror;
use std::{
    io,
    path::{Path, PathBuf},
};
/// Reuse a validated resource view or build one with links instead of copies.
pub fn build_overlay(vanilla: &Path, packs: &[Pack], out: &Path) -> io::Result<()> {
    cache::build(vanilla, packs, out)
}
/// Vanilla is already the resource root when no resource pack is enabled.
pub fn resource_root(vanilla: &Path, packs: &[Pack], cache: &Path) -> io::Result<PathBuf> {
    if packs.is_empty() {
        return Ok(vanilla.to_path_buf());
    }
    let key: String = packs
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let hash = key.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    });
    let out = cache.join(format!("{hash:016x}"));
    build_overlay(vanilla, packs, &out)?;
    Ok(out)
}
