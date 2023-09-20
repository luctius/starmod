mod conflict;
mod downloads;
mod enable;
mod modlist;

use std::path::PathBuf;

use clap::Subcommand;

use anyhow::Result;

use crate::Settings;

#[derive(Subcommand, Debug, Clone)]
pub enum Subcommands {
    ListDownloads,
    ExtractDownloads,
    //Extract { name: String },
    List,
    Show {
        name: String,
    },
    EnableAll,
    Enable {
        name: String,
    },
    DisableAll,
    Disable {
        name: String,
    },
    CreateConfig {
        #[arg(short, long)]
        download_dir: PathBuf,
        #[arg(short, long)]
        game_dir: PathBuf,
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,
    },
    //Remove { name: String },
    //InsertAt { name: String, priority: i32 },
    ShowConfig,
    PurgeConfig,
    PurgeCache,
}
impl Subcommands {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Subcommands::ListDownloads => {
                downloads::list_downloaded_files(&settings.download_dir())
            }
            Subcommands::ExtractDownloads => {
                downloads::extract_downloaded_files(
                    &settings.download_dir(),
                    &settings.cache_dir(),
                )?;
                modlist::list_mods(&settings.cache_dir())
            }
            Subcommands::List => modlist::list_mods(&settings.cache_dir()),
            Subcommands::Show { name } => modlist::show_mod(&settings.cache_dir(), &name),
            Subcommands::EnableAll => {
                enable::enable_all(&settings.cache_dir(), &settings.game_dir())?;
                modlist::list_mods(&settings.cache_dir())
            }
            Subcommands::Enable { name } => {
                enable::enable_mod(&settings.cache_dir(), &settings.game_dir(), &name)?;
                modlist::show_mod(&settings.cache_dir(), &name)
            }
            Subcommands::DisableAll => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                modlist::list_mods(&settings.cache_dir())
            }
            Subcommands::Disable { name } => {
                enable::disable_mod(&settings.cache_dir(), &settings.game_dir(), &name)?;
                modlist::show_mod(&settings.cache_dir(), &name)
            }
            Subcommands::CreateConfig {
                download_dir,
                game_dir,
                cache_dir,
            } => settings.create_config(download_dir, game_dir, cache_dir),
            Subcommands::ShowConfig => {
                println!("{}", &settings);
                Ok(())
            }
            Subcommands::PurgeConfig => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_config()
            }
            Subcommands::PurgeCache => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_cache()
            }
        }
    }
}
