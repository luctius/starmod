use camino::{Utf8Path, Utf8PathBuf};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    fs::{remove_dir_all, remove_file, File},
    io::{BufReader, Read, Write},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    dmodman::DMODMAN_EXTENTION,
    installers::{DATA_DIR_NAME, TEXTURES_DIR_NAME},
    mods::ModKind,
};

//TODO: replace Utf8PathBuf with something that is ressilient to deserialisation of non-utf8 characters

pub const MANIFEST_EXTENTION: &'static str = "ron";

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModState {
    Enabled,
    Disabled,
}
impl ModState {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Enabled => true,
            Self::Disabled => false,
        }
    }
}
impl From<bool> for ModState {
    fn from(v: bool) -> Self {
        match v {
            true => Self::Enabled,
            false => Self::Disabled,
        }
    }
}
impl Display for ModState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModState::Enabled => f.write_str("Enabled"),
            ModState::Disabled => f.write_str("Disabled"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InstallFile {
    source: Utf8PathBuf,
    destination: String,
}
impl InstallFile {
    pub fn new(source: Utf8PathBuf, destination: String) -> Self {
        let destination = format!(
            "{}/{}",
            DATA_DIR_NAME,
            destination
                .as_str()
                .strip_prefix("data")
                .unwrap_or(destination.as_str())
                .to_lowercase()
        )
        .replace("//", "/")
        .replace("/textures/", &format!("/{}/", TEXTURES_DIR_NAME));

        log::trace!("New InstallFile: {} -> {}", source, destination);

        Self {
            source,
            destination,
        }
    }
    pub fn new_raw(source: Utf8PathBuf, destination: String) -> Self {
        log::trace!("New InstallFile: {} -> {}", source, destination);

        Self {
            source,
            destination,
        }
    }
    pub fn source(&self) -> &Utf8Path {
        &self.source
    }
    pub fn destination(&self) -> &str {
        &self.destination
    }
    pub fn enlist(mod_dir: &Utf8Path, source: &Utf8Path, destination: &str) -> Self {
        dbg!(destination);
        // let destination = dbg!(format!(
        //     "{}/{}",
        //     DATA_DIR_NAME,
        //     destination
        //         .strip_prefix("data")
        //         .unwrap_or(destination)
        //         .to_lowercase()
        // )
        // .replace("//", "/")
        // .replace("/textures/", &format!("/{}/", TEXTURES_DIR_NAME)));
        let destination = destination.to_owned();

        log::trace!("New InstallFile: {} -> {}", source, destination);

        Self {
            source: mod_dir.join(source),
            destination,
        }
    }
}
impl From<Utf8PathBuf> for InstallFile {
    fn from(pb: Utf8PathBuf) -> Self {
        Self::from(pb.as_path())
    }
}
impl From<&Utf8Path> for InstallFile {
    fn from(p: &Utf8Path) -> Self {
        let source = p.to_path_buf();
        let destination = format!("{}/{}", DATA_DIR_NAME, p.strip_prefix("data").unwrap_or(p))
            .replace("//", "/")
            .replace("/textures/", &format!("/{}/", TEXTURES_DIR_NAME));

        log::trace!("New InstallFile: {} -> {}", source, destination);
        Self {
            source,
            destination,
        }
    }
}
impl Ord for InstallFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source.cmp(&other.source)
    }
}
impl PartialOrd for InstallFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

//TODO more info about the mod, description, authors, version, etc

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    manifest_dir: Utf8PathBuf,
    name: String,
    version: Option<String>,
    nexus_id: Option<u32>,
    mod_state: ModState,
    mod_kind: ModKind,
    files: Vec<InstallFile>,
    disabled_files: Vec<InstallFile>,
    priority: isize,
}
impl Manifest {
    pub fn new(
        manifest_dir: &Utf8Path,
        name: String,
        nexus_id: Option<u32>,
        version: Option<String>,
        files: Vec<InstallFile>,
        disabled_files: Vec<InstallFile>,
        mod_kind: ModKind,
    ) -> Self {
        let s = Self {
            manifest_dir: manifest_dir.to_path_buf(),
            name,
            nexus_id,
            version,
            files,
            disabled_files,
            mod_state: ModState::Disabled,
            priority: 0,
            mod_kind,
        };
        s
    }
    pub fn set_priority(&mut self, priority: isize) {
        self.priority = priority;
    }
    pub fn from_file(cache_dir: &Utf8Path, archive: &Utf8Path) -> Result<Self> {
        let manifest_file = Utf8PathBuf::from(cache_dir)
            .join(archive)
            .with_extension(MANIFEST_EXTENTION);

        let file = File::open(manifest_file)?;
        Self::try_from(file)
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
    pub fn files(&self) -> &[InstallFile] {
        &self.files
    }
    pub fn dest_files(&self) -> Vec<String> {
        let mut dest_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            dest_files.push(f.destination.clone());
        }
        dest_files
    }
    pub fn origin_files(&self) -> Vec<Utf8PathBuf> {
        let mut origin_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let origin = f.source.as_path();
            let origin = self.manifest_dir.to_path_buf().join(origin);
            origin_files.push(origin)
        }
        origin_files
    }
    // pub fn enlist_files(&self, conflict_list: &HashMap<String, Vec<String>>) -> Vec<InstallFile> {
    //     let mut enlisted_files = Vec::new();

    //     for f in &self.files {
    //         if let Some(winners) = conflict_list.get(&f.destination) {
    //             if let Some(winner) = winners.last() {
    //                 if *winner == self.name {
    //                     enlisted_files.push(InstallFile {
    //                         source: self.manifest_dir.to_path_buf().join(f.source.clone()),
    //                         destination: f.destination.clone(),
    //                     });
    //                 }
    //             }
    //         } else {
    //             enlisted_files.push(InstallFile {
    //                 source: self.manifest_dir.to_path_buf().join(f.source.clone()),
    //                 destination: f.destination.clone(),
    //             });
    //         }
    //     }

    //     enlisted_files
    // }
    pub fn disabled_files(&self) -> &[InstallFile] {
        &self.disabled_files
    }
    pub fn priority(&self) -> isize {
        self.priority
    }
    // pub fn find_config_files(&self, ext: Option<&str>) -> Vec<Utf8PathBuf> {
    //     let mut config_files = Vec::new();

    //     let ext_vec = if let Some(ext) = ext {
    //         vec![ext]
    //     } else {
    //         vec!["ini", "json", "yaml", "xml", "config", "toml"]
    //     };

    //     for f in self.origin_files() {
    //         if let Some(file_ext) = f.extension() {
    //             let file_ext = file_ext.to_string();

    //             if ext_vec.contains(&file_ext.as_str()) {
    //                 config_files.push(f);
    //             }
    //         }
    //     }
    //     config_files
    // }
}
impl TryFrom<File> for Manifest {
    type Error = anyhow::Error;

    fn try_from(file: File) -> std::result::Result<Self, Self::Error> {
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let manifest: Manifest = ron::from_str(&contents)?;

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
        //Order around priority, or if equal around alfabethic order
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
