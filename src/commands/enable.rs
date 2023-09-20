use std::path::Path;

use anyhow::Result;

use crate::commands::modlist;

use super::modlist::find_mod;

pub fn enable_all(cache_dir: &Path, game_dir: &Path) -> Result<()> {
    let mod_list = modlist::gather_mods(cache_dir)?;

    for manifest in mod_list {
        manifest.enable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn disable_all(cache_dir: &Path, game_dir: &Path) -> Result<()> {
    let mod_list = modlist::gather_mods(cache_dir)?;

    for manifest in mod_list {
        manifest.disable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn enable_mod(cache_dir: &Path, game_dir: &Path, name: &str) -> Result<()> {
    let mod_list = modlist::gather_mods(cache_dir)?;
    if let Some(manifest) = find_mod(&mod_list, &name) {
        manifest.enable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn disable_mod(cache_dir: &Path, game_dir: &Path, name: &str) -> Result<()> {
    let mod_list = modlist::gather_mods(cache_dir)?;
    if let Some(manifest) = find_mod(&mod_list, &name) {
        manifest.disable(cache_dir, game_dir)?;
    }

    Ok(())
}
