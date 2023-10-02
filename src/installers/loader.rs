use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use walkdir::WalkDir;

use crate::{
    dmodman::{DmodMan, DMODMAN_EXTENTION},
    manifest::{install_file::InstallFile, Manifest},
    mods::ModKind,
};

pub fn create_loader_manifest(
    mod_kind: ModKind,
    cache_dir: &Utf8Path,
    mod_dir: &Utf8Path,
) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut disabled_files = Vec::new();

    let mut archive_dir = Utf8PathBuf::from(cache_dir);
    archive_dir.push(mod_dir);

    let dmodman = archive_dir.with_extension(DMODMAN_EXTENTION);

    let walker = WalkDir::new(&archive_dir)
        .min_depth(1)
        .max_depth(usize::MAX)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(false);

    for entry in walker {
        let entry = entry?;
        let entry_path = Utf8PathBuf::try_from(entry.path().to_path_buf())?;

        if entry_path.is_file() {
            if let Some(ext) = entry_path.extension() {
                match ext {
                    "dll" | "exe" => {
                        let file = entry_path.strip_prefix(&archive_dir)?.to_path_buf();

                        files.push(dbg!(InstallFile::new_raw(
                            file.clone(),
                            file.file_name().unwrap().to_string(),
                        )));
                    }
                    _ => (),
                }
            }
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

    let mut version = None;
    let mut nexus_id = None;
    let mut name = mod_dir.to_string();
    if let Ok(dmodman) = DmodMan::try_from(dmodman.as_path()) {
        nexus_id = Some(dmodman.mod_id());
        version = dmodman.version();
        name = dmodman.name();
    }

    Ok(Manifest::new(
        cache_dir,
        mod_dir,
        name,
        nexus_id,
        version,
        files,
        disabled_files,
        mod_kind,
    ))
}
