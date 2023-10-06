use thiserror::Error;

pub mod custom;
pub mod data;
pub mod fomod;
pub mod label;
pub mod loader;

// These are existing directories in the Starfield game dir
// Ensure we use the same casing to avoid multiple similar directories.
pub const DATA_DIR_NAME: &str = "Data";
pub const TEXTURES_DIR_NAME: &str = "Textures";

#[derive(Error, Debug)]
pub enum InstallerError {
    #[allow(unused)]
    #[error("the mod {0} has unmet dependencies.")]
    DependenciesNotMet(String),
    #[error("the mod {0} has multiple data directories.")]
    MultipleDataDirectories(String),
    #[error("the installer of mod {0} has been cancelled.")]
    InstallerCancelled(String),
}

pub mod stdin {
    use std::{fmt::Display, num::ParseIntError, str::FromStr};

    #[derive(Copy, Clone, Debug)]
    pub enum Input {
        Digit(u8),
        Exit,
    }
    impl Display for Input {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "input")
        }
    }
    impl FromStr for Input {
        type Err = ParseIntError;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            if s.to_lowercase() == "e" || s.to_lowercase() == "exit" {
                Ok(Self::Exit)
            } else {
                Ok(Self::Digit(s.parse::<u8>()?))
            }
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub enum InputWithDone {
        Input(Input),
        Done,
    }
    impl From<Input> for InputWithDone {
        fn from(i: Input) -> Self {
            Self::Input(i)
        }
    }
    impl Display for InputWithDone {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "input")
        }
    }
    impl FromStr for InputWithDone {
        type Err = ParseIntError;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            if s.to_lowercase() == "d" || s.to_lowercase() == "done" {
                Ok(Self::Done)
            } else {
                Input::from_str(s).map(Self::from)
            }
        }
    }
}
