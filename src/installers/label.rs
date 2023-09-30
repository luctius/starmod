use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use walkdir::WalkDir;

use crate::{
    manifest::{InstallFile, Manifest},
    mods::ModKind,
};

pub fn create_label_manifest(
    mod_kind: ModKind,
    cache_dir: &Utf8Path,
    name: &Utf8Path,
) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut disabled_files = Vec::new();

    let archive_dir = cache_dir.join(name);

    let version = Some("Label".to_owned());
    let nexus_id = None;

    Ok(Manifest::new(
        name,
        name.to_string(),
        nexus_id,
        version,
        files,
        disabled_files,
        mod_kind,
    ))
}
