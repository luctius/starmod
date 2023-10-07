#![deny(
    nonstandard_style,
    rust_2018_idioms,
    future_incompatible,
    unused_extern_crates,
    unused_import_braces,
    // unused_results,
    // unused_qualifications,
    //warnings,
    //unused,
    unsafe_code,
// missing_docs,
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::wildcard_dependencies
)]

use anyhow::Result;
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use comfy_table::{Cell, Color};
use flexi_logger::{detailed_format, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use game::Game;
use shadow_rs::shadow;

mod commands;
mod decompress;
use commands::Subcommands;
mod conflict;
mod dmodman;
mod errors;
mod game;
mod installers;
mod manifest;
mod modlist;
mod mods;
mod settings;
mod tag;
mod utils;

use settings::{LogLevel, Settings};

use crate::{errors::SettingErrors, settings::create_table};
shadow!(build);

/// Simple Starfield Modding Application
#[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
#[command(author, version, about, multicall = true)]
pub struct StarMod {
    #[command(subcommand)]
    applet: AppLet,
}

/// Simple Starfield Modding Application
#[derive(Subcommand, Debug, Clone)]
#[command(author, version, about, long_about = None, rename_all="lower")]
pub enum AppLet {
    StarMod(AppLetArgs),
    // SkyMod(AppLetArgs),
    // ObMod(AppLetArgs),
    // MorMod(AppLetArgs),
}
impl AppLet {
    #[must_use]
    pub fn unwrap(self) -> (Game, AppLetArgs) {
        match self {
            /*Self::MorMod(a) | Self::ObMod(a) | Self::SkyMod(a) |*/
            Self::StarMod(a) => (Game::Starfield, a),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
pub struct AppLetArgs {
    /// Set output to verbose
    #[arg(short, long, value_enum, default_value_t = LogLevel::Info)]
    verbose: LogLevel,

    /// Generate shell completion scripts for the given shell
    #[arg(long)]
    generator: Option<Shell>,

    #[command(subcommand)]
    cmd: Option<Subcommands>,

    /// Show information related to this build of starmod
    #[arg(short = 'V', long)]
    version: bool,

    /// Show information related to this build of starmod
    #[arg(long)]
    long_version: bool,

    /// Show Long Help
    #[arg(long)]
    list_commands: bool,
}

fn log_stdout(
    w: &mut dyn std::io::Write,
    _now: &mut flexi_logger::DeferredNow,
    record: &log::Record<'_>,
) -> Result<(), std::io::Error> {
    write!(w, "{}", record.args())
}

pub fn main() -> Result<()> {
    let applet = StarMod::parse();
    let (game, args) = applet.applet.unwrap();

    let settings = Settings::read_config(game, args.verbose)?;

    let _logger = Logger::try_with_env_or_str("trace")?
        .log_to_file(FileSpec::try_from(settings.log_file())?)
        .write_mode(WriteMode::BufferDontFlush)
        .append()
        .rotate(
            Criterion::Size(100 * 1024),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10),
        )
        .duplicate_to_stdout(args.verbose.into())
        .format_for_stdout(log_stdout)
        .format_for_files(detailed_format)
        .write_mode(WriteMode::Direct)
        .start()?;

    if args.long_version {
        println!("version:{}", build::CLAP_LONG_VERSION);
        return Ok(());
    } else if args.version {
        let tag = build::TAG;
        let tag = if tag.is_empty() {
            build::SHORT_COMMIT
        } else {
            tag
        };

        println!("{} ({})", build::PKG_VERSION, tag);
        return Ok(());
    }
    if args.list_commands {
        list_commands();
        return Ok(());
    }
    if let Some(generator) = args.generator {
        let mut cmd = AppLetArgs::command();
        log::info!("Generating completion file for {generator}...");
        print_completions(generator, &mut cmd);
        return Ok(());
    }

    log::trace!("cmd: {:?}", args.cmd);

    // Only allow create-config to be run when no valid settings are found
    if settings.valid_config() {
        args.cmd.unwrap_or_default().execute(&settings)?;
    } else if let Some(cmd @ Subcommands::Config { .. }) = args.cmd {
        cmd.execute(&settings)?;
    } else {
        return Err(SettingErrors::ConfigNotFound(settings.cmd_name().to_owned()).into());
    }

    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

fn list_commands() {
    let mut table = create_table(vec!["Command", "Help"]);
    let mut list = vec![];

    list.extend_from_slice(&gather_commands(
        &AppLetArgs::command(),
        AppLetArgs::command().get_name(),
    ));

    list.sort();

    for (prev_cmd, c, help) in list {
        let mut cmdtable = create_table(vec!["", ""]);
        cmdtable.add_row(vec![
            Cell::new(prev_cmd).fg(Color::DarkCyan),
            Cell::new(c).fg(Color::White),
        ]);

        table.add_row(vec![
            Cell::new(format!("{}", cmdtable.lines().last().unwrap())),
            Cell::new(help),
        ]);
    }

    log::info!("");
    log::info!("{table}");
}

fn gather_commands(
    cmd: &clap::Command,
    previous_cmds: &str,
) -> Vec<(String, String, clap::builder::StyledStr)> {
    let mut list = Vec::new();

    for cmd in cmd.get_subcommands() {
        list.push((
            previous_cmds.to_string(),
            cmd.get_name().to_string(),
            cmd.get_about().unwrap_or_default().to_owned(),
        ));

        if cmd.has_subcommands() {
            let lcmd = previous_cmds.to_string() + " " + cmd.get_name();
            list.extend_from_slice(&gather_commands(cmd, &lcmd));
        }
    }
    list
}
