mod conflict;
mod downloads;
mod enable;
mod modlist;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::Subcommand;
use comfy_table::{Cell, Color};

use crate::{
    commands::conflict::{conflict_list_by_file, conflict_list_by_mod},
    manifest::Manifest,
    settings::create_table,
    Settings,
};

use self::modlist::{find_mod, gather_mods};

#[derive(Subcommand, Debug, Clone)]
pub enum Subcommands {
    UpdateConfig {
        #[arg(short, long)]
        download_dir: Option<PathBuf>,
        #[arg(short, long)]
        game_dir: Option<PathBuf>,
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,
        #[arg(short, long)]
        proton_dir: Option<PathBuf>,
        #[arg(short, long)]
        user_dir: Option<PathBuf>,
        #[arg(short, long)]
        editor: Option<String>,
        // #[arg(short, long)]
        // find_compat: bool,
        // #[arg(short, long)]
        // find_proton: bool,
        // #[arg(short, long)]
        // find_proton_home_dir: bool,
    },
    Disable {
        name: String,
    },
    DisableAll,
    EditGameConfig {
        #[arg(short, long)]
        config_name: Option<String>,
    },
    EditModConfig {
        name: String,
        #[arg(short, long)]
        config_name: Option<String>,
        #[arg(short, long)]
        extension: Option<String>,
    },
    Enable {
        name: String,
        priority: Option<isize>,
    },
    EnableAll,
    Extract {
        name: String,
    },
    ExtractDownloads,
    List,
    ListDownloads,
    PurgeCache,
    PurgeConfig,
    Remove {
        name: String,
    },
    // RenameMod {
    //     old_mod_name: String,
    //     new_mod_name: String,
    // },
    Run {
        #[arg(short, long)]
        loader: bool,
    },
    SetPriority {
        name: String,
        priority: isize,
    },
    Show {
        name: String,
    },
    ShowLegenda,
    ShowConfig,
}
impl Subcommands {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Subcommands::ListDownloads => {
                downloads::list_downloaded_files(&settings.download_dir())
            }
            Subcommands::ExtractDownloads => {
                downloads::extract_downloaded_files(
                    &settings.download_dir(),
                    &settings.cache_dir(),
                )?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::Extract { name } => {
                downloads::find_and_extract_archive(
                    &settings.download_dir(),
                    &settings.cache_dir(),
                    name.as_str(),
                )?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::List => list_mods(&settings.cache_dir()),
            Subcommands::Show { name } => show_mod(&settings.cache_dir(), &name),
            Subcommands::EnableAll => {
                enable::enable_all(&settings.cache_dir(), &settings.game_dir())?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::Enable { name, priority } => {
                enable::enable_mod(&settings.cache_dir(), &settings.game_dir(), &name, priority)?;
                if priority.is_none() {
                    show_mod(&settings.cache_dir(), &name)
                } else {
                    list_mods(&settings.cache_dir())
                }
            }
            Subcommands::DisableAll => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::Disable { name } => {
                enable::disable_mod(&settings.cache_dir(), &settings.game_dir(), &name)?;
                show_mod(&settings.cache_dir(), &name)
            }
            Subcommands::UpdateConfig {
                download_dir,
                game_dir,
                cache_dir,
                proton_dir,
                user_dir,
                editor,
            } => {
                let settings = settings.create_config(
                    download_dir,
                    game_dir,
                    cache_dir,
                    proton_dir,
                    user_dir,
                    editor,
                )?;
                println!("{}", &settings);
                Ok(())
            }
            Subcommands::ShowConfig => {
                println!("{}", &settings);
                Ok(())
            }
            Subcommands::EditModConfig {
                name,
                config_name,
                extension,
            } => edit_mod_config_files(&settings, name, config_name, extension),
            Subcommands::EditGameConfig { config_name: _ } => todo!(),
            Subcommands::PurgeConfig => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_config()
            }
            Subcommands::PurgeCache => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_cache()
            }
            Subcommands::Remove { name } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(manifest) = find_mod(&mod_list, &name) {
                    manifest.remove(&settings.cache_dir())?;
                }
                Ok(())
            }
            Subcommands::Run { loader: _ } => todo!(),
            Subcommands::ShowLegenda => show_legenda(),
            Subcommands::SetPriority { name, priority } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(mut m) = find_mod(&mod_list, &name) {
                    m.set_priority(priority);
                    m.write_manifest(&settings.cache_dir())?;
                    list_mods(&settings.cache_dir())?;
                }
                Ok(())
            }
        }
    }
}

