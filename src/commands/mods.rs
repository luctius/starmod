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

//TODO: create custom and tag sub-commands

/// Commands related to mods; defaults to showing the mod-list
#[derive(Debug, Clone, Parser, Default)]
#[clap(
    after_help = "Note: The name of a mod can be (a part of) the litteral name or the index in the mod-list."
)]
pub enum ModCmd {
    /// Copy 'file_name' from mod 'origin_mod' to mod 'custom_mod'
    CopyToCustom {
        /// The source mod to copy <file_name> from.
        source: String,
        /// The destination mod to copy <file_name> to.
        destination: String,
        /// The <file_name> from <source> mod to copy.
        file: String,
    },
    /// Create a new label with 'name'
    CreateLabel {
        /// Name of the label
        name: String,
    },
    /// Create a custom mod 'name', optionally, instead of creating a directory, link to 'origin'
    CreateCustom {
        /// Name of the new custom mod.
        name: String,
        /// Path to the underlying directory which will be symlinked into the cache directory.
        origin: Option<Utf8PathBuf>,
    },
    /// Disable mod 'name'
    #[clap(visible_aliases = &["dis", "d"])]
    Disable {
        /// Name of the mod to disable
        name: String,
    },
    /// Disable all mods
    DisableAll,
    /// Disable 'file_name' from mod 'mod_name'
    DisableFile {
        /// Name of the mod which hosts <file>
        name: String,
        file: String,
    },
    /// Copy file from mod 'name' 'config_name' to mod 'custom_mod_name' and run 'EDITOR' or 'xdg-open' on the new file.
    EditConfig {
        /// name of the mod which hosts the config file
        name: String,
        /// name of the mod which should host the modified config file
        destination_mod_name: String,
        /// Config file name, should not be used together with <--extention>
        #[arg(short, long, group = "config")]
        config_name: Option<String>,
        /// Config file extention. Should not be used together with <--config_name>
        #[arg(short, long, group = "config")]
        extension: Option<String>,
    },
    /// Enable mod 'name', optionally with priority 'priority'
    #[clap(visible_aliases = &["en", "e"])]
    Enable {
        /// Name of the mod to enable
        name: String,
        /// Optional: set mod to <priority> before enabling
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
    Show {
        /// Name of the mod to show.
        name: String,
    },
    /// Add tag <tag> to mod <name>
    TagAdd {
        /// Name of the mod to add <tag> to.
        name: String,
        /// Name of the tag
        tag: String,
    },
    /// Remove tag <tag> from mod <name>
    TagRemove {
        /// Name of the mod to add <tag> to.
        name: String,
        /// Name of the tag.
        tag: String,
    },
    /// Remove mod 'name' from installation.
    /// Does not remove the mod from the downloads directory.
    Remove {
        /// Name of the mod to remove from the mod-list..
        name: String,
    },
    /// Rename mod 'old_mod_name' to 'new_mod_name'
    #[clap(visible_aliases = &["ren", "r"])]
    Rename {
        old_mod_name: String,
        new_mod_name: String,
    },
    /// Set mod to new priority;
    /// Setting a priority below zero disables the mod.
    #[clap(visible_aliases = &["set-prio", "sp"])]
    SetPriority {
        /// Name of the mod to set to the new priority
        name: String,
        /// value of the new priority.
        /// Setting this below zero permanently disabled the mod.
        priority: isize,
    },
}
impl ModCmd {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Self::Disable { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    list_mods(settings)
                } else {
                    log::info!("Couldn't find a mod by name '{name}'");
                    Ok(())
                }
            }
            Self::DisableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.disable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings)
            }
            Self::DisableFile {
                name: mod_name,
                file: file_name,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.find_mod(&mod_name).map_or_else(
                    || {
                        log::info!("Couldn't find a mod by name '{mod_name}'");
                        Ok(())
                    },
                    |idx| {
                        if !mod_list[idx].disable_file(&file_name) {
                            log::info!(
                                "Couldn't find a file by name '{file_name}' in mod: {mod_name}"
                            );
                        }
                        Ok(())
                    },
                )
            }
            Self::Enable { name, priority } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    if let Some(prio) = priority {
                        mod_list[idx].set_priority(prio)?;
                    }
                    mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    list_mods(settings)
                } else {
                    log::info!("Couldn't find a mod by name '{name}'");
                    Ok(())
                }
            }
            Self::EnableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.enable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings)
            }
            Self::EditConfig {
                name,
                destination_mod_name,
                config_name,
                extension,
            } => edit_mod_config_files(
                settings,
                &name,
                &destination_mod_name,
                &config_name,
                &extension,
            ),
            Self::List => list_mods(settings),
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
            Self::CreateLabel { name: _ } => {
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
                    list_mods(settings)?;
                } else {
                    log::warn!("Mod '{name}' not found.");
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
                    list_mods(settings)?;
                } else {
                    log::warn!("Mod '{old_mod_name}' not found.");
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
                    crate::commands::list::list_mods(settings)?;
                } else {
                    log::warn!("Mod '{name}' not found.");
                }
                Ok(())
            }
            Self::TagAdd { name, tag } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    if mod_list[idx].add_tag(&tag)? {
                        log::info!("Added tag {tag} to mod {name}.");
                    } else {
                        log::warn!("Unable to add tag {tag} to mod {name}.");
                    }
                } else {
                    log::warn!("Mod '{name}' not found.");
                }
                Ok(())
            }
            Self::TagRemove { name, tag } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(&name) {
                    if mod_list[idx].remove_tag(&tag)? {
                        log::info!("Removed tag {tag} from mod {name}.");
                    } else {
                        log::warn!("Unable to remove tag {tag} from mod {name}.");
                    }
                } else {
                    log::warn!("Mod '{name}' not found.");
                }
                Ok(())
            }
            Self::CopyToCustom {
                source: origin_mod,
                destination: custom_mod,
                file: file_name,
            } => {
                let mod_list = Vec::gather_mods(settings.cache_dir())?;
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
    let conflict_list_file = conflict_list_by_file(mod_list)?;
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
                .map_or("<Unknown>".to_owned(), |nid| nid.to_string()),
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
        let color = if conflict_list_file.contains_key(&isf.destination().to_string()) {
            if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == name
            {
                Color::Green
            } else {
                Color::Red
            }
        } else {
            Color::White
        };

        table.add_row(vec![
            Cell::new(isf.source().to_string()).fg(color),
            Cell::new(isf.destination().to_string()).fg(color),
        ]);
    }

    table.add_row_if(|idx, _row| idx.eq(&0), vec![Cell::new("No files found.")]);

    log::info!("{table}");

    log::info!("");

    if !md.disabled_files().is_empty() {
        let mut table = create_table(vec!["Disabled File"]);

        let color = Color::Grey;
        for isf in md.disabled_files() {
            table.add_row(vec![Cell::new(isf.source().to_string()).fg(color)]);
        }

        log::info!("{table}");
    }

    Ok(())
}

