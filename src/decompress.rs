use std::{
    fmt::Display,
    fs::{self, remove_dir_all, DirBuilder, File, OpenOptions, Permissions},
    os::unix::{fs::DirBuilderExt, prelude::PermissionsExt},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecompressError {
    #[error("the file `{0}` is in an unsuported format")]
    Unsupported(PathBuf),
}
fn path_result(path: &Path) -> String {
    let spath = path.to_str();
    match spath {
        Some(p) => String::from(p),
        None => String::from("path missing!"),
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SupportedArchives {
    SevenZip,
    Zip,
    TarGz,
    TarXz,
    Rar,
}
impl SupportedArchives {
    pub fn from_path(path: &Path) -> Result<Self> {
        let path_str = path.as_os_str().to_string_lossy();

        if path_str.ends_with(".tar.gz") {
            Ok(Self::TarGz)
        } else if path_str.ends_with(".tar.xz") {
            Ok(Self::TarXz)
        } else if path_str.ends_with(".7z") || path_str.ends_with(".7zip") {
            Ok(Self::SevenZip)
        } else if path_str.ends_with(".zip") {
            Ok(Self::Zip)
        } else if path_str.ends_with(".rar") {
            Ok(Self::Rar)
        } else {
            Err(DecompressError::Unsupported(path.to_path_buf()))?
        }
    }
    pub fn decompress(&self, from_path: &Path, destination_path: &Path) -> Result<()> {
        match self {
            SupportedArchives::SevenZip => decompress_7z(from_path, destination_path),
            SupportedArchives::Zip => decompress_zip(from_path, destination_path).or_else(|e| {
                decompress_zip_with_permission_override(from_path, destination_path).or(Err(e))
            }),
            SupportedArchives::TarGz => decompress_tar_gz(from_path, destination_path),
            SupportedArchives::TarXz => decompress_tar_xz(from_path, destination_path),
            SupportedArchives::Rar => decompress_rar(from_path, destination_path),
        }
    }
}
impl Display for SupportedArchives {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let typ_str = match self {
            SupportedArchives::SevenZip => "7zip",
            SupportedArchives::Zip => "zip",
            SupportedArchives::TarGz => "tar.gz",
            SupportedArchives::TarXz => "tar.xz",
            SupportedArchives::Rar => "rar",
        };
        f.write_str(typ_str)
    }
}

fn decompress_tar_gz(from_path: &Path, destination_path: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = File::open(from_path)
        .with_context(|| format!("Failed to open file from Path: {}", path_result(from_path),))?;

    let mut archive = Archive::new(GzDecoder::new(file));

    archive.unpack(destination_path).with_context(|| {
        format!(
            "Failed to unpack into destination : {}",
            path_result(destination_path)
        )
    })?;
    Ok(())
}

fn decompress_tar_xz(from_path: &Path, destination_path: &Path) -> Result<()> {
    use lzma::reader::LzmaReader;
    use tar::Archive;

    let file = File::open(from_path)
        .with_context(|| format!("Failed to open file from Path: {}", path_result(from_path),))?;

    let mut archive = Archive::new(LzmaReader::new_decompressor(file).unwrap());

    archive.unpack(destination_path).with_context(|| {
        format!(
            "Failed to unpack into destination : {}",
            path_result(destination_path)
        )
    })?;
    Ok(())
}

fn decompress_7z(from_path: &Path, destination_path: &Path) -> Result<()> {
    use sevenz_rust::decompress_file;

    decompress_file(from_path, destination_path).with_context(|| {
        format!(
            "Failed to unpack into destination : {}",
            path_result(destination_path)
        )
    })?;

    Ok(())
}

// This was created to fix a problem with a file setting only read-only permissions to a file
fn decompress_zip_with_permission_override(
    from_path: &Path,
    destination_path: &Path,
) -> Result<()> {
    use zip::read::ZipArchive;

    println!("Retrying unzip with forced permissions");
    remove_dir_all(destination_path)?;

    let file = File::open(from_path)
        .with_context(|| format!("Failed to open file from Path: {}", path_result(from_path),))?;

    let mut zip = ZipArchive::new(file)?;
    for idx in 0..zip.len() {
        let mut file = zip.by_index(idx)?;

        file.enclosed_name();
        let destination = destination_path.join(file.enclosed_name().unwrap());
        if destination.is_file() {
            DirBuilder::new()
                .mode(0o755)
                .recursive(true)
                .create(destination.parent().unwrap())?;

            // let mut dest_file = dbg!(File::open(&destination)?);
            let mut dest_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&destination)?;

            std::io::copy(&mut file, &mut dest_file)?;
            fs::set_permissions(
                destination,
                Permissions::from_mode(file.unix_mode().unwrap_or(0o755)),
            )?;
        } else {
            DirBuilder::new()
                .mode(0o755)
                .recursive(true)
                .create(destination)?;
        }
    }

    Ok(())
}

fn decompress_zip(from_path: &Path, destination_path: &Path) -> Result<()> {
    use zip::read::ZipArchive;

    let file = File::open(from_path)
        .with_context(|| format!("Failed to open file from Path: {}", path_result(from_path),))?;

    ZipArchive::new(file)?
        .extract(destination_path)
        .with_context(|| {
            format!(
                "Failed to unpack into destination : {}",
                path_result(destination_path)
            )
        })?;

    Ok(())
}

fn decompress_rar(from_path: &Path, destination_path: &Path) -> Result<()> {
    use unrar::Archive;

    let mut archive = Archive::new(from_path)
        .open_for_processing()
        .with_context(|| format!("Failed to open archive: {}", path_result(destination_path)))?;

    while let Some(header) = archive.read_header()? {
        archive = if header.entry().is_file() {
            let mut file_path = destination_path.to_path_buf();
            file_path.push(&header.entry().filename);

            DirBuilder::new()
                .recursive(true)
                .create(file_path.parent().unwrap())?;

            header.extract_to(file_path).with_context(|| {
                format!(
                    "Failed to unpack into destination : {}",
                    path_result(destination_path)
                )
            })?
        } else {
            header.skip()?
        };
    }

    Ok(())
}
