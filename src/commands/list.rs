use std::cmp::Ordering;

use anyhow::Result;
use camino::Utf8Path;
use clap::Parser;
use comfy_table::{Cell, Color};

use crate::{
    conflict::conflict_list_by_file,
    mods::GatherModList,
    settings::{create_table, Settings},
    ui::ModListBuilder,
};

#[derive(Debug, Clone, Parser, Default)]
pub enum ListCmd {
    /// Show all mods
    #[default]
    #[clap(visible_alias = "m")]
    ModList,
    /// Show all conflicting files in the current active mod-list
    #[clap(visible_alias = "c")]
    Conflicts,
    /// Show all files currently in the active mod-list;
    /// Files shown in red are ignored and green files are used instead.
    #[clap(visible_alias = "f")]
    Files,
    /// Show all disabled files
    DisabledFiles,
    ///Show all mods containing <tag>
    Tag,
}
impl ListCmd {
    pub fn execute(self, settings: &Settings) -> Result<()> {
        match self {
            Self::ModList => list_mods(settings),
            Self::Conflicts => list_conflicts(settings.cache_dir()),
            Self::Files => list_files(settings.cache_dir()),
            Self::DisabledFiles => list_disabled_files(settings.cache_dir()),
            Self::Tag => todo!(),
        }
    }
}

pub fn list_mods(settings: &Settings) -> Result<()> {
    let mod_list = Vec::gather_mods(settings.cache_dir())?;

    let table = ModListBuilder::new(&mod_list)
        .with_index()
        .with_priority()
        .with_status()
        .with_version()
        .with_nexus_id()
        .with_mod_type()
        .with_tags()
        .with_notes(settings.download_dir())
        .with_colour()
        .with_headers()
        .build()?
        .join("\n");

    log::info!("");
    log::info!("{table}");

    Ok(())
}

pub fn list_conflicts(cache_dir: &Utf8Path) -> Result<()> {
    let mod_list = Vec::gather_mods(cache_dir)?;
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
        let color = if conflict_list_file.contains_key(&isf.destination().to_string()) {
            if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == &name
            {
                Color::Green
            } else {
                Color::Red
            }
        } else {
            Color::White
        };

        table.add_row(vec![
            Cell::new(isf.destination().to_string()).fg(color),
            Cell::new(name).fg(color),
        ]);
    }

    table.add_row_if(
        |idx, _row| idx.eq(&0),
        vec![Cell::new("No conflicting files found.")],
    );

    log::info!("{table}");
    Ok(())
}

pub fn list_files(cache_dir: &Utf8Path) -> Result<()> {
    let mod_list = Vec::gather_mods(cache_dir)?;
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
        let color = if conflict_list_file.contains_key(&isf.destination().to_string()) {
            if conflict_list_file
                .get(&isf.destination().to_string())
                .unwrap()
                .last()
                .unwrap()
                == name
            {
                Color::Green
            } else {
                Color::Red
            }
        } else {
            Color::White
        };

        table.add_row(vec![
            Cell::new(isf.source().to_string()).fg(color),
            Cell::new(isf.destination().to_string()).fg(color),
            Cell::new(name).fg(color),
        ]);
    }

    table.add_row_if(|idx, _row| idx.eq(&0), vec![Cell::new("No files found.")]);

    log::info!("{table}");

    Ok(())
}

pub fn list_disabled_files(cache_dir: &Utf8Path) -> Result<()> {
    let mod_list = Vec::gather_mods(cache_dir)?;
    let mut disabled_files = Vec::new();

    for m in mod_list {
        for f in m.disabled_files() {
            disabled_files.push((f, m.name().to_string()));
        }
    }

    let mut table = create_table(vec!["File", "Mod"]);
    for (f, mod_name) in disabled_files {
        table.add_row(vec![f.destination().to_string(), mod_name]);
    }

    table.add_row_if(
        |idx, _row| idx.eq(&0),
        vec![Cell::new("No disabled files found.")],
    );

    log::info!("{table}");

    Ok(())
}
