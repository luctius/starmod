use anyhow::{Context, Result};
use comfy_table::{presets::NOTHING, ContentArrangement, Table};
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

use crate::{dmodman::DModManConfig, game::Game};

const CONFIG_EXTENTION: &'static str = "ron";
const EDITOR_ENV: &'static str = "EDITOR";

#[derive(Error, Debug)]
pub enum SettingErrors {
    #[error("The app is run with an unknown name ({0}); Please use on of {1}.")]
    WrongAppName(String, String),
    #[error("No valid config file could be found; Please run '{0} update-config' first.")]
    ConfigNotFound(String),
    #[error("The game directory for {0} cannot be found, Please run '{1} update-config' and provide manually.")]
    NoGameDirFound(String, String),
    #[error("A download directory for cannot be found, Please run '{0} update-config' and provide manually.")]
    NoDownloadDirFound(String),
    #[error(
        "The cache directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoCacheDirFound(String),
    #[error(
        "The proton directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoProtonDirFound(String),
    #[error(
        "The compat directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoCompatDirFound(String),
    #[error(
        "The steam directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoSteamDirFound(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(skip_serializing, default)]
    game: Game,
    #[serde(skip_serializing, default)]
    verbosity: u8,
    cache_dir: PathBuf,
    config_path: PathBuf,
    download_dir: PathBuf,
    game_dir: PathBuf,
    proton_dir: Option<PathBuf>,
    compat_dir: Option<PathBuf>,
    steam_dir: Option<PathBuf>,
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

        let config_file = PathBuf::from(game.name()).with_extension(CONFIG_EXTENTION);

        let xdg_base = BaseDirectories::with_prefix(&name)?;
        let config_path = xdg_base
            .place_config_file(config_file)
            .with_context(|| format!("Cannot create configuration directory for {}", name))?;

        let download_dir = DModManConfig::read().map(|dc| dc.download_dir()).flatten();
        let download_dir = download_dir
            .or_else(|| dirs::download_dir())
            .unwrap_or_default();

        let cache_dir = xdg_base.create_cache_directory("").unwrap_or_default();

        let editor = env::vars().find_map(|(key, val)| (key == EDITOR_ENV).then(|| val));

        let proton_dir = None;
        let compat_dir = None;
        let steam_dir = dirs::home_dir().map(|mut d| {
            d.push(".steam/steam");
            d
        });
        let steam_dir = if let Some(steam_dir) = steam_dir {
            if steam_dir.exists() {
                Some(steam_dir)
            } else {
                None
            }
        } else {
            None
        };

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
            steam_dir,
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
    pub fn game(&self) -> &Game {
        &self.game
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
    pub fn proton_dir(&self) -> Option<&Path> {
        self.proton_dir.as_deref()
    }
    pub fn compat_dir(&self) -> Option<&Path> {
        self.compat_dir.as_deref()
    }
    pub fn steam_dir(&self) -> Option<&Path> {
        self.steam_dir.as_deref()
    }
    pub fn editor(&self) -> String {
        self.editor.clone().unwrap_or("xdg-open".to_owned())
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
        proton_dir: Option<PathBuf>,
        compat_dir: Option<PathBuf>,
        editor: Option<String>,
    ) -> Result<Self> {
        let mut settings = self.clone();

        let cache_dir = cache_dir.unwrap_or(settings.cache_dir);
        let download_dir = download_dir.unwrap_or(settings.download_dir);
        let game_dir = game_dir.unwrap_or(settings.game_dir);

        let game_dir = if game_dir.exists() {
            game_dir
        } else {
            self.game.find_game().unwrap_or(game_dir)
        };

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

        //FIXME TODO check these if they are provided
        settings.proton_dir = proton_dir.or_else(|| self.proton_dir.clone());
        settings.compat_dir = compat_dir.or_else(|| self.compat_dir.clone());
        settings.editor = editor.or_else(|| self.editor.clone());

        let mut file = File::create(&self.config_path)?;

        let serialized = ron::ser::to_string_pretty(&settings, ron::ser::PrettyConfig::default())?;
        file.write_all(serialized.as_bytes())?;

        Ok(settings)
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
        let mut table = create_table(vec!["Setting", "Value"]);
        table
            .add_row(vec![
                "Config File".to_owned(),
                format!("{}", self.config_path.display()),
            ])
            .add_row(vec![
                "Cache Dir".to_owned(),
                format!("{}", self.cache_dir.display()),
            ])
            .add_row(vec![
                "Download Dir".to_owned(),
                format!("{}", self.download_dir.display()),
            ])
            .add_row(vec![
                "Game Dir".to_owned(),
                format!("{}", self.game_dir.display()),
            ])
            .add_row(vec![
                "Steam Proton Dir".to_owned(),
                format!(
                    "{}",
                    self.proton_dir
                        .as_ref()
                        .map(|d| d.display().to_string())
                        .unwrap_or("<Unknown>".to_owned())
                ),
            ])
            .add_row(vec![
                "User Dir".to_owned(),
                format!(
                    "{}",
                    self.compat_dir
                        .as_ref()
                        .map(|d| d.display().to_string())
                        .unwrap_or("<Unknown>".to_owned())
                ),
            ])
            .add_row(vec![
                "Editor".to_owned(),
                format!("{}", self.editor.clone().unwrap_or("<Unknown>".to_owned())),
            ]);

        write!(f, "{}", table)
    }
}

pub fn create_table(headers: Vec<&'static str>) -> Table {
    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(120)
        .set_header(headers);
    table
}
