use anyhow::Result;
use clap::Parser;

//TODO: seperate into a lib
mod commands;
mod decompress;
use commands::Subcommands;
mod game;
mod installers;
mod manifest;
mod mod_types;
mod settings;

use settings::Settings;
use shadow_rs::shadow;

use crate::settings::SettingErrors;
shadow!(build);

/// Simple Starfield Modding Application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Set output to verbose
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Subcommands>,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let settings = Settings::read_config(args.verbose)?;

    // Only allow create-config to be run when no valid settings are found
    if !settings.valid_config() {
        if let Some(cmd @ Subcommands::CreateConfig { .. }) = args.command {
            cmd.execute(&settings)?;
        } else {
            return Err(SettingErrors::ConfigNotFound(settings.cmd_name().to_owned()).into());
        }
    } else {
        let cmd = args.command.unwrap_or(Subcommands::List);
        cmd.execute(&settings)?;
    }

    Ok(())
}
