use std::{
    cmp::Ordering,
    fs::{copy, DirBuilder},
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use comfy_table::{Cell, Color};
use inquire::CustomType;

use crate::{
    conflict::conflict_list_by_file,
    errors::ModErrors,
    manifest::{install_file::SelectFile as _, Manifest},
    mods::{FindInModList, GatherModList, ModKind, ModList},
    settings::{create_table, Settings},
    ui::{InquireBuilder, ModListBuilder},
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
        source: Option<String>,
        /// The destination mod to copy <file_name> to.
        destination: Option<String>,
        /// The <file_name> from <source> mod to copy.
        file: Option<String>,
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
        name: Option<String>,
    },
    /// Disable all mods
    DisableAll,
    /// Disable 'file_name' from mod 'mod_name'
    DisableFile {
        /// Name of the mod which hosts <file>
        name: Option<String>,
        /// File to disable
        file: Option<String>,
    },
    /// Find either <config_name> or all files with <extension> in mod <name>. Then optionally copy those files to <custom_mod>. Finally run the configured editor, which was taken from '$EDITOR', or use 'xdg-open', on those files.
    EditConfig {
        /// name of the mod which hosts the config file
        name: Option<String>,
        /// name of the mod which should host the modified config file
        #[arg(short, long)]
        destination: Option<Option<String>>,
        /// Config file name, should not be used together with <--extention>
        #[arg(short, long, group = "config")]
        config_name: Option<String>,
        /// Config file extention. Should not be used together with <--config_name>
        #[arg(short, long, group = "config")]
        extension: Option<String>,
    },
    /// Enable mod 'name'
    #[clap(visible_aliases = &["en", "e"])]
    Enable {
        /// Name of the mod to enable
        name: Option<String>,
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
        name: Option<String>,
    },
    /// Add tag <tag> to mod <name>
    TagAdd {
        /// Name of the tag
        tag: String,
        /// Name of the mod to add <tag> to.
        name: Option<String>,
    },
    /// Remove tag <tag> from mod <name>
    TagRemove {
        /// Name of the tag.
        tag: String,
        /// Name of the mod to add <tag> to.
        name: Option<String>,
    },
    /// Remove mod 'name' from installation.
    /// Does not remove the mod from the downloads directory.
    Remove {
        /// Name of the mod to remove from the mod-list..
        name: Option<String>,
    },
    /// Rename mod 'old_mod_name' to 'new_mod_name'
    #[clap(visible_aliases = &["ren", "r"])]
    Rename {
        new_mod_name: String,
        old_mod_name: Option<String>,
    },
    /// Set mod to new priority;
    /// Setting a priority below zero disables the mod.
    #[clap(visible_aliases = &["set-prio", "sp"])]
    SetPriority {
        /// Name of the mod to set to the new priority
        name: Option<String>,
        /// value of the new priority.
        /// Setting this below zero permanently disabled the mod.
        priority: Option<isize>,
    },
}
impl ModCmd {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Self::Disable { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(name.as_deref()) {
                    mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    list_mods(settings)
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(name.unwrap_or_default()).into())
                }
            }
            Self::DisableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.disable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings)
            }
            Self::DisableFile {
                name: mod_name,
                file,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(mod_name.as_deref()) {
                    let file_name = if let Some(file) = file {
                        file
                    } else {
                        mod_list[idx]
                            .files()?
                            .select()
                            .unwrap_or_default()
                            .to_string()
                    };

                    if mod_list[idx].disable_file(&file_name) {
                        if mod_list[idx].is_enabled() {
                            mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                        }
                        Ok(())
                    } else {
                        // log::trace!("File '{file_name}' not found within mod '{mod_name}'.");
                        Err(ModErrors::FileNotFound(mod_name.unwrap_or_default(), file_name).into())
                    }
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(mod_name.unwrap_or_default()).into())
                }
            }
            Self::Enable { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(name.as_deref()) {
                    mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    list_mods(settings)
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(name.unwrap_or_default()).into())
                }
            }
            Self::EnableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.enable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings)
            }
            Self::EditConfig {
                name,
                destination,
                config_name,
                extension,
            } => edit_mod_config_files(
                settings,
                name.as_deref(),
                destination,
                &config_name,
                &extension,
            ),
            Self::List => list_mods(settings),
            Self::Show { name } => show_mod(settings.cache_dir(), name.as_deref()),
            Self::CreateCustom { origin, name } => {
                let destination = settings.cache_dir().join(&name);
                if let Some(origin) = origin {
                    std::os::unix::fs::symlink(&origin, &destination)?;
                    log::info!("Creating custom mod {} (link from {})", &name, origin);
                } else {
                    log::info!("Creating custom mod {}", &name);
                    DirBuilder::new().recursive(true).create(destination)?;
                }
                ModKind::Custom
                    .create_mod(settings.cache_dir(), &Utf8PathBuf::from(name))
                    .map(|_| ())
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
                if let Some(idx) = mod_list.find_mod(name.as_deref()) {
                    mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    mod_list[idx].remove()?;
                    log::info!("Removed mod '{}'", mod_list[idx].name());
                    list_mods(settings)
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(name.unwrap_or_default()).into())
                }
            }
            Self::Rename {
                old_mod_name,
                new_mod_name,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(old_mod_name.as_deref()) {
                    mod_list[idx].set_name(new_mod_name)?;
                    list_mods(settings)
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(old_mod_name.unwrap_or_default()).into())
                }
            }
            Self::SetPriority { name, priority } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(name.as_deref()) {
                    let old_prio = mod_list[idx].priority();

                    let priority = if let Some(priority) = priority {
                        priority
                    } else {
                        CustomType::new("Please specify the new priority")
                            // .with_formatter(&|i| format!("${}", i))
                            .with_error_message("Please type a valid number")
                            .with_help_message("Type in a positive or negative number.")
                            .prompt()?
                    };

                    mod_list[idx].set_priority(priority)?;
                    if mod_list[idx].is_disabled() {
                        let priority = if priority > old_prio {
                            priority
                        } else {
                            old_prio
                        };

                        (&mut mod_list[0..priority as usize])
                            .re_enable(settings.cache_dir(), settings.game_dir())?;
                    }

                    crate::commands::list::list_mods(settings)?;
                    Ok(())
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(name.unwrap_or_default()).into())
                }
            }
            Self::TagAdd { name, tag } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(name.as_deref()) {
                    if mod_list[idx].add_tag(&tag)? {
                        // log::info!("Added tag {tag} to mod {name}.");
                        Ok(())
                    } else {
                        // log::trace!("Unable to add tag {tag} to mod {name}.");
                        Err(ModErrors::DuplicateTag(name.unwrap_or_default(), tag).into())
                    }
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(name.unwrap_or_default()).into())
                }
            }
            Self::TagRemove { name, tag } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(idx) = mod_list.find_mod(name.as_deref()) {
                    if mod_list[idx].remove_tag(&tag)? {
                        // log::info!("Removed tag {tag} from mod {name}.");
                        Ok(())
                    } else {
                        // log::trace!("Unable to remove tag {tag} from mod {name}.");
                        Err(ModErrors::TagNotFound(name.unwrap_or_default(), tag).into())
                    }
                } else {
                    // log::trace!("Mod '{name}' not found.");
                    Err(ModErrors::ModNotFound(name.unwrap_or_default()).into())
                }
            }
            Self::CopyToCustom {
                source: origin_mod,
                destination: custom_mod,
                file,
            } => {
                let mod_list = Vec::gather_mods(settings.cache_dir())?;
                if let Some(origin_idx) = mod_list.find_mod(origin_mod.as_deref()) {
                    if let Some(custom_idx) = mod_list.find_mod(custom_mod.as_deref()) {
                        //TODO check that custom_mod is indeed a custom mod
                        let file_name = if let Some(file) = file {
                            file
                        } else {
                            mod_list[origin_idx]
                                .files()?
                                .select()
                                .unwrap_or_default()
                                .to_string()
                        };

                        if let Some(file) = mod_list[origin_idx]
                            .origin_files()?
                            .iter()
                            .find(|f| f.file_name().unwrap().eq(&file_name))
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
                            Ok(())
                        } else {
                            // log::trace!(
                            //     "File '{}' could not be found in mod '{}'.",
                            //     file_name,
                            //     mod_list[origin_idx].name()
                            // );
                            Err(ModErrors::FileNotFound(
                                mod_list[origin_idx].name().to_string(),
                                file_name,
                            )
                            .into())
                        }
                    } else {
                        // log::trace!("Mod '{}' could not be found", custom_mod);
                        Err(ModErrors::ModNotFound(custom_mod.unwrap_or_default()).into())
                    }
                } else {
                    // log::trace!("Mod '{}' could not be found", origin_mod);
                    Err(ModErrors::ModNotFound(origin_mod.unwrap_or_default()).into())
                }
            }
        }
    }
}

