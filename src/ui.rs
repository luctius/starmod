mod list;

use inquire::Select;
pub use list::{FileListBuilder, ListBuilder, ModListBuilder};

mod inquiry;
pub use inquiry::{InquireBuilder, SelectToIdx};

use anyhow::Result;

use crate::mods::FindInModList;

pub struct FindSelectBuilder<'a, B: ListBuilder> {
    msg: Option<&'a str>,
    list_builder: B,
    input: Option<&'a str>,
}
impl<'a, B: ListBuilder> FindSelectBuilder<'a, B> {
    pub fn new(list_builder: B) -> Self {
        Self {
            msg: None,
            list_builder,
            input: None,
        }
    }

    pub fn with_msg(mut self, msg: &'a str) -> Self {
        self.msg = Some(msg);
        self
    }
    pub fn with_input(mut self, input: Option<&'a str>) -> Self {
        self.input = input;
        self
    }
}
impl<'a> FindSelectBuilder<'a, ModListBuilder<'a>> {
    pub fn build(self) -> Result<InquireBuilder<SelectToIdx<'a, String>>> {
        let idx = self
            .input
            .map(|input| self.list_builder.list().find_mod(input))
            .flatten();

        let list = self.list_builder.build()?;

        let select = SelectToIdx::new(self.msg.unwrap_or_default(), list);
        let select = if let Some(input) = self.input {
            select.with_starting_filter_input(input)
        } else {
            select
        };

        Ok(InquireBuilder::new_with_test(idx, select))
    }
}
impl<'a> FindSelectBuilder<'a, FileListBuilder<'a>> {
    pub fn build(self) -> Result<InquireBuilder<Select<'a, String>>> {
        // let idx = self
        //     .input
        //     .map(|input| self.list_builder.list().find_mod(input))
        //     .flatten();

        // let list = self.list_builder.build()?;

        // let select = SelectToIdx::new(self.msg.unwrap_or_default(), list);
        // let select = if let Some(input) = self.input {
        //     select.with_starting_filter_input(input)
        // } else {
        //     select
        // };

        todo!()

        // Ok(InquireBuilder::new_with_test(idx, select))
    }
}
