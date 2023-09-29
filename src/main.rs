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
mod dmodman;
mod game;
mod installers;
mod manifest;
mod mods;
mod settings;
mod tag;

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
#[command(author, version, about, long_about = None)]
pub struct AppLetArgs {
    /// Set output to verbose
    #[arg(short, long, value_enum, default_value_t = LogLevel::Info)]
    verbose: LogLevel,

    #[arg(long)]
    generator: Option<Shell>,

    #[command(subcommand)]
    command: Option<Subcommands>,
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

pub fn print_build() {
    println!("version:{}", build::CLAP_LONG_VERSION);

    println!("tag:{}", build::TAG);
    println!("branch:{}", build::BRANCH);
    println!("commit_id:{}", build::COMMIT_HASH);
    println!("short_commit:{}", build::SHORT_COMMIT);
    println!("commit_date_3339:{}", build::COMMIT_DATE_3339);

    println!("build_os:{}", build::BUILD_OS);
    println!("rust_version:{}", build::RUST_VERSION);
    println!("rust_channel:{}", build::RUST_CHANNEL);
    println!("cargo_version:{}", build::CARGO_VERSION);

    println!("project_name:{}", build::PROJECT_NAME);
    println!("build_time_3339:{}", build::BUILD_TIME_3339);
    println!("build_rust_channel:{}", build::BUILD_RUST_CHANNEL);
}
