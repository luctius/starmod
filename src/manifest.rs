use std::{
    fmt::Display,
    fs::{read_link, remove_dir, remove_dir_all, remove_file, rename, DirBuilder, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{dmodman::DMODMAN_EXTENTION, installers::DATA_DIR_NAME, mod_types::ModType};

//TODO: replace PathBuf with something that is ressilient to deserialisation of non-utf8 characters

pub const MANIFEST_EXTENTION: &'static str = "ron";

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

//TODO more info about the mod, description, authors, version, etc

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    manifest_dir: PathBuf,
    name: String,
    version: Option<String>,
    nexus_id: Option<u32>,
    mod_type: ModType,
    mod_state: ModState,
    files: Vec<InstallFile>,
    disabled_files: Vec<InstallFile>,
    priority: isize,
}
impl Manifest {
    pub fn new(
        manifest_dir: &Path,
        name: String,
        nexus_id: Option<u32>,
        version: Option<String>,
        mod_type: ModType,
        files: Vec<InstallFile>,
        disabled_files: Vec<InstallFile>,
    ) -> Self {
        let s = Self {
            manifest_dir: manifest_dir.to_path_buf(),
            name,
            nexus_id,
            version,
            mod_type,
            files,
            disabled_files,
            mod_state: ModState::Disabled,
            priority: 0,
        };
        s
    }
    pub fn set_priority(&mut self, priority: isize) {
        self.priority = priority;
    }
    pub fn from_file(cache_dir: &Path, archive: &Path) -> Result<Self> {
        let mut manifest_file = PathBuf::from(cache_dir);
        manifest_file.push(archive);
        manifest_file.set_extension(MANIFEST_EXTENTION);

        let file = File::open(manifest_file)?;
        Self::try_from(file)
    }

    pub fn write_manifest(&self, cache_dir: &Path) -> Result<()> {
        let mut path = PathBuf::from(cache_dir);
        path.push(&self.manifest_dir);
        path.set_extension(MANIFEST_EXTENTION);

        if path.exists() {
            remove_file(&path)?;
        }

        let mut file = File::create(&path)?;

        let serialized =
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
    pub fn remove(&self, cache_dir: &Path) -> Result<()> {
        let mut path = PathBuf::from(cache_dir);
        path.push(&self.manifest_dir);
        remove_dir_all(&path)?;
        path.set_extension(MANIFEST_EXTENTION);
        remove_file(&path)?;
        path.set_extension(DMODMAN_EXTENTION);
        remove_file(&path)?;
        Ok(())
    }
    pub fn is_valid(&self) -> bool {
        //TODO: checks to validate the manifest file
        true
    }
    pub fn manifest_dir(&self) -> &Path {
        &self.manifest_dir
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: String) {
        self.name = name
    }
    pub fn nexus_id(&self) -> Option<u32> {
        self.nexus_id
    }
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
    pub fn mod_type(&self) -> &ModType {
        &self.mod_type
    }
    pub fn mod_state(&self) -> ModState {
        self.mod_state
    }
    pub fn enable(&mut self, cache_dir: &Path, game_dir: &Path) -> Result<()> {
        if self.mod_state.is_enabled() {
            return Ok(());
        }
        if self.priority < 0 {
            self.disable(cache_dir, game_dir)?;
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
                game_dir.push(PathBuf::from(df));
                game_dir
            };

            //create intermediate directories
            DirBuilder::new()
                .recursive(true)
                .create(destination.parent().unwrap())?;

            // Remove existing symlinks which point back to our archive dir
            // This ensures that the last mod wins, but we should do conflict
            // detection and resolution before this, so we can inform the user.
            if destination.is_symlink() {
                let target = read_link(&destination)?;

                if target.starts_with(&cache_dir) {
                    remove_file(&destination)?;
                    //TODO verbose println!("removed {} -> {}", destination.display(), target.display());
                } else {
                    let bkp_destination = destination.with_file_name(format!(
                        "{}.starmod_bkp",
                        destination
                            .extension()
                            .map(|s| s.to_str())
                            .flatten()
                            .unwrap_or_default()
                    ));
                    //TODO: can we handle foreign files better than this?
                    // eprintln!("Not removing foreign file: {}", target.display());
                    println!(
                        "renaming foreign file from {} -> {}",
                        destination.display(),
                        bkp_destination.display()
                    );
                    rename(&destination, bkp_destination)?;
                }
            }

            std::os::unix::fs::symlink(&origin, &destination)?;

            println!("link {} to {}", origin.display(), destination.display());
        }

        self.mod_state = ModState::Enabled;

        Ok(())
    }
    pub fn disable(&mut self, cache_dir: &Path, game_dir: &Path) -> Result<()> {
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
                game_dir.push(PathBuf::from(df));
                game_dir
            };

            if destination.is_file()
                && destination.is_symlink()
                && origin == read_link(&destination)?
            {
                remove_file(&destination)?;
                //TODO verbose println!("removed {} -> {}", destination.display(), origin.display());
            } else {
                // dbg!(origin == read_link(&destination)?);
                println!("Skipping {}", destination.display());
            }
        }

        //TODO: this could be optimised
        // right now it will after every disable try to delete
        // all directories in the game dir who are empty.
        let walker = WalkDir::new(&game_dir)
            .min_depth(1)
            .max_depth(usize::MAX)
            .follow_links(false)
            .same_file_system(true)
            .contents_first(false);

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                let _ = remove_dir(entry_path);
            }
        }

        self.mod_state = ModState::Disabled;

        Ok(())
    }
    pub fn dest_files(&self) -> Vec<String> {
        let mut dest_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let destination = f.destination.to_string_lossy().to_string().to_lowercase();

            dest_files.push(
                format!(
                    "Data/{}",
                    destination
                        .strip_prefix(self.mod_type.prefix_to_strip())
                        .unwrap_or(&destination)
                )
                .replace("//", "/"),
            );
        }
        dest_files
    }
    pub fn origin_files(&self) -> Vec<PathBuf> {
        let mut origin_files = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let origin = f.source.as_path();
            let origin = {
                let mut o = self.manifest_dir.to_path_buf();
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
    pub fn find_config_files(&self, ext: Option<&str>) -> Vec<PathBuf> {
        let mut config_files = Vec::new();

        let ext_vec = if let Some(ext) = ext {
            vec![ext]
        } else {
            vec!["ini", "json", "yaml", "xml"]
        };

        for f in self.origin_files() {
            if let Some(file_ext) = f.extension() {
                let file_ext = file_ext.to_string_lossy().to_string();

                if ext_vec.contains(&file_ext.as_str()) {
                    config_files.push(f);
                }
            }
        }
        config_files
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
