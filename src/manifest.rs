use camino::{Utf8Path, Utf8PathBuf};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::{remove_dir_all, remove_file, File},
    io::{BufReader, Read, Write},
};

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::{
    dmodman::{DmodMan, DMODMAN_EXTENSION},
    mods::ModKind,
    utils::AddExtension,
};

mod custom;
mod data;
mod loader;

pub mod install_file;
pub mod mod_state;

use install_file::InstallFile;
use mod_state::ModState;

use self::{data::DataManifest, loader::LoaderManifest};

pub const MANIFEST_EXTENSION: &str = "ron";

#[derive(Clone, Debug, Deserialize, Serialize)]
enum ManifestInternal {
    Data(data::DataManifest),
    Loader(loader::LoaderManifest),
    Custom(custom::CustomManifest),
}
impl ManifestInternal {
    pub fn new(
        mod_kind: ModKind,
        files: Vec<InstallFile>,
        disabled_files: Vec<InstallFile>,
        manifest_dir: &Utf8Path,
    ) -> Self {
        match mod_kind {
            ModKind::FoMod | ModKind::Data => Self::Data(DataManifest::new(files, disabled_files)),
            ModKind::Loader => Self::Loader(LoaderManifest::new(&files)),
            ModKind::Custom => Self::Custom(custom::CustomManifest::new(manifest_dir)),
        }
    }
    pub fn files(&self, cache_dir: &Utf8Path) -> Result<Vec<InstallFile>> {
        match self {
            Self::Data(d) => Ok(d.files(cache_dir)),
            Self::Loader(l) => Ok(l.files(cache_dir)),
            Self::Custom(c) => c.files(cache_dir),
        }
    }
    pub fn dest_files(&self, cache_dir: &Utf8Path) -> Result<Vec<String>> {
        let files = self.files(cache_dir)?;
        let mut dest_files = Vec::with_capacity(files.len());
        for f in &files {
            dest_files.push(f.destination().to_string());
        }
        Ok(dest_files)
    }
    pub fn origin_files(
        &self,
        cache_dir: &Utf8Path,
        manifest_dir: &Utf8Path,
    ) -> Result<Vec<Utf8PathBuf>> {
        let files = self.files(cache_dir)?;
        let mut origin_files = Vec::with_capacity(files.len());
        for f in &files {
            let origin = f.source();
            let origin = manifest_dir.to_path_buf().join(origin);
            origin_files.push(origin);
        }
        Ok(origin_files)
    }
    pub fn disabled_files(&self) -> Vec<InstallFile> {
        match self {
            Self::Data(d) => d.disabled_files(),

            //TODO: does it make sense disabling files in these?
            Self::Loader(_l) => vec![],
            Self::Custom(_c) => vec![],
        }
    }
    pub fn disable_file(&mut self, name: &str) -> bool {
        match self {
            Self::Data(d) => d.disable_file(name),

            //TODO: does it make sense disabling files in these?
            Self::Loader(_l) => false,
            Self::Custom(_c) => false,
        }
    }
}

