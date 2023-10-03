use std::{
    collections::HashSet,
    fmt::Display,
    fs::{self, read_link, remove_dir, remove_file, rename, DirBuilder},
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{
    conflict::conflict_list_by_file,
    installers::{
        custom::create_custom_manifest,
        data::create_data_manifest,
        fomod::{create_fomod_manifest, FOMOD_INFO_FILE, FOMOD_MODCONFIG_FILE},
        loader::create_loader_manifest,
    },
    manifest::{Manifest, MANIFEST_EXTENSION},
    utils::AddExtension,
};

const BACKUP_EXTENTION: &'static str = "starmod_bkp";

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModKind {
    // Goes into Data
    Data,
    //Installer
    FoMod,
    //Goes into the root dir
    Loader,
    // Custom Mods, should always scan their files
    Custom,
}
impl ModKind {
    pub fn detect_mod_type(cache_dir: &Utf8Path, name: &Utf8Path) -> Result<Self> {
        let archive_dir = Utf8PathBuf::from(cache_dir).join(name);

        let walker = WalkDir::new(&archive_dir)
            .min_depth(1)
            .max_depth(2)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(false);

        let mut info = false;
        let mut config = false;

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if let Ok(p) = entry_path.strip_prefix(&archive_dir) {
                if p.to_string_lossy().to_string() == FOMOD_INFO_FILE {
                    info = true;
                }
            }
            if let Ok(p) = entry_path.strip_prefix(&archive_dir) {
                if p.to_string_lossy().to_string() == FOMOD_MODCONFIG_FILE {
                    config = true;
                }
            }

            if info && config {
                return Ok(Self::FoMod);
            }
        }

        let walker = WalkDir::new(&archive_dir)
            .min_depth(1)
            .max_depth(3)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if let Some(ext) = entry_path.extension() {
                if ext == "exe" {
                    return Ok(Self::Loader);
                }
            }
        }

        Ok(Self::Data)
    }
    pub fn create_mod(self, cache_dir: &Utf8Path, name: &Utf8Path) -> Result<Manifest> {
        let md = match self {
            Self::FoMod => create_fomod_manifest(self, cache_dir, name)?,
            Self::Loader => create_loader_manifest(self, cache_dir, name)?,
            Self::Custom => create_custom_manifest(self, cache_dir, name)?,
            Self::Data => create_data_manifest(self, cache_dir, name)?,
        };

        md.write()?;
        Ok(md)
    }
}
impl Display for ModKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Data { .. } => f.write_str("Data"),
            Self::FoMod => f.write_str("FoMod"),
            Self::Loader => f.write_str("Loader"),
            Self::Custom => f.write_str("Custom"),
        }
    }
}

pub trait GatherModList {
    fn gather_mods(cache_dir: &Utf8Path) -> Result<Vec<Manifest>>;
}

impl GatherModList for Vec<Manifest> {
    fn gather_mods(cache_dir: &Utf8Path) -> Result<Vec<Manifest>> {
        let paths = fs::read_dir(cache_dir)?;

        let mut mod_list = Vec::new();

        for path in paths {
            if let Ok(entry) = path {
                if entry
                    .path()
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    .eq(MANIFEST_EXTENSION)
                {
                    mod_list.push(Manifest::try_from(
                        Utf8PathBuf::try_from(entry.path().to_path_buf())?.as_path(),
                    )?);
                }
            }
        }

        mod_list.sort_by(|a, b| a.cmp(b));

        Ok(mod_list)
    }
}

