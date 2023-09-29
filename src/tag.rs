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
                Tag::Enabled => "Enabled",
                Tag::Winner => "Winner",
                Tag::Loser => "Loser",
                Tag::CompleteLoser => "All Files Overwritten",
                Tag::Conflict => "Conflict",
                Tag::Disabled => "Disabled",
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
            Tag::Enabled => Color::White,
            Tag::Winner => Color::Green,
            Tag::Loser => Color::Yellow,
            Tag::CompleteLoser => Color::Red,
            Tag::Conflict => Color::Magenta,
            Tag::Disabled => Color::DarkGrey,
        }
    }
}
impl From<(bool, bool)> for Tag {
    fn from((loser, winner): (bool, bool)) -> Self {
        match (loser, winner) {
            (false, false) => Tag::Enabled,
            (false, true) => Tag::Winner,
            (true, false) => Tag::Loser,
            (true, true) => Tag::Conflict,
        }
    }
}
