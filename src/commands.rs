mod conflict;
mod downloads;
mod enable;
mod modlist;

use std::{
    cmp::Ordering,
    collections::HashMap,
    ffi::OsString,
    fmt::Display,
    fs::{copy, DirBuilder},
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color};
use walkdir::WalkDir;

use crate::{
    commands::conflict::{conflict_list_by_file, conflict_list_by_mod},
    mods::{Mod, ModKind},
    settings::{create_table, SettingErrors},
    Settings,
};

use self::{
    downloads::downloaded_files,
    modlist::{find_mod, gather_mods},
};

#[derive(Debug, Clone, Parser, Default)]
pub enum Subcommands {
    CopyToCustomMod {
        origin_mod: String,
        custom_mod: String,
        file_name: String,
    },
    CreateCustomMod {
        name: String,
        origin: Option<PathBuf>,
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
    ExtractAll,
    #[default]
    List,
    ListDownloads,
    PurgeCache,
    PurgeConfig,
    ReEnableAll,
    ReInstall {
        name: String,
    },
    RemoveMod {
        name: String,
    },
    Rename {
        old_mod_name: String,
        new_mod_name: String,
    },
    Run {
        #[arg(short, long)]
        loader: bool,
    },
    SetPrio {
        name: String,
        priority: isize,
    },
    SetPriority {
        name: String,
        priority: isize,
    },
    Show {
        name: String,
    },
    ShowConfig,
    ShowConflicts,
    ShowFiles {
        name: Option<String>,
    },
    ShowLegenda,
    UpdateConfig {
        #[arg(short, long)]
        download_dir: Option<PathBuf>,
        #[arg(short, long)]
        game_dir: Option<PathBuf>,
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,
        #[arg(short, long)]
        proton_dir: Option<PathBuf>,
        #[arg(short = 'o', long)]
        compat_dir: Option<PathBuf>,
        #[arg(short, long)]
        editor: Option<String>,
        // #[arg(short, long)]
        // find_compat: bool,
        // #[arg(short, long)]
        // find_proton: bool,
        // #[arg(short, long)]
        // find_proton_home_dir: bool,
    },
    UpdateCustomMod {
        name: String,
    },
    #[clap(hide = true)]
    Quit,
}
impl Subcommands {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        //General TODO: Be more consistant in errors, error messages warnings etc.
        //TODO: disable and re-enable all mods when mods are added, removed or changed order
        // To avoid certain files not being properly added or removed.

