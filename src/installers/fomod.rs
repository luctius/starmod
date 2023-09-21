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

pub fn create_fomod_manifest(
    mod_type: ModType,
    cache_dir: &Path,
    manifest_dir: &Path,
) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut archive_dir = PathBuf::from(cache_dir);
    archive_dir.push(manifest_dir);

    let mut config = archive_dir.clone();
    config.push(FOMOD_MODCONFIG_FILE);

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

    let name = info.name.unwrap_or_else(|| {
        let name = manifest_dir.to_string_lossy().to_string();
        name.split_once("-")
            .map(|n| n.0.to_string())
            .unwrap_or(name)
    });

    //FIXME TODO Dependencies

    files.extend(Vec::<InstallFile>::from(
        config.required_install_files.to_own_vec(&archive_dir)?,
    ));

    println!();
    println!();

    println!("FoMod Installer for {}", name,);

    let mut condition_flags = HashSet::new();

    for is in config.install_steps.vec_sorted() {
        println!("Install Step: {}", is.name);
        for g in is.optional_file_groups.vec_sorted() {
            println!();
            println!("Group Name: {}", g.name);

            match g.plugins {
                fomod::GroupType::SelectAtLeastOne(plugins) => {
                    let plugins = plugins.vec_sorted();
                    let choices: Vec<usize> = select_at_least_one(&name, &plugins)?;
                    files.extend(fetch_plugin_files(&choices, &plugins, &archive_dir)?);
                    condition_flags.extend(fetch_plugin_flags(&choices, &plugins));
                }
                fomod::GroupType::SelectAtMostOne(plugins) => {
                    let plugins = plugins.vec_sorted();
                    let choices: Vec<usize> = select_at_most_one(&name, &plugins)?;
                    files.extend(fetch_plugin_files(&choices, &plugins, &archive_dir)?);
                    condition_flags.extend(fetch_plugin_flags(&choices, &plugins));
                }
                fomod::GroupType::SelectExactlyOne(plugins) => {
                    let plugins = plugins.vec_sorted();
                    let choices: Vec<usize> = select_exactly_one(&name, &plugins)?;
                    files.extend(fetch_plugin_files(&choices, &plugins, &archive_dir)?);
                    condition_flags.extend(fetch_plugin_flags(&choices, &plugins));
                }
                fomod::GroupType::SelectAll(plugins) => {
                    let plugins = plugins.vec_sorted();
                    let choices: Vec<usize> = select_all(&name, &plugins)?;
                    files.extend(fetch_plugin_files(&choices, &plugins, &archive_dir)?);
                    condition_flags.extend(fetch_plugin_flags(&choices, &plugins));
                }
                fomod::GroupType::SelectAny(plugins) => {
                    let plugins = plugins.vec_sorted();
                    let choices: Vec<usize> = select_any(&name, &plugins)?;
                    files.extend(fetch_plugin_files(&choices, &plugins, &archive_dir)?);
                    condition_flags.extend(fetch_plugin_flags(&choices, &plugins));
                }
            };
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

    Ok(Manifest::new(
        manifest_dir,
        name,
        mod_type,
        files,
        Vec::new(),
    ))
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
                        PathBuf::from(f.destination.clone().unwrap_or_else(|| String::new()));
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
                                    .unwrap_or_else(|| String::new()),
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

fn fetch_plugin_flags(choices: &[usize], plugins: &[fomod::Plugin]) -> HashSet<FlagDependency> {
    let mut condition_flags = HashSet::new();

    for c in choices {
        if let Some(p) = plugins.get(*c) {
            for flag in &p.condition_flags {
                condition_flags.insert(flag.clone());
            }
        }
    }

    condition_flags
}

fn fetch_plugin_files(
    choices: &[usize],
    plugins: &[fomod::Plugin],
    archive_dir: &Path,
) -> Result<Vec<InstallFile>> {
    let mut files = Vec::new();

    for c in choices {
        if let Some(p) = plugins.get(*c) {
            files.extend(p.files.to_own_vec(archive_dir)?);
        }
    }

    Ok(files)
}

fn select_all(
    _mod_name: &str,
    plugins: &[fomod::Plugin],
    // files: &mut Vec<InstallFile>,
    // condition_flags: &mut HashSet<FlagDependency>,
    // archive_dir: &Path,
) -> Result<Vec<usize>> {
    let mut choices = Vec::with_capacity(plugins.len());
    for (i, p) in plugins.iter().enumerate() {
        println!("{}", p.name);
        println!("{}", p.description);
        choices.push(i)
    }

    Ok(choices)
}

fn select_exactly_one(mod_name: &str, plugins: &[fomod::Plugin]) -> Result<Vec<usize>> {
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

    Ok(vec![usize::from(choice)])
}

fn select_at_least_one(mod_name: &str, plugins: &[fomod::Plugin]) -> Result<Vec<usize>> {
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
                        choices.push(usize::from(d));
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

    Ok(choices)
}

fn select_at_most_one(mod_name: &str, plugins: &[fomod::Plugin]) -> Result<Vec<usize>> {
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

    Ok(choice.map(|c| vec![usize::from(c)]).unwrap_or_default())
}

fn select_any(mod_name: &str, plugins: &[fomod::Plugin]) -> Result<Vec<usize>> {
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
                    let d = usize::from(d);
                    if d < plugins.len() {
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

    Ok(choices)
}
