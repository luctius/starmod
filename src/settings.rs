use anyhow::{Context, Result};
use clap::ValueEnum;
use comfy_table::{presets::NOTHING, ContentArrangement, Table};
use flexi_logger::Duplicate;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Write},
};
use thiserror::Error;
use xdg::BaseDirectories;

use camino::{Utf8Path, Utf8PathBuf};
use log::LevelFilter;

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

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Deserialize, Serialize,
)]
pub enum LogLevel {
    Error,
    #[default]
    Warn,
    Info,
    Debug,
    Trace,
}
impl From<LogLevel> for LevelFilter {
    fn from(ll: LogLevel) -> Self {
        match ll {
            LogLevel::Error => Self::Error,
            LogLevel::Warn => Self::Warn,
            LogLevel::Info => Self::Info,
            LogLevel::Debug => Self::Debug,
            LogLevel::Trace => Self::Trace,
        }
    }
}
impl From<LogLevel> for Duplicate {
    fn from(ll: LogLevel) -> Self {
        // Note: never log less than info, is make the application useless
        match ll {
            LogLevel::Error => Self::Info,
            LogLevel::Warn => Self::Info,
            LogLevel::Info => Self::Info,
            LogLevel::Debug => Self::Debug,
            LogLevel::Trace => Self::Trace,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(skip_serializing, default)]
    game: Game,
    #[serde(skip_serializing, default)]
    verbosity: LogLevel,
    cache_dir: Utf8PathBuf,
    config_path: Utf8PathBuf,
    log_path: Utf8PathBuf,
    download_dir: Utf8PathBuf,
    game_dir: Utf8PathBuf,
    proton_dir: Option<Utf8PathBuf>,
    compat_dir: Option<Utf8PathBuf>,
    steam_dir: Option<Utf8PathBuf>,
    editor: Option<String>,
}
impl Settings {
    fn create(game: Game, verbosity: LogLevel) -> Result<Self> {
        //Extract cmd used to run this application
        let name = game.mod_manager_name();

        let config_file = Utf8PathBuf::from(name).with_extension(CONFIG_EXTENTION);

        let xdg_base = BaseDirectories::with_prefix(&name)?;
        let config_path = Utf8PathBuf::try_from(
            xdg_base
                .place_config_file(config_file)
                .with_context(|| format!("Cannot create configuration directory for {}", name))?,
        )?;
        let log_path = Utf8PathBuf::try_from(config_path.with_extension("log"))?;

        let download_dir = DModManConfig::read().map(|dc| dc.download_dir()).flatten();
        let download_dir = download_dir
            .or_else(|| dirs::download_dir().map(|d| Utf8PathBuf::try_from(d).unwrap()))
            .unwrap_or_default();
        let download_dir = Utf8PathBuf::try_from(download_dir)?;

        let cache_dir =
            Utf8PathBuf::try_from(xdg_base.create_cache_directory("").unwrap_or_default())?;

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
        }
        .map(|sd| Utf8PathBuf::try_from(sd).unwrap());

        Ok(Self {
            game,
            verbosity,
            config_path,
            log_path,
            download_dir,
            cache_dir,
            game_dir: Utf8PathBuf::from(""),
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
        self.game.mod_manager_name()
    }
    pub fn config_file(&self) -> &Utf8Path {
        &self.config_path
    }
    pub fn log_file(&self) -> &Utf8Path {
        &self.log_path
    }
    pub fn download_dir(&self) -> &Utf8Path {
        &self.download_dir
    }
    pub fn cache_dir(&self) -> &Utf8Path {
        &self.cache_dir
    }
    pub fn game_dir(&self) -> &Utf8Path {
        &self.game_dir
    }
    pub fn proton_dir(&self) -> Option<&Utf8Path> {
        self.proton_dir.as_deref()
    }
    pub fn compat_dir(&self) -> Option<&Utf8Path> {
        self.compat_dir.as_deref()
    }
    pub fn steam_dir(&self) -> Option<&Utf8Path> {
        self.steam_dir.as_deref()
    }
    pub fn editor(&self) -> String {
        self.editor.clone().unwrap_or("xdg-open".to_owned())
    }
    pub fn read_config(game: Game, verbosity: LogLevel) -> Result<Self> {
        let settings = Self::create(game, verbosity)?;
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
        download_dir: Option<Utf8PathBuf>,
        game_dir: Option<Utf8PathBuf>,
        cache_dir: Option<Utf8PathBuf>,
        proton_dir: Option<Utf8PathBuf>,
        compat_dir: Option<Utf8PathBuf>,
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
            .map_err(|_| SettingErrors::NoCacheDirFound(self.game.mod_manager_name().to_owned()))?;

        download_dir.read_dir().map_err(|_| {
            SettingErrors::NoDownloadDirFound(self.game.mod_manager_name().to_owned())
        })?;

        game_dir.read_dir().map_err(|_| {
            SettingErrors::NoGameDirFound(
                self.game.game_name().to_owned(),
                self.game.mod_manager_name().to_owned(),
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

        println!("Removing file: {}", self.config_path);
        std::fs::remove_file(&self.config_path)?;
        if let Some(parent) = self.config_path.parent() {
            println!("Removing directory: {}", parent);
            std::fs::remove_dir(parent)?;
        }
        Ok(())
    }
    pub fn purge_cache(&self) -> Result<()> {
        println!(
            "Removing cache directory and it's contents: {}",
            self.cache_dir
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
                format!("{}", self.config_path),
            ])
            .add_row(vec!["Cache Dir".to_owned(), format!("{}", self.cache_dir)])
            .add_row(vec![
                "Download Dir".to_owned(),
                format!("{}", self.download_dir),
            ])
            .add_row(vec!["Game Dir".to_owned(), format!("{}", self.game_dir)])
            .add_row(vec![
                "Steam Proton Dir".to_owned(),
                format!(
                    "{}",
                    self.proton_dir
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or("<Unknown>".to_owned())
                ),
            ])
            .add_row(vec![
                "User Dir".to_owned(),
                format!(
                    "{}",
                    self.compat_dir
                        .as_ref()
                        .map(|d| d.to_string())
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
        // .set_width(120)
        .set_header(headers);
    table
}
