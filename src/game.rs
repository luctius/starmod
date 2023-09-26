use camino::Utf8PathBuf;
use loadorder::GameId;
use serde::{Deserialize, Serialize};

// const STEAM_APPS_NAME: &'static str = "steamapps";

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum Game {
    #[default]
    Starfield,
}
impl Game {
    pub fn mod_manager_name(&self) -> &'static str {
        match self {
            Self::Starfield => "starmod",
        }
    }
    pub fn game_name(&self) -> &'static str {
        match self {
            Self::Starfield => "Starfield",
        }
    }
    pub fn nexus_game_name(&self) -> &'static str {
        match self {
            Self::Starfield => "starfield",
        }
    }
    pub fn game_id(&self) -> GameId {
        match self {
            Self::Starfield => GameId::Starfield,
        }
    }
    pub const fn steam_id(&self) -> usize {
        match self {
            Self::Starfield => 1716740,
        }
    }
    pub const fn exe_name(&self) -> &'static str {
        match self {
            Self::Starfield => "Starfield.exe",
        }
    }
    pub const fn loader_name(&self) -> &'static str {
        match self {
            Self::Starfield => "sfse_loader.exe",
        }
    }
    pub const fn ini_files(&self) -> &[&'static str] {
        match self {
            Self::Starfield => &["Starfield.ini", "StarfieldPrefs.ini", "StarfieldCustom.ini"],
        }
    }
    pub const fn my_game_dir(&self) -> &'static str {
        match self {
            Self::Starfield => "pfx/drive_c/users/steamuser/My Documents/My Games/Starfield",
        }
    }
    pub fn find_game(&self) -> Option<Utf8PathBuf> {
        // dirs::home_dir()
        //     .map(|home_dir| {
        //         let walker = WalkDir::new(&home_dir)
        //             .min_depth(1)
        //             .max_depth(usize::MAX)
        //             .follow_links(false)
        //             .same_file_system(false)
        //             .contents_first(false);

        //         walker
        //             .into_iter()
        //             .filter_entry(|entry| {
        //                 let exe_name = self.exe_name().to_lowercase();

        //                 entry
        //                     .file_name()
        //                     .to_str()
        //                     .map(|s| s.to_lowercase().as_str() == exe_name.as_str())
        //                     .unwrap_or(false)
        //             })
        //             .next()
        //             .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
        //     })
        //     .flatten()
        None
    }
    pub fn find_steam_dirs() -> Vec<Utf8PathBuf> {
        // let mut steam_dirs = Vec::new();

        // if let Some(home_dir) = dirs::home_dir() {
        //     let walker = WalkDir::new(&home_dir)
        //         .min_depth(1)
        //         .max_depth(usize::MAX)
        //         .follow_links(false)
        //         .same_file_system(false)
        //         .contents_first(false);

        //     if let Some(steam_dir) = walker
        //         .into_iter()
        //         .filter_entry(|entry| {
        //             let steamapps = STEAM_APPS_NAME.to_lowercase();

        //             entry
        //                 .file_name()
        //                 .to_str()
        //                 .map(|s| s.to_lowercase().as_str() == steamapps.as_str())
        //                 .unwrap_or(false)
        //         })
        //         .next()
        //         .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
        //     {
        //         steam_dirs.push(steam_dir);
        //     }
        // }

        // steam_dirs
        vec![]
    }
    fn find_compat_dir(&self, steam_dirs: &[Utf8PathBuf]) -> Option<Utf8PathBuf> {
        // for steam_dir in steam_dirs {
        //     let walker = WalkDir::new(&steam_dir)
        //         .min_depth(1)
        //         .max_depth(usize::MAX)
        //         .follow_links(false)
        //         .same_file_system(false)
        //         .contents_first(false);

        //     if let Some(compat_dir) = walker
        //         .into_iter()
        //         .filter_entry(|entry| {
        //             let compat_name = self.steam_id().to_string();

        //             entry
        //                 .file_name()
        //                 .to_str()
        //                 .map(|s| s == compat_name.as_str())
        //                 .unwrap_or(false)
        //         })
        //         .next()
        //         .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
        //     {
        //         return Some(compat_dir);
        //     }
        // }
        None
    }
    fn find_proton_version(_steam_dirs: &[Utf8PathBuf]) -> Option<String> {
        todo!()
    }
    fn find_proton(_steam_dirs: &[Utf8PathBuf]) -> Option<Utf8PathBuf> {
        //     for steam_dir in steam_dirs {
        //         let walker = WalkDir::new(steam_dir)
        //             .min_depth(1)
        //             .max_depth(usize::MAX)
        //             .follow_links(false)
        //             .same_file_system(false)
        //             .contents_first(false);

        //         if let (proton_dir)  walker
        //             .into_iter()
        //             .filter_entry(|entry| {
        //                 let compat_name = Self::STEAM_ID.to_string();

        //                 entry
        //                     .file_name()
        //                     .to_str()
        //                     .map(|s| s.to_lowercase().as_str() == compat_name.as_str())
        //                     .unwrap_or(false)
        //             })
        //             .next()
        //             .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
        // }

        todo!()
    }
}
