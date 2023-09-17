use clap::Parser;

//TODO: seperate into a lib
mod commands;
mod decompress;
use commands::Subcommands;
mod manifest;

use shadow_rs::shadow;
shadow!(build);

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

//TODO read from xdg config dir
#[derive(Clone, Debug)]
pub struct Settings {
    download_dir: String,
    archive_dir: String,
    game_dir: String,
}
impl Settings {
    pub fn new() -> Self {
        Self {
            download_dir: "/home/cor/downloads/dmodman/starfield".to_owned(),
            archive_dir: "/home/cor/tmp/starmod".to_owned(),
            game_dir: "/home/cor/tmp/stargame".to_owned(),
        }
    }
}

pub fn main() {
    let args = Args::parse();

    let settings = Settings::new();

    let cmd = args.command.unwrap_or(Subcommands::List);
    cmd.execute(&settings).unwrap();
}