fn show_mod(cache_dir: &Utf8Path, mod_name: Option<&str>) -> Result<()> {
    let mod_list = Vec::gather_mods(cache_dir)?;

    let select = ModListBuilder::new(&mod_list)
        .with_index()
        .with_priority()
        .with_status()
        .with_version()
        .with_nexus_id()
        .with_mod_type()
        .with_tags()
        // .with_notes(settings.download_dir())
        .with_colour()
        // .with_headers()
        .build()
        .ok()?;

    let idx = InquireBuilder::new_with_test(mod_list.find_mod(mod_name), select).prompt()?;

    show_mod_status(&mod_list, idx)

    // if let Some(idx) =  {
    //     show_mod_status(&mod_list, idx)?;
    //     Ok(())
    // } else {
    //     log::trace!("Mod '{}' could not be found", mod_name.unwrap_or_default());
    //     Err(ModErrors::ModNotFound(mod_name.unwrap_or_default().to_string()).into())
    // }
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
    name: Option<&str>,
    destination_mod_name: Option<Option<String>>,
    config_name: &Option<String>,
    extension: &Option<String>,
) -> Result<()> {
    let mod_list = Vec::gather_mods(settings.cache_dir())?;
    let mod_idx = mod_list.find_mod(name);

    if mod_idx.is_none() {
        log::trace!("Source mod '{}' not found.", name.unwrap_or_default());
        return Err(ModErrors::ModNotFound(name.unwrap_or_default().to_string()))?;
    }

    let name = mod_list[mod_idx.unwrap()].name();

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
        let mut editor_cmd = std::process::Command::new(settings.editor());
        if let Some(destination_mod_name) = destination_mod_name {
            // Copy
            if let Some(idx) = mod_list.find_mod(destination_mod_name.as_deref()) {
                let manifest = &mod_list[idx];

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
            }
        } else {
            for (source, _) in &config_files_to_edit {
                let _ = editor_cmd.arg(source);
            }
        }

        log::info!("Running '{:?}'", editor_cmd);

        let status = editor_cmd.spawn()?.wait()?;
        if !status.success() {
            log::info!("Editor failed with exit status: {}", status);
        }
    } else {
        log::trace!("No relevant config files found.");
        return Err(ModErrors::FileNotFound(
            name.to_string(),
            config_files_to_edit
                .iter()
                .map(|(f, _)| f.file_name().unwrap().to_string())
                .collect::<Vec<_>>()
                .join(","),
        ))?;
    }

    Ok(())
}
