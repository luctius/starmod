mod downloads;
mod enable;
mod modlist;

use clap::Subcommand;

use anyhow::Result;

use crate::Settings;

#[derive(Subcommand, Debug, Clone)]
pub enum Subcommands {
    ListDownloads,
    ExtractDownloads,
    List,
    Enable,
    Disable,
}
impl Subcommands {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Subcommands::ListDownloads => downloads::list_downloaded_files(&settings.download_dir),
            Subcommands::ExtractDownloads => {
                downloads::extract_downloaded_files(&settings.download_dir, &settings.archive_dir)
            }
            Subcommands::List => modlist::list_mods(&settings.archive_dir),
            Subcommands::Enable => enable::enable_all(&settings.archive_dir, &settings.game_dir),
            Subcommands::Disable => enable::disable_all(&settings.archive_dir, &settings.game_dir),
        }
    }
}
