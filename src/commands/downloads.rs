use std::{
    collections::{HashMap, HashSet},
    fs::{self, metadata, remove_dir_all, remove_file},
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    decompress::SupportedArchives,
    dmodman::{DmodMan, DMODMAN_EXTENSION},
    manifest::{Manifest, MANIFEST_EXTENSION},
    mods::{FindInModList, GatherModList, ModKind, ModList},
    settings::{create_table, Settings},
    utils::{rename_recursive, AddExtension},
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use comfy_table::{Cell, Color};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
use thiserror::Error;

use super::list::list_mods;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("the archive {0} cannot be found.")]
    ArchiveNotFound(String),
}

#[derive(Debug, Clone, Parser, Default)]
pub enum DownloadCmd {
    /// List all archives in the download directory
    #[default]
    #[clap(visible_aliases = &["lists", "l"])]
    List,
    /// Extract given archive
    Extract { name: String },
    /// Extract all archives which are not in the cache directory.
    ExtractAll,
    /// Re-install given archive
    ReInstall { name: String },
    /// Update all mods which have an archive in the archive directory with a newer version.
    #[clap(visible_alias = "update-all")]
    UpgradeAll,
}
impl DownloadCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::List => list_downloaded_files(settings.download_dir(), settings.cache_dir()),
            Self::Extract { name } => {
                find_and_extract_archive(
                    settings.download_dir(),
                    settings.cache_dir(),
                    name.as_str(),
                )?;
                list_mods(settings.cache_dir())
            }
            Self::ExtractAll => {
                extract_downloaded_files(settings.download_dir(), settings.cache_dir())?;
                list_mods(settings.cache_dir())
            }
            Self::ReInstall { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    mod_list[idx].remove()?;

                    let mod_type = ModKind::detect_mod_type(
                        settings.cache_dir(),
                        &mod_list[idx].manifest_dir(),
                    )?;
                    mod_type.create_mod(settings.cache_dir(), &mod_list[idx].manifest_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Self::UpgradeAll => {
                let dmodman_list = DmodMan::gather_list(settings.download_dir())?;
                let dmodman_list = HashMap::<_, _>::from_iter(
                    dmodman_list
                        .iter()
                        .map(|dm| ((dm.name(), dm.mod_id()), dm.clone())),
                );
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.retain(|md| {
                    dmodman_list
                        .get(&(
                            md.bare_file_name().to_string(),
                            md.nexus_id().unwrap_or_default(),
                        ))
                        .map(|dmod| md.is_an_update(dmod))
                        .unwrap_or(false)
                });

                for md in mod_list {
                    let priority = md.priority();
                    let enabled = md.is_enabled();
                    let name = dmodman_list
                        .get(&(
                            md.bare_file_name().to_string(),
                            md.nexus_id().unwrap_or_default(),
                        ))
                        .map(|dmod| dmod.file_name())
                        .unwrap_or_default();
                    log::info!("Updating '{name}'");
                    md.remove()?;

                    if let Some(mut manifest) = find_and_extract_archive(
                        settings.download_dir(),
                        settings.cache_dir(),
                        name,
                    )? {
                        manifest.set_priority(priority)?;
                        if enabled {
                            manifest.set_enabled()?;
                        }
                    }
                }

                list_mods(settings.cache_dir())
            }
        }
    }
}

