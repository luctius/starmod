use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};
use xdg::BaseDirectories;

use crate::APP_NAMES;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(skip_serializing, default)]
    name: String,
    config_path: PathBuf,
    download_dir: PathBuf,
    cache_dir: PathBuf,
    game_dir: PathBuf,
}
impl Settings {
    fn create() -> Result<Self> {
        //Extract cmd used to run this application
        let name = PathBuf::from(std::env::args().nth(0).unwrap())
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        if !APP_NAMES.iter().any(|&n| n == &name) {
            panic!(
                "This command cannot be run as {}, use one of {:?}",
                name, APP_NAMES
            );
        }

        let xdg_base = BaseDirectories::with_prefix(&name)?;
        let config_path = xdg_base
            .place_config_file("config.ron")
            .with_context(|| format!("Cannot create configuration directory for {}", name))?;

        let cache_dir = xdg_base
            .create_cache_directory("")
            .with_context(|| format!("Cannot create cache directory for {}", name))?;

        Ok(Self {
            name,
            config_path,
            download_dir: PathBuf::from(""),
            cache_dir,
            game_dir: PathBuf::from(""),
        })
    }
    pub fn valid_config(&self) -> bool {
        self.config_path.exists()
            && self.config_path.is_file()
            && self.download_dir.exists()
            && self.download_dir.is_dir()
            && self.cache_dir.exists()
            && self.cache_dir.is_dir()
            && self.game_dir.exists()
            && self.game_dir.is_dir()
    }
    pub fn cmd_name(&self) -> &str {
        &self.name
    }
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
    pub fn game_dir(&self) -> &Path {
        &self.game_dir
    }
    pub fn read_config() -> Result<Self> {
        let settings = Self::create()?;
        if let Ok(config) = File::open(&settings.config_path) {
            let mut read_settings = Self::try_from(config)?;
            read_settings.name = settings.name;
            Ok(read_settings)
        } else {
            Ok(settings)
        }
    }
    //TODO option to fetch download dir from dmodman's config
    pub fn create_config(
        &self,
        download_dir: PathBuf,
        game_dir: PathBuf,
        cache_dir: Option<PathBuf>,
    ) -> Result<()> {
        let mut settings = self.clone();

        let cache_dir = cache_dir.unwrap_or(settings.cache_dir);

        download_dir
            .read_dir()
            .with_context(|| format!("Failed to read from {}", download_dir.display()))?;

        game_dir
            .read_dir()
            .with_context(|| format!("Failed to read from {}", game_dir.display()))?;

        cache_dir
            .read_dir()
            .with_context(|| format!("Failed to read from {}", cache_dir.display()))?;

        settings.download_dir = download_dir;
        settings.game_dir = game_dir;
        settings.cache_dir = cache_dir;

        let mut file = File::create(&self.config_path)?;

        let serialized = ron::ser::to_string_pretty(&settings, ron::ser::PrettyConfig::default())?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }
    pub fn purge_config(&self) -> Result<()> {
        self.purge_cache()?;

        println!("Removing file: {}", self.config_path.display());
        std::fs::remove_file(&self.config_path)?;
        if let Some(parent) = self.config_path.parent() {
            println!("Removing directory: {}", parent.display());
            std::fs::remove_dir(parent)?;
        }
        Ok(())
    }
    pub fn purge_cache(&self) -> Result<()> {
        println!(
            "Removing cache directory and it's contents: {}",
            self.cache_dir.display()
        );
        std::fs::remove_dir_all(&self.cache_dir)?;
        Ok(())
    }
}
impl TryFrom<File> for Settings {
    type Error = anyhow::Error;

    fn try_from(file: File) -> std::result::Result<Self, Self::Error> {
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let manifest = ron::from_str(&contents)?;

        Ok(manifest)
    }
}
impl Display for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Config file:  {}", self.config_path.display())?;
        writeln!(f, "Cache dir:    {}", self.cache_dir.display())?;
        writeln!(f, "Download dir: {}", self.download_dir.display())?;
        writeln!(f, "Game dir:     {}", self.game_dir.display())
    }
}
