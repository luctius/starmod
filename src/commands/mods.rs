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
    manifest::Manifest,
    mods::{FindInModList, GatherModList, ModKind, ModList},
    settings::{create_table, Settings},
    ui::{FileListBuilder, FindSelectBuilder, InquireBuilder},
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
        name: Option<String>,
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
    /// Enable 'file_name' from mod 'mod_name'
    EnableFile {
        /// Name of the mod which hosts <file>
        name: Option<String>,
        /// File to enable
        file: Option<String>,
    },
    //TODO: Enable File
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
        /// Name of the mod to add <tag> to.
        name: Option<String>,
        /// Name of the tag
        tag: Option<String>,
    },
    /// Remove tag <tag> from mod <name>
    TagRemove {
        /// Name of the mod to add <tag> to.
        name: Option<String>,
        /// Name of the tag.
        tag: Option<String>,
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
        old_mod_name: Option<String>,
        new_mod_name: Option<String>,
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

                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to disable:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;

                mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                list_mods(settings)
            }
            Self::DisableAll => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                mod_list.disable(settings.cache_dir(), settings.game_dir())?;
                list_mods(settings)
            }
            Self::DisableFile { name, file } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select the source mod of the file to be disabled:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;

                let file_name = FindSelectBuilder::new(
                    FileListBuilder::new(&mod_list[idx])
                        .with_origin()
                        .with_colour(),
                )
                .with_msg("Please select a file to disable:")
                .with_input(file.as_deref())
                .build()?
                .prompt()?;

                if mod_list[idx].disable_file(&file_name) {
                    if mod_list[idx].is_enabled() {
                        mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    }
                    Ok(())
                } else {
                    // log::trace!("File '{file_name}' not found within mod '{mod_name}'.");
                    Err(ModErrors::FileNotFound(name.unwrap_or_default(), file_name).into())
                }
            }
            Self::EnableFile { name, file } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select the source mod of the file to be enabled:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;

                let file_name = FindSelectBuilder::new(
                    FileListBuilder::new(&mod_list[idx])
                        .disabled_files()
                        .with_origin()
                        .with_colour(),
                )
                .with_msg("Please select a file to enable:")
                .with_input(file.as_deref())
                .build()?
                .prompt()?;

                if mod_list[idx].enable_file(&file_name) {
                    if mod_list[idx].is_enabled() {
                        mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                    }
                    Ok(())
                } else {
                    // log::trace!("File '{file_name}' not found within mod '{mod_name}'.");
                    Err(ModErrors::FileNotFound(name.unwrap_or_default(), file_name).into())
                }
            }
            Self::Enable { name } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to enable:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;
                mod_list.enable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                list_mods(settings)
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
                let name = InquireBuilder::new_with_test(
                    name,
                    CustomType::new("Please specify the new priority")
                        // .with_formatter(&|i| format!("${}", i))
                        .with_error_message("Please type a valid number")
                        .with_help_message("Type in a positive or negative number."),
                )
                .prompt()?;

                //TODO Use file_path_select to select destination if not given

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
                let idx = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to REMOVE:")
                    .with_input(name.as_deref())
                    .build()?
                    .prompt()?;

                mod_list.disable_mod(settings.cache_dir(), settings.game_dir(), idx)?;
                mod_list[idx].remove()?;
                log::info!("Removed mod '{}'", mod_list[idx].name());
                list_mods(settings)
            }
            Self::Rename {
                old_mod_name,
                new_mod_name,
            } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let (idx, new_mod_name) = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to rename:")
                    .with_input(old_mod_name.as_deref())
                    .build()?
                    .with_test(
                        new_mod_name,
                        CustomType::new("Please specify the new name")
                            // .with_formatter(&|i| format!("${}", i))
                            .with_error_message("Please type a valid number")
                            .with_help_message("Type in a positive or negative number."),
                    )
                    .prompt()?;

                mod_list[idx].set_name(new_mod_name)?;
                list_mods(settings)
            }
            Self::SetPriority { name, priority } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let (idx, priority) = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to rename:")
                    .with_input(name.as_deref())
                    .build()?
                    .with_test(
                        priority,
                        CustomType::new("Please specify the new priority")
                            // .with_formatter(&|i| format!("${}", i))
                            .with_error_message("Please type a valid number")
                            .with_help_message("Type in a positive or negative number."),
                    )
                    .prompt()?;
                let old_prio = mod_list[idx].priority();

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
            }
            Self::TagAdd { name, tag } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let (idx, tag) = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod to tag:")
                    .with_input(name.as_deref())
                    .build()?
                    .with_test(
                        tag,
                        CustomType::new("Please specify the tag")
                            // .with_formatter(&|i| format!("${}", i)) //TODO validate tag
                            .with_error_message("Please type a one-word-tag")
                            .with_help_message("Type in a one-word-tag."),
                    )
                    .prompt()?;

                if mod_list[idx].add_tag(&tag)? {
                    // log::info!("Added tag {tag} to mod {name}.");
                    Ok(())
                } else {
                    // log::trace!("Unable to add tag {tag} to mod {name}.");
                    Err(ModErrors::DuplicateTag(name.unwrap_or_default(), tag).into())
                }
            }
            Self::TagRemove { name, tag } => {
                let mut mod_list = Vec::gather_mods(settings.cache_dir())?;
                let (idx, tag) = FindSelectBuilder::new(mod_list.default_list_builder())
                    .with_msg("Please select a mod from which to remove the tag:")
                    .with_input(name.as_deref())
                    .build()?
                    .with_test(
                        tag,
                        CustomType::new("Please specify the tag")
                            // .with_formatter(&|i| format!("${}", i)) //TODO validate tag
                            .with_error_message("Please type a one-word-tag")
                            .with_help_message("Type in a one-word-tag."),
                    )
                    .prompt()?;

                if mod_list[idx].remove_tag(&tag)? {
                    // log::info!("Removed tag {tag} from mod {name}.");
                    Ok(())
                } else {
                    // log::trace!("Unable to remove tag {tag} from mod {name}.");
                    Err(ModErrors::TagNotFound(name.unwrap_or_default(), tag).into())
                }
            }
            Self::CopyToCustom {
                source,
                destination,
                file,
            } => {
                let mod_list = Vec::gather_mods(settings.cache_dir())?;
                let (source_idx, dest_idx) =
                    FindSelectBuilder::new(mod_list.default_list_builder())
                        .with_msg("Please select the source mod, to copy the file from:")
                        .with_input(source.as_deref())
                        .build()?
                        .with(
                            FindSelectBuilder::new(mod_list.default_list_builder())
                                .with_msg("Please select the destination mod, to copy the file to:")
                                .with_input(destination.as_deref())
                                .build()?,
                        )
                        .prompt()?;

                let file_name = FindSelectBuilder::new(
                    FileListBuilder::new(&mod_list[source_idx])
                        .with_index()
                        .with_origin()
                        .with_colour(),
                )
                .with_msg("Please select a file to copy:")
                .with_input(file.as_deref())
                .build()?
                .prompt()?;

                let file_idx = file_name
                    .clone()
                    .split_whitespace()
                    .skip(1)
                    .next()
                    .ok_or_else(|| {
                        ModErrors::FileNotFound(
                            mod_list[source_idx].name().to_string(),
                            file_name.clone(),
                        )
                    })?
                    .parse::<usize>()
                    .map_err(|_| {
                        ModErrors::FileNotFound(mod_list[source_idx].name().to_string(), file_name)
                    })?;

                let file = &mod_list[source_idx].files()?[file_idx];
                let origin = settings
                    .cache_dir()
                    .join(mod_list[source_idx].manifest_dir())
                    .join(file.source());
                let destination = settings
                    .cache_dir()
                    .join(mod_list[dest_idx].manifest_dir())
                    .join(file.source());

                DirBuilder::new()
                    .recursive(true)
                    .create(destination.parent().unwrap())?;
                copy(origin, destination)?;
                Ok(())
            }
        }
    }
}

