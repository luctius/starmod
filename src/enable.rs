use std::fs::{read_link, remove_file, rename, DirBuilder};

use camino::{Utf8Path, Utf8PathBuf};

use anyhow::Result;

// use crate::commands::modlist;

use crate::mods::{Mod, ModList};

use super::{
    conflict,
    modlist::{find_mod, gather_mods},
};

pub fn enable_all(cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
    let mut mod_list = gather_mods(cache_dir)?;

    mod_list.disable(cache_dir, game_dir)?;
    mod_list.enable(cache_dir, game_dir)?;

    Ok(())
}

pub fn disable_all(cache_dir: &Utf8Path, game_dir: &Utf8Path) -> Result<()> {
    let mut mod_list = gather_mods(cache_dir)?;

    mod_list.disable(cache_dir, game_dir)?;

    Ok(())
}

pub fn enable_mod(
    cache_dir: &Utf8Path,
    game_dir: &Utf8Path,
    name: &str,
    priority: Option<isize>,
) -> Result<()> {
    let mut mod_list = gather_mods(cache_dir)?;

    if let Some((_m, idx)) = find_mod(&mod_list, name) {
        if let Some(prio) = priority {
            mod_list[idx].set_priority(prio)?;
        }
        mod_list[idx].enable(cache_dir, game_dir)?;
        mod_list[0..=idx].as_mut().re_enable(cache_dir, game_dir)?;
    }

    Ok(())
}

pub fn disable_mod(cache_dir: &Utf8Path, game_dir: &Utf8Path, name: &str) -> Result<()> {
    let mut mod_list = gather_mods(cache_dir)?;

    if let Some((_m, idx)) = find_mod(&mod_list, name) {
        mod_list[idx].disable(cache_dir, game_dir)?;
        mod_list[0..=idx].as_mut().re_enable(cache_dir, game_dir)?;
    }

    Ok(())
}