        match self {
            Subcommands::CreateCustomMod { origin, name } => {
                let destination = settings.cache_dir().join(&name);
                if let Some(origin) = origin {
                    std::os::unix::fs::symlink(&origin, &destination)?;
                    log::info!(
                        "Creating custom mod {} (link from {})",
                        &name,
                        origin.display()
                    );
                } else {
                    log::info!("Creating custom mod {}", &name);
                    DirBuilder::new().recursive(true).create(destination)?;
                }
                let mut manifest =
                    ModKind::Custom.create_mod(&settings.cache_dir(), &PathBuf::from(name))?;
                manifest.set_priority(10000)?;
                Ok(())
            }
            Subcommands::UpdateCustomMod { name } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(old_mod) = find_mod(&mod_list, &name) {
                    log::info!("Updating mod '{}'", old_mod.name());
                    let name = old_mod.name();
                    let mut new_mod =
                        ModKind::Custom.create_mod(&settings.cache_dir(), &PathBuf::from(name))?;
                    new_mod.set_priority(old_mod.priority())?;
                    if old_mod.is_enabled() {
                        new_mod.enable(&settings.cache_dir(), &settings.game_dir())?;
                    }
                }
                Ok(())
            }
            Subcommands::CopyToCustomMod {
                origin_mod,
                custom_mod,
                file_name,
            } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(origin_mod) = find_mod(&mod_list, &origin_mod) {
                    if let Some(custom_mod) = find_mod(&mod_list, &custom_mod) {
                        if let Some(file) = origin_mod
                            .origin_files()
                            .iter()
                            .find(|f| f.file_name().unwrap().eq(file_name.as_str()))
                        {
                            let origin = settings.cache_dir().join(file);
                            let destination = settings
                                .cache_dir()
                                .join(custom_mod.manifest_dir())
                                .join(file.strip_prefix(origin_mod.manifest_dir()).unwrap());

                            DirBuilder::new()
                                .recursive(true)
                                .create(destination.parent().unwrap())?;
                            copy(origin, destination)?;

                            let mut new_mod = ModKind::Custom.create_mod(
                                &settings.cache_dir(),
                                &PathBuf::from(custom_mod.name()),
                            )?;
                            new_mod.set_priority(custom_mod.priority())?;
                            if custom_mod.is_enabled() {
                                new_mod.enable(&settings.cache_dir(), &settings.game_dir())?;
                            }
                        } else {
                            log::warn!(
                                "File '{}' could not be found in mod '{}'.",
                                file_name,
                                origin_mod.name()
                            );
                        }
                    } else {
                        log::warn!("Mod '{}' could not be found", custom_mod);
                    }
                } else {
                    log::warn!("Mod '{}' could not be found", origin_mod);
                }
                Ok(())
            }
            Subcommands::ListDownloads => {
                //TODO also show wether or not it is allready installed
                list_downloaded_files(&settings.download_dir(), &settings.cache_dir())
            }
            Subcommands::ExtractAll => {
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
            Subcommands::ShowFiles { name } => show_mod_files(&settings.cache_dir(), name),
            Subcommands::EnableAll => {
                enable::enable_all(&settings.cache_dir(), &settings.game_dir())?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::Enable { name, priority } => {
                enable::enable_mod(&settings.cache_dir(), &settings.game_dir(), &name, priority)?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::DisableAll => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::Disable { name } => {
                enable::disable_mod(&settings.cache_dir(), &settings.game_dir(), &name)?;
                list_mods(&settings.cache_dir())
            }
            Subcommands::UpdateConfig {
                download_dir,
                game_dir,
                cache_dir,
                proton_dir,
                compat_dir,
                editor,
            } => {
                let settings = settings.create_config(
                    download_dir,
                    game_dir,
                    cache_dir,
                    proton_dir,
                    compat_dir,
                    editor,
                )?;
                log::info!("{}", &settings);
                Ok(())
            }
            Subcommands::ShowConfig => {
                log::info!("{}", &settings);
                Ok(())
            }
            Subcommands::EditModConfig {
                name,
                config_name,
                extension,
            } => edit_mod_config_files(&settings, name, config_name, extension),
            Subcommands::EditGameConfig { config_name } => {
                edit_game_config_files(settings, config_name)
            }
            Subcommands::PurgeConfig => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_config()
            }
            Subcommands::PurgeCache => {
                enable::disable_all(&settings.cache_dir(), &settings.game_dir())?;
                settings.purge_cache()
            }
            Subcommands::RemoveMod { name } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(mut md) = find_mod(&mod_list, &name) {
                    md.disable(&settings.cache_dir(), &settings.game_dir())?;
                    md.remove(&settings.cache_dir())?;
                    log::info!("Removed mod '{}'", md.name());
                    list_mods(&settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Subcommands::Run { loader } => run_game(&settings, loader),
            Subcommands::ShowLegenda => show_legenda(),
            Subcommands::SetPrio { name, priority }
            | Subcommands::SetPriority { name, priority } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(mut m) = find_mod(&mod_list, &name) {
                    m.set_priority(priority)?;
                    if priority < 0 {
                        m.disable(&settings.cache_dir(), &settings.game_dir())?;
                    }
                    list_mods(&settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Subcommands::ReInstall { name } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(mut m) = find_mod(&mod_list, &name) {
                    m.disable(&settings.cache_dir(), &settings.game_dir())?;
                    m.remove(&settings.cache_dir())?;

                    let mod_type =
                        ModKind::detect_mod_type(&settings.cache_dir(), &m.manifest_dir())?;
                    mod_type.create_mod(&settings.cache_dir(), &m.manifest_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Subcommands::ReEnableAll {} => {
                let mut mod_list = gather_mods(&settings.cache_dir())?;
                mod_list.retain(|m| m.is_enabled());
                for manifest in mod_list.iter_mut() {
                    manifest.disable(&settings.cache_dir(), &settings.game_dir())?;
                }
                for manifest in mod_list.iter_mut() {
                    manifest.enable(&settings.cache_dir(), &settings.game_dir())?;
                }
                log::info!("Mods re-enabled.");
                Ok(())
            }
            Subcommands::Rename {
                old_mod_name,
                new_mod_name,
            } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some(mut m) = find_mod(&mod_list, &old_mod_name) {
                    m.set_name(new_mod_name)?;
                    list_mods(&settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{old_mod_name}' not found.")
                }
                Ok(())
            }
            Subcommands::ShowConflicts => show_conflicts(&settings.cache_dir()),
            Subcommands::Quit => {
                settings.stop_repl();
                Ok(())
            }
        }
    }
}

fn run_game(settings: &Settings, loader: bool) -> Result<()> {
    if let Some(proton_dir) = settings.proton_dir() {
        if let Some(compat_dir) = settings.compat_dir() {
            if let Some(steam_dir) = settings.steam_dir() {
                let mut compat_dir = compat_dir.to_path_buf();
                if compat_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    != settings.game().steam_id().to_string().as_str()
                {
                    compat_dir.push(settings.game().steam_id().to_string());
                }
                let mut proton_exe = proton_dir.to_path_buf();
                proton_exe.push("proton");
                let mut game_exe = settings.game_dir().to_path_buf();

                if !loader {
                    game_exe.push(settings.game().exe_name());
                } else {
                    game_exe.push(settings.game().loader_name());
                }

                log::info!("Running 'STEAM_COMPAT_DATA_PATH={} STEAM_COMPAT_CLIENT_INSTALL_PATH={} {} run {}'", compat_dir.display(), steam_dir.display(), proton_exe.display(), game_exe.display());

                let output = std::process::Command::new(proton_exe)
                    .arg("run")
                    // .arg("waitforexitandrun")
                    .arg(game_exe)
                    .env("STEAM_COMPAT_DATA_PATH", compat_dir)
                    .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam_dir)
                    .output()?;

                if !output.status.success() {
                    log::info!("{:?}", output.stdout);
                }
                Ok(())
            } else {
                Err(SettingErrors::NoSteamDirFound(settings.cmd_name().to_owned()).into())
            }
        } else {
            Err(SettingErrors::NoCompatDirFound(settings.cmd_name().to_owned()).into())
        }
    } else {
        Err(SettingErrors::NoProtonDirFound(settings.cmd_name().to_owned()).into())
    }
}

fn edit_mod_config_files(
    settings: &Settings,
    name: String,
    config_name: Option<String>,
    extension: Option<String>,
) -> Result<()> {
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
        log::info!("Editing: {:?}", config_files_to_edit);

        let mut editor_cmd = std::process::Command::new(settings.editor());
        for f in config_files_to_edit {
            let _ = editor_cmd.arg(f);
        }

        let status = editor_cmd.spawn()?.wait()?;
        if !status.success() {
            log::info!("Editor failed with exit status: {}", status);
        }
    } else {
        log::info!("No relevant config files found.");
    }

    Ok(())
}

fn edit_game_config_files(settings: &Settings, config_name: Option<String>) -> Result<()> {
    let mut config_files_to_edit = Vec::new();
    let mut game_my_document_dir = settings.compat_dir().unwrap().to_path_buf();
    game_my_document_dir.push(settings.game().steam_id().to_string());
    game_my_document_dir.push(settings.game().my_game_dir());

    if let Some(config_name) = config_name {
        game_my_document_dir.push(config_name);
        config_files_to_edit.push(game_my_document_dir);
    } else {
        WalkDir::new(game_my_document_dir.as_path())
            .min_depth(1)
            .max_depth(usize::MAX)
            .follow_links(false)
            .same_file_system(false)
            .contents_first(false)
            .into_iter()
            .filter_entry(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|f| settings.game().ini_files().contains(&f))
                    .unwrap_or(false)
            })
            .for_each(|f| {
                if let Ok(f) = f {
                    config_files_to_edit.push(f.into_path())
                }
            });
    }

    if !config_files_to_edit.is_empty() {
        log::info!("Editing: {:?}", config_files_to_edit);

        let mut editor_cmd = std::process::Command::new(settings.editor());
        for f in config_files_to_edit {
            editor_cmd.arg(f);
        }
        editor_cmd.spawn()?.wait()?;
    } else {
        log::info!("No relevant config files found.");
    }

    Ok(())
}

pub fn list_downloaded_files(download_dir: &Path, cache_dir: &Path) -> Result<()> {
    let sf = downloaded_files(download_dir);

    let mut table = create_table(vec!["Archive", "Status"]);

    for (_, f) in sf {
        let mut archive = PathBuf::from(cache_dir);
        let file = f.to_string_lossy().to_string().to_lowercase();
        archive.push(file.clone());
        archive.set_extension("ron");

        table.add_row(vec![
            Cell::new(f.to_string_lossy()).fg(Color::White),
            Cell::new(match archive.exists() && archive.is_file() {
                true => "Installed".to_string(),
                false => "New".to_string(),
            })
            .fg(Color::White),
        ]);
    }

    log::info!("{table}");
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

    for (idx, md) in mod_list.iter().enumerate() {
        let is_loser = conflict_list
            .get(&md.name().to_string())
            .map(|c| !c.losing_to().is_empty())
            .unwrap_or(false);
        let is_winner = conflict_list
            .get(&md.name().to_string())
            .map(|c| !c.winning_over().is_empty())
            .unwrap_or(false);

        let tag = Tag::from((is_loser, is_winner));

        // Detect if we all files of this manifest are overwritten by other mods
        let tag = if is_loser {
            let mut file_not_lost = false;
            let conflict_list = conflict_list_by_file(&mod_list)?;

            for f in md.dest_files() {
                if let Some(contenders) = conflict_list.get(&f) {
                    if let Some(c) = contenders.last() {
                        if c == md.name() {
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
        let tag = if md.is_enabled() { tag } else { Tag::Disabled };

        let color = Color::from(tag);

        table.add_row(vec![
            Cell::new(idx.to_string()).fg(color),
            Cell::new(md.name().to_string()).fg(color),
            Cell::new(md.priority().to_string()).fg(color),
            Cell::new(tag).fg(color),
            Cell::new(md.version().unwrap_or("<Unknown>").to_string()).fg(color),
            Cell::new(
                md.nexus_id()
                    .map(|nid| nid.to_string())
                    .unwrap_or("<Unknown>".to_owned()),
            )
            .fg(color),
            Cell::new(md.kind().to_string()).fg(color),
        ]);
    }

    log::info!("{table}");

    Ok(())
}

pub fn show_mod(cache_dir: &Path, mod_name: &str) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some(m) = find_mod(&mod_list, mod_name) {
        show_mod_status(&m, &mod_list)?;
    } else {
        log::info!("-> No mod found by that name: {}", mod_name);
    }

    Ok(())
}

pub fn show_mod_status(md: &Mod, mod_list: &[Mod]) -> Result<()> {
    let conflict_list_file = conflict_list_by_file(&mod_list)?;
    let conflict_list_mod = conflict_list_by_mod(&mod_list)?;

    let color = Color::White;

    let mut table = create_table(vec![
        "Name", "Priority", "Status", "Mod Type", "Version", "Nexus Id",
    ]);
    table.add_row(vec![
        Cell::new(md.name().to_string()).fg(color),
        Cell::new(md.priority().to_string()).fg(color),
        Cell::new(md.mod_state().to_string()).fg(color),
        Cell::new(md.kind().to_string()).fg(color),
        Cell::new(md.version().unwrap_or("<Unknown>").to_string()).fg(color),
        Cell::new(
            md.nexus_id()
                .map(|nid| nid.to_string())
                .unwrap_or("<Unknown>".to_owned()),
        )
        .fg(color),
    ]);

    log::info!("{table}");

    if let Some(conflict) = conflict_list_mod.get(&md.name().to_string()) {
        let mut table = create_table(vec![
            "Conflicting file",
            "This mod is overwritten by",
            "This mod overwrites",
        ]);

        for f in conflict.conflict_files() {
            let mut winners = Vec::new();
            let mut losers = Vec::new();

            if let Some(contenders) = conflict_list_file.get(f) {
                let mut found_self = false;
                for contender in contenders {
                    if contender == md.name() {
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
                    Cell::new(format!("{:?}", winners)).fg(color),
                    Cell::new(format!("{:?}", losers)).fg(color),
                ]);
            }
        }

        log::info!("");
        log::info!("{table}");
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

    log::info!("{table}");
    Ok(())
}

pub fn show_mod_files(cache_dir: &Path, mod_name: Option<String>) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list_file = conflict_list_by_file(&mod_list)?;

    if let Some(mod_name) = mod_name {
        if let Some(m) = find_mod(&mod_list, &mod_name) {
            show_files_for_mod(&m, &conflict_list_file)
        } else {
            log::warn!("Mod '{}' could not be found", mod_name);
        }
    } else {
        show_files(&mod_list, &conflict_list_file)
    }

    Ok(())
}

fn show_files(mod_list: &[Mod], conflict_list_file: &HashMap<String, Vec<String>>) {
    let mut files = Vec::new();

    for m in mod_list {
        files.extend(m.files().iter().map(|i| (i, (m.name(), m.priority()))));
    }

    files.sort_unstable_by(|(ia, (_, pa)), (ib, (_, pb))| {
        let o = ia.destination().cmp(ib.destination());
        if o == Ordering::Equal {
            pa.cmp(pb)
        } else {
            o
        }
    });

    log::info!("File overview");
    log::info!("");
    let mut table = create_table(vec!["File", "Destination", "Mod"]);

    for (isf, (name, _priority)) in files {
        let mut color = Color::White;
        if conflict_list_file.contains_key(&isf.destination().to_string()) {
            color = if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == name
            {
                Color::Green
            } else {
                Color::Red
            };
        }

        table.add_row(vec![
            Cell::new(isf.source().to_string_lossy().to_string()).fg(color),
            Cell::new(isf.destination().to_string()).fg(color),
            Cell::new(name).fg(color),
        ]);
    }

    log::info!("{table}");
}

fn show_files_for_mod(m: &Mod, conflict_list_file: &HashMap<String, Vec<String>>) {
    log::info!("File overview of {}", m.name());
    log::info!("");
    let mut table = create_table(vec!["File", "Destination"]);

    let mut files = m.files().to_vec();
    files.sort_unstable();

    for f in files {
        let mut color = Color::White;
        if conflict_list_file.contains_key(&f.destination().to_string()) {
            color = if conflict_list_file
                .get(&f.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == m.name()
            {
                Color::Green
            } else {
                Color::Red
            };
        }

        table.add_row(vec![
            Cell::new(f.source().to_string_lossy().to_string()).fg(color),
            Cell::new(f.destination().to_string()).fg(color),
            Cell::new(m.name()).fg(color),
        ]);
    }

    log::info!("{table}");

    if !m.disabled_files().is_empty() {
        log::info!("");
        let mut table = create_table(vec!["Disabled Files"]);

        for d in m.disabled_files() {
            table.add_row(vec![d
                .source()
                .strip_prefix(m.manifest_dir())
                .unwrap()
                .to_string_lossy()
                .to_string()]);
        }

        log::info!("{table}");
    }
}

pub fn show_conflicts(cache_dir: &Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list_file = conflict_list_by_file(&mod_list)?;
    let mut files = Vec::new();

    for m in mod_list {
        files.extend(
            m.files()
                .iter()
                .map(|i| (i.clone(), (m.name().to_owned(), m.priority()))),
        );
    }

    files.retain(|(f, _)| conflict_list_file.contains_key(f.destination()));

    files.sort_unstable_by(|(ia, (_, pa)), (ib, (_, pb))| {
        let o = ia.destination().cmp(ib.destination());
        if o == Ordering::Equal {
            pa.cmp(pb)
        } else {
            o
        }
    });

    log::info!("Conflict overview");
    log::info!("");
    let mut table = create_table(vec!["File", "Mod"]);

    for (isf, (name, _priority)) in files {
        let mut color = Color::White;
        if conflict_list_file.contains_key(&isf.destination().to_string()) {
            color = if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == &name
            {
                Color::Green
            } else {
                Color::Red
            };
        }

        table.add_row(vec![
            Cell::new(isf.destination().to_string()).fg(color),
            Cell::new(name).fg(color),
        ]);
    }

    log::info!("{table}");
    Ok(())
}
