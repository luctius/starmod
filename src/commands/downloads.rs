use std::{
    ffi::OsString,
    fs::{self, metadata, remove_dir_all, remove_file},
};

use crate::{
    decompress::SupportedArchives, dmodman::DMODMAN_EXTENTION, manifest::Manifest, mods::ModKind,
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("the archive {0} cannot be found.")]
    ArchiveNotFound(String),
}

pub fn downloaded_files(download_dir: &Utf8Path) -> Vec<(SupportedArchives, OsString)> {
    let mut supported_files = Vec::new();
    let paths = fs::read_dir(download_dir).unwrap();

    for path in paths {
        if let Ok(path) = path {
            if let Ok(typ) = SupportedArchives::from_path(&path.path()) {
                supported_files.push((typ, path.file_name()));
            }
        }
    }

    supported_files
}

pub fn extract_downloaded_files(download_dir: &Utf8Path, cache_dir: &Utf8Path) -> Result<()> {
    let sf = downloaded_files(download_dir);
    for (typ, f) in sf {
        extract_downloaded_file(download_dir, cache_dir, typ, f)?;
    }
    Ok(())
}

pub fn find_and_extract_archive(
    download_dir: &Utf8Path,
    cache_dir: &Utf8Path,
    name: &str,
) -> Result<()> {
    let sf = downloaded_files(download_dir);
    if let Some((sa, f)) = find_archive_by_name(&sf, &name) {
        extract_downloaded_file(download_dir, cache_dir, sa, f)
    } else if let Some((sa, f)) = find_mod_by_name_fuzzy(&sf, &name) {
        extract_downloaded_file(download_dir, cache_dir, sa, f)
    } else {
        Err(DownloadError::ArchiveNotFound(name.to_owned()).into())
    }
}

fn extract_downloaded_file(
    download_dir: &Utf8Path,
    cache_dir: &Utf8Path,
    archive_type: SupportedArchives,
    file: OsString,
) -> Result<()> {
    //destination:
    //Force utf-8 compatible strings, in lower-case, here to simplify futher code.
    let file = file.to_string_lossy().to_string();

    let download_file = Utf8PathBuf::from(download_dir).join(&file);

    log::info!("Extracting {}", file);

    let file = file.to_lowercase();
    let archive = Utf8PathBuf::from(cache_dir)
        .join(file.clone())
        .with_extension("");

    let ext = download_file.extension();
    let dmodman_file = download_file.with_extension(&format!("{}.json", ext.unwrap_or_default()));

    //TODO use dmodman file to verify if file belongs to our current game.

    let mut name = Utf8PathBuf::from(file);
    name.set_extension("");

    if metadata(&archive).map(|m| m.is_dir()).unwrap_or(false)
        && Manifest::from_file(cache_dir, &archive)
            .map(|m| m.is_valid())
            .unwrap_or(false)
    {
        // Archive exists and is valid
        // Nothing to do
        log::debug!("skipping {}", download_file);
    } else {
        //TODO: if either one of Dir or Manifest file is missing or corrupt, remove them,

        if archive.exists() {
            if archive.is_dir() {
                remove_dir_all(&archive)?;
            } else if archive.is_file() {
                remove_file(&archive)?;
            }
        }

        log::trace!("extracting {} -> {}", download_file, archive);
        archive_type
            .decompress(download_file.as_std_path(), archive.as_std_path())
            .unwrap();

        // Rename all extracted files to their lower-case counterpart
        // This is especially important for fomod mods, because otherwise we would
        // not know if their name in the fomod package matches their actual names.
        rename_recursive(&archive)?;

        if dmodman_file.exists() {
            let archive_dmodman = archive.with_extension(DMODMAN_EXTENTION);

            log::trace!(
                "copying dmondman file: {} -> {}",
                dmodman_file,
                archive_dmodman
            );
            std::fs::copy(&dmodman_file, &archive_dmodman)?;
        }

        let mod_kind = ModKind::detect_mod_type(&cache_dir, &name)?;
        let _md = dbg!(mod_kind.create_mod(&cache_dir, &name)?);
    }

    Ok(())
}

fn rename_recursive(path: &Utf8Path) -> Result<()> {
    let walker = WalkDir::new(path)
        .min_depth(1)
        .max_depth(usize::MAX)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(true);

    for entry in walker {
        let entry = entry?;
        let entry_path = Utf8PathBuf::try_from(entry.path().to_path_buf())?;

        if entry_path.is_dir() || entry_path.is_file() {
            lower_case(&entry_path)?;
        } else {
            continue;
        }
    }

    Ok(())
}

fn lower_case(path: &Utf8Path) -> Result<()> {
    let name = path.file_name().unwrap();
    let name = name.to_lowercase();
    // let name = OsString::from(name);
    // let name = name.as_os_str();
    let name = path.with_file_name(name);

    log::trace!("ren {} -> {}", path, name);

    std::fs::rename(path, path.with_file_name(name).as_std_path())?;

    Ok(())
}

pub fn find_archive_by_name(
    archive_list: &[(SupportedArchives, OsString)],
    name: &str,
) -> Option<(SupportedArchives, OsString)> {
    archive_list.iter().find_map(|(archive_type, f)| {
        (f.to_string_lossy() == name).then(|| (archive_type.clone(), f.clone()))
    })
}
pub fn find_mod_by_name_fuzzy(
    archive_list: &[(SupportedArchives, OsString)],
    fuzzy_name: &str,
) -> Option<(SupportedArchives, OsString)> {
    let matcher = SkimMatcherV2::default();
    let mut match_vec = Vec::new();

    archive_list.iter().for_each(|(st, f)| {
        let i = matcher
            .fuzzy_match(f.to_string_lossy().to_string().as_str(), &fuzzy_name)
            .unwrap_or(0);
        match_vec.push((st, f, i));
    });

    match_vec.sort_unstable_by(|(_, _, ia), (_, _, ib)| ia.cmp(ib));

    match_vec
        .last()
        .map(|(sa, f, _)| ((*sa).clone(), (*f).clone()))
}
