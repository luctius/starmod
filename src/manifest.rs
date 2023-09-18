use std::{
    ffi::OsStr,
    fmt::Display,
    fs::{self, read_link, remove_file, DirBuilder, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

//TODO: replace PathBuf with something that is ressilient to deserialisation of non-utf8 characters

const MANIFEST_EXTENTION_NAME: &'static str = "ron";
const DATA_DIR_NAME: &'static str = "data";

const FOMOD_INFO_FILE: &'static str = "fomod/info.xml";
const FOMOD_MODCONFIG_FILE: &'static str = "fomod/moduleconfig.xml";

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModState {
    Enabled,
    Disabled,
}
impl ModState {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Enabled => true,
            Self::Disabled => false,
        }
    }
}
impl From<bool> for ModState {
    fn from(v: bool) -> Self {
        match v {
            true => Self::Enabled,
            false => Self::Disabled,
        }
    }
}
impl Display for ModState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModState::Enabled => f.write_str("enabled"),
            ModState::Disabled => f.write_str("disabled"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    name: String,
    mod_type: ModType,
    mod_state: ModState,
    files: Vec<PathBuf>,
    priority: isize,
}
impl Manifest {
    pub fn new(archive_dir: &str, archive: &OsStr) -> Self {
        let mut typ = ModType::DataMod;
        let mut files = Vec::new();

        let name = PathBuf::from(archive).with_extension("");

        let mut dir = PathBuf::from(archive_dir);
        dir.push(name.clone());
        let dir = dir;

        Self::traverse_dir(&dir, &dir, &mut files);

        if files.iter().any(|p| *p == PathBuf::from(FOMOD_INFO_FILE)) {
            if files
                .iter()
                .any(|p| *p == PathBuf::from(FOMOD_MODCONFIG_FILE))
            {
                typ = ModType::FoMod;
            }

            //TODO: check for a data dir further in the file tree
            // to detect mods with an extra dir
            // we can also make a list of approved dirs for data mods
            // and warn the user about other dirs.

            // filter some file types like readme.txt??
        }

        Self {
            name: name.to_string_lossy().to_string(),
            mod_type: typ,
            files,
            mod_state: ModState::Disabled,
            priority: 0,
        }
    }
    pub fn from_file(archive_dir: &str, archive: &Path) -> Result<Self> {
        let mut manifest_file = PathBuf::from(archive_dir);
        manifest_file.push(archive);
        manifest_file.set_extension(MANIFEST_EXTENTION_NAME);

        let file = File::open(manifest_file)?;
        Self::try_from(file)
    }

