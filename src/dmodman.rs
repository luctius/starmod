use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use anyhow::{Error, Result};
use serde::Deserialize;
use xdg::BaseDirectories;

pub const DMODMAN_EXTENTION: &'static str = "dmodman";

#[derive(Clone, Debug, Deserialize)]
pub struct DmodMan {
    game: String,
    file_name: String,
    mod_id: u32,
    file_id: u64,
    update_status: UpdateStatus,
}
impl DmodMan {
    pub fn name(&self) -> String {
        self.file_name
            .to_lowercase()
            .split_once(&format!("-{}-", self.mod_id))
            .map(|(name, _rest)| name.to_owned())
            .unwrap()
    }
    pub fn mod_id(&self) -> u32 {
        self.mod_id
    }
    pub fn timestamp(&self) -> Option<String> {
        self.file_name
            .to_lowercase()
            .split_once(&format!("-{}-", self.mod_id))
            .map(|(_name, rest)| rest)
            .map(|s| s.rsplit_once("."))
            .flatten()
            .map(|(rest, _ext)| rest)
            .map(|s| s.rsplit_once("-"))
            .flatten()
            .map(|(_version, timestamp)| timestamp.to_owned())
    }
    pub fn version(&self) -> Option<String> {
        self.file_name
            .to_lowercase()
            .split_once(&format!("-{}-", self.mod_id))
            .map(|(_name, rest)| rest)
            .map(|s| s.rsplit_once("."))
            .flatten()
            .map(|(rest, _ext)| rest)
            .map(|s| s.rsplit_once("-"))
            .flatten()
            .map(|(version, _timestamp)| version)
            .map(|s| s.replace("-", "."))
    }
}
impl TryFrom<File> for DmodMan {
    type Error = serde_json::Error;

    fn try_from(file: File) -> Result<Self, Self::Error> {
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
    }
}
impl TryFrom<&Path> for DmodMan {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let dmodman = Self::try_from(File::open(path)?)?;
        Ok(dmodman)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub enum UpdateStatus {
    UpToDate(u64),     // time of your newest file,
    HasNewFile(u64),   // time of your newest file
    OutOfDate(u64),    // time of your newest file
    IgnoredUntil(u64), // time of latest file in update list
}

impl UpdateStatus {
    pub fn time(&self) -> u64 {
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
    api_key: Option<String>,
}
impl DModManConfig {
    pub fn read() -> Option<Self> {
        let path = Self::path().ok()?;
        let mut contents = String::new();
        let mut f = File::open(&path).ok()?;
        f.read_to_string(&mut contents).ok()?;
        toml::from_str(&contents).ok()
    }
    pub fn download_dir(&self) -> Option<PathBuf> {
        let ddir = self.download_dir.as_deref()?;
        let mut ddir = PathBuf::from(ddir);

        if let Some(profile) = self.profile.as_deref() {
            ddir.push(profile)
        }
        Some(ddir)
    }
    pub fn path() -> Result<PathBuf> {
        let xdg_base = BaseDirectories::with_prefix("dmodman")?;
        Ok(xdg_base.get_config_file("config.toml"))
    }
}
