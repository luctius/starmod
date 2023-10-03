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
use flexi_logger::{detailed_format, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use game::Game;
use shadow_rs::shadow;

mod commands;
mod decompress;
use commands::Subcommands;
mod conflict;
mod dmodman;
mod game;
mod installers;
mod manifest;
mod modlist;
mod mods;
mod settings;
mod tag;
mod utils;

use settings::{LogLevel, Settings};

use crate::settings::SettingErrors;
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
    command: Option<Subcommands>,

    /// Show information related to this build of starmod
    #[arg(short = 'V', long)]
    version: bool,

    /// Show information related to this build of starmod
    #[arg(long)]
    long_version: bool,
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

    let mut settings = Settings::read_config(game, args.verbose)?;

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

    // let multi = MultiProgress::new();
    // LogWrapper::new(multi.clone(), logger).try_init().unwrap();

    if let Some(generator) = args.generator {
        let mut cmd = AppLetArgs::command();
        log::info!("Generating completion file for {generator}...");
        print_completions(generator, &mut cmd);
        return Ok(());
    }

    // Only allow create-config to be run when no valid settings are found
    if !settings.valid_config() {
        if let Some(cmd @ Subcommands::Config { .. }) = args.command {
            cmd.execute(&mut settings)?;
        } else {
            return Err(SettingErrors::ConfigNotFound(settings.cmd_name().to_owned()).into());
        }
    } else {
        args.command.unwrap_or_default().execute(&mut settings)?;
    }

    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}
