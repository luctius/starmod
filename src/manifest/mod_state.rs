use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModState {
    Enabled,
    Disabled,
}
impl ModState {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Enabled => true,
            Self::Disabled => false,
        }
    }
}
impl From<bool> for ModState {
    fn from(v: bool) -> Self {
        match v {
            true => Self::Enabled,
            false => Self::Disabled,
        }
    }
}
impl Display for ModState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModState::Enabled => f.write_str("Enabled"),
            ModState::Disabled => f.write_str("Disabled"),
        }
    }
}
