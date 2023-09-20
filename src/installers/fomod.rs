pub const FOMOD_INFO_FILE: &'static str = "fomod/info.xml";
pub const FOMOD_MODCONFIG_FILE: &'static str = "fomod/moduleconfig.xml";

use encoding_rs_io::DecodeReaderBytes;

use anyhow::Result;
use fomod::{Config, Dependency, DependencyOperator, FlagDependency, Info};
use read_stdin::prompt_until_ok;
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use crate::{
    installers::{
        stdin::{Input, InputWithDone},
        InstallerError,
    },
    manifest::{InstallFile, Manifest},
    mod_types::ModType,
};

use super::DATA_DIR_NAME;

pub fn create_fomod_manifest(mod_type: ModType, cache_dir: &Path, name: &Path) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut archive_dir = PathBuf::from(cache_dir);
    archive_dir.push(name);

    let mut config = archive_dir.clone();
    config.push(FOMOD_MODCONFIG_FILE);

    //Allow for names set by fomod
    let name = name.to_string_lossy().to_string();

    let info = {
        let mut info = archive_dir.clone();
        info.push(FOMOD_INFO_FILE);
        let file = File::open(info)?;
        let mut file = DecodeReaderBytes::new(file);
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        Info::try_from(contents.as_str())?
    };

    let config = {
        let mut config = archive_dir.clone();
        config.push(FOMOD_MODCONFIG_FILE);
        let file = File::open(config)?;
        let mut file = DecodeReaderBytes::new(file);
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        Config::try_from(contents.as_str())?
    };

    //FIXME TODO Dependencies

    files.extend(Vec::<InstallFile>::from(
        config.required_install_files.to_own_vec(&archive_dir)?,
    ));

    println!();
    println!();

    println!(
        "FoMod Installer for {} ({})",
        info.name.unwrap_or_default(),
        name,
    );

    let mut condition_flags = HashSet::new();

    for is in config.install_steps.vec_sorted() {
        println!("Install Step: {}", is.name);
        for g in is.optional_file_groups.vec_sorted() {
            println!();
            println!("Group Name: {}", g.name);

            match g.plugins {
                fomod::GroupType::SelectAtLeastOne(plugins) => {
                    select_at_least_one(
                        &name,
                        &plugins.vec_sorted(),
                        &mut files,
                        &mut condition_flags,
                        &archive_dir,
                    )?;
                }
                fomod::GroupType::SelectAtMostOne(plugins) => {
                    select_at_most_one(
                        &name,
                        &plugins.vec_sorted(),
                        &mut files,
                        &mut condition_flags,
                        &archive_dir,
                    )?;
                }
                fomod::GroupType::SelectExactlyOne(plugins) => {
                    select_exactly_one(
                        &name,
                        &plugins.vec_sorted(),
                        &mut files,
                        &mut condition_flags,
                        &archive_dir,
                    )?;
                }
                fomod::GroupType::SelectAll(plugins) => {
                    select_all(
                        &name,
                        &plugins.vec_sorted(),
                        &mut files,
                        &mut condition_flags,
                        &archive_dir,
                    )?;
                }
                fomod::GroupType::SelectAny(plugins) => {
                    select_any(
                        &name,
                        &plugins.vec_sorted(),
                        &mut files,
                        &mut condition_flags,
                        &archive_dir,
                    )?;
                }
            }
        }
    }

    for cip in config.conditional_file_installs {
        let has_deps = match cip.dependencies {
            Dependency::Flag(f) => condition_flags.contains(&f),
            Dependency::Dependency(d) => match d {
                DependencyOperator::And(flag_list) => flag_list.iter().all(|dep| match dep {
                    Dependency::Flag(f) => condition_flags.contains(f),
                    _ => todo!(),
                }),
                DependencyOperator::Or(flag_list) => flag_list.iter().any(|dep| match dep {
                    Dependency::Flag(f) => condition_flags.contains(f),
                    _ => todo!(),
                }),
            },
            _ => todo!(),
        };

        if has_deps {
            files.extend(Vec::<InstallFile>::from(
                cip.files.to_own_vec(&archive_dir)?,
            ));
        }
    }

    Ok(Manifest::new(name, mod_type, files, Vec::new()))
}

