use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::settings::SettingErrors;

const STEAM_APPS_NAME: &'static str = "steamapps";

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Game {
    StarMod,
}
impl Game {
    pub const fn allowed_names() -> &'static [&'static str] {
        &[
            "starmod",
            //"skymod", "mormod", "obmod", "fo3mod", "fo4mod", "fnvmod"
        ]
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::StarMod => Self::allowed_names()[0],
        }
    }
    pub fn game_name(&self) -> &'static str {
        match self {
            Self::StarMod => "Starfield",
        }
    }
    pub fn create_from_name(app_name: &str) -> Result<Self> {
        for (i, n) in Self::allowed_names().iter().enumerate() {
            if *n == app_name.to_lowercase().as_str() {
                match i {
                    0 => return Ok(Self::StarMod),
                    _ => (),
                }
            }
        }
        Err(SettingErrors::WrongAppName(
            app_name.to_owned(),
            format!("{:?}", Game::allowed_names()),
        )
        .into())
    }
    pub const fn steam_id(&self) -> usize {
        match self {
            Self::StarMod => 1716740,
        }
    }
    pub const fn exe_name(&self) -> &'static str {
        match self {
            Self::StarMod => "Starfield.exe",
        }
    }
    pub const fn loader_name(&self) -> &'static str {
        match self {
            Self::StarMod => "sfse.exe",
        }
    }
    pub const fn ini_files(&self) -> &[&'static str] {
        match self {
            Self::StarMod => &["Starfield.ini", "StarfieldPrefs.ini", "StarfieldCustom.ini"],
        }
    }
    pub fn find_game(&self) -> Option<PathBuf> {
        dirs::home_dir()
            .map(|home_dir| {
                let walker = WalkDir::new(&home_dir)
                    .min_depth(1)
                    .max_depth(usize::MAX)
                    .follow_links(false)
                    .same_file_system(false)
                    .contents_first(false);

                walker
                    .into_iter()
                    .filter_entry(|entry| {
                        let exe_name = self.exe_name().to_lowercase();

                        entry
                            .file_name()
                            .to_str()
                            .map(|s| s.to_lowercase().as_str() == exe_name.as_str())
                            .unwrap_or(false)
                    })
                    .next()
                    .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
            })
            .flatten()
    }
    pub fn find_steam_dirs() -> Vec<PathBuf> {
        let mut steam_dirs = Vec::new();

        if let Some(home_dir) = dirs::home_dir() {
            let walker = WalkDir::new(&home_dir)
                .min_depth(1)
                .max_depth(usize::MAX)
                .follow_links(false)
                .same_file_system(false)
                .contents_first(false);

            if let Some(steam_dir) = walker
                .into_iter()
                .filter_entry(|entry| {
                    let steamapps = STEAM_APPS_NAME.to_lowercase();

                    entry
                        .file_name()
                        .to_str()
                        .map(|s| s.to_lowercase().as_str() == steamapps.as_str())
                        .unwrap_or(false)
                })
                .next()
                .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
            {
                steam_dirs.push(steam_dir);
            }
        }

        steam_dirs
    }
    fn find_compat_dir(&self, steam_dirs: &[PathBuf]) -> Option<PathBuf> {
        for steam_dir in steam_dirs {
            let walker = WalkDir::new(&steam_dir)
                .min_depth(1)
                .max_depth(usize::MAX)
                .follow_links(false)
                .same_file_system(false)
                .contents_first(false);

            if let Some(compat_dir) = walker
                .into_iter()
                .filter_entry(|entry| {
                    let compat_name = self.steam_id().to_string();

                    entry
                        .file_name()
                        .to_str()
                        .map(|s| s == compat_name.as_str())
                        .unwrap_or(false)
                })
                .next()
                .map(|de| de.map(|de| de.into_path()).unwrap_or_default())
            {
                return Some(compat_dir);
            }
        }
        None
    }
    fn find_proton_version(_steam_dirs: &[PathBuf]) -> Option<String> {
        todo!()
    }
    fn find_proton(_steam_dirs: &[PathBuf]) -> Option<PathBuf> {
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
