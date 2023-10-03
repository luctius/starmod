use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use walkdir::WalkDir;

pub trait AddExtension {
    fn add_extension(&self, extension: impl AsRef<str>) -> Utf8PathBuf;
}
impl<'a> AddExtension for &'a Utf8Path {
    fn add_extension(&self, extension: impl AsRef<str>) -> Utf8PathBuf {
        let orig_extension = self.extension();
        if let Some(orig_extension) = orig_extension {
            self.with_extension(format!("{}.{}", orig_extension, extension.as_ref()))
        } else {
            self.with_extension(extension)
        }
    }
}
impl AddExtension for Utf8PathBuf {
    fn add_extension(&self, extension: impl AsRef<str>) -> Utf8PathBuf {
        self.as_path().add_extension(extension)
    }
}

pub fn rename_recursive(path: &Utf8Path) -> Result<()> {
    let walker = WalkDir::new(path)
        .min_depth(1)
        .max_depth(usize::MAX)
        .follow_links(false)
        .same_file_system(true)
        .contents_first(true);

    for entry in walker {
        let entry = entry?;
        let entry_path = Utf8PathBuf::try_from(entry.path().to_path_buf())?;

        if entry_path.is_dir() || entry_path.is_file() {
            lower_case(&entry_path)?;
        } else {
            continue;
        }
    }

    Ok(())
}

pub fn lower_case(path: &Utf8Path) -> Result<()> {
    let name = path.file_name().unwrap();
    let name = name.to_lowercase();
    let name = path.with_file_name(name);

    log::trace!("rename lower-case {} -> {}", path, name);

    std::fs::rename(path, path.with_file_name(name).as_std_path())?;

    Ok(())
}
