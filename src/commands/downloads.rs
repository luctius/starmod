use std::{
    ffi::OsString,
    fs::{self, metadata},
    path::PathBuf,
};

use crate::{decompress::SupportedArchives, manifest::Manifest};

use anyhow::Result;

fn downloaded_files(download_dir: &str) -> Vec<(SupportedArchives, OsString)> {
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

pub fn list_downloaded_files(download_dir: &str) -> Result<()> {
    let sf = downloaded_files(download_dir);
    for (_, f) in sf {
        let mut download_file = PathBuf::from(download_dir);
        download_file.push(f.clone());
        println!("\t- {}", download_file.display());
    }
    Ok(())
}

//TODO: sort
//TODO: lower_case names
//TODO: remove spaces
pub fn extract_downloaded_files(download_dir: &str, archive_dir: &str) -> Result<()> {
    let sf = downloaded_files(download_dir);
    for (typ, f) in sf {
        let mut download_file = PathBuf::from(download_dir);
        download_file.push(f.clone());
        let mut archive = PathBuf::from(archive_dir);

        //destination:
        //Force utf-8 complatible strings here to simplify futher code.
        let f = f.clone();
        let f = OsString::from(
            f.to_string_lossy()
                .to_string()
                .replace(" ", "_")
                .to_lowercase(),
        );
        archive.push(f.clone());
        archive.set_extension("");

        if metadata(&archive).map(|m| m.is_dir()).unwrap_or(false)
            && Manifest::from_file(archive_dir, &archive)
                .map(|m| m.is_valid())
                .unwrap_or(false)
        {
            // Archive exists and is valid
            // println!("skipping {}", download_file.display());
        } else {
            //TODO: Remove Dir and Manifest file, because eiter one is missing or the manifest is corrupt

            println!("{} -> {}", download_file.display(), archive.display());
            typ.decompress(&download_file, &archive).unwrap();

            let manifest = Manifest::new(archive_dir, &f);
            manifest.write_manifest(archive_dir)?;
        }
    }
    Ok(())
}
