use camino::Utf8PathBuf;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum SettingErrors {
    #[error("No valid config file could be found; Please run '{0} update-config' first.")]
    ConfigNotFound(String),
    #[error("The game directory for {0} cannot be found, Please run '{1} update-config' and provide manually.")]
    NoGameDirFound(String, String),
    #[error("A download directory for cannot be found, Please run '{0} update-config' and provide manually.")]
    NoDownloadDirFound(String),
    #[error(
        "The cache directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoCacheDirFound(String),
    #[error(
        "The proton directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoProtonDirFound(String),
    #[error(
        "The compat directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoCompatDirFound(String),
    #[error(
        "The steam directory cannot be found, Please run '{0} update-config' and provide manually."
    )]
    NoSteamDirFound(String),
    #[error("The executable could not be found: {0}.")]
    ExecutableNotFound(Utf8PathBuf),
}

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum GameErrors {
    #[error("Could not find file(s) '{0}' in the game directories.")]
    ConfigNotFound(String),
}

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum ModErrors {
    #[error("The mod '{0}' could not be found. Is the mod installed?")]
    ModNotFound(String),
    #[error("Could not find the file(s) '{1}' in mod {0}.")]
    FileNotFound(String, String),
    #[error("Could not find tag '{1}' in mod {0}. Did you perhaps mispel it?")]
    TagNotFound(String, String),
    #[error("Could not add tag '{1}' to mod {0}. Perhaps the mod al-ready has that tag?")]
    DuplicateTag(String, String),
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("the archive {0} cannot be found.")]
    ArchiveNotFound(String),
}
