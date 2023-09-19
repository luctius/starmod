use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{
    installers::{
        data::create_data_manifest,
        fomod::{create_fomod_manifest, FOMOD_INFO_FILE, FOMOD_MODCONFIG_FILE},
        plugin::create_plugin_manifest,
    },
    manifest::Manifest,
};

// use FOMOD_INFO_FILE;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModType {
    // Goes into Data
    DataMod,
    //Installer
    FoMod,
    //Goes into the root dir
    Plugin,
}
impl ModType {
    pub fn create_manifest(self, cache_dir: &Path, name: &Path) -> Result<Manifest> {
        match self {
            Self::DataMod => create_data_manifest(self, cache_dir, name),
            Self::FoMod => create_fomod_manifest(self, cache_dir, name),
            Self::Plugin => create_plugin_manifest(self, cache_dir, name),
        }
    }
    pub fn detect_mod_type(cache_dir: &Path, name: &Path) -> Result<Self> {
        let mut archive_dir = PathBuf::from(cache_dir);
        archive_dir.push(name);

        let walker = WalkDir::new(&archive_dir)
            .min_depth(1)
            .max_depth(2)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(false);

        let mut info = false;
        let mut config = false;

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if let Ok(p) = entry_path.strip_prefix(&archive_dir) {
                if p.to_string_lossy().to_string() == FOMOD_INFO_FILE {
                    info = true;
                }
            }
            if let Ok(p) = entry_path.strip_prefix(&archive_dir) {
                if p.to_string_lossy().to_string() == FOMOD_MODCONFIG_FILE {
                    config = true;
                }
            }

            if info && config {
                return Ok(Self::FoMod);
            }
        }

        let walker = WalkDir::new(archive_dir)
            .min_depth(1)
            .max_depth(3)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(true);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if let Some(ext) = entry_path.extension() {
                if ext == "dll" {
                    return Ok(Self::Plugin);
                }
            }
        }

        Ok(Self::DataMod)
    }
}
impl Display for ModType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataMod => f.write_str("Data"),
            Self::FoMod => f.write_str("FoMod"),
            Self::Plugin => f.write_str("Plugin"),
        }
    }
}
