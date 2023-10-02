use camino::{Utf8Path, Utf8PathBuf};
use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::installers::{DATA_DIR_NAME, TEXTURES_DIR_NAME};

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
