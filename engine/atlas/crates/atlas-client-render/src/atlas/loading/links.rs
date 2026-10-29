//! Read-only resource links with a complete copy fallback.
#[cfg(not(unix))]
use std::fs;
use std::{io, path::Path};
#[cfg(unix)]
pub(crate) fn link(target: &Path, at: &Path) -> io::Result<()> {
    // Relative sources are relative to the process, not the overlay directory.
    std::os::unix::fs::symlink(std::path::absolute(target)?, at)
}

#[cfg(not(unix))]
pub(crate) fn link(target: &Path, at: &Path) -> io::Result<()> {
    if target.is_dir() {
        junction::create(std::path::absolute(target)?, at)
    } else {
        // Overlay trees are read-only resource views. Like Unix symlinks,
        // NTFS hard links avoid copying thousands of unchanged vanilla files.
        // Cross-volume/unsupported filesystems retain the copy fallback.
        if at.exists() {
            fs::remove_file(at)?;
        }
        fs::hard_link(target, at).or_else(|_| fs::copy(target, at).map(|_| ()))
    }
}
