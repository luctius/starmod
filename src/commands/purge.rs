use anyhow::Result;
use clap::Parser;

use crate::{
    mods::{GatherModList, ModList},
    settings::Settings,
};

#[derive(Debug, Clone, Parser)]
pub enum PurgeCmd {
    /// Remove both config and cache; This removes all of starmod's generated files.
    Config,
    /// Remove cache directory, but keep the config files
    Cache,
}
impl PurgeCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Config => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.disable(settings.cache_dir(), settings.game_dir())?;
                settings.purge_config()
            }
            Self::Cache => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.disable(settings.cache_dir(), settings.game_dir())?;
                settings.purge_cache()
            }
        }
    }
}
