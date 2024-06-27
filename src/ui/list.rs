use std::collections::HashMap;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use comfy_table::{Cell, Color};

use crate::{
    commands::downloads::downloaded_files,
    conflict::{conflict_list_by_file, conflict_list_by_mod},
    decompress::SupportedArchives,
    dmodman::DmodMan,
    manifest::Manifest,
    mods::GatherModList,
    settings::create_table,
    tag::Tag,
    utils::AddExtension,
};

pub trait ListBuilder {
    fn build(self) -> Result<Vec<String>>;
}

pub struct ModListBuilder<'a> {
    list: &'a [Manifest],
    download_dir: Option<Utf8PathBuf>,
    with_index: bool,
    with_priority: bool,
    with_status: bool,
    with_version: bool,
    with_nexus_id: bool,
    with_mod_type: bool,
    with_tags: bool,
    with_notes: bool,
    with_colour: bool,
    with_headers: bool,
}
impl<'a> ModListBuilder<'a> {
    pub fn new(list: &'a [Manifest]) -> Self {
        Self {
            list,
            with_index: false,
            with_priority: false,
            with_status: false,
            with_version: false,
            with_nexus_id: false,
            with_mod_type: false,
            with_tags: false,
            with_notes: false,
            with_colour: false,
            with_headers: false,
            download_dir: None,
        }
    }
    pub fn with_index(mut self) -> Self {
        self.with_index = true;
        self
    }
    pub fn with_priority(mut self) -> Self {
        self.with_priority = true;
        self
    }
    pub fn with_status(mut self) -> Self {
        self.with_status = true;
        self
    }
    pub fn with_version(mut self) -> Self {
        self.with_version = true;
        self
    }
    pub fn with_nexus_id(mut self) -> Self {
        self.with_nexus_id = true;
        self
    }
    pub fn with_mod_type(mut self) -> Self {
        self.with_mod_type = true;
        self
    }
    pub fn with_tags(mut self) -> Self {
        self.with_tags = true;
        self
    }
    pub fn with_notes(mut self, download_dir: &Utf8Path) -> Self {
        self.with_notes = true;
        self.download_dir = Some(download_dir.to_owned());
        self
    }
    pub fn with_colour(mut self) -> Self {
        self.with_colour = true;
        self
    }
    pub fn with_headers(mut self) -> Self {
        self.with_headers = true;
        self
    }
    pub fn list(&self) -> &[Manifest] {
        self.list
    }
    pub fn build(self) -> Result<Vec<String>> {
        log::trace!("Building Mod List");

        let conflict_list = conflict_list_by_mod(self.list)?;
        let file_conflist_list = conflict_list_by_file(self.list)?;

        let headers = if self.with_headers {
            let mut headers = Vec::new();
            if self.with_index {
                headers.push("Index");
            }
            headers.push("Name");
            if self.with_priority {
                headers.push("Priority");
            }
            if self.with_status {
                headers.push("Status");
            }
            if self.with_version {
                headers.push("Version");
            }
            if self.with_nexus_id {
                headers.push("Nexus Id");
            }
            if self.with_mod_type {
                headers.push("Mod Type");
            }
            if self.with_tags {
                headers.push("Tags");
            }
            if self.with_notes {
                headers.push("Notes");
            }
            headers
        } else {
            vec![]
        };

        let mut table = create_table(headers);

        let dmodman_list = if self.with_notes {
            DmodMan::gather_list(&self.download_dir.unwrap())?
        } else {
            vec![]
        };

        for (idx, m) in self.list.iter().enumerate() {
            let mut row = Vec::new();

            let is_loser = conflict_list
                .get(&m.name().to_string())
                .is_some_and(|c| !c.losing_to().is_empty());
            let is_winner = conflict_list
                .get(&m.name().to_string())
                .is_some_and(|c| !c.winning_over().is_empty());

            // Detect if we all files of this manifest are overwritten by other mods
            let tag = Tag::from((is_loser, is_winner));
            let tag = if is_loser {
                let mut file_not_lost = false;

                for f in m.dest_files()? {
                    if let Some(contenders) = file_conflist_list.get(&f) {
                        if let Some(c) = contenders.last() {
                            if c == m.name() {
                                file_not_lost = true;
                            }
                        }
                    } else {
                        file_not_lost = true;
                    }
                }

                if file_not_lost {
                    tag
                } else {
                    Tag::CompleteLoser
                }
            } else {
                tag
            };
            let tag = if m.is_enabled() { tag } else { Tag::Disabled };

            let (color, idx_color) = if self.with_colour {
                let color = Color::from(tag);
                if color == Color::White {
                    (color, Color::Reset)
                } else {
                    (color, color)
                }
            } else {
                (Color::Reset, Color::Reset)
            };

            if self.with_index {
                row.push(Cell::new(idx.to_string()).fg(idx_color));
            }
            row.push(Cell::new(m.name().to_string()).fg(color));
            if self.with_priority {
                row.push(Cell::new(m.priority().to_string()).fg(color));
            }
            if self.with_status {
                row.push(Cell::new(m.mod_state().to_string()).fg(color));
            }
            if self.with_version {
                row.push(Cell::new(m.version().unwrap_or("<Unknown>").to_string()).fg(color));
            }
            if self.with_nexus_id {
                row.push(
                    Cell::new(
                        m.nexus_id()
                            .map_or("<Unknown>".to_owned(), |nid| nid.to_string()),
                    )
                    .fg(color),
                );
            }
            if self.with_mod_type {
                row.push(Cell::new(m.kind().to_string()).fg(color));
            }
            if self.with_tags {
                row.push(Cell::new(format!("{}", m.tags().join(","))));
            }
            if self.with_notes {
                let notes = {
                    if dmodman_list.iter().any(|dmod| m.is_an_update(dmod)) {
                        "Update Available"
                    } else {
                        ""
                    }
                };
                row.push(Cell::new(notes));
            }

            table.add_row(row);
        }

        let skip = if self.with_headers { 0 } else { 1 };

        log::trace!("Finished Building Mod List");
        Ok(table.lines().skip(skip).collect::<Vec<_>>())
    }
}
impl<'a> ListBuilder for ModListBuilder<'a> {
    fn build(self) -> Result<Vec<String>> {
        self.build()
    }
}

