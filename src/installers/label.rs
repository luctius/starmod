// use anyhow::Result;
// use camino::Utf8Path;

// use crate::{manifest::Manifest, mods::ModKind};

// pub fn create_label_manifest(
//     mod_kind: ModKind,
//     cache_dir: &Utf8Path,
//     name: &Utf8Path,
// ) -> Result<Manifest> {
//     let files = Vec::new();
//     let disabled_files = Vec::new();

//     // let archive_dir = cache_dir.join(name);

//     let version = Some("Label".to_owned());
//     let nexus_id = None;

//     Ok(Manifest::new(
//         cache_dir,
//         name,
//         name.to_string(),
//         name.to_string(),
//         nexus_id,
//         version,
//         files,
//         disabled_files,
//         mod_kind,
//     ))
// }