pub trait ModList {
    fn enable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()>;
    fn disable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()>;
    fn re_enable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()>;
    fn enable_mod(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path, idx: usize) -> Result<()>;
    fn disable_mod(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path, idx: usize) -> Result<()>;
}
impl ModList for Vec<Manifest> {
    fn enable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
        self.as_mut_slice().enable(cache_dir, game_dir)
    }
    fn disable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
        self.as_mut_slice().disable(cache_dir, game_dir)
    }
    fn re_enable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
        self.as_mut_slice().re_enable(cache_dir, game_dir)
    }
    fn enable_mod(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path, idx: usize) -> Result<()> {
        self.as_mut_slice().enable_mod(cache_dir, game_dir, idx)
    }
    fn disable_mod(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path, idx: usize) -> Result<()> {
        self.as_mut_slice().disable_mod(cache_dir, game_dir, idx)
    }
}
impl ModList for &mut [Manifest] {
    fn enable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
        let conflict_list = conflict_list_by_file(self)?;
        let mut file_list = Vec::with_capacity(conflict_list.len());
        let mut dir_cache = Vec::new();

        log::debug!("Collecting File List");
        for m in self.iter() {
            file_list.extend(m.enlist_files(&conflict_list)?);
        }

        log::trace!("file_list: {:?}", file_list);

        log::debug!("Installing Files");
        for f in file_list {
            let origin = cache_dir.clone().join(f.source());
            let destination = game_dir.clone().join(Utf8PathBuf::from(f.destination()));
            log::trace!("starting with file: {} -> {}", origin, destination);

            let destination_base = destination.parent().unwrap().to_path_buf();
            if !dir_cache.contains(&destination_base) {
                //create intermediate directories
                DirBuilder::new()
                    .recursive(true)
                    .create(&destination_base)?;
                dir_cache.push(destination_base);
            }

            // Remove existing symlinks which point back to our archive dir
            // This ensures that the last mod wins, but we should do conflict
            // detection and resolution before this, so we can inform the user.
            if destination.is_symlink() {
                let target = Utf8PathBuf::try_from(read_link(&destination)?)?;

                if target.starts_with(&cache_dir) {
                    remove_file(&destination)?;
                    log::debug!("overrule {} ({} > {})", destination, origin, target);
                }
            }

            if destination.is_file() {
                let bkp_destination = destination.add_extension(BACKUP_EXTENTION);
                log::info!(
                    "renaming foreign file from {} -> {}",
                    destination,
                    bkp_destination
                );
                rename(&destination, bkp_destination)?;
            }

            std::os::unix::fs::symlink(&origin, &destination)?;

            log::debug!("link {} to {}", origin, destination);
        }

        log::debug!("Set Mods to Enabled");
        for m in self.iter_mut() {
            m.set_enabled()?;
        }

