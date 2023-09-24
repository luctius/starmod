use std::{
    fs::{self},
    path::Path,
};

use anyhow::Result;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::{manifest::MANIFEST_EXTENTION, mods::Mod};

pub fn gather_mods(cache_dir: &Path) -> Result<Vec<Mod>> {
    let paths = fs::read_dir(cache_dir)?;

    let mut mod_list = Vec::new();

    for path in paths {
        if let Ok(entry) = path {
            if entry
                .path()
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .eq(MANIFEST_EXTENTION)
            {
                mod_list.push(Mod::try_from(entry.path())?);
            }
        }
    }

    mod_list.sort_by(|a, b| a.cmp(b));

    Ok(mod_list)
}

pub fn find_mod(mod_list: &[Mod], mod_name: &str) -> Option<Mod> {
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

pub fn find_mod_by_index(mod_list: &[Mod], idx: usize) -> Option<Mod> {
    mod_list.get(idx).map(|m| m.clone())
}
pub fn find_mod_by_name(mod_list: &[Mod], name: &str) -> Option<Mod> {
    mod_list
        .iter()
        .find_map(|m| (m.name() == name).then(|| m.clone()))
}
pub fn find_mod_by_name_fuzzy(mod_list: &[Mod], fuzzy_name: &str) -> Option<Mod> {
    let matcher = SkimMatcherV2::default();
    let mut match_vec = Vec::new();

    mod_list.iter().for_each(|m| {
        let i = matcher.fuzzy_match(m.name(), &fuzzy_name).unwrap_or(0);
        match_vec.push((m, i));
    });

    match_vec.sort_unstable_by(|(_, ia), (_, ib)| ia.cmp(ib));

    match_vec.last().map(|(m, _)| (*m).clone())
}
