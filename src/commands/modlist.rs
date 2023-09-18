use std::{
    cmp::Ordering,
    fs::{self, File},
    path::PathBuf,
};

use anyhow::Result;

use crate::manifest::Manifest;

//TODO Sort list before printing

pub fn gather_mods(archive_dir: &str) -> Result<Vec<Manifest>> {
    let paths = fs::read_dir(archive_dir).unwrap();
    let archive_dir = PathBuf::from(archive_dir);

    let mut manifest_list = Vec::new();

    for path in paths {
        if let Ok(path) = path {
            if let Ok(file) = File::open(path.path()) {
                if file.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    if let Ok(manifest) = Manifest::try_from(file) {
                        let mut mod_dir = archive_dir.clone();
                        mod_dir.push(manifest.name());

                        manifest_list.push(manifest);
                    }
                }
            }
        }
    }

    manifest_list.sort_by(|a, b| {
        //Order around priority, or if equal around alfabethic order

        let o = a.priority().cmp(&b.priority());
        if o == Ordering::Equal {
            a.name().cmp(b.name())
        } else {
            o
        }
    });

    Ok(manifest_list)
}

//TODO: fancier printing
pub fn list_mods(archive_dir: &str) -> Result<()> {
    let mod_list = gather_mods(archive_dir)?;

    for manifest in mod_list {
        if manifest.mod_state().is_enabled() {
            println!(
                "\t[{}] {} ({})",
                manifest.priority(),
                manifest.name(),
                manifest.mod_type()
            );
        } else {
            println!("\t[Disabled] {} ({})", manifest.name(), manifest.mod_type());
        }
    }

    Ok(())
}
