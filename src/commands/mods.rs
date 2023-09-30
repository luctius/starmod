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
    enable::{disable_all, disable_mod, enable_all, enable_mod},
    modlist::{self, find_mod, gather_mods},
    mods::{Mod, ModKind, ModList},
    settings::{create_table, Settings},
};

use super::list::list_mods;

#[derive(Debug, Clone, Parser, Default)]
pub enum ModCmd {
    CopyToCustom {
        origin_mod: String,
        custom_mod: String,
        file_name: String,
    },
    CreateLabel {
        name: String,
    },
    CreateCustom {
        name: String,
        origin: Option<Utf8PathBuf>,
    },
    Disable {
        name: String,
    },
    DisableAll,
    EditConfig {
        name: String,
        destination_mod_name: String,
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
    #[default]
    List,
    Show {
        name: String,
    },
    ReEnableAll,
    Remove {
        name: String,
    },
    Rename {
        old_mod_name: String,
        new_mod_name: String,
    },
    SetPrio {
        name: String,
        priority: isize,
    },
    SetPriority {
        name: String,
        priority: isize,
    },
}
impl ModCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Disable { name } => {
                disable_mod(&settings.cache_dir(), &settings.game_dir(), &name)?;
                list_mods(&settings.cache_dir())
            }
            Self::DisableAll => {
                disable_all(&settings.cache_dir(), &settings.game_dir())?;
                list_mods(&settings.cache_dir())
            }
            Self::Enable { name, priority } => {
                enable_mod(&settings.cache_dir(), &settings.game_dir(), &name, priority)?;
                list_mods(&settings.cache_dir())
            }
            Self::EnableAll => {
                enable_all(&settings.cache_dir(), &settings.game_dir())?;
                list_mods(&settings.cache_dir())
            }
            Self::EditConfig {
                name,
                destination_mod_name,
                config_name,
                extension,
            } => edit_mod_config_files(
                &settings,
                name,
                destination_mod_name,
                config_name,
                extension,
            ),
            Self::List => list_mods(&settings.cache_dir()),
            Self::Show { name } => show_mod(&settings.cache_dir(), &name),
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
                    ModKind::Custom.create_mod(&settings.cache_dir(), &Utf8PathBuf::from(name))?;
                Ok(())
            }
            Self::CreateLabel { name } => {
                let destination = settings.cache_dir().join(&name);
                log::info!("Creating label {}", &name);
                DirBuilder::new().recursive(true).create(destination)?;
                let _ =
                    ModKind::Label.create_mod(&settings.cache_dir(), &Utf8PathBuf::from(name))?;
                Ok(())
            }
            Self::Remove { name } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some((mut md, _idx)) = find_mod(&mod_list, &name) {
                    md.disable(&settings.cache_dir(), &settings.game_dir())?;
                    md.remove(&settings.cache_dir())?;
                    log::info!("Removed mod '{}'", md.name());
                    list_mods(&settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{name}' not found.")
                }
                Ok(())
            }
            Self::ReEnableAll {} => {
                let mut mod_list = gather_mods(&settings.cache_dir())?;
                mod_list.re_enable(&settings.cache_dir(), &settings.game_dir())?;
                log::info!("Mods re-enabled.");
                list_mods(&settings.cache_dir())?;
                Ok(())
            }
            Self::Rename {
                old_mod_name,
                new_mod_name,
            } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some((mut m, _idx)) = find_mod(&mod_list, &old_mod_name) {
                    m.set_name(new_mod_name)?;
                    list_mods(&settings.cache_dir())?;
                } else {
                    log::warn!("Mod '{old_mod_name}' not found.")
                }
                Ok(())
            }
            Self::SetPrio { name, priority } | Self::SetPriority { name, priority } => {
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some((mut m, _idx)) = find_mod(&mod_list, &name) {
                    m.set_priority(priority)?;
                    if priority < 0 {
                        m.disable(&settings.cache_dir(), &settings.game_dir())?;
                    }
                    crate::commands::list::list_mods(&settings.cache_dir())?;
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
                let mod_list = gather_mods(&settings.cache_dir())?;
                if let Some((origin_mod, _idx)) = find_mod(&mod_list, &origin_mod) {
                    if let Some((custom_mod, _idx)) = find_mod(&mod_list, &custom_mod) {
                        if let Some(file) = origin_mod
                            .origin_files()?
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
                                &Utf8PathBuf::from(custom_mod.name()),
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
        }
    }
}

fn show_mod(cache_dir: &Utf8Path, mod_name: &str) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some((_, idx)) = find_mod(&mod_list, mod_name) {
        show_mod_status(&mod_list[idx], &mod_list)?;
    } else {
        log::info!("-> No mod found by that name: {}", mod_name);
    }

    Ok(())
}

fn show_mod_status(md: &Mod, mod_list: &[Mod]) -> Result<()> {
    let conflict_list_file = conflict_list_by_file(&mod_list)?;

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
    let mod_list = gather_mods(&settings.cache_dir())?;
    if let Some((md, _idx)) = modlist::find_mod(&mod_list, &name) {
        let config_list = md.find_config_files(extension.as_deref())?;
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
        if let Some(destination_manifest) = modlist::find_mod(&mod_list, &destination_mod_name) {
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