fn run_game(game_dir: &Path) {}

fn edit_mod_config_files(
    settings: &Settings,
    name: String,
    config_name: Option<String>,
    extension: Option<String>,
) -> Result<()> {
    if let Some(editor) = settings.editor() {
        let mut config_files_to_edit = Vec::new();
        let mod_list = gather_mods(&settings.cache_dir())?;
        if let Some(manifest) = modlist::find_mod(&mod_list, &name) {
            let config_list = manifest.find_config_files(extension.as_deref());
            if let Some(config_name) = config_name {
                if let Some(cf) = config_list.iter().find(|f| {
                    f.file_name()
                        .map(|f| f.to_str())
                        .flatten()
                        .unwrap_or_default()
                        == config_name
                }) {
                    let mut config_path = settings.cache_dir().to_path_buf();
                    config_path.push(cf);
                    config_files_to_edit.push(config_path)
                }
            } else {
                for cf in config_list {
                    let mut config_path = settings.cache_dir().to_path_buf();
                    config_path.push(cf);
                    config_files_to_edit.push(config_path)
                }
            }
        }

        if !config_files_to_edit.is_empty() {
            println!("Editing: {:?}", config_files_to_edit);

            let mut editor_cmd = std::process::Command::new(editor);
            for f in config_files_to_edit {
                editor_cmd.arg(f);
            }
            editor_cmd.output()?;
        } else {
            println!("No relevant config files found.");
        }
    } else {
        println!("Editor not configured.");
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tag {
    Enabled,
    Winner,
    Loser,
    CompleteLoser,
    Conflict,
    Disabled,
}
impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Tag::Enabled => "Enabled",
                Tag::Winner => "Winner",
                Tag::Loser => "Loser",
                Tag::CompleteLoser => "All Files Overwritten",
                Tag::Conflict => "Conflict",
                Tag::Disabled => "Disabled",
            }
        )
    }
}
impl From<Tag> for char {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Enabled => 'e',
            Tag::Winner => 'w',
            Tag::Loser => 'l',
            Tag::CompleteLoser => 'L',
            Tag::Conflict => 'c',
            Tag::Disabled => 'D',
        }
    }
}
impl From<Tag> for Color {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Enabled => Color::White,
            Tag::Winner => Color::Green,
            Tag::Loser => Color::Yellow,
            Tag::CompleteLoser => Color::Red,
            Tag::Conflict => Color::Magenta,
            Tag::Disabled => Color::DarkGrey,
        }
    }
}
impl From<(bool, bool)> for Tag {
    fn from((loser, winner): (bool, bool)) -> Self {
        match (loser, winner) {
            (false, false) => Tag::Enabled,
            (false, true) => Tag::Winner,
            (true, false) => Tag::Loser,
            (true, true) => Tag::Conflict,
        }
    }
}