pub struct FileListBuilder<'a> {
    manifest: &'a Manifest,
    disabled_files: bool,
    with_index: bool,
    with_origin: bool,
    with_headers: bool,
    with_colour: bool,
}
impl<'a> FileListBuilder<'a> {
    pub fn new(manifest: &'a Manifest) -> Self {
        Self {
            manifest,
            disabled_files: false,
            with_index: false,
            with_origin: false,
            with_headers: false,
            with_colour: false,
        }
    }
    pub fn disabled_files(mut self) -> Self {
        self.disabled_files = true;
        self
    }
    pub fn with_index(mut self) -> Self {
        self.with_index = true;
        self
    }
    pub fn with_origin(mut self) -> Self {
        self.with_origin = true;
        self
    }
    pub fn with_headers(mut self) -> Self {
        self.with_headers = true;
        self
    }
    pub fn build(self) -> Result<Vec<String>> {
        let headers = if self.with_headers {
            let mut headers = Vec::new();
            if self.with_index {
                headers.push("Index");
            }
            if self.with_origin {
                headers.push("Source");
            }
            headers.push("Destination");
            headers
        } else {
            vec![]
        };

        let mut table = create_table(headers);

        let files = if self.disabled_files {
            self.manifest.disabled_files()
        } else {
            self.manifest.files()?
        };

        for (idx, isf) in files.iter().enumerate() {
            let color = Color::White;
            let mut row = vec![];

            if self.with_index {
                row.push(Cell::new(idx).fg(color))
            }
            if self.with_origin {
                row.push(Cell::new(isf.source().to_string()).fg(color));
            }
            row.push(Cell::new(isf.destination().to_string()).fg(color));

            table.add_row(row);
        }

        let skip = if self.with_headers { 0 } else { 1 };

        Ok(table.lines().skip(skip).collect::<Vec<_>>())
    }
}
impl<'a> ListBuilder for FileListBuilder<'a> {
    fn build(self) -> Result<Vec<String>> {
        self.build()
    }
}

