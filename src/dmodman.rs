use camino::{Utf8Path, Utf8PathBuf};
use std::{
    fs::File,
    io::{BufReader, Read},
};
use walkdir::WalkDir;

use anyhow::{Error, Result};
use serde::Deserialize;
use xdg::BaseDirectories;

pub const DMODMAN_EXTENSION: &str = "dmodman";

#[derive(Clone, Debug, Deserialize)]
pub struct DmodMan {
    game: String,
    file_name: String,
    mod_id: u32,
    #[allow(unused)]
    file_id: u64,
    #[allow(unused)]
    update_status: UpdateStatus,
}
impl DmodMan {
    pub fn gather_list(cache_dir: &Utf8Path) -> Result<Vec<Self>> {
        let mut dmodman_list = Vec::new();
        let walker = WalkDir::new(cache_dir)
            .min_depth(1)
            .max_depth(2)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = Utf8PathBuf::try_from(entry.path().to_path_buf())?;

            if entry_path.extension().unwrap_or_default() == "json" {
                dmodman_list.push(Self::try_from(entry_path.as_path())?);
            }
        }

        Ok(dmodman_list)
    }
    pub fn file_name(&self) -> &str {
        &self.file_name
    }
    pub fn name(&self) -> String {
        self.file_name
            .to_lowercase()
            .split_once(&format!("-{}-", self.mod_id))
            .map(|(name, _rest)| name.to_owned())
            .unwrap()
    }
    pub const fn mod_id(&self) -> u32 {
        self.mod_id
    }
    #[allow(unused)]
    pub fn timestamp(&self) -> Option<String> {
        self.file_name
            .to_lowercase()
            .split_once(&format!("-{}-", self.mod_id))
            .map(|(_name, rest)| rest)
            .and_then(|s| s.rsplit_once('.'))
            .map(|(rest, _ext)| rest)
            .and_then(|s| s.rsplit_once('-'))
            .map(|(_version, timestamp)| timestamp.to_owned())
    }
    pub fn version(&self) -> Option<String> {
        self.file_name
            .to_lowercase()
            .split_once(&format!("-{}-", self.mod_id))
            .map(|(_name, rest)| rest)
            .and_then(|s| s.rsplit_once('.'))
            .map(|(rest, _ext)| rest)
            .and_then(|s| s.rsplit_once('-'))
            .map(|(version, _timestamp)| version)
            .map(|s| s.replace('-', "."))
    }
}
impl TryFrom<File> for DmodMan {
    type Error = serde_json::Error;

    fn try_from(file: File) -> Result<Self, Self::Error> {
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
    }
}
impl TryFrom<&Utf8Path> for DmodMan {
    type Error = Error;

    fn try_from(path: &Utf8Path) -> Result<Self, Self::Error> {
        let dmodman = Self::try_from(File::open(path)?)?;
        Ok(dmodman)
    }
}
impl TryFrom<Utf8PathBuf> for DmodMan {
    type Error = Error;

    fn try_from(path: Utf8PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(path.as_path())
    }
}
impl PartialEq for DmodMan {
    fn eq(&self, other: &Self) -> bool {
        self.game == other.game && self.mod_id == other.mod_id && self.name() == other.name()
    }
}
impl Eq for DmodMan {}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub enum UpdateStatus {
    UpToDate(u64),     // time of your newest file,
    HasNewFile(u64),   // time of your newest file
    OutOfDate(u64),    // time of your newest file
    IgnoredUntil(u64), // time of latest file in update list
}

impl UpdateStatus {
    #[allow(unused)]
    pub const fn time(&self) -> u64 {
        match self {
            Self::UpToDate(t)
            | Self::HasNewFile(t)
            | Self::OutOfDate(t)
            | Self::IgnoredUntil(t) => *t,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct DModManConfig {
    download_dir: Option<String>,
    profile: Option<String>,
    #[allow(unused)]
    api_key: Option<String>,
}
impl DModManConfig {
    pub fn read() -> Option<Self> {
        let path = Self::path().ok()?;
        let mut contents = String::new();
        let mut f = File::open(path).ok()?;
        f.read_to_string(&mut contents).ok()?;
        toml::from_str(&contents).ok()
    }
    pub fn download_dir(&self) -> Option<Utf8PathBuf> {
        let ddir = self.download_dir.as_deref()?;
        let mut ddir = Utf8PathBuf::from(ddir);

        if let Some(profile) = self.profile.as_deref() {
            ddir.push(profile);
        }
        Some(ddir)
    }
    pub fn path() -> Result<Utf8PathBuf> {
        let xdg_base = BaseDirectories::with_prefix("dmodman")?;
        Ok(Utf8PathBuf::try_from(
            xdg_base.get_config_file("config.toml"),
        )?)
    }
}

pub trait FindInDmodManList {
    fn find_dmodman_by_name(&self, mod_name: &str) -> Option<usize>;
    fn find_dmodman_by_id(&self, nexus_id: u32) -> Option<usize>;
}
impl FindInDmodManList for Vec<DmodMan> {
    fn find_dmodman_by_name(&self, mod_name: &str) -> Option<usize> {
        self.as_slice().find_dmodman_by_name(mod_name)
    }
    fn find_dmodman_by_id(&self, nexus_id: u32) -> Option<usize> {
        self.as_slice().find_dmodman_by_id(nexus_id)
    }
}
impl FindInDmodManList for &[DmodMan] {
    fn find_dmodman_by_name(&self, mod_name: &str) -> Option<usize> {
        self.iter()
            .enumerate()
            .find(|(_, dm)| dm.name().eq(mod_name))
            .map(|(idx, _)| idx)
    }
    fn find_dmodman_by_id(&self, nexus_id: u32) -> Option<usize> {
        self.iter()
            .enumerate()
            .find(|(_, dm)| dm.mod_id().eq(&nexus_id))
            .map(|(idx, _)| idx)
    }
}