//TODO more info about the mod, description, authors, version, etc

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    #[serde(skip_serializing, default)]
    cache_dir: Utf8PathBuf,
    manifest_dir: Utf8PathBuf,
    bare_file_name: String,
    name: String,
    version: Option<String>,
    nexus_id: Option<u32>,
    mod_state: ModState,
    mod_kind: ModKind,
    priority: isize,
    internal: ManifestInternal,
    tags: Vec<String>,
}
impl Manifest {
    pub fn new(
        cache_dir: &Utf8Path,
        manifest_dir: &Utf8Path,
        bare_file_name: String,
        name: String,
        nexus_id: Option<u32>,
        version: Option<String>,
        files: Vec<InstallFile>,
        disabled_files: Vec<InstallFile>,
        mod_kind: ModKind,
    ) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
            manifest_dir: manifest_dir.to_path_buf(),
            bare_file_name,
            name,
            nexus_id,
            version,
            mod_state: ModState::Disabled,
            priority: 0,
            mod_kind,
            internal: ManifestInternal::new(mod_kind, files, disabled_files, manifest_dir),
            tags: Vec::new(), //TODO: shall we add modkind as a tag?
        }
    }
    pub fn set_priority(&mut self, priority: isize) -> Result<()> {
        self.priority = priority;
        self.write()
    }
    pub fn from_file(cache_dir: &Utf8Path, archive: &Utf8Path) -> Result<Self> {
        let manifest_file = Utf8PathBuf::from(cache_dir)
            .join(archive)
            .add_extension(MANIFEST_EXTENSION);

        Self::try_from(manifest_file.as_path())
    }

    pub fn write(&self) -> Result<()> {
        let path = Utf8PathBuf::from(self.cache_dir.as_path())
            .join(self.manifest_dir.as_path())
            .add_extension(MANIFEST_EXTENSION);

        if !path.exists() {
            log::trace!("Creating Manifest at '{}'", path);
        }
        let mut file = File::create(&path)?;

        let serialized =
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        log::trace!("Updating manifest file '{}'.", path);
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
    pub fn remove(&self) -> Result<()> {
        let path = self.cache_dir.join(&self.manifest_dir);
        remove_dir_all(&path)?;
        let manifest_file = path.add_extension(MANIFEST_EXTENSION);
        remove_file(&manifest_file)?;
        let dmodman_file = manifest_file.with_extension(DMODMAN_EXTENSION);
        remove_file(dmodman_file)?;
        Ok(())
    }
    pub const fn is_valid(&self) -> bool {
        //TODO: checks to validate the manifest file
        true
    }
    pub fn manifest_dir(&self) -> &Utf8Path {
        &self.manifest_dir
    }
    pub fn bare_file_name(&self) -> &str {
        &self.bare_file_name
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: String) -> Result<()> {
        self.name = name;
        self.write()
    }
    pub fn set_enabled(&mut self) -> Result<()> {
        self.mod_state = ModState::Enabled;
        self.write()
    }
    pub fn set_disabled(&mut self) -> Result<()> {
        self.mod_state = ModState::Disabled;
        self.write()
    }
    pub const fn nexus_id(&self) -> Option<u32> {
        self.nexus_id
    }
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
    pub const fn mod_state(&self) -> ModState {
        self.mod_state
    }
    pub fn files(&self) -> Result<Vec<InstallFile>> {
        self.internal.files(&self.cache_dir)
    }
    pub fn enlist_files(
        &self,
        conflict_list: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<InstallFile>> {
        let mut enlisted_files = Vec::new();

        for f in &self.files()? {
            if let Some(winners) = conflict_list.get(f.destination()) {
                if let Some(winner) = winners.last() {
                    if *winner == self.name() {
                        enlisted_files.push(InstallFile::new_raw(
                            self.manifest_dir().join(f.source()),
                            f.destination().to_owned(),
                        ));
                    }
                }
            } else {
                enlisted_files.push(InstallFile::new_raw(
                    self.manifest_dir().join(f.source()),
                    f.destination().to_owned(),
                ));
            }
        }

        Ok(enlisted_files)
    }
    pub fn dest_files(&self) -> Result<Vec<String>> {
        self.internal.dest_files(&self.cache_dir)
    }
    pub fn origin_files(&self) -> Result<Vec<Utf8PathBuf>> {
        self.internal
            .origin_files(&self.cache_dir, &self.manifest_dir)
    }
    pub fn disabled_files(&self) -> Vec<InstallFile> {
        self.internal.disabled_files()
    }
    pub fn disable_file(&mut self, name: &str) -> bool {
        self.internal.disable_file(name)
    }
    pub const fn priority(&self) -> isize {
        self.priority
    }
    pub fn find_config_files(&self, extension: Option<&str>) -> Result<Vec<Utf8PathBuf>> {
        let mut config_files = Vec::new();

        let ext_vec = extension.map_or_else(
            || vec!["ini", "json", "yaml", "xml", "config", "toml"],
            |ext| vec![ext],
        );

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
    pub const fn is_enabled(&self) -> bool {
        self.mod_state().is_enabled()
    }
    // #[allow(unused)]
    // pub const fn is_disabled(&self) -> bool {
    //     !self.mod_state().is_enabled()
    // }
    pub const fn kind(&self) -> ModKind {
        self.mod_kind
    }
    pub fn is_an_update(&self, dmodman: &DmodMan) -> bool {
        dmodman.name() == self.bare_file_name
            && dmodman.mod_id() == self.nexus_id.unwrap_or_default()
            && dmodman.version().unwrap_or_default() > self.version.clone().unwrap_or_default()
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    pub fn add_tag(&mut self, tag: &str) -> Result<bool> {
        let tag = tag.to_lowercase();
        if self.tags.contains(&tag) {
            Ok(false)
        } else {
            self.tags.push(tag);
            self.write().map(|()| true)
        }
    }
    pub fn remove_tag(&mut self, tag: &str) -> Result<bool> {
        let tag = tag.to_lowercase();

        if let Some(idx) = self
            .tags
            .iter()
            .enumerate()
            .find(|(_, t)| *t == &tag)
            .map(|(idx, _)| idx)
        {
            self.tags.swap_remove(idx);
            self.write().map(|()| true)
        } else {
            Ok(true)
        }
    }
}
impl<'a> TryFrom<&'a Utf8Path> for Manifest {
    type Error = Error;

    fn try_from(file_path: &Utf8Path) -> std::result::Result<Self, Self::Error> {
        let file = File::open(file_path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let mut manifest: Self = ron::from_str(&contents)?;
        manifest.cache_dir = file_path.parent().unwrap().to_path_buf();

        log::trace!("Opening manifest: {}", manifest.name());
        Ok(manifest)
    }
}
impl PartialOrd for Manifest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Manifest {
    fn cmp(&self, other: &Self) -> Ordering {
        //Order around priority or, if equal, around alfabethic order
        let o = self.priority().cmp(&other.priority());
        if o == Ordering::Equal {
            self.name().cmp(other.name())
        } else {
            o
        }
    }
}
impl PartialEq for Manifest {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
            && self.version.eq(&other.version)
            && self.nexus_id.eq(&other.nexus_id)
            && self.manifest_dir.eq(&other.manifest_dir)
            && self.mod_state.eq(&other.mod_state)
            && self.mod_kind.eq(&other.mod_kind)
    }
}
impl Eq for Manifest {}
