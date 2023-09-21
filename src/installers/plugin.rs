use anyhow::Result;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::{
    manifest::{InstallFile, Manifest},
    mod_types::ModType,
};

pub fn create_plugin_manifest(
    mod_type: ModType,
    cache_dir: &Path,
    manifest_dir: &Path,
) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut disabled_files = Vec::new();

    let mut archive_dir = PathBuf::from(cache_dir);
    archive_dir.push(manifest_dir);

    let walker = WalkDir::new(&archive_dir)
        .min_depth(1)
        .max_depth(usize::MAX)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(false);

    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_file() {
            files.push(entry_path.to_path_buf().strip_prefix(&archive_dir)?.into());
        }
    }

    // Disable all files containing 'readme' in the name
    files.retain(|f: &InstallFile| {
        if f.source
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("readme")
        {
            disabled_files.push(f.clone());
            false
        } else {
            true
        }
    });

    let name = manifest_dir.to_string_lossy().to_string();
    let name = name
        .split_once("-")
        .map(|n| n.0.to_string())
        .unwrap_or(name);

    Ok(Manifest::new(
        manifest_dir,
        name,
        mod_type,
        files,
        disabled_files,
    ))
}
