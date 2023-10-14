use std::fmt::Display;

mod sealed {
    use super::InquireBuilder;

    pub trait InquireExt: Sized {
        type Output;
        fn prompt(self) -> inquire::error::InquireResult<Self::Output>;
        fn with<B: InquireExt>(self, branch: B) -> InquireBuilder2<InquireBuilder<Self>, B> {
            InquireBuilder::new(self).with(branch)
        }
        fn with_test<B: InquireExt>(
            self,
            test: Option<<B as InquireExt>::Output>,
            branch: B,
        ) -> InquireBuilder2<InquireBuilder<Self>, B> {
            InquireBuilder::new(self).with_test(test, branch)
        }
    }

    pub struct InquireBuilder2<I: InquireExt, B: InquireExt> {
        pub(super) test: Option<<B as InquireExt>::Output>,
        pub(super) branch: I,
        pub(super) leaf: B,
    }
}
use inquire::error::InquireResult;
use sealed::{InquireBuilder2, InquireExt};

use crate::{errors::ModErrors, settings::default_page_size};

pub struct InquireBuilder<I: InquireExt> {
    test: Option<<I as InquireExt>::Output>,
    inquire: I,
}
impl<I: InquireExt> InquireBuilder<I> {
    pub fn new(inquire: I) -> Self {
        Self {
            test: None,
            inquire,
        }
    }
    pub fn new_with_test(test: Option<<I as InquireExt>::Output>, inquire: I) -> Self {
        Self { test, inquire }
    }
    pub fn with<B: InquireExt>(self, next_inquire: B) -> InquireBuilder2<InquireBuilder<I>, B> {
        InquireBuilder2 {
            test: None,
            branch: self,
            leaf: next_inquire,
        }
    }
    pub fn with_test<B: InquireExt>(
        self,
        test: Option<<B as InquireExt>::Output>,
        next_inquire: B,
    ) -> InquireBuilder2<InquireBuilder<I>, B> {
        InquireBuilder2 {
            test,
            branch: self,
            leaf: next_inquire,
        }
    }
    pub fn prompt(self) -> InquireResult<<I as InquireExt>::Output> {
        if let Some(test) = self.test {
            Ok(test)
        } else {
            self.inquire.prompt()
        }
    }
}
impl<I: InquireExt> InquireExt for InquireBuilder<I> {
    type Output = <I as InquireExt>::Output;

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}
impl<'a, T: InquireExt> InquireBuilder<SelectToIdx<'a, T>>
where
    T: Display + Clone + PartialEq,
{
    pub fn with_starting_filter_input(mut self, starting_filter_input: &'a str) -> Self {
        self.inquire = self
            .inquire
            .with_starting_filter_input(starting_filter_input);
        self
    }
    pub fn with_vim_mode(mut self, vim_mode: bool) -> Self {
        self.inquire = self.inquire.with_vim_mode(vim_mode);
        self
    }
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.inquire = self.inquire.with_page_size(page_size);
        self
    }
    pub fn with_help_message(mut self, message: &'a str) -> Self {
        self.inquire = self.inquire.with_help_message(message);
        self
    }
}

impl<I: InquireExt, B: InquireExt> InquireBuilder2<I, B> {
    pub fn with<B2: InquireExt>(
        self,
        next_inquire: B2,
    ) -> InquireBuilder2<InquireBuilder2<I, B>, B2> {
        InquireBuilder2 {
            test: None,
            branch: self,
            leaf: next_inquire,
        }
    }
    pub fn with_test<B2: InquireExt>(
        self,
        test: Option<<B2 as InquireExt>::Output>,
        next_inquire: B2,
    ) -> InquireBuilder2<InquireBuilder2<I, B>, B2> {
        InquireBuilder2 {
            test,
            branch: self,
            leaf: next_inquire,
        }
    }
    pub fn prompt(self) -> InquireResult<(<I as InquireExt>::Output, <B as InquireExt>::Output)> {
        let t = self.branch.prompt()?;

        let t2 = if let Some(test) = self.test {
            test
        } else {
            self.leaf.prompt()?
        };

        Ok((t, t2))
    }
}
impl<I: InquireExt, B: InquireExt> InquireExt for InquireBuilder2<I, B> {
    type Output = (<I as InquireExt>::Output, <B as InquireExt>::Output);

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}