pub struct ArchiveListBuilder<'a> {
    download_dir: &'a Utf8Path,
    cache_dir: &'a Utf8Path,
    with_index: bool,
    with_status: bool,
    with_headers: bool,
    with_colour: bool,
}
impl<'a> ArchiveListBuilder<'a> {
    pub fn new(download_dir: &'a Utf8Path, cache_dir: &'a Utf8Path) -> Self {
        Self {
            download_dir,
            cache_dir,
            with_index: false,
            with_status: false,
            with_headers: false,
            with_colour: false,
        }
    }
    pub fn with_index(mut self) -> Self {
        self.with_index = true;
        self
    }
    pub fn with_status(mut self) -> Self {
        self.with_status = true;
        self
    }
    pub fn with_headers(mut self) -> Self {
        self.with_headers = true;
        self
    }
    pub fn with_colour(mut self) -> Self {
        self.with_colour = true;
        self
    }
    pub fn list(&self) -> Result<Vec<(SupportedArchives, Utf8PathBuf)>> {
        downloaded_files(self.download_dir)
    }
    pub fn build(self) -> Result<Vec<String>> {
        let sf = self.list()?;
        let mod_list = Vec::gather_mods(self.cache_dir)?;
        let mod_list = mod_list
            .iter()
            .map(|m| (m.bare_file_name().to_string(), m))
            .collect::<HashMap<_, _>>();

        let headers = if self.with_headers {
            let mut headers = Vec::new();
            if self.with_index {
                headers.push("Index");
            }
            headers.push("File");
            if self.with_status {
                headers.push("Status");
            }
            headers
        } else {
            vec![]
        };

        let mut table = create_table(headers);

        for (idx, (_, f)) in sf.iter().enumerate() {
            let dmodman = DmodMan::try_from(self.download_dir.join(&f).add_extension("json")).ok();
            let archive = dmodman.as_ref().map_or_else(
                || f.with_extension("").as_str().to_lowercase(),
                DmodMan::name,
            );
            let manifest = mod_list.get(&archive);

            log::trace!("testing {} against {}.", f.as_str(), archive);

            let state = if self.with_status {
                (
                    // is installed
                    manifest.is_some(),
                    // is an upgrade
                    dmodman
                        .and_then(|dmod| manifest.map(|m| m.is_an_update(&dmod)))
                        .unwrap_or(false),
                )
            } else {
                (true, false)
            };

            let state_name = if self.with_status {
                match state {
                    (true, false) => "Installed",
                    (true, true) => "Upgrade",
                    (false, _) => "New",
                }
            } else {
                ""
            };

            let colour = if self.with_colour {
                match state {
                    (true, false) => Color::Grey,
                    (true, true) => Color::Yellow,
                    (false, _) => Color::Green,
                }
            } else {
                Color::Reset
            };

            let mut row = vec![];
            if self.with_index {
                row.push(Cell::new(idx).fg(colour));
            }
            row.push(Cell::new(f).fg(colour));
            if self.with_status {
                row.push(Cell::new(state_name).fg(colour));
            }

            table.add_row(row);
        }

        let skip = if self.with_headers { 0 } else { 1 };

        Ok(table.lines().skip(skip).collect::<Vec<_>>())
    }
}
impl<'a> ListBuilder for ArchiveListBuilder<'a> {
    fn build(self) -> Result<Vec<String>> {
        self.build()
    }
}
