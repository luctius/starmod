pub mod config;
pub mod downloads;
pub mod game;
pub mod list;
pub mod mods;
pub mod purge;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color};

use crate::{settings::create_table, tag::Tag, Settings};

use self::{
    config::ConfigCmd,
    downloads::DownloadCmd,
    game::{GameCmd, RunCmd},
    list::ListCmd,
    mods::ModCmd,
    purge::PurgeCmd,
};

#[cfg(feature = "loadorder")]
pub mod plugins;
#[cfg(feature = "loadorder")]
use self::plugins::PluginCmd;

#[derive(Debug, Clone, Parser)]
pub enum Subcommands {
    /// Config related commands
    Config {
        #[command(subcommand)]
        cmd: Option<ConfigCmd>,
    },
    /// Various lists
    List {
        #[command(subcommand)]
        cmd: Option<ListCmd>,
    },
    /// Commands related to mods
    Mod {
        #[command(subcommand)]
        cmd: Option<ModCmd>,
    },
    /// Alias for Mod
    Mods {
        #[command(subcommand)]
        cmd: Option<ModCmd>,
    },
    /// Commands related to download archives.
    Download {
        #[command(subcommand)]
        cmd: Option<DownloadCmd>,
    },
    /// Alias for Download
    Downloads {
        #[command(subcommand)]
        cmd: Option<DownloadCmd>,
    },
    /// Game related commands
    Game {
        #[command(subcommand)]
        cmd: Option<GameCmd>,
    },
    /// Alias for Game Run
    Run {
        #[command(subcommand)]
        cmd: Option<RunCmd>,
    },
    /// Dangerous: removal of starmod's files
    Purge {
        #[command(subcommand)]
        cmd: PurgeCmd,
    },
    /// Show explanation of the colours used by starmod
    Legenda,
    /// Show information related to this build of starmod
    Version,

    #[cfg(feature = "loadorder")]
    /// Plugin related commands
    Plugin {
        #[command(subcommand)]
        cmd: Option<PluginCmd>,
    },
}
impl Subcommands {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        //General TODO: Be more consistant in errors, error messages warnings etc.
        //TODO: disable and re-enable all mods when mods are added, removed or changed order
        // To avoid certain files not being properly added or removed.

        match self {
            Subcommands::Config { cmd } => ConfigCmd::execute(cmd.unwrap_or_default(), settings),
            Subcommands::List { cmd } => ListCmd::execute(cmd.unwrap_or_default(), settings),
            Subcommands::Mod { cmd } | Subcommands::Mods { cmd } => {
                ModCmd::execute(cmd.unwrap_or_default(), settings)
            }
            Subcommands::Download { cmd } | Subcommands::Downloads { cmd } => {
                DownloadCmd::execute(cmd.unwrap_or_default(), settings)
            }
            Subcommands::Run { cmd } => RunCmd::execute(cmd.unwrap_or_default(), settings),
            Subcommands::Game { cmd } => GameCmd::execute(cmd.unwrap_or_default(), settings),
            Subcommands::Purge { cmd } => PurgeCmd::execute(cmd, settings),
            Subcommands::Legenda => show_legenda(),
            Subcommands::Version => {
                crate::print_build();
                Ok(())
            }

            #[cfg(feature = "loadorder")]
            Subcommands::Plugin { cmd } => PluginCmd::execute(cmd.unwrap_or_default(), settings),
        }
    }
}
impl Default for Subcommands {
    fn default() -> Self {
        Self::List {
            cmd: Some(ListCmd::default()),
        }
    }
}

pub fn show_legenda() -> Result<()> {
    let mut table = create_table(vec!["Tag", "Color", "Meaning"]);

    let tag = Tag::Enabled;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("White").fg(color),
        Cell::new("Nothing to see here; move along citizen.").fg(color),
    ]);

    let tag = Tag::Winner;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Green").fg(color),
        Cell::new("Conflict winner").fg(color),
    ]);

    let tag = Tag::Loser;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Yellow").fg(color),
        Cell::new("Conflict loser").fg(color),
    ]);

    let tag = Tag::CompleteLoser;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Red").fg(color),
        Cell::new("Complete conflict loser; ALL files are overwitten by other mods").fg(color),
    ]);

    let tag = Tag::Conflict;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Magenta").fg(color),
        Cell::new("Conflict winner for some files, conflict loser for other files.").fg(color),
    ]);

    let tag = Tag::Disabled;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("DarkGray").fg(color),
        Cell::new("Mod is disabled.").fg(color),
    ]);

    log::info!("{table}");
    Ok(())
}
