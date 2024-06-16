use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use walkdir::WalkDir;

use crate::{
    dmodman::{DmodMan, DMODMAN_EXTENSION},
    manifest::{install_file::InstallFile, Manifest},
    mods::ModKind,
    utils::AddExtension,
};

use super::InstallerError;

pub fn create_data_manifest(
    mod_kind: ModKind,
    cache_dir: &Utf8Path,
    name: &Utf8Path,
) -> Result<Manifest> {
    let manifest_dir = cache_dir.join(name);
    let mut data_path = None;
    let walker = WalkDir::new(&manifest_dir)
        .min_depth(1)
        .max_depth(2)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(true);

    // Check for a 'Data' dir in the root directories
    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() && entry.path().file_name().unwrap() == "data" {
            if data_path.is_none() {
                log::debug!("Setting Data dir to root 'Data'.");
                let entry_path = entry_path.to_path_buf();
                data_path = Some(entry_path.strip_prefix(&manifest_dir)?.to_path_buf());
            } else {
                Err(InstallerError::MultipleDataDirectories(name.to_string()))?;
            }
        }
    }

    if data_path.is_none() {
        // Check for the 'Data' dir in any directories

        let walker = WalkDir::new(&manifest_dir)
            .min_depth(1)
            .max_depth(5)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() && entry.path().file_name().unwrap() == "data" {
                if data_path.is_none() {
                    log::debug!("Setting Data dir to {}.", entry_path.display());
                    data_path = Some(
                        entry_path
                            .to_path_buf()
                            .strip_prefix(&manifest_dir)?
                            .to_path_buf(),
                    );
                } else {
                    Err(InstallerError::MultipleDataDirectories(name.to_string()))?;
                }
            }
        }
    }

    if data_path.is_none() {
        // Check for any 'esm' or 'esp' files...

        let walker = WalkDir::new(&manifest_dir)
            .min_depth(1)
            .max_depth(5)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            // Avoid '*.esp' files for they should not be used with Starfield.
            // TODO: FIXME: NOTE: disable this somehow for other games....
            if entry_path.is_file() && entry_path.extension().unwrap() == "esp" {
                Err(InstallerError::MultipleDataDirectories(name.to_string()))?;
            }

            if entry_path.is_file() && entry_path.extension().unwrap() == "esm" {
                if data_path.is_none() {
                    log::debug!("Setting Esm dir to {}.", entry_path.display());
                    data_path = Some(
                        entry_path
                            .parent()
                            .unwrap()
                            .to_path_buf()
                            .strip_prefix(&manifest_dir)?
                            .to_path_buf(),
                    );
                } else {
                    Err(InstallerError::MultipleDataDirectories(name.to_string()))?;
                }
            }
        }
    }

    if data_path.is_none() {
        // Check for any 'esl' files...

        let walker = WalkDir::new(&manifest_dir)
            .min_depth(1)
            .max_depth(5)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() && entry_path.extension().unwrap() == "esl" {
                if data_path.is_none() {
                    log::debug!("Setting Esl dir to {}.", entry_path.display());
                    data_path = Some(
                        entry_path
                            .parent()
                            .unwrap()
                            .to_path_buf()
                            .strip_prefix(&manifest_dir)?
                            .to_path_buf(),
                    );
                } else {
                    Err(InstallerError::MultipleDataDirectories(name.to_string()))?;
                }
            }
        }
    }

    if data_path.is_none() {
        log::debug!("Setting Data dir to default.");
    }

    let data_path = Utf8PathBuf::try_from(data_path.unwrap_or_default())?;

    let mut files = Vec::new();
    let mut disabled_files = Vec::new();

    let archive_dir = cache_dir.join(name);
    let dmodman = archive_dir.add_extension(DMODMAN_EXTENSION);

    let walker = WalkDir::new(&archive_dir.join(&data_path))
        .min_depth(1)
        .max_depth(usize::MAX)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(false);

    for entry in walker {
        let entry = entry?;
        let entry_path = Utf8PathBuf::try_from(entry.path().to_path_buf())?;

        if entry_path.is_file() {
            let source = entry_path
                .to_path_buf()
                .strip_prefix(&archive_dir)?
                .to_path_buf();

            let destination = source.to_string();
            let destination = destination
                .strip_prefix(data_path.as_str())
                .map(std::borrow::ToOwned::to_owned)
                .unwrap_or(destination);

            files.push(InstallFile::new(source, &destination));
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
    let manifest_dir = name.to_path_buf();
    let mut name = name.to_string();
    if let Ok(dmodman) = DmodMan::try_from(dmodman.as_path()) {
        nexus_id = Some(dmodman.mod_id());
        version = dmodman.version();
        name = dmodman.name();
    }

    Ok(Manifest::new(
        cache_dir,
        manifest_dir.as_path(),
        name.clone(),
        name,
        nexus_id,
        version,
        files,
        disabled_files,
        mod_kind,
    ))
}
