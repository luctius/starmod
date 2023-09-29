use anyhow::Result;
use clap::Parser;

use crate::{enable, settings::Settings};

#[derive(Debug, Clone, Parser)]
pub enum PurgeCmd {
    Config,
    Cache,
}
impl PurgeCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Config => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_config()
            }
            Self::Cache => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_cache()
            }
        }
    }
}
