use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use walkdir::WalkDir;

use crate::{
    // dmodman::{DmodMan, DMODMAN_EXTENTION},
    manifest::{install_file::InstallFile, Manifest},
    mods::ModKind,
};

pub fn create_custom_manifest(
    mod_kind: ModKind,
    cache_dir: &Utf8Path,
    name: &Utf8Path,
) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut disabled_files = Vec::new();

    let archive_dir = cache_dir.join(name);

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
            let source = Utf8PathBuf::try_from(entry_path.to_path_buf())?
                .strip_prefix(&archive_dir)?
                .to_path_buf();

            let destination = source.to_string().to_lowercase();

            files.push(InstallFile::new(source.into(), destination));
        }
    }

    // Disable all files containing 'readme' in the name
    files.retain(|f: &InstallFile| {
        if f.source().file_name().unwrap().contains("readme") {
            disabled_files.push(f.clone());
            false
        } else {
            true
        }
    });

    let version = Some("Custom".to_owned());
    let nexus_id = None;

    let mut m = Manifest::new(
        name,
        name.to_string(),
        nexus_id,
        version,
        files,
        disabled_files,
        mod_kind,
    );

    m.set_priority(1000);
    Ok(m)
}
