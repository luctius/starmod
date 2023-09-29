use std::fs::{read_link, remove_file, rename, DirBuilder};

use camino::{Utf8Path, Utf8PathBuf};

use anyhow::Result;

// use crate::commands::modlist;

use crate::mods::Mod;

use super::{
    conflict,
    modlist::{find_mod, gather_mods},
};

pub fn enable_all(cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;

    for mut md in mod_list {
        md.enable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn disable_all(cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;

    for mut md in mod_list {
        md.disable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn enable_mod(
    cache_dir: &Utf8Path,
    game_dir: &Utf8Path,
    name: &str,
    priority: Option<isize>,
) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some(mut md) = find_mod(&mod_list, &name) {
        if let Some(priority) = priority {
            md.set_priority(priority)?;
        }
        md.enable(cache_dir, game_dir)?;

        // Disable and re-enable all mods to account for file conflicts
        let mut list = gather_mods(cache_dir)?;
        list.retain(|m| m.is_enabled());

        for m in &mut list {
            m.disable(cache_dir, game_dir)?;
        }
        for m in &mut list {
            m.enable(cache_dir, game_dir)?;
        }
    }

    Ok(())
}

pub fn disable_mod(cache_dir: &Utf8Path, game_dir: &Utf8Path, name: &str) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some(mut md) = find_mod(&mod_list, &name) {
        md.disable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn enable_mods(cache_dir: &Utf8Path, game_dir: &Utf8Path, mods: &[Mod]) -> Result<()> {
    let conflict_list = conflict::conflict_list_by_file(mods)?;
    let mut file_list = Vec::with_capacity(conflict_list.len());
    let mut dir_cache = Vec::new();

    for m in mods {
        file_list.extend(m.enlist_files(&conflict_list));
    }

    for f in file_list {
        let origin = cache_dir.clone().join(f.source());
        let destination = game_dir.clone().join(Utf8PathBuf::from(f.destination()));

        let destination_base = destination.parent().unwrap().to_path_buf();
        if !dir_cache.contains(&destination_base) {
            //create intermediate directories
            DirBuilder::new()
                .recursive(true)
                .create(&destination_base)?;
            dir_cache.push(destination_base);
        }

        // Remove existing symlinks which point back to our archive dir
        // This ensures that the last mod wins, but we should do conflict
        // detection and resolution before this, so we can inform the user.
        if destination.is_symlink() {
            let target = Utf8PathBuf::try_from(read_link(&destination)?)?;

            if target.starts_with(&cache_dir) {
                remove_file(&destination)?;
                log::debug!("overrule {} ({} > {})", destination, origin, target);
            } else {
                let bkp_destination = destination.with_file_name(format!(
                    "{}.starmod_bkp",
                    destination.extension().unwrap_or_default()
                ));
                log::info!(
                    "renaming foreign file from {} -> {}",
                    destination,
                    bkp_destination
                );
                rename(&destination, bkp_destination)?;
            }
        }

        std::os::unix::fs::symlink(&origin, &destination)?;

        log::trace!("link {} to {}", origin, destination);
    }
    Ok(())
}

pub fn disable_mods(cache_dir: &Utf8Path, game_dir: &Utf8Path, mods: &[Mod]) -> Result<()> {
    todo!()
    Ok(())
}
