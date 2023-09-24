use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::mods::Mod;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflicts {
    conflict_files: Vec<String>,
    losing_to_mods: HashSet<String>,
    winning_over_mods: HashSet<String>,
}
impl Conflicts {
    pub fn conflict_files(&self) -> &[String] {
        &self.conflict_files
    }
    pub fn losing_to(&self) -> &HashSet<String> {
        &self.losing_to_mods
    }
    pub fn winning_over(&self) -> &HashSet<String> {
        &self.winning_over_mods
    }
}

pub fn conflict_list_by_file(mods: &[Mod]) -> Result<HashMap<String, Vec<String>>> {
    let mut all_files = HashMap::new();

    // populate with all files
    mods.iter().for_each(|m| {
        if m.is_enabled() {
            dbg!(&m);
            // dbg!(&m.dest_files());
            m.dest_files().iter().for_each(|f| {
                all_files.insert(f.clone(), Vec::new());
            })
        }
    });

    // insert conflicting mods
    mods.iter().for_each(|m| {
        if m.is_enabled() {
            m.dest_files().iter().for_each(|f| {
                if let Some(v) = all_files.get_mut(f) {
                    v.push(m.name().to_string());
                }
            })
        }
    });

    // Remove all files without conflicts
    // all_files.retain(|_k, v| v.len() > 1);
    all_files.retain(|_k, v| {
        if v.len() > 1 {
            true
        } else {
            // dbg!(_k);
            false
        }
    });

    Ok(all_files)
}

pub fn conflict_list_by_mod(mods: &[Mod]) -> Result<HashMap<String, Conflicts>> {
    let list = conflict_list_by_file(mods)?;

    let mut mods_conflicts = HashMap::new();
    mods.iter().for_each(|m| {
        let mut conflicts = Vec::new();
        let mut losing = HashSet::new();
        let mut winning = HashSet::new();

        list.iter().for_each(|(f, vec)| {
            let mut found_self = false;

            if m.dest_files().contains(f) {
                for a in vec.iter() {
                    if a.as_str() == m.name() {
                        found_self = true;
                        conflicts.push(f.clone());
                    } else if found_self {
                        winning.insert(a.to_string());
                    } else {
                        losing.insert(a.to_string());
                    }
                }
            }
        });

        if !conflicts.is_empty() {
            mods_conflicts.insert(
                m.name().to_string(),
                Conflicts {
                    conflict_files: conflicts,
                    winning_over_mods: losing,
                    losing_to_mods: winning,
                },
            );
        }
    });

    Ok(mods_conflicts)
}
