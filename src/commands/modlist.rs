use std::{
    cmp::Ordering,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::Result;
use comfy_table::{presets::NOTHING, ContentArrangement, Table};

use crate::manifest::Manifest;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

//TODO Sort list before printing

pub fn gather_mods(cache_dir: &Path) -> Result<Vec<Manifest>> {
    let paths = fs::read_dir(cache_dir).unwrap();
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

//TODO: fancier printing
pub fn list_mods(cache_dir: &Path) -> Result<()> {
    let mod_list = gather_mods(cache_dir)?;

    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        // .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(120)
        .set_header(vec!["Index", "Name", "Priority", "Status", "Mod Type"]);

    for (idx, manifest) in mod_list.iter().enumerate() {
        table.add_row(vec![
            idx.to_string(),
            manifest.name().to_string(),
            manifest.priority().to_string(),
            manifest.mod_state().to_string(),
            manifest.mod_type().to_string(),
        ]);
    }

    println!("{table}");

    Ok(())
}

pub fn show_mod(cache_dir: &Path, mod_name: &str) -> Result<()> {
    if let Some(m) = find_mod(cache_dir, mod_name)? {
        show_mod_status(&m);
    } else {
        println!("No mod found by that name: {}", mod_name);
    }

    Ok(())
}

pub fn find_mod(cache_dir: &Path, mod_name: &str) -> Result<Option<Manifest>> {
    if let Some(m) = find_mod_by_name(cache_dir, &mod_name)? {
        Ok(Some(m))
    } else if let Ok(idx) = usize::from_str_radix(&mod_name, 10) {
        find_mod_by_index(cache_dir, idx)
    } else if let Some(m) = find_mod_by_name_fuzzy(cache_dir, &mod_name)? {
        Ok(Some(m))
    } else {
        Ok(None)
    }
}

pub fn find_mod_by_index(cache_dir: &Path, idx: usize) -> Result<Option<Manifest>> {
    Ok(gather_mods(cache_dir)?.get(idx).map(|m| m.clone()))
}
pub fn find_mod_by_name(cache_dir: &Path, name: &str) -> Result<Option<Manifest>> {
    Ok(gather_mods(cache_dir)?
        .iter()
        .find_map(|m| (m.name() == name).then(|| m.clone())))
}
pub fn find_mod_by_name_fuzzy(cache_dir: &Path, fuzzy_name: &str) -> Result<Option<Manifest>> {
    let matcher = SkimMatcherV2::default();
    let mut match_vec = Vec::new();

    let mods = gather_mods(cache_dir)?;
    mods.iter().for_each(|m| {
        let i = matcher.fuzzy_match(m.name(), &fuzzy_name).unwrap_or(0);
        match_vec.push((m, i));
    });

    match_vec.sort_unstable_by(|(_, ia), (_, ib)| ia.cmp(ib));

    Ok(match_vec.last().map(|(m, _)| (*m).clone()))
}

//TODO move this to manifest Display
pub fn show_mod_status(manifest: &Manifest) {
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
}
