use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::install_file::InstallFile;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoaderManifest {
    dll: Utf8PathBuf,
    exe: Utf8PathBuf,
}
impl LoaderManifest {
    pub fn new(files: Vec<InstallFile>) -> Self {
        //TODO fix unwraps
        let exe = files
            .iter()
            .find_map(|isf| {
                if isf.source().extension().unwrap_or_default().eq("exe") {
                    Some(isf.source())
                } else {
                    None
                }
            })
            .unwrap()
            .to_path_buf();
        let dll = files
            .iter()
            .find_map(|isf| {
                if isf.source().extension().unwrap_or_default().eq("exe") {
                    Some(isf.source())
                } else {
                    None
                }
            })
            .unwrap()
            .to_path_buf();

        Self { exe, dll }
    }
    pub fn files(
        &self,
        _cache_dir: &Utf8Path,
        manifest_dir: &Utf8Path,
    ) -> Result<Vec<InstallFile>> {
        Ok(vec![
            InstallFile::from(manifest_dir.join(&self.dll)),
            InstallFile::from(manifest_dir.join(&self.exe)),
        ])
    }
}
