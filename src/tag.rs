use std::fmt::Display;

use comfy_table::Color;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    Enabled,
    Winner,
    Loser,
    CompleteLoser,
    Conflict,
    Disabled,
}
impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Enabled => "Enabled",
                Self::Winner => "Winner",
                Self::Loser => "Loser",
                Self::CompleteLoser => "All Files Overwritten",
                Self::Conflict => "Conflict",
                Self::Disabled => "Disabled",
            }
        )
    }
}
impl From<Tag> for char {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Enabled => 'e',
            Tag::Winner => 'w',
            Tag::Loser => 'l',
            Tag::CompleteLoser => 'L',
            Tag::Conflict => 'c',
            Tag::Disabled => 'D',
        }
    }
}
impl From<Tag> for Color {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Enabled => Self::White,
            Tag::Winner => Self::Green,
            Tag::Loser => Self::Yellow,
            Tag::CompleteLoser => Self::Red,
            Tag::Conflict => Self::Magenta,
            Tag::Disabled => Self::DarkGrey,
        }
    }
}
impl From<(bool, bool)> for Tag {
    fn from((loser, winner): (bool, bool)) -> Self {
        match (loser, winner) {
            (false, false) => Self::Enabled,
            (false, true) => Self::Winner,
            (true, false) => Self::Loser,
            (true, true) => Self::Conflict,
        }
    }
}
