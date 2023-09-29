use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;

use crate::settings::Settings;

#[derive(Debug, Clone, Parser, Default)]
pub enum ConfigCmd {
    #[default]
    Show,
    Update {
        #[arg(short, long)]
        download_dir: Option<Utf8PathBuf>,
        #[arg(short, long)]
        game_dir: Option<Utf8PathBuf>,
        #[arg(short, long)]
        cache_dir: Option<Utf8PathBuf>,
        #[arg(short, long)]
        proton_dir: Option<Utf8PathBuf>,
        #[arg(short = 'o', long)]
        compat_dir: Option<Utf8PathBuf>,
        #[arg(short, long)]
        editor: Option<String>,
        // #[arg(short, long)]
        // find_compat: bool,
        // #[arg(short, long)]
        // find_proton: bool,
        // #[arg(short, long)]
        // find_proton_home_dir: bool,
    },
}
impl ConfigCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
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
            } => {
                let settings = settings.create_config(
                    download_dir,
                    game_dir,
                    cache_dir,
                    proton_dir,
                    compat_dir,
                    editor,
                )?;
                log::info!("{}", &settings);
                Ok(())
            }
        }
    }
}
