use anyhow::Result;
use clap::Parser;
use loadorder::GameSettings;

use crate::settings::Settings;

#[derive(Debug, Clone, Parser, Default)]
pub enum PluginCmd {
    #[default]
    Show,
    Sort,
}
impl PluginCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Show => {
                log::info!("{}", "not yet implemented");
                Ok(())
            }
            Self::Sort => {
                GameSettings::new(
                    settings.game().game_id(),
                    settings
                        .game_dir()
                        .to_path_buf()
                        .into_std_path_buf()
                        .as_path(),
                )?
                .into_load_order()
                .save()?;
                Ok(())
            }
        }
    }
}
