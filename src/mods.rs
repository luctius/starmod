use std::{
    ffi::OsString,
    fmt::Display,
    fs::{read_link, remove_dir, remove_file, rename, DirBuilder, File},
    path::{Path, PathBuf},
};

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{
    installers::{
        custom::create_custom_manifest,
        data::create_data_manifest,
        fomod::{create_fomod_manifest, FOMOD_INFO_FILE, FOMOD_MODCONFIG_FILE},
        loader::create_loader_manifest,
        InstallerError,
    },
    manifest::{InstallFile, Manifest, ModState, MANIFEST_EXTENTION},
};

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
    // Label,
}
impl ModKind {
    pub fn detect_mod_type(cache_dir: &Path, name: &Path) -> Result<Self> {
        let archive_dir = PathBuf::from(cache_dir).join(name);

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
    pub fn create_mod(self, cache_dir: &Path, name: &Path) -> Result<Mod> {
        let md = match self {
            Self::FoMod => Mod::Data(
                cache_dir.to_path_buf(),
                create_fomod_manifest(self, cache_dir, name)?,
            ),
            Self::Loader => Mod::Data(
                cache_dir.to_path_buf(),
                create_loader_manifest(self, cache_dir, name)?,
            ),
            Self::Custom => Mod::Data(
                cache_dir.to_path_buf(),
                create_custom_manifest(self, cache_dir, name)?,
            ),
            Self::Data => Mod::Data(cache_dir.to_path_buf(), {
                let manifest_dir = cache_dir.join(name);
                let mut data_path = None;
                let walker = WalkDir::new(&manifest_dir)
                    .min_depth(1)
                    .max_depth(2)
                    .follow_links(false)
                    .same_file_system(true)
                    .contents_first(true);

                for entry in walker {
                    let entry = entry?;
                    let entry_path = entry.path();
                    if entry_path.is_dir()
                        && entry.path().file_name().unwrap() == OsString::from("data")
                    {
                        if data_path.is_none() {
                            let entry_path = entry_path.to_path_buf();
                            data_path = Some(entry_path.strip_prefix(&cache_dir)?.to_path_buf());
                        } else {
                            Err(InstallerError::MultipleDataDirectories(
                                name.to_string_lossy().to_string(),
                            ))?;
                        }
                    }
                }

                if data_path.is_none() {
                    let walker = WalkDir::new(&manifest_dir)
                        .min_depth(3)
                        .max_depth(5)
                        .follow_links(false)
                        .same_file_system(true)
                        .contents_first(true);

                    for entry in walker {
                        let entry = entry?;
                        let entry_path = entry.path();
                        if entry_path.is_dir()
                            && entry.path().file_name().unwrap() == OsString::from("data")
                        {
                            if data_path.is_none() {
                                data_path = Some(
                                    entry_path
                                        .to_path_buf()
                                        .strip_prefix(&cache_dir)?
                                        .to_path_buf(),
                                );
                            } else {
                                Err(InstallerError::MultipleDataDirectories(
                                    name.to_string_lossy().to_string(),
                                ))?;
                            }
                        }
                    }
                }

                create_data_manifest(
                    self,
                    cache_dir,
                    name,
                    data_path
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                        .as_str(),
                )
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
            // Self::Label => f.write_str("Label"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mod {
    // Goes into Data
    Data(PathBuf, Manifest),
    // Virtual mod to better organise the list
    // Label(Manifest),
}
//TODO let mods also contain the cache_dir
//so they can write the manifest file on each chance.
impl Mod {
    pub fn kind(&self) -> ModKind {
        match self {
            Self::Data(.., m) => m.mod_kind(),
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Data(.., m) => m.name(),
        }
    }
    pub fn set_name(&mut self, name: String) -> Result<()> {
        match self {
            Self::Data(_, m) => {
                m.set_name(name);
            }
        }
        self.write()
    }
    pub fn priority(&self) -> isize {
        match self {
            Self::Data(.., m) => m.priority(),
        }
    }
    pub fn set_priority(&mut self, prio: isize) -> Result<()> {
        match self {
            Self::Data(_, m) => {
                m.set_priority(prio);
            }
        }
        self.write()
    }
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Data(.., m) => m.mod_state().is_enabled(),
        }
    }
    pub fn mod_state(&self) -> ModState {
        match self {
            Self::Data(.., m) => m.mod_state(),
        }
    }
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Data(.., m) => m.version(),
        }
    }
    pub fn nexus_id(&self) -> Option<u32> {
        match self {
            Self::Data(.., m) => m.nexus_id(),
        }
    }
    pub fn enable(&mut self, cache_dir: &Path, game_dir: &Path) -> Result<()> {
        if self.is_enabled() {
            return Ok(());
        }
        if self.priority() < 0 {
            self.disable(cache_dir, game_dir)?;
            return Ok(());
        }

        log::trace!("Enabling {}", self.name());

        match self {
            //TODO use cache_dir from mod
            Self::Data(_, _) => {
                let cache_dir = PathBuf::from(cache_dir);
                let game_dir = PathBuf::from(game_dir);

                for (of, df) in self.origin_files().iter().zip(self.dest_files().iter()) {
                    let origin = {
                        let mut cache_dir = cache_dir.clone();
                        cache_dir.push(of);
                        cache_dir
                    };

                    let destination = {
                        let mut game_dir = game_dir.clone();
                        game_dir.push(PathBuf::from(df));
                        game_dir
                    };

                    //create intermediate directories
                    DirBuilder::new()
                        .recursive(true)
                        .create(destination.parent().unwrap())?;

                    // Remove existing symlinks which point back to our archive dir
                    // This ensures that the last mod wins, but we should do conflict
                    // detection and resolution before this, so we can inform the user.
                    if destination.is_symlink() {
                        let target = read_link(&destination)?;

                        if target.starts_with(&cache_dir) {
                            remove_file(&destination)?;
                            log::debug!(
                                "overrule {} ({} > {})",
                                destination.display(),
                                origin.display(),
                                target.display()
                            );
                        } else {
                            let bkp_destination = destination.with_file_name(format!(
                                "{}.starmod_bkp",
                                destination
                                    .extension()
                                    .map(|s| s.to_str())
                                    .flatten()
                                    .unwrap_or_default()
                            ));
                            log::info!(
                                "renaming foreign file from {} -> {}",
                                destination.display(),
                                bkp_destination.display()
                            );
                            rename(&destination, bkp_destination)?;
                        }
                    }

                    std::os::unix::fs::symlink(&origin, &destination)?;

                    log::trace!("link {} to {}", origin.display(), destination.display());
                }
            }
        }

        self.set_enabled()?;
        log::trace!("Enabled {}", self.name());

        Ok(())
    }
    pub fn disable(&mut self, cache_dir: &Path, game_dir: &Path) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        log::trace!("Disabling {}", self.name());

        let cache_dir = PathBuf::from(cache_dir);
        let game_dir = PathBuf::from(game_dir);

        for (of, df) in self.origin_files().iter().zip(self.dest_files().iter()) {
            let origin = {
                let mut cache_dir = cache_dir.clone();
                cache_dir.push(of);
                cache_dir
            };
            let destination = {
                let mut game_dir = game_dir.clone();
                game_dir.push(PathBuf::from(df));
                game_dir
            };

            if destination.is_file()
                && destination.is_symlink()
                && origin == read_link(&destination)?
            {
                remove_file(&destination)?;
                log::trace!("removed {} -> {}", destination.display(), origin.display());

                //TODO: move backup file back in place
            } else {
                log::debug!("passing-over {}", destination.display());
            }
        }

        //TODO: this could be optimised
        // right now it will after every disable try to delete
        // all directories in the game dir who are empty.
        let walker = WalkDir::new(&game_dir)
            .min_depth(1)
            .max_depth(usize::MAX)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(false);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                let _ = remove_dir(entry_path);
            }
        }

        self.set_disabled()?;
        log::trace!("Disabled {}", self.name());

        Ok(())
    }
    pub fn manifest_dir(&self) -> &Path {
        match self {
            Self::Data(.., m) => m.manifest_dir(),
        }
    }
    pub fn origin_files(&self) -> Vec<PathBuf> {
        match self {
            Self::Data(.., m) => m.origin_files(),
        }
    }
    pub fn dest_files(&self) -> Vec<String> {
        match self {
            Self::Data(.., m) => m.dest_files(),
        }
    }
    pub fn files(&self) -> &[InstallFile] {
        match self {
            Self::Data(.., m) => m.files(),
        }
    }
    pub fn disabled_files(&self) -> &[InstallFile] {
        match self {
            Self::Data(.., m) => m.disabled_files(),
        }
    }
    pub fn remove(&self, cache_dir: &Path) -> Result<()> {
        match self {
            Self::Data(.., m) => m.remove(cache_dir),
        }
    }
    pub fn find_config_files(&self, extension: Option<&str>) -> Vec<PathBuf> {
        match self {
            Self::Data(.., m) => m.find_config_files(extension),
        }
    }
    fn write(&self) -> Result<()> {
        match self {
            Self::Data(dir, m) => {
                m.write_manifest(dir)?;
            }
        }
        Ok(())
    }
    fn set_enabled(&mut self) -> Result<()> {
        match self {
            Self::Data(_, m) => {
                m.set_enabled();
            }
        }
        self.write()
    }
    fn set_disabled(&mut self) -> Result<()> {
        match self {
            Self::Data(_, m) => {
                m.set_disabled();
            }
        }
        self.write()
    }
}
impl TryFrom<PathBuf> for Mod {
    type Error = Error;

    fn try_from(mut path: PathBuf) -> std::result::Result<Self, Self::Error> {
        let ext = path
            .extension()
            .map(|ext| ext.to_str().to_owned())
            .flatten()
            .unwrap_or("none");
        if ext != MANIFEST_EXTENTION {
            path.set_extension(MANIFEST_EXTENTION);
        }

        if let Ok(file) = File::open(&path) {
            if let Ok(manifest) = Manifest::try_from(file) {
                return Ok(match manifest.mod_kind() {
                    ModKind::FoMod | ModKind::Loader | ModKind::Custom | ModKind::Data => {
                        Mod::Data(path.parent().unwrap().to_path_buf(), manifest)
                    }
                });
            }
        }

        todo!()
    }
}