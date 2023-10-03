use std::{
    cmp::Ordering,
    fs::{copy, DirBuilder},
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use comfy_table::{Cell, Color};

use crate::{
    conflict::conflict_list_by_file,
    manifest::Manifest,
    mods::{FindInModList, GatherModList, ModKind, ModList},
    settings::{create_table, Settings},
};

use super::list::list_mods;

#[derive(Debug, Clone, Parser, Default)]
pub enum ModCmd {
    /// Copy 'file_name' from mod 'origin_mod' to mod 'custom_mod'
    CopyToCustom {
        origin_mod: String,
        custom_mod: String,
        file_name: String,
    },
    /// Create a new label with 'name'
    CreateLabel { name: String },
    /// Create a custom mod 'name', optionally, instead of creating a directory, link to 'origin'
    CreateCustom {
        name: String,
        origin: Option<Utf8PathBuf>,
    },
    /// Disable mod 'name'
    #[clap(visible_aliases = &["dis", "d"])]
    Disable { name: String },
    /// Disable all mods
    DisableAll,
    /// Disable 'file_name' from mod 'mod_name'
    DisableFile { mod_name: String, file_name: String },
    /// Copy file from mod 'name' 'config_name' to mod 'custom_mod_name' and run 'EDITOR' or 'xdg-open' on the new file.
    EditConfig {
        name: String,
        destination_mod_name: String,
        #[arg(short, long)]
        config_name: Option<String>,
        #[arg(short, long)]
        extension: Option<String>,
    },
    /// Enable mod 'name', optionally with priority 'priority'
    #[clap(visible_aliases = &["en", "e"])]
    Enable {
        name: String,
        priority: Option<isize>,
    },
    /// Enable all mods
    EnableAll,
    #[default]
    #[clap(visible_aliases = &["lists","l"])]
    /// Show all mods; Alias from 'mod list'
    List,
    #[clap(visible_alias = "s")]
    /// Show the details of mod 'name'
    Show { name: String },
    /// Remove mod 'name' from installation.
    /// Does not remove the mod from the downloads directory.
    Remove { name: String },
    /// Rename mod 'old_mod_name' to 'new_mod_name'
    #[clap(visible_aliases = &["ren", "r"])]
    Rename {
        old_mod_name: String,
        new_mod_name: String,
    },
    /// Set mod to new priority;
    /// Setting a priority below zero disables the mod.
    #[clap(visible_aliases = &["set-prio", "sp"])]
    SetPriority { name: String, priority: isize },
}
impl ModCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Disable { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    list_mods(settings.cache_dir())
                } else {
                    log::info!("Couldn't find a mod by name '{name}'");
                    Ok(())
                }
            }
            Self::DisableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.disable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings.cache_dir())
            }
            Self::DisableFile {
                mod_name,
                file_name,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&mod_name) {
                    if !mod_list[idx].disable_file(&file_name)? {
                        log::info!("Couldn't find a file by name '{file_name}' in mod: {mod_name}");
                    }
                    Ok(())
                } else {
                    log::info!("Couldn't find a mod by name '{mod_name}'");
                    Ok(())
                }
            }
            Self::Enable { name, priority } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    if let Some(prio) = priority {
                        mod_list[idx].set_priority(prio)?;
                    }
                    mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    list_mods(settings.cache_dir())
                } else {
                    log::info!("Couldn't find a mod by name '{name}'");
                    Ok(())
                }
            }
            Self::EnableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.enable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings.cache_dir())
            }
            Self::EditConfig {
                name,
                destination_mod_name,
                config_name,
                extension,
            } => {
                edit_mod_config_files(settings, name, destination_mod_name, config_name, extension)
            }
            Self::List => list_mods(settings.cache_dir()),
            Self::Show { name } => show_mod(settings.cache_dir(), &name),
            Self::CreateCustom { origin, name } => {
                let destination = settings.cache_dir().join(&name);
                if let Some(origin) = origin {
                    std::os::unix::fs::symlink(&origin, &destination)?;
                    log::info!("Creating custom mod {} (link from {})", &name, origin);
                } else {
                    log::info!("Creating custom mod {}", &name);
                    DirBuilder::new().recursive(true).create(destination)?;
                }
                let _ =
                    ModKind::Custom.create_mod(settings.cache_dir(), &Utf8PathBuf::from(name))?;
                Ok(())
            }
            Self::CreateLabel { name } => {
                todo!()
                // let destination = settings.cache_dir().join(&name);
                // log::info!("Creating label {}", &name);
                // DirBuilder::new().recursive(true).create(destination)?;
                // let _ =
                //     ModKind::Label.create_mod(settings.cache_dir(), &Utf8PathBuf::from(name))?;
                // Ok(())
            }
            Self::Remove { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    mod_list[idx].remove()?;
                    log::info!("Removed mod '{}'", mod_list[idx].name());
                    list_mods(settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Self::Rename {
                old_mod_name,
                new_mod_name,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&old_mod_name) {
                    mod_list[idx].set_name(new_mod_name)?;
                    list_mods(settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{old_mod_name}' not found.")
                }
                Ok(())
            }
            Self::SetPriority { name, priority } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    mod_list[idx].set_priority(priority)?;
                    if priority < 0 {
                        mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    }
                    crate::commands::list::list_mods(settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Self::CopyToCustom {
                origin_mod,
                custom_mod,
                file_name,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(origin_idx) = mod_list.find_mod(&origin_mod) {
                    if let Some(custom_idx) = mod_list.find_mod(&custom_mod) {
                        //TODO check that custom_mod is indeed a custom mod

                        if let Some(file) = mod_list[origin_idx]
                            .origin_files()?
                            .iter()
                            .find(|f| f.file_name().unwrap().eq(file_name.as_str()))
                        {
                            let origin = settings.cache_dir().join(file);
                            let destination = settings
                                .cache_dir()
                                .join(mod_list[custom_idx].manifest_dir())
                                .join(
                                    file.strip_prefix(mod_list[origin_idx].manifest_dir())
                                        .unwrap(),
                                );

                            DirBuilder::new()
                                .recursive(true)
                                .create(destination.parent().unwrap())?;
                            copy(origin, destination)?;
                        } else {
                            log::warn!(
                                "File '{}' could not be found in mod '{}'.",
                                file_name,
                                mod_list[origin_idx].name()
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
        }
    }
}

fn show_mod(cache_dir: &Utf8Path, mod_name: &str) -> Result<()> {
    let mod_list = Vec::gather_mods(cache_dir)?;
    if let Some(idx) = mod_list.find_mod(mod_name) {
        show_mod_status(&mod_list, idx)?;
    } else {
        log::info!("-> No mod found by that name: {}", mod_name);
    }

    Ok(())
}

fn show_mod_status(mod_list: &[Manifest], idx: usize) -> Result<()> {
    let conflict_list_file = conflict_list_by_file(&mod_list)?;
    let md = &mod_list[idx];

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

    let mut files = md
        .files()?
        .iter()
        .map(|i| (i.clone(), (md.name(), md.priority())))
        .collect::<Vec<_>>();

    files.sort_unstable_by(|(ia, (_, pa)), (ib, (_, pb))| {
        let o = ia.destination().cmp(ib.destination());
        if o == Ordering::Equal {
            pa.cmp(pb)
        } else {
            o
        }
    });

    log::info!("");
    let mut table = create_table(vec!["File", "Destination"]);

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
            Cell::new(isf.source().to_string()).fg(color),
            Cell::new(isf.destination().to_string()).fg(color),
        ]);
    }

    log::info!("{table}");

    log::info!("");
    let mut table = create_table(vec!["Disabled File"]);

    let color = Color::Grey;
    for isf in md.disabled_files()? {
        table.add_row(vec![Cell::new(isf.source().to_string()).fg(color)]);
    }

    log::info!("{table}");

    Ok(())
}

fn edit_mod_config_files(
    settings: &Settings,
    name: String,
    destination_mod_name: String,
    config_name: Option<String>,
    extension: Option<String>,
) -> Result<()> {
    let mut config_files_to_edit = Vec::new();
    let mod_list = Vec::gather_mods(settings.cache_dir())?;
    if let Some(idx) = mod_list.find_mod(&name) {
        let config_list = mod_list[idx].find_config_files(extension.as_deref())?;
        if let Some(config_name) = config_name {
            if let Some(cf) = config_list
                .iter()
                .find(|f| f.file_name().unwrap_or_default() == config_name)
            {
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
        if let Some(destination_manifest) = mod_list.find_mod(&destination_mod_name) {
            todo!();
            // for f in config_files_to_edit {
            //     f.strip_prefix(settings.cache_dir())
            //         .unwrap()
            //         .strip_prefix(manifest)
            // }

            log::info!("Editing: {:?}", config_files_to_edit);

            let mut editor_cmd = std::process::Command::new(settings.editor());
            for f in config_files_to_edit {
                let _ = editor_cmd.arg(f);
            }

            let status = editor_cmd.spawn()?.wait()?;
            if !status.success() {
                log::info!("Editor failed with exit status: {}", status);
            }
        }
    } else {
        log::info!("No relevant config files found.");
    }

    Ok(())
}
