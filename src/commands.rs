mod conflict;
mod downloads;
mod enable;
mod modlist;

use std::path::PathBuf;

use clap::Subcommand;

use anyhow::Result;

use crate::Settings;

use self::modlist::{find_mod, gather_mods};

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
        priority: Option<isize>,
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
    SetPriority {
        name: String,
        priority: isize,
    },
    //InsertAt { name: String, priority: isize },
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
            Subcommands::Enable { name, priority } => {
                enable::enable_mod(&settings.cache_dir(), &settings.game_dir(), &name, priority)?;
                if priority.is_none() {
                    modlist::show_mod(&settings.cache_dir(), &name)
                } else {
                    modlist::list_mods(&settings.cache_dir())
                }
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
            Subcommands::SetPriority { name, priority } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(mut m) = find_mod(&mod_list, &name) {
                    m.set_priority(priority);
                    m.write_manifest(&settings.cache_dir())?;
                    modlist::list_mods(&settings.cache_dir())?;
                }
                Ok(())
            }
        }
    }
}
