use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::manifest::Manifest;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflicts {
    conflict_files: Vec<String>,
    losing_to_mods: HashSet<String>,
    winning_over_mods: HashSet<String>,
}
impl Conflicts {
    #[allow(unused)]
    pub fn conflict_files(&self) -> &[String] {
        &self.conflict_files
    }
    pub const fn losing_to(&self) -> &HashSet<String> {
        &self.losing_to_mods
    }
    pub const fn winning_over(&self) -> &HashSet<String> {
        &self.winning_over_mods
    }
}

pub fn conflict_list_by_file(mods: &[Manifest]) -> Result<HashMap<String, Vec<String>>> {
    log::trace!("Building Conflict List by File");
    let mut all_files = HashMap::new();

    // populate with all files
    for m in mods {
        if m.is_enabled() {
            m.dest_files()?.iter().for_each(|f| {
                all_files.insert(f.clone(), Vec::new());
            });
        }
    }

    // insert conflicting mods
    for m in mods {
        if m.is_enabled() {
            m.dest_files()?.iter().for_each(|f| {
                if let Some(v) = all_files.get_mut(f) {
                    v.push(m.name().to_string());
                }
            });
        }
    }

    // Remove all files without conflicts
    all_files.retain(|_k, v| v.len() > 1);

    log::trace!("Finished Building Conflict List by File");
    Ok(all_files)
}

pub fn conflict_list_by_mod(mods: &[Manifest]) -> Result<HashMap<String, Conflicts>> {
    log::trace!("Building Conflict List");

    let list = conflict_list_by_file(mods)?;

    let mut mods_conflicts = HashMap::new();
    for m in mods {
        let mut conflicts = Vec::new();
        let mut losing = HashSet::new();
        let mut winning = HashSet::new();

        for (f, vec) in &list {
            let mut found_self = false;

            if m.dest_files()?.contains(f) {
                for a in vec {
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
        }

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
    }

    log::trace!("Finished Building Conflict List");
    Ok(mods_conflicts)
}
