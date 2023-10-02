use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::{self, read_link, remove_dir, remove_file, rename, DirBuilder, File},
};

use anyhow::{Error, Result};
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
        label::create_label_manifest,
        loader::create_loader_manifest,
    },
    manifest::{install_file::InstallFile, mod_state::ModState, Manifest, MANIFEST_EXTENTION},
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
    // Virtual mod to better organise the list
    Label,
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
    pub fn create_mod(self, cache_dir: &Utf8Path, name: &Utf8Path) -> Result<Mod> {
        let md = match self {
            Self::Label => Mod::Label(
                cache_dir.to_path_buf(),
                create_label_manifest(self, cache_dir, name)?,
            ),
            Self::FoMod => Mod::Data(
                cache_dir.to_path_buf(),
                create_fomod_manifest(self, cache_dir, name)?,
            ),
            Self::Loader => Mod::Data(
                cache_dir.to_path_buf(),
                create_loader_manifest(self, cache_dir, name)?,
            ),
            Self::Custom => Mod::Custom(
                cache_dir.to_path_buf(),
                create_custom_manifest(self, cache_dir, name)?,
            ),
            Self::Data => Mod::Data(cache_dir.to_path_buf(), {
                create_data_manifest(self, cache_dir, name)
            }?),
        };

        md.write()?;
        Ok(md)
    }
    // pub fn prefix_to_strip(&self) -> &str {
    //     match self {
    //         Self::FoMod | Self::Loader | Self::Custom => "",
    //         Self::DataMod { data_start } => data_start.as_str(),
    //     }
    // }
}
impl Display for ModKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Data { .. } => f.write_str("Data"),
            Self::FoMod => f.write_str("FoMod"),
            Self::Loader => f.write_str("Loader"),
            Self::Custom => f.write_str("Custom"),
            Self::Label => f.write_str("Label"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mod {
    // Goes into Data
    Data(Utf8PathBuf, Manifest),
    //Loader(Utf8PathBuf, Manifest),
    Custom(Utf8PathBuf, Manifest),
    // Virtual mod to better organise the list
    Label(Utf8PathBuf, Manifest),
}
//TODO Rewrite
impl Mod {
    pub fn kind(&self) -> ModKind {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(.., m) => m.mod_kind(),
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(.., m) => m.name(),
        }
    }
    pub fn set_name(&mut self, name: String) -> Result<()> {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(_, m) => {
                m.set_name(name);
            }
        }
        self.write()
    }
    pub fn priority(&self) -> isize {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(.., m) => m.priority(),
        }
    }
    pub fn set_priority(&mut self, prio: isize) -> Result<bool> {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(_, m) => {
                if prio < 0 && m.mod_state().is_enabled() {
                    return Ok(false);
                } else {
                    m.set_priority(prio);
                    if m.priority() < 0 {
                        m.set_disabled();
                    }
                }
            }
        }
        self.write().map(|_| true)
    }
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(.., m) => {
                m.mod_state().is_enabled()
            }
        }
    }
    pub fn mod_state(&self) -> ModState {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(.., m) => m.mod_state(),
        }
    }
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Data(.., m) => m.version(),
            Self::Label(..) | Self::Custom(..) => None,
        }
    }
    pub fn nexus_id(&self) -> Option<u32> {
        match self {
            Self::Data(.., m) => m.nexus_id(),
            Self::Label(..) | Self::Custom(..) => None,
        }
    }

    pub fn enlist_files(
        &self,
        conflict_list: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<InstallFile>> {
        match self {
            Self::Data(..) | Self::Custom(..) => {
                let mut enlisted_files = Vec::new();

                for f in &self.files()? {
                    if let Some(winners) = conflict_list.get(f.destination()) {
                        if let Some(winner) = winners.last() {
                            if *winner == self.name() {
                                enlisted_files.push(InstallFile::new_raw(
                                    self.manifest_dir().join(f.source()),
                                    f.destination().to_owned(),
                                ))
                            }
                        }
                    } else {
                        enlisted_files.push(InstallFile::new_raw(
                            self.manifest_dir().join(f.source()),
                            f.destination().to_owned(),
                        ))
                    }
                }

                Ok(enlisted_files)
            }
            Self::Label(..) => Ok(vec![]),
        }
    }
    pub fn manifest_dir(&self) -> &Utf8Path {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(.., m) => m.manifest_dir(),
        }
    }
    pub fn origin_files(&self) -> Result<Vec<Utf8PathBuf>> {
        match self {
            Self::Data(.., m) => m.origin_files(),
            Self::Custom(..) => Ok(self
                .files()?
                .iter()
                .map(|isf| isf.source().to_path_buf())
                .collect::<Vec<_>>()),
            Self::Label(..) => Ok(vec![]),
        }
    }
    pub fn dest_files(&self) -> Result<Vec<String>> {
        match self {
            Self::Data(.., m) => m.dest_files(),
            Self::Custom(..) => Ok(self
                .files()?
                .iter()
                .map(|isf| isf.destination().to_lowercase())
                .collect::<Vec<_>>()),
            Self::Label(..) => Ok(vec![]),
        }
    }
    pub fn files(&self) -> Result<Vec<InstallFile>> {
        match self {
            Self::Data(.., m) => m.files(),
            Self::Custom(dir, m) => {
                let mut files = Vec::new();
                let walker = WalkDir::new(dir.join(m.manifest_dir()))
                    .min_depth(1)
                    .max_depth(usize::MAX)
                    .follow_links(false)
                    .same_file_system(true)
                    .contents_first(true);

                for entry in walker {
                    let entry = entry?;
                    let entry_path = Utf8PathBuf::try_from(
                        entry
                            .path()
                            .strip_prefix(dir)?
                            .strip_prefix(m.manifest_dir())?
                            .to_path_buf(),
                    )?;

                    files.push(entry_path.into());
                    // dbg!(entry_path);
                }

                Ok(files)
            }
            Self::Label(..) => Ok(vec![]),
        }
    }
    pub fn disabled_files(&self) -> Result<Vec<InstallFile>> {
        match self {
            Self::Data(.., m) => m.disabled_files(),
            Self::Label(..) | Self::Custom(..) => Ok(vec![]),
        }
    }
    pub fn disable_file(&mut self, name: &str) -> Result<bool> {
        match self {
            Self::Data(.., m) => m.disable_file(name),
            Self::Label(..) | Self::Custom(..) => Ok(false),
        }
    }
    pub fn remove(&self, cache_dir: &Utf8Path) -> Result<()> {
        match self {
            Self::Custom(.., m) | Self::Data(.., m) => m.remove(cache_dir),
            Self::Label(.., m) => {
                let mut path = Utf8PathBuf::from(cache_dir).join(m.manifest_dir());
                path.set_extension(MANIFEST_EXTENTION);
                remove_file(&path)?;
                Ok(())
            }
        }
    }
    pub fn find_config_files(&self, extension: Option<&str>) -> Result<Vec<Utf8PathBuf>> {
        match self {
            Self::Data(..) | Self::Custom(..) => {
                let mut config_files = Vec::new();

                let ext_vec = if let Some(ext) = extension {
                    vec![ext]
                } else {
                    vec!["ini", "json", "yaml", "xml", "config", "toml"]
                };

                for f in self.origin_files()? {
                    if let Some(file_ext) = f.extension() {
                        let file_ext = file_ext.to_string();

                        if ext_vec.contains(&file_ext.as_str()) {
                            config_files.push(f);
                        }
                    }
                }
                Ok(config_files)
            }
            Self::Label(..) => Ok(vec![]),
        }
    }
    fn write(&self) -> Result<()> {
        match self {
            Self::Label(dir, m) | Self::Custom(dir, m) | Self::Data(dir, m) => {
                m.write_manifest(dir)?;
            }
        }
        Ok(())
    }
    fn set_enabled(&mut self) -> Result<()> {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(_, m) => {
                m.set_enabled();
            }
        }
        self.write()
    }
    fn set_disabled(&mut self) -> Result<()> {
        match self {
            Self::Label(.., m) | Self::Custom(.., m) | Self::Data(_, m) => {
                m.set_disabled();
            }
        }
        self.write()
    }
}
impl TryFrom<Utf8PathBuf> for Mod {
    type Error = Error;