pub fn list_downloaded_files(download_dir: &Utf8Path, cache_dir: &Utf8Path) -> Result<()> {
    let sf = downloaded_files(download_dir)?;
    let mod_list = Vec::gather_mods(cache_dir)?;
    let mod_list =
        HashMap::<_, _>::from_iter(mod_list.iter().map(|m| (m.bare_file_name().to_string(), m)));

    let mut table = create_table(vec!["Archive", "Status"]);

    for (_, f) in sf {
        let dmodman = DmodMan::try_from(download_dir.join(&f).add_extension("json")).ok();
        let archive = dmodman
            .as_ref()
            .map(|dmod| dmod.name())
            .unwrap_or_else(|| f.with_extension("").as_str().to_lowercase());
        let manifest = mod_list.get(&archive);

        log::trace!("testing {} against {}.", f.as_str(), archive);

        let (state, color) = match (
            // is installed
            manifest.is_some(),
            // is an upgrade
            dmodman
                .map(|dmod| manifest.map(|m| m.is_an_update(&dmod)))
                .flatten()
                .unwrap_or(false),
        ) {
            (true, false) => ("Installed", Color::Grey),
            (true, true) => ("Upgrade", Color::Yellow),
            (false, _) => ("New", Color::Green),
        };

        table.add_row(vec![
            Cell::new(f).fg(Color::White),
            Cell::new(state).fg(color),
        ]);
    }

    log::info!("{table}");
    Ok(())
}

pub fn downloaded_files(download_dir: &Utf8Path) -> Result<Vec<(SupportedArchives, Utf8PathBuf)>> {
    let mut supported_files = Vec::new();
    let paths = fs::read_dir(download_dir).unwrap();

    // TODO check for a dmodman file
    // and check for the game in that file

    for path in paths {
        if let Ok(path) = path {
            if let Ok(typ) = SupportedArchives::from_path(&path.path()) {
                let path = Utf8PathBuf::try_from(path.file_name().to_str().unwrap_or_default())?;
                supported_files.push((typ, path));
            }
        }
    }

    Ok(supported_files)
}

pub fn extract_downloaded_files(download_dir: &Utf8Path, cache_dir: &Utf8Path) -> Result<()> {
    let sf = downloaded_files(download_dir)?;
    let extracted_files = Vec::with_capacity(sf.len());
    let extracted_files = Arc::new(Mutex::new(extracted_files));

    let sty = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}").unwrap();
    let multi = MultiProgress::new();
    let running = AtomicBool::new(true);

    let mut progress_bars = Vec::with_capacity(sf.len());

    for (_, f) in &sf {
        let p = ProgressBar::new(1).with_style(sty.clone());
        multi.add(p.clone());
        progress_bars.push(p.clone());
        p.set_message(format!("Extracting: {}", f));
    }
    let progress_bars = Arc::new(progress_bars);

    thread::scope(|s| {
        s.spawn(|| {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                for pb in progress_bars.iter() {
                    if !pb.is_finished() {
                        pb.tick();
                    }
                }
                thread::sleep(Duration::from_millis(70));
            }
        });
        use rayon::prelude::*;

        sf.par_iter().enumerate().try_for_each(|(idx, (typ, f))| {
            if extract_downloaded_file(download_dir, cache_dir, *typ, f)? {
                extracted_files.lock().unwrap().push(f.as_path());
                progress_bars[idx].inc(1);
                progress_bars[idx].finish_with_message(format!("Extracting: {} ... => Done.", f));
            } else {
                progress_bars[idx].finish_with_message(format!("Skipped: {} ... => Done.", f));
            }
            Ok::<(), anyhow::Error>(())
        })?;

        running.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok::<(), anyhow::Error>(())
    })?;

    for name in extracted_files.lock().unwrap().iter() {
        install_downloaded_file(&cache_dir, name)?;
    }

    Ok(())
}

pub fn find_and_extract_archive(
    download_dir: &Utf8Path,
    cache_dir: &Utf8Path,
    name: &str,
) -> Result<Option<Manifest>> {
    let sf = downloaded_files(download_dir)?;
    if let Some((sa, f)) = find_archive_by_name(&sf, &name) {
        if extract_downloaded_file(download_dir, cache_dir, sa, f.as_path())? {
            install_downloaded_file(&cache_dir, &f).map(|md| Some(md))
        } else {
            Ok(None)
        }
    } else if let Some((sa, f)) = find_mod_by_name_fuzzy(&sf, &name) {
        if extract_downloaded_file(download_dir, cache_dir, sa, f.as_path())? {
            install_downloaded_file(&cache_dir, &f).map(|md| Some(md))
        } else {
            Ok(None)
        }
    } else {
        Err(DownloadError::ArchiveNotFound(name.to_owned()).into())
    }
}

