use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use comfy_table::{Cell, Color};

use crate::{
    commands::{
        conflict::{conflict_list_by_file, conflict_list_by_mod},
        modlist::gather_mods,
    },
    settings::{create_table, Settings},
    tag::Tag,
};

use super::{modlist::find_mod, show_mod};

#[derive(Debug, Clone, Parser, Default)]
pub enum ShowCmd {
    Mod {
        name: String,
    },
    #[default]
    Legenda,
    Files {
        name: String,
    },
}
impl ShowCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Mod { name } => show_mod(&settings.cache_dir(), &name),
            Self::Legenda => show_legenda(),
            Self::Files { name } => show_files_for_mod(&settings.cache_dir(), name),
        }
    }
}

pub fn show_legenda() -> Result<()> {
    let mut table = create_table(vec!["Tag", "Color", "Meaning"]);

    let tag = Tag::Enabled;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("White").fg(color),
        Cell::new("Nothing to see here; move along citizen.").fg(color),
    ]);

    let tag = Tag::Winner;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Green").fg(color),
        Cell::new("Conflict winner").fg(color),
    ]);

    let tag = Tag::Loser;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Yellow").fg(color),
        Cell::new("Conflict loser").fg(color),
    ]);

    let tag = Tag::CompleteLoser;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Red").fg(color),
        Cell::new("Complete conflict loser; ALL files are overwitten by other mods").fg(color),
    ]);

    let tag = Tag::Conflict;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("Magenta").fg(color),
        Cell::new("Conflict winner for some files, conflict loser for other files.").fg(color),
    ]);

    let tag = Tag::Disabled;
    let (color, chr) = (Color::from(tag), char::from(tag));
    table.add_row(vec![
        Cell::new(chr.to_string()).fg(color),
        Cell::new("DarkGray").fg(color),
        Cell::new("Mod is disabled.").fg(color),
    ]);

    log::info!("{table}");
    Ok(())
}

pub fn show_files_for_mod(cache_dir: &Utf8Path, mod_name: String) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list_file = conflict_list_by_file(&mod_list)?;

    if let Some(m) = find_mod(&mod_list, &mod_name) {
        log::info!("File overview of {}", m.name());
        log::info!("");
        let mut table = create_table(vec!["File", "Destination"]);

        let mut files = m.files().to_vec();
        files.sort_unstable();

        for f in files {
            let mut color = Color::White;
            if conflict_list_file.contains_key(&f.destination().to_string()) {
                color = if conflict_list_file
                    .get(&f.destination().to_string())
                    .unwrap()
                    .last()
                    .unwrap()
                    == m.name()
                {
                    Color::Green
                } else {
                    Color::Red
                };
            }

            table.add_row(vec![
                Cell::new(f.source().to_string()).fg(color),
                Cell::new(f.destination().to_string()).fg(color),
                Cell::new(m.name()).fg(color),
            ]);
        }

        log::info!("{table}");

        if !m.disabled_files().is_empty() {
            log::info!("");
            let mut table = create_table(vec!["Disabled Files"]);

            for d in m.disabled_files() {
                dbg!(d);
                table.add_row(vec![d
                    .source()
                    .strip_prefix(m.manifest_dir())
                    .unwrap()
                    .to_string()]);
            }

            log::info!("{table}");
        }
    } else {
        log::warn!("Mod '{}' could not be found", mod_name);
    }

    Ok(())
}
