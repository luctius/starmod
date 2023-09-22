use anyhow::Result;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::{
    dmodman::{DmodMan, DMODMAN_EXTENTION},
    manifest::{InstallFile, Manifest},
    mod_types::ModType,
};

pub fn create_data_manifest(
    mod_type: ModType,
    cache_dir: &Path,
    mod_dir: &Path,
) -> Result<Manifest> {
    if let ModType::DataMod { data_start } = &mod_type {
        let mut files = Vec::new();
        let mut disabled_files = Vec::new();

        let mut archive_dir = PathBuf::from(cache_dir);
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
            let entry_path = entry.path();

            if entry_path.is_file() {
                let file = entry_path
                    .to_path_buf()
                    .strip_prefix(&archive_dir)?
                    .to_path_buf();

                files.push(dbg!(file.into()));
            }
        }

        // Disable all files containing 'readme' in the name
        files.retain(|f: &InstallFile| {
            if !f.source.starts_with(&data_start)
                || f.source
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

        let mut version = None;
        let mut nexus_id = None;
        let mut name = mod_dir.to_string_lossy().to_string();
        if let Ok(dmodman) = DmodMan::try_from(dmodman.as_path()) {
            nexus_id = Some(dmodman.mod_id());
            version = dmodman.version();
            name = dmodman.name();
        }

        Ok(Manifest::new(
            mod_dir,
            name,
            nexus_id,
            version,
            mod_type,
            files,
            disabled_files,
        ))
    } else {
        panic!("no data mod");
    }
}
