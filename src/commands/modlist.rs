use std::{
    cmp::Ordering,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::manifest::Manifest;

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
