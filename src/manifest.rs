use camino::{Utf8Path, Utf8PathBuf};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::{remove_dir_all, remove_file, File},
    io::{BufReader, Read, Write},
};

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::{dmodman::DMODMAN_EXTENTION, mods::ModKind};

mod custom;
mod data;
mod loader;

pub mod install_file;
pub mod mod_state;

use install_file::InstallFile;
use mod_state::ModState;

use self::{data::DataManifest, loader::LoaderManifest};

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
    ) -> Self {
        match mod_kind {
            ModKind::FoMod | ModKind::Data => Self::Data(DataManifest::new(files, disabled_files)),
            ModKind::Loader => Self::Loader(LoaderManifest::new(files)),
            ModKind::Label => todo!(),
            ModKind::Custom => Self::Custom(custom::CustomManifest {}),
        }
    }
    pub fn files(&self, cache_dir: &Utf8Path, manifest_dir: &Utf8Path) -> Result<Vec<InstallFile>> {
        match self {
            Self::Data(d) => d.files(cache_dir, manifest_dir),
            Self::Loader(l) => l.files(cache_dir, manifest_dir),
            Self::Custom(c) => c.files(cache_dir, manifest_dir),
        }
    }
    pub fn dest_files(&self, cache_dir: &Utf8Path, manifest_dir: &Utf8Path) -> Result<Vec<String>> {
        let files = self.files(cache_dir, manifest_dir)?;
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
        let files = self.files(cache_dir, manifest_dir)?;
        let mut origin_files = Vec::with_capacity(files.len());
        for f in &files {
            let origin = f.source();
            let origin = manifest_dir.to_path_buf().join(origin);
            origin_files.push(origin)
        }
        Ok(origin_files)
    }
    pub fn disabled_files(&self) -> Result<Vec<InstallFile>> {
        match self {
            Self::Data(d) => Ok(d.disabled_files()),

            //TODO: does it make sense disabling files in these?
            Self::Loader(_l) => Ok(vec![]),
            Self::Custom(_c) => Ok(vec![]),
        }
    }
    pub fn disable_file(&mut self, name: &str) -> Result<bool> {
        match self {
            Self::Data(d) => d.disable_file(name),

            //TODO: does it make sense disabling files in these?
            Self::Loader(_l) => Ok(false),
            Self::Custom(_c) => Ok(false),
        }
    }
}

pub const MANIFEST_EXTENTION: &'static str = "ron";

//TODO more info about the mod, description, authors, version, etc

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    #[serde(skip_serializing, default)]
    cache_dir: Utf8PathBuf,
    manifest_dir: Utf8PathBuf,
    name: String,
    version: Option<String>,
    nexus_id: Option<u32>,
    mod_state: ModState,
    mod_kind: ModKind,
    priority: isize,
    internal: ManifestInternal,
}
impl Manifest {
    pub fn new(
        cache_dir: &Utf8Path,
        manifest_dir: &Utf8Path,
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
            name,
            nexus_id,
            version,
            mod_state: ModState::Disabled,
            priority: 0,
            mod_kind,
            internal: ManifestInternal::new(mod_kind, files, disabled_files),
        }
    }
    pub fn set_priority(&mut self, priority: isize) {
        self.priority = priority;
    }
    pub fn from_file(cache_dir: &Utf8Path, archive: &Utf8Path) -> Result<Self> {
        let manifest_file = Utf8PathBuf::from(cache_dir)
            .join(archive)
            .with_extension(MANIFEST_EXTENTION);

        Self::try_from(manifest_file.as_path())
    }

    pub fn write_manifest(&self, cache_dir: &Utf8Path) -> Result<()> {
        let path = Utf8PathBuf::from(cache_dir)
            .join(self.manifest_dir.file_stem().unwrap())
            .with_extension(MANIFEST_EXTENTION);

        // if path.exists() {
        //     log::trace!("Removing manifest file '{}' before update.", path.display());
        //     remove_file(&path)?;
        // }

        let mut file = File::create(&path)?;

        let serialized =
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        log::trace!("Updating manifest file '{}'.", path);
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
    pub fn remove(&self, cache_dir: &Utf8Path) -> Result<()> {
        let mut path = Utf8PathBuf::from(cache_dir).join(&self.manifest_dir);
        remove_dir_all(&path)?;
        path.set_extension(MANIFEST_EXTENTION);
        remove_file(&path)?;
        path.set_extension(DMODMAN_EXTENTION);
        remove_file(&path)?;
        Ok(())
    }
    pub fn is_valid(&self) -> bool {
        //TODO: checks to validate the manifest file
        true
    }
    pub fn manifest_dir(&self) -> &Utf8Path {
        &self.manifest_dir
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: String) {
        self.name = name
    }
    pub fn set_enabled(&mut self) {
        self.mod_state = ModState::Enabled;
    }
    pub fn set_disabled(&mut self) {
        self.mod_state = ModState::Disabled;
    }
    pub fn nexus_id(&self) -> Option<u32> {
        self.nexus_id
    }
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
    pub fn mod_state(&self) -> ModState {
        self.mod_state
    }
    pub fn mod_kind(&self) -> ModKind {
        self.mod_kind
    }
    pub fn files(&self) -> Result<Vec<InstallFile>> {
        self.internal.files(&self.cache_dir, &self.manifest_dir)
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
    pub fn dest_files(&self) -> Result<Vec<String>> {
        self.internal
            .dest_files(&self.cache_dir, &self.manifest_dir)
    }
    pub fn origin_files(&self) -> Result<Vec<Utf8PathBuf>> {
        self.internal
            .origin_files(&self.cache_dir, &self.manifest_dir)
    }
    pub fn disabled_files(&self) -> Result<Vec<InstallFile>> {
        self.internal.disabled_files()
    }
    pub fn disable_file(&mut self, name: &str) -> Result<bool> {
        self.internal.disable_file(name)
    }
    pub fn priority(&self) -> isize {
        self.priority
    }
}
impl<'a> TryFrom<&'a Utf8Path> for Manifest {
    type Error = Error;

    fn try_from(file_path: &Utf8Path) -> std::result::Result<Self, Self::Error> {
        let file = File::open(file_path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let mut manifest: Manifest = ron::from_str(&contents)?;
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
