pub mod config;
pub mod downloads;
pub mod game;
pub mod list;
pub mod mods;
pub mod purge;

use anyhow::Result;
use clap::{builder::styling, Parser};
use comfy_table::{Cell, Color};

use crate::{list_commands, settings::create_table, tag::Tag, Settings};

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

//TODO: we should probably add the most used commands here too
// like set-priority etc

const STYLE: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::BrightYellow.on_default())
    .usage(styling::AnsiColor::BrightYellow.on_default())
    .literal(styling::AnsiColor::BrightWhite.on_default())
    .placeholder(styling::AnsiColor::Cyan.on_default());

#[derive(Debug, Clone, Parser)]
#[command()]
#[clap(styles=STYLE)]
pub enum Subcommands {
    /// Config related commands; defaults to showing the current settings.
    #[clap(visible_aliases = &["configs", "c"])]
    Config {
        #[command(subcommand)]
        cmd: Option<ConfigCmd>,
    },
    /// Various lists; defaults to showing the mod-list
    #[clap(visible_aliases = &["lists", "l"])]
    List {
        #[command(subcommand)]
        cmd: Option<ListCmd>,
    },
    /// Commands related to mods; defaults to showing the mod-list
    #[clap(visible_aliases = &["mod", "m"])]
    Mods {
        #[command(subcommand)]
        cmd: Option<ModCmd>,
    },
    /// Commands related to download archives; defaults to showing the downloaded files.
    #[clap(visible_aliases = &["download", "down", "d"])]
    Downloads {
        #[command(subcommand)]
        cmd: Option<DownloadCmd>,
    },
    /// Game related commands; defaults to running the game.
    #[clap(visible_alias = "g")]
    Game {
        #[command(subcommand)]
        cmd: Option<GameCmd>,
    },
    /// Alias for Game Run; defaults to running the game.
    #[clap(visible_alias = "r")]
    Run {
        #[command(subcommand)]
        cmd: Option<RunCmd>,
    },
    /// Dangerous: commands related to the removal of starmod's files.
    Purge {
        #[command(subcommand)]
        cmd: PurgeCmd,
    },
    /// Show explanation of the colours used by starmod.
    Legenda,
    /// Show a flattened list all commands
    ListCommands,

    #[cfg(feature = "loadorder")]
    /// Plugin related commands
    Plugin {
        #[command(subcommand)]
        cmd: Option<PluginCmd>,
    },
}
impl Subcommands {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        //General TODO: Be more consistant in errors, error messages warnings etc.

        match self {
            Self::Config { cmd } => ConfigCmd::execute(cmd.unwrap_or_default(), settings),
            Self::List { cmd } => ListCmd::execute(cmd.unwrap_or_default(), settings),
            Self::Mods { cmd } => ModCmd::execute(cmd.unwrap_or_default(), settings),
            Self::Downloads { cmd } => DownloadCmd::execute(cmd.unwrap_or_default(), settings),
            Self::Run { cmd } => RunCmd::execute(cmd.unwrap_or_default(), settings),
            Self::Game { cmd } => GameCmd::execute(cmd.unwrap_or_default(), settings),
            Self::Purge { cmd } => PurgeCmd::execute(cmd, settings),
            Self::ListCommands => {
                list_commands();
                Ok(())
            }
            Self::Legenda => {
                show_legenda();
                Ok(())
            }

            #[cfg(feature = "loadorder")]
            Self::Plugin { cmd } => PluginCmd::execute(cmd.unwrap_or_default(), settings),
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

pub fn show_legenda() {
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
}
