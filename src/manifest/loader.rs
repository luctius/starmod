use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use super::install_file::InstallFile;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoaderManifest {
    dll: InstallFile,
    exe: InstallFile,
}
impl LoaderManifest {
    pub fn new(files: &[InstallFile]) -> Self {
        //TODO fix unwraps
        let exe = files
            .iter()
            .find(|isf| isf.source().extension().unwrap_or_default().eq("exe"))
            .unwrap()
            .clone();
        let dll = files
            .iter()
            .find(|isf| isf.source().extension().unwrap_or_default().eq("dll"))
            .unwrap()
            .clone();

        Self { dll, exe }
    }
    pub fn files(&self, _cache_dir: &Utf8Path) -> Vec<InstallFile> {
        vec![self.dll.clone(), self.exe.clone()]
    }
}