    fn try_from(mut path: Utf8PathBuf) -> std::result::Result<Self, Self::Error> {
        let ext = path
            .extension()
            .map(|ext| ext.to_owned())
            .unwrap_or("none".to_owned());
        if ext != MANIFEST_EXTENTION {
            path.set_extension(MANIFEST_EXTENTION);
        }

        if let Ok(file) = File::open(&path) {
            if let Ok(manifest) = Manifest::try_from(file) {
                return Ok(match manifest.mod_kind() {
                    ModKind::FoMod | ModKind::Loader | ModKind::Data => {
                        Mod::Data(path.parent().unwrap().to_path_buf(), manifest)
                    }
                    ModKind::Custom => Mod::Custom(path.parent().unwrap().to_path_buf(), manifest),
                    ModKind::Label => Mod::Label(path.parent().unwrap().to_path_buf(), manifest),
                });
            }
        }

        todo!()
    }
}

pub trait GatherModList {
    fn gather_mods(cache_dir: &Utf8Path) -> Result<Vec<Mod>>;
}

impl GatherModList for Vec<Mod> {
    fn gather_mods(cache_dir: &Utf8Path) -> Result<Vec<Mod>> {
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
                    .eq(MANIFEST_EXTENTION)
                {
                    mod_list.push(Mod::try_from(Utf8PathBuf::try_from(
                        entry.path().to_path_buf(),
                    )?)?);
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
impl ModList for Vec<Mod> {
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
impl ModList for &mut [Mod] {
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
                let bkp_destination = destination.with_extension(format!(
                    "{}.{}",
                    destination.extension().unwrap_or_default(),
                    BACKUP_EXTENTION,
                ));
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

impl FindInModList for Vec<Mod> {
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
impl FindInModList for &[Mod] {
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
