use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use xdg::BaseDirectories;

use crate::game::Game;

const EDITOR_ENV: &'static str = "EDITOR";

#[derive(Error, Debug)]
pub enum SettingErrors {
    #[error("the app is run with an unknown name ({0}), use on of {1}.")]
    WrongAppName(String, String),
    #[error("no valid config file could be found; Please run '{0} create-config' first.")]
    ConfigNotFound(String),
    #[error("game directory for {0} cannot be found, Please run '{1} create-config' and provide manually.")]
    NoGameDirFound(String, String),
    #[error("download directory for cannot be found, Please run '{0} create-config' and provide manually.")]
    NoDownloadDirFound(String),
    #[error(
        "cache directory cannot be found, Please run '{0} create-config' and provide manually."
    )]
    NoCacheDirFound(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(skip_serializing)]
    game: Game,
    #[serde(skip_serializing, default)]
    verbosity: u8,
    config_path: PathBuf,
    download_dir: PathBuf,
    cache_dir: PathBuf,
    game_dir: PathBuf,
    proton_dir: Option<PathBuf>,
    compat_dir: Option<PathBuf>,
    editor: Option<String>,
}
impl Settings {
    fn create(verbosity: u8) -> Result<Self> {
        //Extract cmd used to run this application
        let name = PathBuf::from(std::env::args().nth(0).unwrap())
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let game = Game::create_from_name(name.as_str())?;

        let xdg_base = BaseDirectories::with_prefix(&name)?;
        let config_path = xdg_base
            .place_config_file("config.ron")
            .with_context(|| format!("Cannot create configuration directory for {}", name))?;

        let download_dir = dirs::download_dir().unwrap_or_default();

        let cache_dir = xdg_base.create_cache_directory("").unwrap_or_default();

        let editor = env::vars().find_map(|(key, val)| (key == EDITOR_ENV).then(|| val));

        let proton_dir = None;
        let compat_dir = None;

        Ok(Self {
            game,
            verbosity,
            config_path,
            download_dir,
            cache_dir,
            game_dir: PathBuf::from(""),
            editor,
            proton_dir,
            compat_dir,
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
        self.game.name()
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
    pub fn editor(&self) -> Option<&str> {
        self.editor.as_deref()
    }
    pub fn read_config(verbosity: u8) -> Result<Self> {
        let settings = Self::create(verbosity)?;
        if let Ok(config) = File::open(&settings.config_path) {
            let mut read_settings = Self::try_from(config)?;
            read_settings.game = settings.game;
            read_settings.verbosity = verbosity;
            Ok(read_settings)
        } else {
            Ok(settings)
        }
    }
    //TODO option to fetch download dir from dmodman's config
    pub fn create_config(
        &self,
        download_dir: Option<PathBuf>,
        game_dir: Option<PathBuf>,
        cache_dir: Option<PathBuf>,
    ) -> Result<()> {
        let mut settings = self.clone();

        let cache_dir = cache_dir.unwrap_or(settings.cache_dir);
        let game_dir = game_dir.unwrap_or(settings.game_dir);
        let download_dir = download_dir.unwrap_or(settings.download_dir);

        cache_dir
            .read_dir()
            .map_err(|_| SettingErrors::NoCacheDirFound(self.game.name().to_owned()))?;

        download_dir
            .read_dir()
            .map_err(|_| SettingErrors::NoDownloadDirFound(self.game.name().to_owned()))?;

        game_dir.read_dir().map_err(|_| {
            SettingErrors::NoGameDirFound(
                self.game.game_name().to_owned(),
                self.game.name().to_owned(),
            )
        })?;

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
