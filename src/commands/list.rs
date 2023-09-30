use std::cmp::Ordering;

use anyhow::Result;
use camino::Utf8Path;
use clap::Parser;
use comfy_table::{Cell, Color};

use crate::{
    conflict::{conflict_list_by_file, conflict_list_by_mod},
    modlist::gather_mods,
    settings::{create_table, Settings},
    tag::Tag,
};

#[derive(Debug, Clone, Parser, Default)]
pub enum ListCmd {
    #[default]
    ModList,
    Conflicts,
    Files,
}
impl ListCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::ModList => list_mods(&settings.cache_dir()),
            Self::Conflicts => list_conflicts(&settings.cache_dir()),
            Self::Files => list_files(&settings.cache_dir()),
        }
    }
}

pub fn list_mods(cache_dir: &Utf8Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list = conflict_list_by_mod(&mod_list)?;

    //TODO: create seperate tables for each label we encounter.

    let mut table = create_table(vec![
        "Index", "Name", "Priority", "Status", "Version", "Nexus Id", "Mod Type",
    ]);

    for (idx, md) in mod_list.iter().enumerate() {
        let is_loser = conflict_list
            .get(&md.name().to_string())
            .map(|c| !c.losing_to().is_empty())
            .unwrap_or(false);
        let is_winner = conflict_list
            .get(&md.name().to_string())
            .map(|c| !c.winning_over().is_empty())
            .unwrap_or(false);

        let tag = Tag::from((is_loser, is_winner));

        // Detect if we all files of this manifest are overwritten by other mods
        let tag = if is_loser {
            let mut file_not_lost = false;
            let conflict_list = conflict_list_by_file(&mod_list)?;

            for f in md.dest_files()? {
                if let Some(contenders) = conflict_list.get(&f) {
                    if let Some(c) = contenders.last() {
                        if c == md.name() {
                            file_not_lost = true;
                        }
                    }
                } else {
                    file_not_lost = true;
                }
            }

            if !file_not_lost {
                Tag::CompleteLoser
            } else {
                tag
            }
        } else {
            tag
        };
        let tag = if md.is_enabled() { tag } else { Tag::Disabled };

        let color = Color::from(tag);

        table.add_row(vec![
            Cell::new(idx.to_string()).fg(color),
            Cell::new(md.name().to_string()).fg(color),
            Cell::new(md.priority().to_string()).fg(color),
            Cell::new(tag).fg(color),
            Cell::new(md.version().unwrap_or("<Unknown>").to_string()).fg(color),
            Cell::new(
                md.nexus_id()
                    .map(|nid| nid.to_string())
                    .unwrap_or("<Unknown>".to_owned()),
            )
            .fg(color),
            Cell::new(md.kind().to_string()).fg(color),
        ]);
    }

    log::info!("{table}");

    Ok(())
}

pub fn list_conflicts(cache_dir: &Utf8Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list_file = conflict_list_by_file(&mod_list)?;
    let mut files = Vec::new();

    for m in mod_list {
        files.extend(
            m.files()?
                .iter()
                .map(|i| (i.clone(), (m.name().to_owned(), m.priority()))),
        );
    }

    files.retain(|(f, _)| conflict_list_file.contains_key(f.destination()));

    files.sort_unstable_by(|(ia, (_, pa)), (ib, (_, pb))| {
        let o = ia.destination().cmp(ib.destination());
        if o == Ordering::Equal {
            pa.cmp(pb)
        } else {
            o
        }
    });

    log::info!("Conflict overview");
    log::info!("");
    let mut table = create_table(vec!["File", "Mod"]);

    for (isf, (name, _priority)) in files {
        let mut color = Color::White;
        if conflict_list_file.contains_key(&isf.destination().to_string()) {
            color = if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == &name
            {
                Color::Green
            } else {
                Color::Red
            };
        }

        table.add_row(vec![
            Cell::new(isf.destination().to_string()).fg(color),
            Cell::new(name).fg(color),
        ]);
    }

    log::info!("{table}");
    Ok(())
}

pub fn list_files(cache_dir: &Utf8Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    let conflict_list_file = conflict_list_by_file(&mod_list)?;

    let mut files = Vec::new();

    for m in &mod_list {
        files.extend(
            m.files()?
                .iter()
                .map(|i| (i.clone(), (m.name(), m.priority()))),
        );
    }

    files.sort_unstable_by(|(ia, (_, pa)), (ib, (_, pb))| {
        let o = ia.destination().cmp(ib.destination());
        if o == Ordering::Equal {
            pa.cmp(pb)
        } else {
            o
        }
    });

    log::info!("File overview");
    log::info!("");
    let mut table = create_table(vec!["File", "Destination", "Mod"]);

    for (isf, (name, _priority)) in files {
        let mut color = Color::White;
        if conflict_list_file.contains_key(&isf.destination().to_string()) {
            color = if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == name
            {
                Color::Green
            } else {
                Color::Red
            };
        }

        table.add_row(vec![
            Cell::new(isf.source().to_string()).fg(color),
            Cell::new(isf.destination().to_string()).fg(color),
            Cell::new(name).fg(color),
        ]);
    }

    log::info!("{table}");

    Ok(())
}
