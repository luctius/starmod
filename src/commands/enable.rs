use std::path::Path;

use anyhow::Result;

// use crate::commands::modlist;

use super::modlist::{find_mod, gather_mods};

pub fn enable_all(cache_dir: &Path, game_dir: &Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;

    for mut md in mod_list {
        md.enable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn disable_all(cache_dir: &Path, game_dir: &Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;

    for mut md in mod_list {
        md.disable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn enable_mod(
    cache_dir: &Path,
    game_dir: &Path,
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

pub fn disable_mod(cache_dir: &Path, game_dir: &Path, name: &str) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some(mut md) = find_mod(&mod_list, &name) {
        md.disable(cache_dir, game_dir)?;
    }

    Ok(())
}
