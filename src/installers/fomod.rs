pub const FOMOD_INFO_FILE: &str = "fomod/info.xml";
pub const FOMOD_MODCONFIG_FILE: &str = "fomod/moduleconfig.xml";

use encoding_rs_io::DecodeReaderBytes;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use fomod::{Config, Dependency, DependencyOperator, FlagDependency, Info};
use read_stdin::prompt_until_ok;
use std::{collections::HashSet, fs::File, io::Read};
use walkdir::WalkDir;

use crate::{
    dmodman::{DmodMan, DMODMAN_EXTENSION},
    installers::{
        stdin::{Input, InputWithDone},
        InstallerError,
    },
    manifest::{install_file::InstallFile, Manifest},
    mods::ModKind,
    utils::AddExtension,
};

pub fn create_fomod_manifest(
    mod_kind: ModKind,
    cache_dir: &Utf8Path,
    mod_dir: &Utf8Path,
) -> Result<Manifest> {
    let mut files = Vec::new();
    let mut archive_dir = Utf8PathBuf::from(cache_dir);
    archive_dir.push(mod_dir);

    let mut config = archive_dir.clone();
    config.push(FOMOD_MODCONFIG_FILE);

    let dmodman = archive_dir.add_extension(DMODMAN_EXTENSION);

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

    let mut bare_file_name = mod_dir.to_string();
    let mut name = info.name;
    let mut version = info.version;
    let mut nexus_id = None;
    if let Ok(dmodman) = DmodMan::try_from(dmodman.as_path()) {
        nexus_id = Some(dmodman.mod_id());
        version = dmodman.version();
        name.get_or_insert_with(|| dmodman.name());
        bare_file_name = dmodman.name();
    }
    let name = name.unwrap_or_else(|| mod_dir.to_string());

    //FIXME TODO Dependencies

    files.extend(config.required_install_files.to_own_vec(&archive_dir)?);

    println!();
    println!();

    println!("FoMod Installer for {name}");

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
                    let choices: Vec<usize> = select_all(&name, &plugins);
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
            files.extend(cip.files.to_own_vec(&archive_dir)?);
        }
    }

    let mut unique_files = HashSet::new();
    let mut conflicts = Vec::new();
    for f in &files {
        if !unique_files.insert(f.destination()) {
            conflicts.push(f.destination().to_string());
        }
    }
    for c in conflicts {
        let idx = files
            .iter()
            .enumerate()
            .find(|(_, isf)| isf.destination() == c)
            .map(|(idx, _)| idx)
            .unwrap();
        files.remove(idx);
    }

    Ok(Manifest::new(
        cache_dir,
        mod_dir,
        bare_file_name,
        name,
        nexus_id,
        version,
        files,
        Vec::new(),
        mod_kind,
    ))
}

trait FomodInstallVecExt {
    fn to_own_vec(&self, archive_dir: &Utf8Path) -> Result<Vec<InstallFile>>;
}
impl FomodInstallVecExt for Vec<fomod::FileTypeEnum> {
    fn to_own_vec(&self, archive_dir: &Utf8Path) -> Result<Vec<InstallFile>> {
        let mut files = Vec::with_capacity(self.len());
        for fte in self {
            match fte {
                fomod::FileTypeEnum::File(f) => {
                    let mut f = f.clone();
                    f.source = f.source.replace('\\', "/");
                    f.destination = f.destination.map(|d| d.replace('\\', "/"));

                    let destination = f.destination.clone().unwrap_or_else(String::new);
                    let source = Utf8PathBuf::from(f.source.clone().to_lowercase());

                    files.push(InstallFile::new(source, &destination));
                }
                fomod::FileTypeEnum::Folder(f) => {
                    let mut f = f.clone();
                    f.source = f.source.replace('\\', "/").to_lowercase();
                    f.destination = f.destination.map(|d| d.replace('\\', "/"));
                    f.destination = f
                        .destination
                        .as_deref()
                        .and_then(|d| d.strip_prefix("data/").map(str::to_lowercase))
                        .or(f.destination);

                    let mut plugin_dir = archive_dir.to_path_buf();
                    plugin_dir.push(Utf8PathBuf::from(f.source.to_lowercase()));

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
                            let source = Utf8PathBuf::try_from(entry_path.to_path_buf())?
                                .strip_prefix(archive_dir)?
                                .to_path_buf();

                            let destination = format!(
                                "{}/{}",
                                f.destination.clone().unwrap_or_default(),
                                source.strip_prefix(&f.source).unwrap()
                            );

                            files.push(InstallFile::new(source, &destination));
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
    archive_dir: &Utf8Path,
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
    // archive_dir: &Utf8Path,
) -> Vec<usize> {
    let mut choices = Vec::with_capacity(plugins.len());
    for (i, p) in plugins.iter().enumerate() {
        println!("{}", p.name);
        println!("{}", p.description);
        choices.push(i);
    }

    choices
}

fn select_exactly_one(mod_name: &str, plugins: &[fomod::Plugin]) -> Result<Vec<usize>> {
    println!();
    println!("Please select one of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("E) Exit Installer");
    println!();

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
    println!();
    println!("Please select at-least one of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("D) Done with the selection");
    println!("E) Exit Installer");
    println!();

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
                }
                println!("Please select at-least one option.");
            }
        }
    }

    Ok(choices)
}

fn select_at_most_one(mod_name: &str, plugins: &[fomod::Plugin]) -> Result<Vec<usize>> {
    println!();
    println!("Please select at-most one of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("D) Done with the selection");
    println!("E) Exit Installer");
    println!();

    let choice: Option<u8> = loop {
        let input: InputWithDone = prompt_until_ok("Select : ");
        match input {
            InputWithDone::Input(i) => match i {
                Input::Digit(d) => {
                    if (d as usize) < plugins.len() {
                        break Some(d);
                    }
                    println!("Invalid choice..");
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
    println!();
    println!("Please select any of the following: ");
    for (i, p) in plugins.iter().enumerate() {
        println!("{}) {}: {}", i, p.name, p.description);
    }
    println!("D) Done with the selection");
    println!("E) Exit Installer");
    println!();

    let mut choices = Vec::with_capacity(4);
    loop {
        let input: InputWithDone = prompt_until_ok("Select : ");
        match input {
            InputWithDone::Input(i) => match i {
                Input::Digit(d) => {
                    let d = usize::from(d);
                    if d < plugins.len() {
                        choices.push(d);
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
