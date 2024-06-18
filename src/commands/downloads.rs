use std::{
    collections::HashMap,
    fs::{self, metadata, remove_dir_all, remove_file},
    io::{stdin, IsTerminal},
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    decompress::SupportedArchives,
    dmodman::{DmodMan, DMODMAN_EXTENSION},
    errors::DownloadError,
    installers::stdin::{Input, InputWithDefault},
    manifest::Manifest,
    mods::{FindInModList, GatherModList, ModKind, ModList},
    settings::Settings,
    ui::{ArchiveListBuilder, FindSelectBuilder},
    utils::{rename_recursive, AddExtension},
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use read_stdin::prompt_until_ok;

use super::list::list_mods;

#[derive(Debug, Clone, Parser, Default)]
pub enum DownloadCmd {
    /// List all archives in the download directory
    #[default]
    #[clap(visible_aliases = &["lists", "l"])]
    List,
    /// Extract given archive
    Extract { name: Option<String> },
    /// Extract all archives which are not in the cache directory.
    ExtractAll,
    /// Re-install given archive
    ReInstall { name: Option<String> },
    /// Update all mods which have an archive in the archive directory with a newer version.
    #[clap(visible_alias = "update-all")]
    UpgradeAll,
    /// Update mod which have an archive in the archive directory with a newer version.
    #[clap(visible_alias = "update")]
    Upgrade { name: Option<String> },
}
impl DownloadCmd {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Self::List => list_downloaded_files(settings.download_dir(), settings.cache_dir()),
            Self::Extract { name } => {
                let name = FindSelectBuilder::new(
                    ArchiveListBuilder::new(settings.download_dir(), settings.cache_dir())
                        .with_index()
                        .with_status()
                        .with_colour(),
                )
                .with_msg("Please select an archive to extract:")
                .with_input(name.as_deref())
                .build()?
                .prompt()?;

                let idx = name.split_whitespace().skip(1).next().unwrap();

                find_and_extract_archive(settings.download_dir(), settings.cache_dir(), idx)?;

                list_mods(settings)
            }
            Self::ExtractAll => {
                extract_downloaded_files(settings.download_dir(), settings.cache_dir())?;
                list_mods(settings)
            }
            Self::ReInstall { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to re-install:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;

                mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                mod_list[idx].remove()?;

                let mod_type =
                    ModKind::detect_mod_type(settings.cache_dir(), mod_list[idx].manifest_dir())?;
                mod_type.create_mod(settings.cache_dir(), mod_list[idx].manifest_dir())?;
                Ok(())
            }
            Self::UpgradeAll => {
                let dmodman_list = DmodMan::gather_list(settings.download_dir())?;
                let dmodman_list = dmodman_list
                    .iter()
                    .map(|dm| ((dm.name(), dm.mod_id()), dm.clone()))
                    .collect::<HashMap<_, _>>();
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.retain(|md| {
                    dmodman_list
                        .get(&(
                            md.bare_file_name().to_string(),
                            md.nexus_id().unwrap_or_default(),
                        ))
                        .is_some_and(|dmod| md.is_an_update(dmod))
                });

                for md in mod_list {
                    //TODO Move this to manifest::upgrade
                    let priority = md.priority();
                    let enabled = md.is_enabled();
                    let name = dmodman_list
                        .get(&(
                            md.bare_file_name().to_string(),
                            md.nexus_id().unwrap_or_default(),
                        ))
                        .map(DmodMan::file_name)
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

                list_mods(settings)
            }
            Self::Upgrade { name } => {
                let dmodman_list = DmodMan::gather_list(settings.download_dir())?;
                let mod_list = Vec::gather_mods(settings.cache_dir())?;
                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to upgrade:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;
                let md = &mod_list[idx];

                let dmodman = dmodman_list.iter().find(|dm| {
                    dm.name() == md.name() && dm.mod_id() == md.nexus_id().unwrap_or_default()
                });

                if let Some(dmod) = dmodman {
                    //TODO Move this to manifest::upgrade
                    let priority = md.priority();
                    let enabled = md.is_enabled();
                    let name = dmod.file_name();

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
                Ok(())
            }
        }
    }
}

pub fn list_downloaded_files(download_dir: &Utf8Path, cache_dir: &Utf8Path) -> Result<()> {
    let list = ArchiveListBuilder::new(download_dir, cache_dir)
        .with_index()
        .with_status()
        .with_headers()
        .with_colour()
        .build()?;

    log::info!("{}", list.join("\n"));
    Ok(())
}

pub fn downloaded_files(download_dir: &Utf8Path) -> Result<Vec<(SupportedArchives, Utf8PathBuf)>> {
    let mut supported_files = Vec::new();
    let paths = fs::read_dir(download_dir).unwrap();

    // TODO check for a dmodman file
    // and check for the game in that file

    for path in paths.flatten() {
        if let Ok(typ) = SupportedArchives::from_path(&path.path()) {
            let path = Utf8PathBuf::try_from(path.file_name().to_str().unwrap_or_default())?;
            supported_files.push((typ, path));
        }
    }

    Ok(supported_files)
}

pub fn extract_downloaded_files(download_dir: &Utf8Path, cache_dir: &Utf8Path) -> Result<()> {
    use rayon::prelude::*;

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
        p.set_message(format!("Extracting: {f}"));
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

        sf.par_iter().enumerate().try_for_each(|(idx, (typ, f))| {
            if extract_downloaded_file(download_dir, cache_dir, *typ, f)? {
                extracted_files.lock().unwrap().push(f.as_path());
                progress_bars[idx].inc(1);
                progress_bars[idx].finish_with_message(format!("Extracting: {f} ... => Done."));
            } else {
                progress_bars[idx].finish_with_message(format!("Skipped: {f} ... => Done."));
            }
            Ok::<(), anyhow::Error>(())
        })?;

        running.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok::<(), anyhow::Error>(())
    })?;

    let extracted_files = extracted_files.lock().unwrap();
    for name in extracted_files.iter() {
        install_downloaded_file(cache_dir, name)?;
    }

    Ok(())
}

