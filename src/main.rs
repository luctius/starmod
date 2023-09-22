use std::fs::File;

use anyhow::Result;
use clap::Parser;

//TODO: seperate into a lib
mod commands;
mod decompress;
use commands::Subcommands;
mod dmodman;
mod game;
mod installers;
mod manifest;
mod mod_types;
mod settings;

use settings::{LogLevel, Settings};
use shadow_rs::shadow;
use simplelog::{
    ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};

use crate::settings::SettingErrors;
shadow!(build);

/// Simple Starfield Modding Application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Set output to verbose
    #[arg(short, long, value_enum, default_value_t = LogLevel::Warn)]
    verbose: LogLevel,

    #[arg(short, long)]
    term_log: bool,

    #[command(subcommand)]
    command: Option<Subcommands>,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let settings = Settings::read_config(args.verbose)?;

    if args.term_log {
        CombinedLogger::init(vec![
            TermLogger::new(
                args.verbose.into(),
                Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            WriteLogger::new(
                args.verbose.into(),
                Config::default(),
                File::create(settings.log_file()).unwrap(),
            ),
        ])
        .unwrap();
    } else {
        CombinedLogger::init(vec![WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(settings.log_file()).unwrap(),
        )])
        .unwrap();
    }

    // Only allow create-config to be run when no valid settings are found
    if !settings.valid_config() {
        if let Some(cmd @ Subcommands::UpdateConfig { .. }) = args.command {
            cmd.execute(&settings)?;
        } else {
            return Err(SettingErrors::ConfigNotFound(settings.cmd_name().to_owned()).into());
        }
    } else {
        let cmd = args.command.unwrap_or(Subcommands::List);
        cmd.execute(&settings).unwrap();
    }

    Ok(())
}
