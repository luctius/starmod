use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum ModState {
    Enabled,
    #[default]
    Disabled,
}
impl ModState {
    pub const fn is_enabled(self) -> bool {
        match self {
            Self::Enabled => true,
            Self::Disabled => false,
        }
    }
}
impl From<bool> for ModState {
    fn from(v: bool) -> Self {
        if v {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}
impl From<ModState> for bool {
    fn from(ms: ModState) -> Self {
        ms.is_enabled()
    }
}
impl Display for ModState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => f.write_str("Enabled"),
            Self::Disabled => f.write_str("Disabled"),
        }
    }
}
