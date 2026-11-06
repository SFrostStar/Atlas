//! Atlas parallel resource overlay materialization.
use super::{Dependencies, link};
use crate::packs::{Filter, Provided, junction};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};
/// A zip entry to write: its pack, its name in the zip and where it goes.
pub(crate) type Extract<'a> = (usize, &'a str, PathBuf);
#[allow(clippy::too_many_arguments)]
pub(crate) fn mirror<'a>(
    rel: &str,
    vanilla: &Path,
    out: &Path,
    touched: &BTreeSet<String>,
    hidden: &BTreeSet<String>,
    files: &'a BTreeMap<String, (usize, Provided)>,
    merged: &BTreeMap<String, Vec<u8>>,
    filters: &[&Filter],
    dependencies: &mut Dependencies,
    extract: &mut Vec<Extract<'a>>,
) -> io::Result<()> {
    let src = if rel.is_empty() {
        vanilla.to_path_buf()
    } else {
        vanilla.join(rel)
    };
    let dst = if rel.is_empty() {
        out.to_path_buf()
    } else {
        out.join(rel)
    };
    fs::create_dir_all(&dst)?;
    dependencies.directory(&src);
    let mut names: BTreeSet<String> = BTreeSet::new();
    if let Ok(entries) = fs::read_dir(&src) {
        names.extend(
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned()),
        );
    }
    let prefix = if rel.is_empty() {
        String::new()
    } else {
        format!("{rel}/")
    };
    // The files under this directory are one run of the sorted map: walking only that run
    // keeps a pack of tens of thousands of files in thousands of folders from being
    // scanned once per folder.
    for f in files
        .range(prefix.clone()..)
        .map_while(|(f, _)| f.strip_prefix(&prefix))
    {
        names.insert(f.split('/').next().unwrap_or("").to_owned());
    }
    for name in names {
        let child = format!("{prefix}{name}");
        // Vanilla's files in one file: it describes vanilla's root, not this mirror.
        if hidden.contains(&child) || child == atlas_core::prepared_files::FILE_NAME {
            continue;
        }
        if let Some(bytes) = merged.get(&child) {
            fs::write(out.join(&child), bytes)?;
        } else if let Some((owner, (path, inner))) = files.get(&child) {
            let target = out.join(&child);
            match path {
                Some(p) => link(p, &target)?,
                // Inflated and written after the walk, on every thread.
                None => extract.push((*owner, inner.as_str(), target)),
            }
        } else if filters.is_empty()
            && !touched.contains(&child)
            && vanilla.join(&child).is_dir()
            && junction(&vanilla.join(&child), &out.join(&child)).is_ok()
        {
            // Linked whole, like the unix symlink: no file in it is visited or copied.
        } else if touched.contains(&child)
            // Without a junction Windows cannot link a directory the way unix does (`link`
            // above), so an untouched one has to be mirrored: skipping it hid textures/font
            // and the title panorama from the overlay, and a font json that names those
            // sheets then failed to load (#1688).
            || (vanilla.join(&child).is_dir() && (cfg!(not(unix)) || !filters.is_empty()))
        {
            mirror(
                &child,
                vanilla,
                out,
                touched,
                hidden,
                files,
                merged,
                filters,
                dependencies,
                extract,
            )?;
        } else if filters.iter().any(|f| f.blocks(&child)) {
            // Hidden by a pack's filter.
        } else {
            #[cfg(not(unix))]
            if vanilla.join(&child).is_file() {
                dependencies.file(&vanilla.join(&child));
            }
            link(&vanilla.join(&child), &out.join(&child))?;
        }
    }
    Ok(())
}