trait FomodInstallVecExt {
    fn to_own_vec(&self, archive_dir: &Path) -> Result<Vec<InstallFile>>;
}
impl FomodInstallVecExt for Vec<fomod::FileTypeEnum> {
    fn to_own_vec(&self, archive_dir: &Path) -> Result<Vec<InstallFile>> {
        let mut files = Vec::with_capacity(self.len());
        for fte in self {
            match fte {
                fomod::FileTypeEnum::File(f) => {
                    let mut f = f.clone();
                    f.source = f.source.replace("\\", "/");
                    f.destination = f.destination.map(|d| d.replace("\\", "/"));

                    let mut destination =
                        PathBuf::from(f.destination.clone().unwrap_or_else(|| f.source.clone()));
                    destination.as_mut_os_str().make_ascii_lowercase();
                    let destination = destination
                        .clone()
                        .strip_prefix(DATA_DIR_NAME)
                        .map_or_else(|_| destination, |d| d.to_path_buf());
                    let mut source = PathBuf::from(f.source.clone());
                    source.as_mut_os_str().make_ascii_lowercase();

                    files.push(InstallFile {
                        destination,
                        source,
                    });
                }
                fomod::FileTypeEnum::Folder(f) => {
                    let mut f = f.clone();
                    f.source = f.source.replace("\\", "/");
                    f.destination = f.destination.map(|d| d.replace("\\", "/"));

                    let mut plugin_dir = archive_dir.to_path_buf();
                    plugin_dir.push(PathBuf::from(f.source.to_lowercase()));

                    let walker = WalkDir::new(&plugin_dir)
                        .min_depth(1)
                        .max_depth(usize::MAX)
                        .follow_links(false)
                        .same_file_system(true)
                        .contents_first(false);

                    for entry in walker {
                        let entry = entry?;
                        let entry_path = entry.path();

                        if entry_path.is_file() {
                            let source = entry_path
                                .to_path_buf()
                                .strip_prefix(&archive_dir)?
                                .to_path_buf();

                            let mut destination = PathBuf::from(
                                f.destination
                                    .as_ref()
                                    .map(|d| d.to_lowercase())
                                    .unwrap_or_else(|| f.source.to_lowercase()),
                            );
                            destination
                                .push(source.strip_prefix(PathBuf::from(f.source.to_lowercase()))?);
                            let destination = destination
                                .clone()
                                .strip_prefix(DATA_DIR_NAME)
                                .map_or_else(|_| destination, |d| d.to_path_buf());

                            files.push(InstallFile {
                                destination,
                                source,
                            });
                        }
                    }
                }
            }
        }

        Ok(files)
    }
}

fn select_all(
    _mod_name: &str,
    plugins: &[fomod::Plugin],
    files: &mut Vec<InstallFile>,
    condition_flags: &mut HashSet<FlagDependency>,
    archive_dir: &Path,
) -> Result<()> {
    for p in plugins {
        println!("{}", p.name);
        println!("{}", p.description);

        files.extend(p.files.to_own_vec(archive_dir)?);

        for flag in &p.condition_flags {
            condition_flags.insert(flag.clone());
        }
    }
    Ok(())
}

fn select_exactly_one(
    mod_name: &str,
    plugins: &[fomod::Plugin],
    files: &mut Vec<InstallFile>,
    condition_flags: &mut HashSet<FlagDependency>,
    archive_dir: &Path,
) -> Result<()> {
    println!("Please select one of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("E) Exit Installer");

    let choice: u8 = loop {
        let input: Input = prompt_until_ok("Select : ");
        match input {
            Input::Exit => {
                return Err(InstallerError::InstallerCancelled(mod_name.to_string()).into())
            }
            Input::Digit(d) => {
                if (d as usize) < plugins.len() {
                    break d;
                }
            }
        }
    };

    if let Some(p) = plugins.get(choice as usize) {
        files.extend(p.files.to_own_vec(archive_dir)?);

        for flag in &p.condition_flags {
            condition_flags.insert(flag.clone());
        }
    }

    Ok(())
}