fn show_mod(cache_dir: &Utf8Path, name: Option<&str>) -> Result<()> {
    let mod_list = Vec::gather_mods(cache_dir)?;
    let idx = FindSelectBuilder::new(mod_list.default_list_builder())
        .with_msg("Please select a mod to show:")
        .with_input(name.as_deref())
        .build()?
        .prompt()?;

    show_mod_status(&mod_list, idx)
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
    let mod_idx = FindSelectBuilder::new(mod_list.default_list_builder())
        .with_msg("Please select the source mod of the config file:")
        .with_input(name.as_deref())
        .build()?
        .prompt()?;

    let name = mod_list[mod_idx].name();

    let config_files_to_edit = {
        let manifest = &mod_list[mod_idx];
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
    };

    if !config_files_to_edit.is_empty() {
        let mut editor_cmd = std::process::Command::new(settings.editor());
        // if let Some(destination_mod_name) = destination_mod_name {
        //     // Copy
        //     if let Some(idx) = mod_list.find_mod(destination_mod_name.as_deref()) {
        //         let manifest = &mod_list[idx];

        //         for (source, dest) in &config_files_to_edit {
        //             let dest = settings
        //                 .cache_dir()
        //                 .join(manifest.manifest_dir())
        //                 .join(dest);
        //             log::trace!("Copying config file {} to {}", source, &dest);

        //             DirBuilder::new()
        //                 .recursive(true)
        //                 .create(dest.parent().unwrap())?;

        //             copy(source, &dest)?;
        //             let _ = editor_cmd.arg(dest);
        //         }
        //     }
        // } else {
        for (source, _) in &config_files_to_edit {
            let _ = editor_cmd.arg(source);
        }
        // }

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