fn extract_downloaded_file(
    download_dir: &Utf8Path,
    cache_dir: &Utf8Path,
    archive_type: SupportedArchives,
    file: &Utf8Path,
) -> Result<bool> {
    //destination:
    //Force utf-8 compatible strings, in lower-case, here to simplify futher code.
    let download_file = Utf8PathBuf::from(download_dir).join(file);

    let file = file.as_str().to_lowercase();
    let archive = cache_dir.join(file.as_str()).with_extension("");
    let dmodman_file = download_file.add_extension("json");
    let name = Utf8PathBuf::from(file).with_extension("");

    //TODO use dmodman file to verify if file belongs to our current game.

    if metadata(&archive).map(|m| m.is_dir()).unwrap_or(false)
        && Manifest::from_file(cache_dir, &name)
            .map(|m| m.is_valid())
            .unwrap_or(false)
    {
        // Archive exists and is valid
        // Nothing to do
        log::debug!("Skipping already extracted {}", download_file);
        Ok(false)
    } else {
        //TODO: if either one of Dir or Manifest file is missing or corrupt, remove them,

        if archive.exists() {
            if archive.is_dir() {
                remove_dir_all(&archive)?;
            } else if archive.is_file() {
                remove_file(&archive)?;
            }
        }

        // log::info!("Extracting {}", download_file);
        log::debug!("Extracting {} to {}", download_file, archive);
        archive_type
            .decompress(download_file.as_std_path(), archive.as_std_path())
            .unwrap();

        // Rename all extracted files to their lower-case counterpart
        // This is especially important for fomod mods, because otherwise we would
        // not know if their name in the fomod package matches their actual names.
        rename_recursive(&archive)?;

        // TODO: Right now we just copy the dmodman file
        // we should incorporate it into the manifest
        if dmodman_file.exists() {
            let archive_dmodman = archive.add_extension(DMODMAN_EXTENSION);

            log::trace!(
                "copying dmondman file: {} -> {}",
                dmodman_file,
                archive_dmodman
            );
            std::fs::copy(&dmodman_file, &archive_dmodman)?;
        }
        Ok(true)
    }
}

fn install_downloaded_file(cache_dir: &Utf8Path, file: &Utf8Path) -> Result<Manifest> {
    let file = Utf8PathBuf::from(file.as_str().to_lowercase()).with_extension("");
    let mod_kind = ModKind::detect_mod_type(&cache_dir, &file)?;
    mod_kind.create_mod(&cache_dir, &file)
}

pub fn find_archive_by_name(
    archive_list: &[(SupportedArchives, Utf8PathBuf)],
    name: &str,
) -> Option<(SupportedArchives, Utf8PathBuf)> {
    archive_list
        .iter()
        .find_map(|(archive_type, f)| (f == name).then(|| (archive_type.clone(), f.clone())))
}
pub fn find_mod_by_name_fuzzy(
    archive_list: &[(SupportedArchives, Utf8PathBuf)],
    fuzzy_name: &str,
) -> Option<(SupportedArchives, Utf8PathBuf)> {
    let matcher = SkimMatcherV2::default();
    let mut match_vec = Vec::new();

    archive_list.iter().for_each(|(st, f)| {
        let i = matcher.fuzzy_match(f.as_str(), &fuzzy_name).unwrap_or(0);
        match_vec.push((st, f, i));
    });

    match_vec.sort_unstable_by(|(_, _, ia), (_, _, ib)| ia.cmp(ib));

    match_vec
        .last()
        .map(|(sa, f, _)| ((*sa).clone(), (*f).clone()))
}