fn select_at_least_one(
    mod_name: &str,
    plugins: &[fomod::Plugin],
    files: &mut Vec<InstallFile>,
    condition_flags: &mut HashSet<FlagDependency>,
    archive_dir: &Path,
) -> Result<()> {
    println!("Please select at-least one of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("D) Done with the selection");
    println!("E) Exit Installer");

    let mut selected = false;
    let mut choices = Vec::with_capacity(4);
    loop {
        let input: InputWithDone = prompt_until_ok("Select : ");
        match input {
            InputWithDone::Input(i) => match i {
                Input::Digit(d) => {
                    if (d as usize) < plugins.len() {
                        choices.push(d);
                        selected = true;
                    } else {
                        println!("Invalid choice..");
                    }
                }
                Input::Exit => {
                    return Err(InstallerError::InstallerCancelled(mod_name.to_string()).into())
                }
            },
            InputWithDone::Done => {
                if selected {
                    break;
                } else {
                    println!("Please select at-least one option.");
                }
            }
        }
    }

    for c in choices {
        if let Some(p) = plugins.get(c as usize) {
            files.extend(p.files.to_own_vec(archive_dir)?);

            for flag in &p.condition_flags {
                condition_flags.insert(flag.clone());
            }
        }
    }

    Ok(())
}

fn select_at_most_one(
    mod_name: &str,
    plugins: &[fomod::Plugin],
    files: &mut Vec<InstallFile>,
    condition_flags: &mut HashSet<FlagDependency>,
    archive_dir: &Path,
) -> Result<()> {
    println!("Please select at-most one of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("D) Done with the selection");
    println!("E) Exit Installer");

    let choice: Option<u8> = loop {
        let input: InputWithDone = prompt_until_ok("Select : ");
        match input {
            InputWithDone::Input(i) => match i {
                Input::Digit(d) => {
                    if (d as usize) < plugins.len() {
                        break Some(d);
                    } else {
                        println!("Invalid choice..");
                    }
                }
                Input::Exit => {
                    return Err(InstallerError::InstallerCancelled(mod_name.to_string()).into())
                }
            },
            InputWithDone::Done => {
                break None;
            }
        }
    };

    if let Some(c) = choice {
        if let Some(p) = plugins.get(c as usize) {
            files.extend(p.files.to_own_vec(archive_dir)?);

            for flag in &p.condition_flags {
                condition_flags.insert(flag.clone());
            }
        }
    }

    Ok(())
}

fn select_any(
    mod_name: &str,
    plugins: &[fomod::Plugin],
    files: &mut Vec<InstallFile>,
    condition_flags: &mut HashSet<FlagDependency>,
    archive_dir: &Path,
) -> Result<()> {
    println!("Please select any of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("D) Done with the selection");
    println!("E) Exit Installer");

    let mut choices = Vec::with_capacity(4);
    loop {
        let input: InputWithDone = prompt_until_ok("Select : ");
        match input {
            InputWithDone::Input(i) => match i {
                Input::Digit(d) => {
                    if (d as usize) < plugins.len() {
                        choices.push(d)
                    } else {
                        println!("Invalid choice..");
                    }
                }
                Input::Exit => {
                    return Err(InstallerError::InstallerCancelled(mod_name.to_string()).into())
                }
            },
            InputWithDone::Done => break,
        }
    }

    for c in choices {
        if let Some(p) = plugins.get(c as usize) {
            files.extend(p.files.to_own_vec(archive_dir)?);

            for flag in &p.condition_flags {
                condition_flags.insert(flag.clone());
            }
        }
    }

    Ok(())
}
