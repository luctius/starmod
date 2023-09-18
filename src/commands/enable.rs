use anyhow::Result;

use crate::commands::modlist;

pub fn enable_all(archive_dir: &str, game_dir: &str) -> Result<()> {
    let mod_list = modlist::gather_mods(archive_dir)?;

    for manifest in mod_list {
        manifest.enable(archive_dir, game_dir)?;
    }

    //TODO verbose
    modlist::list_mods(archive_dir)
}

pub fn disable_all(archive_dir: &str, game_dir: &str) -> Result<()> {
    let mod_list = modlist::gather_mods(archive_dir)?;

    for manifest in mod_list {
        manifest.disable(archive_dir, game_dir)?;
    }

    //TODO verbose
    modlist::list_mods(archive_dir)
}
