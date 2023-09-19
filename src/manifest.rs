use std::{
    fmt::Display,
    fs::{read_link, remove_file, DirBuilder, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use fomod::FileType;
use serde::{Deserialize, Serialize};

use crate::mod_types::ModType;

//TODO: replace PathBuf with something that is ressilient to deserialisation of non-utf8 characters

const MANIFEST_EXTENTION_NAME: &'static str = "ron";
const DATA_DIR_NAME: &'static str = "data";

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
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
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
            ModState::Enabled => f.write_str("Enabled"),
            ModState::Disabled => f.write_str("Disabled"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallFile {
    pub source: PathBuf,
    pub destination: PathBuf,
}
impl From<PathBuf> for InstallFile {
    fn from(pb: PathBuf) -> Self {
        Self {
            source: pb.clone(),
            destination: pb,
        }
    }
}
impl From<&Path> for InstallFile {
    fn from(p: &Path) -> Self {
        Self {
            source: p.to_path_buf(),
            destination: p.to_path_buf(),
        }
    }
}
impl From<FileType> for InstallFile {
    fn from(ft: FileType) -> Self {
        Self {
            destination: PathBuf::from(ft.destination.unwrap_or_else(|| ft.source.clone())),
            source: PathBuf::from(ft.source),
        }
    }
}

//TODO more info about the mod, description, authors, version, etc

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    name: String,
    mod_type: ModType,
    mod_state: ModState,
    files: Vec<InstallFile>,
    disabled_files: Vec<InstallFile>,
    priority: isize,
}
impl Manifest {
    //TODO: should probably move this to a decicated installer function
    pub fn new(
        name: String,
        mod_type: ModType,
        files: Vec<InstallFile>,
        disabled_files: Vec<InstallFile>,
    ) -> Self {
        let s = Self {
            name,
            mod_type,
            files,
            disabled_files,
            mod_state: ModState::Disabled,
            priority: 0,
        };
        println!("Creating Manifest: {:?}", s);
        s
    }
    pub fn from_file(cache_dir: &Path, archive: &Path) -> Result<Self> {
        let mut manifest_file = PathBuf::from(cache_dir);
        manifest_file.push(archive);
        manifest_file.set_extension(MANIFEST_EXTENTION_NAME);

        let file = File::open(manifest_file)?;
        Self::try_from(file)
    }

    pub fn write_manifest(&self, cache_dir: &Path) -> Result<()> {
        let mut path = PathBuf::from(cache_dir);
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
    // pub fn archive_name(&self) -> &str {
    //     &self.name
    // }
    pub fn name(&self) -> &str {
        //TODO Allow for names set by fomod
        &self.name.split_once("-").unwrap().0
    }
    pub fn mod_type(&self) -> ModType {
        self.mod_type
    }
    pub fn mod_state(&self) -> ModState {
        self.mod_state
    }
    pub fn enable(&self, cache_dir: &Path, game_dir: &Path) -> Result<()> {
        if self.mod_state.is_enabled() {
            return Ok(());
        }

        // TODO allow all mod types
        if self.mod_type() != ModType::DataMod {
            return Ok(());
        }

        let cache_dir = PathBuf::from(cache_dir);
        let game_dir = PathBuf::from(game_dir);

        for (of, df) in self.origin_files().iter().zip(self.dest_files().iter()) {
            let origin = {
                let mut cache_dir = cache_dir.clone();
                cache_dir.push(of);
                cache_dir
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

                if target.starts_with(&cache_dir) {
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
        manifest.write_manifest(&cache_dir)?;

        Ok(())
    }
    pub fn disable(&self, cache_dir: &Path, game_dir: &Path) -> Result<()> {
        if self.mod_state.is_disabled() {
            return Ok(());
        }

        let cache_dir = PathBuf::from(cache_dir);
        let game_dir = PathBuf::from(game_dir);

        for (of, df) in self.origin_files().iter().zip(self.dest_files().iter()) {
            let origin = {
                let mut cache_dir = cache_dir.clone();
                cache_dir.push(of);
                cache_dir
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
        manifest.write_manifest(&cache_dir)?;

        Ok(())
    }
    pub fn dest_files(&self) -> Vec<String> {
        let mut dest_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let destination = f.destination.to_string_lossy().to_string().to_lowercase();

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
            let origin = f.source.as_path();
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