fn edit_mod_config_files(
    settings: &Settings,
    name: &str,
    destination_mod_name: &str,
    config_name: &Option<String>,
    extension: &Option<String>,
) -> Result<()> {
    let mod_list = Vec::gather_mods(settings.cache_dir())?;
    let mod_idx = mod_list.find_mod(&name);

    if mod_idx.is_none() {
        log::info!("Source mod '{}' not found.", name);
        return Ok(());
    }

    let config_files_to_edit = if let Some(idx) = mod_idx {
        let manifest = &mod_list[idx];
        let config_list = manifest.find_config_files(extension.as_deref())?;
        if let Some(config_name) = config_name {
            if let Some(cf) = config_list
                .iter()
                .find(|f| f.file_name().unwrap_or_default() == config_name)
            {
                let config_path = settings.cache_dir().join(cf);
                vec![(
                    config_path,
                    cf.strip_prefix(manifest.manifest_dir())?.to_path_buf(),
                )]
            } else {
                Vec::new()
            }
        } else {
            let mut list = Vec::new();
            for cf in config_list {
                let config_path = settings.cache_dir().to_path_buf().join(&cf);
                list.push((
                    config_path,
                    cf.strip_prefix(manifest.manifest_dir())?.to_path_buf(),
                ));
            }
            list
        }
    } else {
        Vec::new()
    };

    if !config_files_to_edit.is_empty() {
        if let Some(idx) = mod_list.find_mod(&destination_mod_name) {
            let manifest = &mod_list[idx];

            let mut editor_cmd = std::process::Command::new(settings.editor());
            for (source, dest) in &config_files_to_edit {
                let dest = settings
                    .cache_dir()
                    .join(manifest.manifest_dir())
                    .join(dest);
                log::trace!("Copying config file {} to {}", source, &dest);

                DirBuilder::new()
                    .recursive(true)
                    .create(dest.parent().unwrap())?;

                copy(source, &dest)?;
                let _ = editor_cmd.arg(dest);
            }

            log::info!(
                "Running '{} {}'",
                settings.editor(),
                config_files_to_edit
                    .iter()
                    .map(|(_, d)| d)
                    .map(|d| settings
                        .cache_dir()
                        .join(manifest.manifest_dir())
                        .join(d)
                        .to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );

            let status = editor_cmd.spawn()?.wait()?;
            if !status.success() {
                log::info!("Editor failed with exit status: {}", status);
            }
        } else {
            log::info!("Destination mod not found.");
        }
    } else {
        log::info!("No relevant config files found.");
    }

    Ok(())
}