        Ok(())
    }
    fn disable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
        let conflict_list = conflict_list_by_file(self)?;
        let mut file_list = Vec::with_capacity(conflict_list.len());

        log::debug!("Collecting File List");
        for m in self.iter() {
            file_list.extend(m.enlist_files(&conflict_list)?);
        }

        log::trace!("file_list: {:?}", file_list);

        log::debug!("Start Removing files");
        for f in file_list {
            let origin = cache_dir.clone().join(f.source());
            let destination = game_dir.clone().join(Utf8PathBuf::from(f.destination()));

            if destination.is_file()
                && destination.is_symlink()
                && read_link(&destination)? == origin
            {
                remove_file(&destination)?;
                log::debug!("removed {} -> {}", destination, origin);
            } else {
                log::debug!("passing-over {}", destination);
            }
        }

        log::debug!("Clean-up Game Dir");
        let walker = WalkDir::new(&game_dir)
            .min_depth(1)
            .max_depth(usize::MAX)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            // Restore backupped files
            if entry_path.is_file() {
                if entry_path
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    == BACKUP_EXTENTION
                {
                    let new = entry_path.with_extension("");
                    if !new.exists() {
                        log::debug!(
                            "Restoring Backup: {} -> {}.",
                            &entry_path.display(),
                            new.display()
                        );
                        rename(entry_path, new)?;
                    }
                }
            }
            // Remove empty directories
            if entry_path.is_dir() {
                log::debug!("Trying to remove dir {}.", entry_path.display());
                let _ = remove_dir(entry_path);
            }
        }

        log::debug!("Set Mods to Disabled.");
        for m in self.iter_mut() {
            m.set_disabled()?;
        }

        Ok(())
    }
    fn re_enable(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
        let mut mod_cache = HashSet::with_capacity(self.len());
        self.iter()
            .enumerate()
            .filter(|(_, m)| m.is_enabled())
            .map(|(idx, _m)| idx)
            .for_each(|idx| {
                mod_cache.insert(idx);
            });

        self.disable(cache_dir, game_dir)?;

        let mut mod_cache = self
            .iter()
            .enumerate()
            .filter(|(idx, _m)| mod_cache.contains(idx))
            .map(|(_idx, m)| m.clone())
            .collect::<Vec<_>>();
        mod_cache.enable(cache_dir, game_dir)?;

        Ok(())
    }
    fn enable_mod(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path, idx: usize) -> Result<()> {
        if let Some(md) = self.get(idx) {
            if md.is_enabled() {
                self.disable_mod(cache_dir, game_dir, idx)?;
            }
        } else {
            todo!()
        }
        if let Some(md) = self.get_mut(idx) {
            log::debug!("Enabling {}", md.name());
            md.set_enabled()?;
            self[0..=idx].as_mut().re_enable(cache_dir, game_dir)?;
            Ok(())
        } else {
            todo!()
        }
    }
    fn disable_mod(&mut self, cache_dir: &Utf8Path, game_dir: &Utf8Path, idx: usize) -> Result<()> {
        if let Some(md) = self.get_mut(idx) {
            log::debug!("Disabling {}", md.name());

            md.set_disabled()?;
            self[0..=idx].as_mut().re_enable(cache_dir, game_dir)?;
            Ok(())
        } else {
            todo!()
        }
    }
}

pub trait FindInModList {
    fn find_mod(&self, mod_name: &str) -> Option<usize>;
    fn find_mod_by_name(&self, name: &str) -> Option<usize>;
    fn find_mod_by_name_fuzzy(&self, fuzzy_name: &str) -> Option<usize>;
}

impl FindInModList for Vec<Manifest> {
    fn find_mod(&self, mod_name: &str) -> Option<usize> {
        self.as_slice().find_mod(mod_name)
    }
    fn find_mod_by_name(&self, mod_name: &str) -> Option<usize> {
        self.as_slice().find_mod_by_name(mod_name)
    }
    fn find_mod_by_name_fuzzy(&self, fuzzy_name: &str) -> Option<usize> {
        self.as_slice().find_mod_by_name_fuzzy(fuzzy_name)
    }
}
impl FindInModList for &[Manifest] {
    fn find_mod(&self, mod_name: &str) -> Option<usize> {
        if let Some(m) = self.find_mod_by_name(&mod_name) {
            Some(m)
        } else if let Ok(idx) = usize::from_str_radix(&mod_name, 10) {
            self.get(idx).map(|_| idx)
        } else if let Some(m) = self.find_mod_by_name_fuzzy(&mod_name) {
            Some(m)
        } else {
            None
        }
    }

    fn find_mod_by_name(&self, name: &str) -> Option<usize> {
        self.iter()
            .enumerate()
            .find_map(|(idx, m)| (m.name() == name).then(|| idx))
    }
    fn find_mod_by_name_fuzzy(&self, fuzzy_name: &str) -> Option<usize> {
        let matcher = SkimMatcherV2::default();
        let mut match_vec = Vec::new();

        self.iter().enumerate().for_each(|(idx, m)| {
            let i = matcher.fuzzy_match(m.name(), &fuzzy_name).unwrap_or(0);
            match_vec.push((idx, i));
        });

        match_vec.sort_unstable_by(|(_, ia), (_, ib)| ia.cmp(ib));

        match_vec.last().map(|(idx, _)| *idx)
    }
}
