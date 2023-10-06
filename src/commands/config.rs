use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;

use crate::settings::{RunCmdKind, Settings};

#[derive(Debug, Clone, Parser, Default)]
pub enum ConfigCmd {
    /// Show starmod's configuration values
    #[default]
    #[clap(visible_alias = "s")]
    Show,
    /// Update settings
    #[clap(visible_alias = "u")]
    Update {
        #[arg(short = 'd', long)]
        download_dir: Option<Utf8PathBuf>,
        #[arg(short = 'g', long)]
        game_dir: Option<Utf8PathBuf>,
        #[arg(short = 'c', long)]
        cache_dir: Option<Utf8PathBuf>,
        #[arg(short = 'p', long)]
        proton_dir: Option<Utf8PathBuf>,
        #[arg(short = 'o', long)]
        compat_dir: Option<Utf8PathBuf>,
        #[arg(short = 'e', long)]
        editor: Option<String>,
        #[arg(short = 'x', long)]
        xedit_dir: Option<Utf8PathBuf>,
        #[arg(long, value_enum)]
        default_run: Option<RunCmdKind>,
        // #[arg(short, long, value_enum)]
        // loot_type: Option<LootType>, FIXME
        #[arg(long)]
        loot_data_dir: Option<Utf8PathBuf>,
    },
}
impl ConfigCmd {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Self::Show => {
                log::info!("{}", &settings);
                Ok(())
            }
            Self::Update {
                download_dir,
                game_dir,
                cache_dir,
                proton_dir,
                compat_dir,
                editor,
                default_run,
                xedit_dir,
                // loot_type,
                loot_data_dir,
            } => {
                let loot_type = None;
                let settings = settings.create_config(
                    download_dir,
                    game_dir,
                    cache_dir,
                    proton_dir,
                    compat_dir,
                    editor,
                    default_run,
                    xedit_dir,
                    loot_type,
                    loot_data_dir,
                )?;
                log::info!("{}", &settings);
                Ok(())
            }
        }
    }
}
