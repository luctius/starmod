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
use xdg::BaseDirectories;

use camino::{Utf8Path, Utf8PathBuf};
use log::LevelFilter;

use crate::{commands::game::RunCmd, dmodman::DModManConfig, errors::SettingErrors, game::Game};

const CONFIG_EXTENTION: &str = "ron";
const EDITOR_ENV: &str = "EDITOR";

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
pub enum RunCmdKind {
    Game,
    Loader,
    Loot,
    XEdit,
}
impl From<RunCmdKind> for RunCmd {
    fn from(kind: RunCmdKind) -> Self {
        match kind {
            RunCmdKind::Game => Self::Game,
            RunCmdKind::Loader => Self::Loader,
            RunCmdKind::Loot => Self::Loot,
            RunCmdKind::XEdit => Self::XEdit,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LootType {
    Windows(Utf8PathBuf),
    FlatPack,
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
impl From<u8> for LogLevel {
    fn from(verbose: u8) -> Self {
        match verbose {
            0 => Self::Info,
            1 => Self::Debug,
            2 | _ => Self::Trace,
        }
    }
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
            LogLevel::Error | LogLevel::Warn | LogLevel::Info => Self::Info,
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
    #[serde(default)]
    proton_dir: Option<Utf8PathBuf>,
    #[serde(default)]
    compat_dir: Option<Utf8PathBuf>,
    #[serde(default)]
    steam_dir: Option<Utf8PathBuf>,
    loot: LootType,
    loot_data_dir: Utf8PathBuf,
    #[serde(default)]
    xedit_dir: Option<Utf8PathBuf>,
    #[serde(default)]
    default_run: Option<RunCmdKind>,
    #[serde(default)]
    editor: Option<String>,
}
impl Settings {
    fn create(game: Game, verbosity: LogLevel) -> Result<Self> {
        //Extract cmd used to run this application
        let name = game.mod_manager_name();

        let config_file = Utf8PathBuf::from(name).with_extension(CONFIG_EXTENTION);

        let xdg_base = BaseDirectories::with_prefix(name)?;
        let config_path = Utf8PathBuf::try_from(
            xdg_base
                .place_config_file(config_file)
                .with_context(|| format!("Cannot create configuration directory for {name}"))?,
        )?;
        let log_path = config_path.with_extension("log");

        let download_dir = DModManConfig::read().and_then(|dc| dc.download_dir());
        let download_dir = download_dir
            .or_else(|| dirs::download_dir().map(|d| Utf8PathBuf::try_from(d).unwrap()))
            .unwrap_or_default();

        let cache_dir =
            Utf8PathBuf::try_from(xdg_base.create_cache_directory("").unwrap_or_default())?;

        let editor = env::vars().find_map(|(key, val)| (key == EDITOR_ENV).then_some(val));

        let loot = LootType::FlatPack;
        let proton_dir = None;
        let compat_dir = None;
        let xedit_dir = None;
        let steam_dir = dirs::home_dir().map(|mut d| {
            d.push(".steam/steam");
            d
        });

        let steam_dir = steam_dir
            .and_then(|steam_dir| {
                if steam_dir.exists() {
                    Some(steam_dir)
                } else {
                    None
                }
            })
            .map(|sd| Utf8PathBuf::try_from(sd).unwrap());

        let default_run = None;

        let loot_data_dir = Utf8PathBuf::try_from(
            xdg_base
                .create_config_directory("loot")
                .with_context(|| format!("Cannot create configuration directory for {name}"))?,
        )?;

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
            loot,
            loot_data_dir,
            xedit_dir,
            default_run,
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
    pub const fn game(&self) -> &Game {
        &self.game
    }
    pub const fn cmd_name(&self) -> &str {
        self.game.mod_manager_name()
    }
    #[allow(unused)]
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
    pub const fn loot(&self) -> &LootType {
        &self.loot
    }
    pub fn loot_data_dir(&self) -> &Utf8Path {
        self.loot_data_dir.as_path()
    }
    pub fn xedit_dir(&self) -> Option<&Utf8Path> {
        self.xedit_dir.as_deref()
    }
    pub const fn default_run(&self) -> Option<RunCmdKind> {
        self.default_run
    }
    pub fn editor(&self) -> String {
        self.editor.clone().unwrap_or_else(|| "xdg-open".to_owned())
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
    #[allow(clippy::too_many_arguments)]
    pub fn create_config(
        &self,
        download_dir: Option<Utf8PathBuf>,
        game_dir: Option<Utf8PathBuf>,
        cache_dir: Option<Utf8PathBuf>,
        proton_dir: Option<Utf8PathBuf>,
        compat_dir: Option<Utf8PathBuf>,
        editor: Option<String>,
        default_run: Option<RunCmdKind>,
        xedit_dir: Option<Utf8PathBuf>,
        loot_type: Option<LootType>,
        loot_data_dir: Option<Utf8PathBuf>,
    ) -> Result<Self> {
        let mut settings = self.clone();

        let cache_dir = cache_dir.unwrap_or(settings.cache_dir);
        let download_dir = download_dir.unwrap_or(settings.download_dir);
        let game_dir = game_dir.unwrap_or(settings.game_dir);

        let game_dir = if game_dir.exists() {
            game_dir
        } else {
            Game::find_game().unwrap_or(game_dir)
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
        settings.default_run = default_run.or(self.default_run);
        settings.xedit_dir = xedit_dir.or_else(|| self.xedit_dir.clone());
        settings.loot_data_dir = loot_data_dir.unwrap_or_else(|| self.loot_data_dir.clone());
        settings.loot = loot_type.unwrap_or_else(|| self.loot.clone());

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
            println!("Removing directory: {parent}");
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
                        .map_or_else(|| "<Unknown>".to_owned(), ToString::to_string)
                ),
            ])
            .add_row(vec![
                "Xedit Dir".to_owned(),
                format!(
                    "{}",
                    self.xedit_dir
                        .as_ref()
                        .map_or_else(|| "<Unknown>".to_owned(), ToString::to_string)
                ),
            ])
            .add_row(vec![
                "User Dir".to_owned(),
                format!(
                    "{}",
                    self.compat_dir
                        .as_ref()
                        .map_or_else(|| "<Unknown>".to_owned(), ToString::to_string)
                ),
            ])
            .add_row(vec![
                "Editor".to_owned(),
                format!(
                    "{}",
                    self.editor
                        .clone()
                        .unwrap_or_else(|| "<Unknown>".to_owned())
                ),
            ]);

        write!(f, "{table}")
    }
}

pub fn create_table(headers: Vec<&'static str>) -> Table {
    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        // .set_content_arrangement(ContentArrangement::Disabled)
        .set_header(headers);
    table
}

pub fn default_page_size() -> usize {
    const MAX: usize = 50;
    let h = term_size::dimensions_stdout().map(|d| d.1).unwrap_or(MAX);
    if h > MAX {
        MAX
    } else {
        h
    }
}
