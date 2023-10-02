use anyhow::Result;
use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use super::install_file::InstallFile;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DataManifest {
    files: Vec<InstallFile>,
    disabled_files: Vec<InstallFile>,
}
impl DataManifest {
    pub fn new(files: Vec<InstallFile>, disabled_files: Vec<InstallFile>) -> Self {
        Self {
            files,
            disabled_files,
        }
    }
    pub fn files(
        &self,
        _cache_dir: &Utf8Path,
        _manifest_dir: &Utf8Path,
    ) -> Result<Vec<InstallFile>> {
        Ok(self.files.clone())
    }
    pub fn disabled_files(&self) -> Vec<InstallFile> {
        self.disabled_files.clone()
    }
}
