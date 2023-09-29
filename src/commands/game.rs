use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;
use walkdir::WalkDir;

use crate::settings::{SettingErrors, Settings};

#[derive(Clone, Debug, Parser)]
pub enum GameCmd {
    Run {
        #[command(subcommand)]
        cmd: Option<RunCmd>,
    },
    EditGameConfig {
        #[arg(short, long)]
        config_name: Option<String>,
    },
}
impl Default for GameCmd {
    fn default() -> Self {
        Self::Run {
            cmd: Some(RunCmd::default()),
        }
    }
}
impl GameCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Run { cmd } => cmd.unwrap_or_default().execute(settings),
            Self::EditGameConfig { config_name } => edit_game_config_files(settings, config_name),
        }
    }
}

#[derive(Clone, Debug, Parser, Default)]
pub enum RunCmd {
    #[default]
    Game,
    Loader,
    Loot,
    XEdit,
}
impl RunCmd {
    pub fn execute(self, settings: &mut Settings) -> Result<()> {
        match self {
            Self::Game => run_game(&settings, false),
            Self::Loader => run_game(&settings, true),
            Self::Loot => todo!(),
            Self::XEdit => todo!(),
        }
    }
}

fn run_game(settings: &Settings, loader: bool) -> Result<()> {
    if let Some(proton_dir) = settings.proton_dir() {
        if let Some(compat_dir) = settings.compat_dir() {
            if let Some(steam_dir) = settings.steam_dir() {
                let mut compat_dir = compat_dir.to_path_buf();
                if compat_dir.file_name().unwrap_or_default()
                    != settings.game().steam_id().to_string().as_str()
                {
                    compat_dir.push(settings.game().steam_id().to_string());
                }
                let mut proton_exe = proton_dir.to_path_buf();
                proton_exe.push("proton");
                let mut game_exe = settings.game_dir().to_path_buf();

                if !loader {
                    game_exe.push(settings.game().exe_name());
                } else {
                    game_exe.push(settings.game().loader_name());
                }

                if game_exe.exists() {
                    log::info!("Running 'STEAM_COMPAT_DATA_PATH={} STEAM_COMPAT_CLIENT_INSTALL_PATH={} {} run {}'", compat_dir, steam_dir, proton_exe, game_exe);

                    let output = std::process::Command::new(proton_exe)
                        .arg("run")
                        // .arg("waitforexitandrun")
                        .arg(game_exe)
                        .env("STEAM_COMPAT_DATA_PATH", compat_dir)
                        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam_dir)
                        .output()?;

                    if !output.status.success() {
                        log::info!("{:?}", output.stdout);
                    }
                    Ok(())
                } else {
                    Err(SettingErrors::GameExeNotFound(game_exe).into())
                }
            } else {
                Err(SettingErrors::NoSteamDirFound(settings.cmd_name().to_owned()).into())
            }
        } else {
            Err(SettingErrors::NoCompatDirFound(settings.cmd_name().to_owned()).into())
        }
    } else {
        Err(SettingErrors::NoProtonDirFound(settings.cmd_name().to_owned()).into())
    }
}

fn edit_game_config_files(settings: &Settings, config_name: Option<String>) -> Result<()> {
    let mut config_files_to_edit = Vec::new();
    let mut game_my_document_dir = settings.compat_dir().unwrap().to_path_buf();
    game_my_document_dir.push(settings.game().steam_id().to_string());
    game_my_document_dir.push(settings.game().my_game_dir());

    if let Some(config_name) = config_name {
        game_my_document_dir.push(config_name);
        config_files_to_edit.push(game_my_document_dir);
    } else {
        WalkDir::new(game_my_document_dir.as_path())
            .min_depth(1)
            .max_depth(usize::MAX)
            .follow_links(false)
            .same_file_system(false)
            .contents_first(false)
            .into_iter()
            .filter_entry(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|f| settings.game().ini_files().contains(&f))
                    .unwrap_or(false)
            })
            .for_each(|f| {
                if let Ok(f) = f {
                    config_files_to_edit.push(Utf8PathBuf::try_from(f.into_path()).unwrap())
                }
            });
    }

    if !config_files_to_edit.is_empty() {
        log::info!("Editing: {:?}", config_files_to_edit);

        let mut editor_cmd = std::process::Command::new(settings.editor());
        for f in config_files_to_edit {
            editor_cmd.arg(f);
        }
        editor_cmd.spawn()?.wait()?;
    } else {
        log::info!("No relevant config files found.");
    }

    Ok(())
}
