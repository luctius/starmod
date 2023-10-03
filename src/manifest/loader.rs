use anyhow::Result;
use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use super::install_file::InstallFile;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoaderManifest {
    dll: InstallFile,
    exe: InstallFile,
}
impl LoaderManifest {
    pub fn new(files: Vec<InstallFile>) -> Self {
        //TODO fix unwraps
        let exe = files
            .iter()
            .find_map(|isf| {
                if isf.source().extension().unwrap_or_default().eq("exe") {
                    Some(isf)
                } else {
                    None
                }
            })
            .unwrap()
            .clone();
        let dll = files
            .iter()
            .find_map(|isf| {
                if isf.source().extension().unwrap_or_default().eq("dll") {
                    Some(isf)
                } else {
                    None
                }
            })
            .unwrap()
            .clone();

        Self { exe, dll }
    }
    pub fn files(&self, _cache_dir: &Utf8Path) -> Result<Vec<InstallFile>> {
        Ok(vec![self.dll.clone(), self.exe.clone()])
    }
}