    pub fn write_manifest(&self, archive_dir: &str) -> Result<()> {
        let mut path = PathBuf::from(archive_dir);
        path.push(&self.name);
        path.set_extension(MANIFEST_EXTENTION_NAME);

        if path.exists() {
            remove_file(&path)?;
        }

        let mut file = File::create(&path)?;

        let serialized =
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
    pub fn is_valid(&self) -> bool {
        //TODO: checks to validate the manifest file
        true
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn mod_type(&self) -> ModType {
        self.mod_type
    }
    pub fn mod_state(&self) -> ModState {
        self.mod_state
    }
    pub fn enable(&self, archive_dir: &str, game_dir: &str) -> Result<()> {
        if self.mod_state == ModState::Enabled {
            return Ok(());
        }

        // TODO allow all mod types
        if self.mod_type() != ModType::DataMod {
            return Ok(());
        }

        let archive_dir = PathBuf::from(archive_dir);
        let game_dir = PathBuf::from(game_dir);

        for (of, df) in self.origin_files().iter().zip(self.dest_files().iter()) {
            let origin = {
                let mut archive_dir = archive_dir.clone();
                archive_dir.push(of);
                archive_dir
            };
            let destination = {
                let mut game_dir = game_dir.clone();
                game_dir.push(df);
                game_dir
            };

            if destination.is_dir() {
                //TODO do dirbuilder only on directories?
                continue;
            }

            //create intermediate directories
            DirBuilder::new()
                .recursive(true)
                .create(destination.parent().unwrap())?;

            //TODO conflict detection and resolution should prevent this

            // Remove existing symlinks which point back to our archive dir
            // This ensures that the last mod wins, but we should do conflict
            // detection and resolution before this, so we can inform the user.
            if destination.is_symlink() {
                let target = read_link(&destination)?;

                if target.starts_with(&archive_dir) {
                    remove_file(&destination)?;
                    //TODO verbose println!("removed {} -> {}", destination.display(), target.display());
                } else {
                    //TODO: can we handle foreign files better than this?
                    eprintln!("Not removing forein file: {}", target.display());
                    continue;
                }
            }

            std::os::unix::fs::symlink(&origin, &destination)?;

            //TODO: verbose println!("link {} to {}", origin.display(), destination.display());
        }

        let mut manifest = self.clone();
        manifest.mod_state = ModState::Enabled;
        manifest.write_manifest(&archive_dir.to_string_lossy().to_string())?;

        Ok(())
    }
    pub fn disable(&self, archive_dir: &str, game_dir: &str) -> Result<()> {
        if self.mod_state == ModState::Disabled {
            return Ok(());
        }

        let archive_dir = PathBuf::from(archive_dir);
        let game_dir = PathBuf::from(game_dir);

        for (of, df) in self.origin_files().iter().zip(self.dest_files().iter()) {
            let origin = {
                let mut archive_dir = archive_dir.clone();
                archive_dir.push(of);
                archive_dir
            };
            let destination = {
                let mut game_dir = game_dir.clone();
                game_dir.push(df);
                game_dir
            };

            if destination.is_file()
                && destination.is_symlink()
                && origin == read_link(&destination)?
            {
                remove_file(&destination)?;
                //TODO verbose println!("removed {} -> {}", destination.display(), origin.display());
            } else if destination.is_dir() {
                //TODO remove empty dirs?
            } else {
                //TODO verbose println!("Skipping {}", destination.display());
            }
        }

        let mut manifest = self.clone();
        manifest.mod_state = ModState::Disabled;
        manifest.write_manifest(&archive_dir.to_string_lossy().to_string())?;

        Ok(())
    }
    pub fn dest_files(&self) -> Vec<String> {
        let mut dest_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let destination = f.to_string_lossy().to_string().to_lowercase();

            let destination = if destination.starts_with(DATA_DIR_NAME) {
                destination
            } else {
                let mut p = PathBuf::from(DATA_DIR_NAME);
                p.push(destination);
                p.to_string_lossy().to_string()
            };

            dest_files.push(destination);
        }
        dest_files
    }
    pub fn origin_files(&self) -> Vec<PathBuf> {
        let mut origin_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let origin = f;
            let origin = {
                let mut o = PathBuf::from(&self.name);
                o.push(origin);
                o
            };
            origin_files.push(origin)
        }
        origin_files
    }
    pub fn priority(&self) -> isize {
        self.priority
    }
    fn traverse_dir(base: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
        // use walkdir?

        let paths = fs::read_dir(dir).unwrap();

        for path in paths {
            if let Ok(path) = path {
                if let Ok(ft) = path.file_type() {
                    if ft.is_file() {
                        //TODO Skip files like readme.txt?

                        if let Ok(path) = path.path().strip_prefix(base) {
                            files.push(path.to_path_buf())
                        }
                    } else if ft.is_dir() {
                        let mut dir = dir.to_path_buf();
                        dir.push(path.path());

                        Self::traverse_dir(base, &dir, files);
                    }
                }
            }
        }
    }
}
impl TryFrom<File> for Manifest {
    type Error = anyhow::Error;

    fn try_from(file: File) -> std::result::Result<Self, Self::Error> {
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let manifest = ron::from_str(&contents)?;

        Ok(manifest)
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModType {
    // Goes into Data
    DataMod,
    //Installer
    FoMod,
    //Goes into the root dir
    AppMod,
}
impl Display for ModType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataMod => f.write_str("Data Mod"),
            Self::FoMod => f.write_str("FoMod"),
            Self::AppMod => f.write_str("AppMod"),
        }
    }
}
