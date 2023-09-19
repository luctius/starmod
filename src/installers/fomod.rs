pub const FOMOD_INFO_FILE: &'static str = "fomod/info.xml";
pub const FOMOD_MODCONFIG_FILE: &'static str = "fomod/moduleconfig.xml";

use encoding_rs_io::DecodeReaderBytes;

use anyhow::Result;
use fomod::{Config, Info};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use crate::{
    manifest::{InstallFile, Manifest},
    mod_types::ModType,
};

pub enum FoModError {
    DependenciesNotMet,
}

pub fn create_fomod_manifest(mod_type: ModType, cache_dir: &Path, name: &Path) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut archive_dir = PathBuf::from(cache_dir);
    archive_dir.push(name);

    let mut config = archive_dir.clone();
    config.push(FOMOD_MODCONFIG_FILE);

    //Allow for names set by fomod
    let name = name.to_string_lossy().to_string();

    let info = {
        let mut info = archive_dir.clone();
        info.push(FOMOD_INFO_FILE);
        let file = File::open(info)?;
        let mut file = DecodeReaderBytes::new(file);
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        Info::try_from(contents.as_str())?
    };

    let mut config = {
        let mut config = archive_dir.clone();
        config.push(FOMOD_MODCONFIG_FILE);
        let file = File::open(config)?;
        let mut file = DecodeReaderBytes::new(file);
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        Config::try_from(contents.as_str())?
    };

    files.extend(
        config
            .required_install_files
            .iter()
            .map(|rif| InstallFile::from(rif.clone())),
    );

    println!(
        "FoMod Installer for {} ({})",
        info.name.unwrap_or_default(),
        name,
    );

    for is in config.install_steps.vec_sorted() {
        println!("Install Step: {}", is.name);
        for g in is.optional_file_groups.vec_sorted() {
            println!("Group Name: {}", g.name);
        }
    }

    Ok(Manifest::new(name, mod_type, files, Vec::new()))
}