pub fn find_and_extract_archive(
    download_dir: &Utf8Path,
    cache_dir: &Utf8Path,
    name: &str,
) -> Result<Option<Manifest>> {
    let sf = downloaded_files(download_dir)?;
    if let Some(idx) = name.parse::<usize>().ok() {
        if let Some((sa, f)) = sf.get(idx).cloned() {
            if extract_downloaded_file(download_dir, cache_dir, sa, f.as_path())? {
                install_downloaded_file(cache_dir, &f).map(Some)
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else if let Some((sa, f)) = find_archive_by_name(&sf, name) {
        if extract_downloaded_file(download_dir, cache_dir, sa, f.as_path())? {
            install_downloaded_file(cache_dir, &f).map(Some)
        } else {
            Ok(None)
        }
    } else if let Some((sa, f)) = find_archive_by_name_fuzzy(&sf, name) {
        if extract_downloaded_file(download_dir, cache_dir, sa, f.as_path())? {
            install_downloaded_file(cache_dir, &f).map(Some)
        } else {
            Ok(None)
        }
    } else {
        log::trace!("Archive \'{name}\' not found");
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
    let mod_kind = ModKind::detect_mod_type(cache_dir, &file)?;
    mod_kind.create_mod(cache_dir, &file)
}

pub fn find_archive_by_name(
    archive_list: &[(SupportedArchives, Utf8PathBuf)],
    name: &str,
) -> Option<(SupportedArchives, Utf8PathBuf)> {
    archive_list
        .iter()
        .find_map(|(archive_type, f)| (f == name).then(|| (*archive_type, f.clone())))
}
pub fn find_archive_by_name_fuzzy(
    archive_list: &[(SupportedArchives, Utf8PathBuf)],
    fuzzy_name: &str,
) -> Option<(SupportedArchives, Utf8PathBuf)> {
    let matcher = SkimMatcherV2::default();
    let mut match_vec = Vec::new();

    for (st, f) in archive_list {
        let i = matcher.fuzzy_match(f.as_str(), fuzzy_name).unwrap_or(0);
        match_vec.push((st, f, i));
    }

    match_vec.sort_unstable_by(|(_, _, ia), (_, _, ib)| ia.cmp(ib));
    let match_vec = match_vec
        .iter()
        .rev()
        .enumerate()
        .take_while(|(i, (_, _, mv))| *i <= 5 && *mv > 50)
        .map(|(_, (sa, f, _))| (*(*sa), (*f).clone()))
        .collect::<Vec<_>>();

    if match_vec.len() == 1 {
        match_vec.first().cloned()
    } else if match_vec.len() > 1 {
        let choice = if stdin().is_terminal() {
            //TODO more color and stuff

            log::info!(
                "Multiple matches found; Please choose one: (Defaults to 0/'{}' on Enter)",
                match_vec.first().unwrap().1
            );
            for (i, (_, f)) in match_vec.iter().enumerate() {
                log::info!("{i}) {}", f);
            }
            log::info!("E) Exit");

            loop {
                let input: InputWithDefault = prompt_until_ok("Select : ");
                match input {
                    InputWithDefault::Input(Input::Exit) => {
                        return None?;
                    }
                    InputWithDefault::Default => {
                        break 0;
                    }
                    InputWithDefault::Input(Input::Digit(d)) => {
                        if (d as usize) < match_vec.len() {
                            break d as usize;
                        }
                    }
                }
            }
        } else {
            0
        };

        match_vec.get(choice).cloned()
    } else {
        None
    }
}