pub fn list_mods(cache_dir: &Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list = conflict_list_by_mod(&mod_list)?;

    let mut table = create_table(vec![
        "Index", "Name", "Priority", "Status", "Version", "Nexus Id", "Mod Type",
    ]);

    for (idx, manifest) in mod_list.iter().enumerate() {
        let is_loser = conflict_list
            .get(&manifest.name().to_string())
            .map(|c| !c.losing_to().is_empty())
            .unwrap_or(false);
        let is_winner = conflict_list
            .get(&manifest.name().to_string())
            .map(|c| !c.winning_over().is_empty())
            .unwrap_or(false);

        let tag = Tag::from((is_loser, is_winner));

        // Detect if we all files of this manifest are overwritten by other mods
        let tag = if is_loser {
            let mut file_not_lost = false;
            let conflict_list = conflict_list_by_file(&mod_list)?;

            for f in manifest.dest_files() {
                if let Some(contenders) = conflict_list.get(&f) {
                    if let Some(c) = contenders.last() {
                        if c == manifest.name() {
                            file_not_lost = true;
                        }
                    }
                } else {
                    file_not_lost = true;
                }
            }

            if !file_not_lost {
                Tag::CompleteLoser
            } else {
                tag
            }
        } else {
            tag
        };
        let tag = if manifest.mod_state().is_enabled() {
            tag
        } else {
            Tag::Disabled
        };

        let color = Color::from(tag);

        table.add_row(vec![
            Cell::new(idx.to_string()).fg(color),
            Cell::new(manifest.name().to_string()).fg(color),
            Cell::new(manifest.priority().to_string()).fg(color),
            Cell::new(tag).fg(color),
            Cell::new(manifest.version().unwrap_or("<None>").to_string()).fg(color),
            Cell::new(
                manifest
                    .nexus_id()
                    .map(|nid| nid.to_string())
                    .unwrap_or("<None>".to_owned()),
            )
            .fg(color),
            Cell::new(manifest.mod_type().to_string()).fg(color),
        ]);
    }

    println!("{table}");

    Ok(())
}

pub fn show_mod(cache_dir: &Path, mod_name: &str) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some(m) = find_mod(&mod_list, mod_name) {
        show_mod_status(&m, &mod_list)?;
    } else {
        println!("No mod found by that name: {}", mod_name);
    }

    Ok(())
}

pub fn show_mod_status(manifest: &Manifest, mod_list: &[Manifest]) -> Result<()> {
    let conflict_list_file = conflict_list_by_file(&mod_list)?;
    let conflict_list_mod = conflict_list_by_mod(&mod_list)?;

    let mut table = create_table(vec![
        "Name", "Priority", "Status", "Mod Type", "Version", "Nexus Id",
    ]);
    table.add_row(vec![
        manifest.name().to_string(),
        manifest.priority().to_string(),
        manifest.mod_state().to_string(),
        manifest.mod_type().to_string(),
        manifest.version().unwrap_or("<None>").to_string(),
        manifest
            .nexus_id()
            .map(|nid| nid.to_string())
            .unwrap_or("<None>".to_owned()),
    ]);

    println!("{table}");

    if let Some(conflict) = conflict_list_mod.get(&manifest.name().to_string()) {
        let mut table = create_table(vec![
            "Conflicting file",
            "This mod overwrites",
            "This mod is overwritten by",
        ]);

        for f in conflict.conflict_files() {
            let mut winners = Vec::new();
            let mut losers = Vec::new();

            if let Some(contenders) = conflict_list_file.get(f) {
                let mut found_self = false;
                for contender in contenders {
                    if contender == manifest.name() {
                        found_self = true;
                    } else if !found_self {
                        losers.push(contender.to_owned());
                    } else {
                        winners.push(contender.to_owned());
                    }
                }

                let color = if winners.is_empty() {
                    Color::Green
                } else {
                    Color::Red
                };

                if losers.is_empty() {
                    losers.push("None".to_owned());
                }
                if winners.is_empty() {
                    winners.push("None".to_owned());
                }

                table.add_row(vec![
                    Cell::new(f.clone()).fg(color),
                    Cell::new(format!("{:?}", losers)).fg(color),
                    Cell::new(format!("{:?}", winners)).fg(color),
                ]);
            }
        }

        println!("");
        println!("{table}");
    }

    Ok(())
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

    println!("{table}");
    Ok(())
}
