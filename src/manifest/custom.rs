use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::install_file::InstallFile;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomManifest {
    manifest_dir: Utf8PathBuf,
}
impl CustomManifest {
    pub fn new(manifest_dir: &Utf8Path) -> Self {
        Self {
            manifest_dir: manifest_dir.to_path_buf(),
        }
    }
    pub fn files(&self, cache_dir: &Utf8Path) -> Result<Vec<InstallFile>> {
        let dir = cache_dir.join(&self.manifest_dir);

        let mut files = Vec::new();
        let walker = WalkDir::new(&dir)
            .min_depth(1)
            .max_depth(usize::MAX)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = Utf8PathBuf::try_from(entry.path().strip_prefix(&dir)?.to_path_buf())?;

            if entry_path.is_file() {
                files.push(entry_path.into());
            }
            // dbg!(entry_path);
        }

        Ok(files)
    }
}
