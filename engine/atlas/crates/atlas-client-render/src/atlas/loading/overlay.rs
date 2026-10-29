//! Materialize changed paths and keep untouched resource subtrees linked.
use super::{Dependencies, link};
use crate::zip::Zip;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};
type Provided = (Option<PathBuf>, String);
#[allow(clippy::too_many_arguments)]
pub(crate) fn mirror(
    rel: &str,
    vanilla: &Path,
    out: &Path,
    touched: &BTreeSet<String>,
    files: &BTreeMap<String, (usize, Provided)>,
    merged: &BTreeMap<String, Vec<u8>>,
    blocked: &dyn Fn(&str) -> bool,
    has_filters: bool,
    zips: &[Option<Zip>],
    dependencies: &mut Dependencies,
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
    for f in files.keys().filter_map(|f| f.strip_prefix(&prefix)) {
        names.insert(f.split('/').next().unwrap_or("").to_owned());
    }
    for name in names {
        let child = format!("{prefix}{name}");
        if let Some(bytes) = merged.get(&child) {
            fs::write(out.join(&child), bytes)?;
        } else if let Some((owner, (path, inner))) = files.get(&child) {
            let target = out.join(&child);
            match path {
                Some(p) => link(p, &target)?,
                None => {
                    if let Some(Some(z)) = zips.get(*owner) {
                        fs::write(&target, z.read(inner)?)?;
                    }
                }
            }
        } else if touched.contains(&child)
            // A filter can hide descendants, so that subtree must be materialized.
            || (vanilla.join(&child).is_dir() && has_filters)
        {
            mirror(
                &child,
                vanilla,
                out,
                touched,
                files,
                merged,
                blocked,
                has_filters,
                zips,
                dependencies,
            )?;
        } else if blocked(&child) {
            // Hidden by a pack's filter.
        } else {
            #[cfg(windows)]
            if vanilla.join(&child).is_dir() {
                // Untouched subtrees remain live views of vanilla. A junction
                // needs no elevation and avoids thousands of file operations.
                if link(&vanilla.join(&child), &out.join(&child)).is_err() {
                    // Network/non-NTFS volumes keep the complete mirror fallback.
                    mirror(
                        &child,
                        vanilla,
                        out,
                        touched,
                        files,
                        merged,
                        blocked,
                        has_filters,
                        zips,
                        dependencies,
                    )?;
                }
                continue;
            }
            #[cfg(not(unix))]
            if vanilla.join(&child).is_file() {
                dependencies.file(&vanilla.join(&child));
            }
            link(&vanilla.join(&child), &out.join(&child))?;
        }
    }
    Ok(())
}
