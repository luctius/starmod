use std::{
    cmp::Ordering,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::Result;
use comfy_table::{presets::NOTHING, Cell, Color, ContentArrangement, Table};

use crate::{
    commands::conflict::{conflict_list_by_file, conflict_list_by_mod},
    manifest::Manifest,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub fn gather_mods(cache_dir: &Path) -> Result<Vec<Manifest>> {
    let paths = fs::read_dir(cache_dir)?;
    let cache_dir = PathBuf::from(cache_dir);

    let mut manifest_list = Vec::new();

    for path in paths {
        if let Ok(path) = path {
            if let Ok(file) = File::open(path.path()) {
                if file.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    if let Ok(manifest) = Manifest::try_from(file) {
                        let mut mod_dir = cache_dir.clone();
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

pub fn list_mods(cache_dir: &Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;

    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(120)
        .set_header(vec!["Index", "Name", "Priority", "Status", "Mod Type"]);

    for (idx, manifest) in mod_list.iter().enumerate() {
        let conflict_list = conflict_list_by_mod(&mod_list)?;
        let is_loser = conflict_list
            .get(&manifest.name().to_string())
            .map(|c| !c.losing_to().is_empty())
            .unwrap_or(false);
        let is_winner = conflict_list
            .get(&manifest.name().to_string())
            .map(|c| !c.winning_over().is_empty())
            .unwrap_or(false);

        let color = match (is_loser, is_winner) {
            (false, false) => Color::White,
            (false, true) => Color::Green,
            (true, false) => Color::Red,
            (true, true) => Color::Blue,
        };
        let color = if manifest.mod_state().is_enabled() {
            color
        } else {
            Color::DarkGrey
        };

        table.add_row(vec![
            Cell::new(idx.to_string()).fg(color),
            Cell::new(manifest.name().to_string()).fg(color),
            Cell::new(manifest.priority().to_string()).fg(color),
            Cell::new(manifest.mod_state().to_string()).fg(color),
            Cell::new(manifest.mod_type().to_string()).fg(color),
        ]);
    }

    println!("{table}");

    Ok(())
}

pub fn show_mod(cache_dir: &Path, mod_name: &str) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;
    if let Some(m) = find_mod(&mod_list, mod_name) {
        show_mod_status(&m, &mod_list)?;
    } else {
        println!("No mod found by that name: {}", mod_name);
    }

    Ok(())
}

pub fn find_mod(mod_list: &[Manifest], mod_name: &str) -> Option<Manifest> {
    if let Some(m) = find_mod_by_name(mod_list, &mod_name) {
        Some(m)
    } else if let Ok(idx) = usize::from_str_radix(&mod_name, 10) {
        find_mod_by_index(mod_list, idx)
    } else if let Some(m) = find_mod_by_name_fuzzy(mod_list, &mod_name) {
        Some(m)
    } else {
        None
    }
}

pub fn find_mod_by_index(mod_list: &[Manifest], idx: usize) -> Option<Manifest> {
    mod_list.get(idx).map(|m| m.clone())
}
pub fn find_mod_by_name(mod_list: &[Manifest], name: &str) -> Option<Manifest> {
    mod_list
        .iter()
        .find_map(|m| (m.name() == name).then(|| m.clone()))
}
pub fn find_mod_by_name_fuzzy(mod_list: &[Manifest], fuzzy_name: &str) -> Option<Manifest> {
    let matcher = SkimMatcherV2::default();
    let mut match_vec = Vec::new();

    mod_list.iter().for_each(|m| {
        let i = matcher.fuzzy_match(m.name(), &fuzzy_name).unwrap_or(0);
        match_vec.push((m, i));
    });

    match_vec.sort_unstable_by(|(_, ia), (_, ib)| ia.cmp(ib));

    match_vec.last().map(|(m, _)| (*m).clone())
}

//TODO: fancier printing
//TODO move this to manifest Display
pub fn show_mod_status(manifest: &Manifest, mod_list: &[Manifest]) -> Result<()> {
    let conflict_list_file = conflict_list_by_file(&mod_list)?;
    let conflict_list_mod = conflict_list_by_mod(&mod_list)?;

    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(120)
        .set_header(vec!["Name", "Priority", "Status", "Mod Type"])
        .add_row(vec![
            manifest.name().to_string(),
            manifest.priority().to_string(),
            manifest.mod_state().to_string(),
            manifest.mod_type().to_string(),
        ]);

    println!("{table}");

    if let Some(conflict) = conflict_list_mod.get(&manifest.name().to_string()) {
        let mut table = Table::new();
        table
            .load_preset(NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(120)
            .set_header(vec!["Conflicting File", "Contenders"]);

        for f in conflict.conflict_files() {
            if let Some(contenders) = conflict_list_file.get(f) {
                table.add_row(vec![f.clone(), format!("{:?}", contenders)]);
            }
        }

        println!("");
        println!("{table}");
    }

    Ok(())
}