impl<'a, T: Display> InquireExt for inquire::Select<'a, T> {
    type Output = T;

    fn prompt(self) -> InquireResult<Self::Output> {
        inquire::Select::prompt(self)
    }
}

impl<'a, T: Display + Clone> InquireExt for inquire::CustomType<'a, T> {
    type Output = T;

    fn prompt(self) -> InquireResult<Self::Output> {
        inquire::CustomType::prompt(self)
    }
}

impl<'a, T: Display> InquireExt for inquire::MultiSelect<'a, T> {
    type Output = Vec<T>;

    fn prompt(self) -> InquireResult<Self::Output> {
        inquire::MultiSelect::prompt(self)
    }
}

pub struct SelectToIdx<'a, T> {
    list: Vec<T>,
    select: inquire::Select<'a, T>,
}
impl<'a, T: Display + Clone> SelectToIdx<'a, T> {
    pub fn new(message: &'a str, list: Vec<T>) -> Self {
        let select =
            inquire::Select::new(message, list.to_vec()).with_page_size(default_page_size());
        Self { list, select }
    }
    // pub fn new_with(select: inquire::Select<'a, T>, list: &'a [T]) -> Self {
    //     Self { list, select }
    // }
    pub fn with_starting_filter_input(mut self, starting_filter_input: &'a str) -> Self {
        self.select = self
            .select
            .with_starting_filter_input(starting_filter_input);
        self
    }
    pub fn with_vim_mode(mut self, vim_mode: bool) -> Self {
        self.select = self.select.with_vim_mode(vim_mode);
        self
    }
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.select = self.select.with_page_size(page_size);
        self
    }
    pub fn with_help_message(mut self, message: &'a str) -> Self {
        self.select = self.select.with_help_message(message);
        self
    }
}
impl<'a, T: Display + Clone + PartialEq> SelectToIdx<'a, T> {
    pub fn prompt(self) -> InquireResult<<Self as InquireExt>::Output> {
        let choice = self.select.prompt()?;

        self.list
            .iter()
            .enumerate()
            .find_map(|(idx, t)| (choice == *t).then_some(idx))
            .ok_or_else(|| {
                inquire::InquireError::Custom(Box::new(ModErrors::ModNotFound(String::new())))
            })
    }
}
impl<'a, T: Display + Clone + PartialEq> InquireExt for SelectToIdx<'a, T> {
    type Output = usize;

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}

pub struct MultiSelectToIdx<'a, T> {
    list: Vec<T>,
    select: inquire::MultiSelect<'a, T>,
}
impl<'a, T: Display + Clone> MultiSelectToIdx<'a, T> {
    pub fn new(message: &'a str, list: Vec<T>) -> Self {
        let select =
            inquire::MultiSelect::new(message, list.to_vec()).with_page_size(default_page_size());
        Self { list, select }
    }
    // pub fn new_with(select: inquire::Select<'a, T>, list: &'a [T]) -> Self {
    //     Self { list, select }
    // }
    // pub fn with_starting_filter_input(mut self, starting_filter_input: &'a str) -> Self {
    //     self.select = self
    //         .select
    //         .with_starting_filter_input(starting_filter_input);
    //     self
    // }
    pub fn with_vim_mode(mut self, vim_mode: bool) -> Self {
        self.select = self.select.with_vim_mode(vim_mode);
        self
    }
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.select = self.select.with_page_size(page_size);
        self
    }
    pub fn with_help_message(mut self, message: &'a str) -> Self {
        self.select = self.select.with_help_message(message);
        self
    }
}
impl<'a, T: Display + Clone + PartialEq> MultiSelectToIdx<'a, T> {
    pub fn prompt(self) -> InquireResult<<Self as InquireExt>::Output> {
        let choice = self.select.prompt()?;

        let mut idx_list = Vec::with_capacity(choice.len());

        for c in choice {
            let idx = self
                .list
                .iter()
                .enumerate()
                .find_map(|(idx, t)| (c == *t).then_some(idx))
                .ok_or_else(|| {
                    inquire::InquireError::Custom(Box::new(ModErrors::ModNotFound(String::new())))
                })?;
            idx_list.push(idx);
        }

        Ok(idx_list)
    }
}
impl<'a, T: Display + Clone + PartialEq> InquireExt for MultiSelectToIdx<'a, T> {
    type Output = Vec<usize>;

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}
