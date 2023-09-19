use anyhow::Result;
use clap::Parser;

//TODO: seperate into a lib
mod commands;
mod decompress;
use commands::Subcommands;
mod installers;
mod manifest;
mod mod_types;
mod settings;

use settings::Settings;
use shadow_rs::shadow;
shadow!(build);

const APP_NAMES: [&'static str; 1] = ["starmod"];

/// Simple Starfield Modding Application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Set output to verbose
    #[arg(short, long, action = clap::ArgAction::Count, group = "verbosity")]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Subcommands>,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let settings = Settings::read_config()?;

    if !settings.valid_config() {
        if let Some(cmd @ Subcommands::CreateConfig { .. }) = args.command {
            cmd.execute(&settings)?;
        } else {
            println!(
                "Not valid config file found; Please run {} create-config first.",
                settings.cmd_name()
            );
        }
    } else {
        let cmd = args.command.unwrap_or(Subcommands::List);
        cmd.execute(&settings)?;
    }

    Ok(())
}
