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
    pub fn files(&self, _cache_dir: &Utf8Path) -> Vec<InstallFile> {
        self.files.clone()
    }
    pub fn disabled_files(&self) -> Vec<InstallFile> {
        self.disabled_files.clone()
    }
    pub fn disable_file(&mut self, name: &str) -> bool {
        if let Some((idx, _isf)) = self.files.iter().enumerate().find(|(_, isf)| {
            if isf.source().to_string().eq(name) {
                true
            } else {
                isf.source().file_name().unwrap_or_default().eq(name)
            }
        }) {
            self.disabled_files.push(self.files.remove(idx));
            true
        } else {
            false
        }
    }
    pub fn enable_file(&mut self, name: &str) -> bool {
        if let Some((idx, _isf)) = self.disabled_files.iter().enumerate().find(|(_, isf)| {
            if isf.source().to_string().eq(name) {
                true
            } else {
                isf.source().file_name().unwrap_or_default().eq(name)
            }
        }) {
            self.files.push(self.disabled_files.remove(idx));
            true
        } else {
            false
        }
    }
}
