use std::{
    ffi::OsString,
    fs::{self, metadata},
    path::{Path, PathBuf},
};

use crate::{decompress::SupportedArchives, manifest::Manifest, mod_types::ModType};

use anyhow::Result;
use walkdir::WalkDir;

fn downloaded_files(download_dir: &Path) -> Vec<(SupportedArchives, OsString)> {
    let mut supported_files = Vec::new();
    let paths = fs::read_dir(download_dir).unwrap();

    for path in paths {
        if let Ok(path) = path {
            if let Ok(typ) = SupportedArchives::from_path(&path.path()) {
                supported_files.push((typ, path.file_name()));
                // println!("Name: {}, type: {}", path.path().display(), typ);
            }
        }
    }

    supported_files
}

pub fn list_downloaded_files(download_dir: &Path) -> Result<()> {
    let sf = downloaded_files(download_dir);
    for (_, f) in sf {
        let mut download_file = PathBuf::from(download_dir);
        download_file.push(f.clone());
        println!("\t- {}", download_file.display());
    }
    Ok(())
}

pub fn extract_downloaded_files(download_dir: &Path, cache_dir: &Path) -> Result<()> {
    let sf = downloaded_files(download_dir);
    for (typ, f) in sf {
        let mut download_file = PathBuf::from(download_dir);
        download_file.push(f.clone());
        let mut archive = PathBuf::from(cache_dir);

        //destination:
        //Force utf-8 compatible strings, in lower-case, here to simplify futher code.
        let f = f.clone();
        let f = OsString::from(f.to_string_lossy().to_string().to_lowercase());
        archive.push(f.clone());
        archive.set_extension("");

        let mut name = PathBuf::from(f.clone());
        name.set_extension("");

        if metadata(&archive).map(|m| m.is_dir()).unwrap_or(false)
            && Manifest::from_file(cache_dir, &archive)
                .map(|m| m.is_valid())
                .unwrap_or(false)
        {
            // Archive exists and is valid
            // Nothing to do
            // TODO: println!("skipping {}", download_file.display());
        } else {
            //TODO: if either one of Dir or Manifest file is missing or corrupt, remove them,

            println!("{} -> {}", download_file.display(), archive.display());
            typ.decompress(&download_file, &archive).unwrap();

            // Rename all extracted files to their lower-case counterpart
            // This is especially important for fomod mods, because otherwise we would
            // not know if their name in the fomod package matches their actual names.
            rename_recursive(&archive)?;

            let mod_type = ModType::detect_mod_type(&cache_dir, &name)?;
            let manifest = mod_type.create_manifest(&cache_dir, &name)?;
            manifest.write_manifest(cache_dir)?;
        }
    }
    Ok(())
}

fn rename_recursive(path: &Path) -> Result<()> {
    let walker = WalkDir::new(path)
        .min_depth(1)
        .max_depth(usize::MAX)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(true);

    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() || entry_path.is_file() {
            lower_case(entry_path)?;
        } else {
            continue;
        }
    }

    Ok(())
}

fn lower_case(path: &Path) -> Result<()> {
    let name = path.file_name().unwrap().to_string_lossy();
    let name = name.to_lowercase();
    let name = OsString::from(name);
    let name = name.as_os_str();
    let name = path.with_file_name(name);

    // println!("ren {} -> {}", path.display(), name.display());

    std::fs::rename(path, path.with_file_name(name).as_path())?;

    Ok(())
}
