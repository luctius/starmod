use std::{fmt::Display, marker::PhantomData};

mod sealed {
    use std::marker::PhantomData;

    pub trait InquireExt<T>: Sized {
        type Output;
        fn prompt(self) -> inquire::error::InquireResult<Self::Output>;
    }

    pub struct InquireBuilder2<T, T2, I: InquireExt<T>, B: InquireExt<T2>> {
        pub(super) test: Option<<B as InquireExt<T2>>::Output>,
        pub(super) branch: I,
        pub(super) leaf: B,
        pub(super) _p1: PhantomData<T>,
        pub(super) _p2: PhantomData<T2>,
    }
}
use inquire::error::InquireResult;
use sealed::{InquireBuilder2, InquireExt};

use crate::{errors::ModErrors, settings::default_page_size};

pub struct InquireBuilder<T, I: InquireExt<T>> {
    test: Option<<I as InquireExt<T>>::Output>,
    inquire: I,
    _p: PhantomData<T>,
}
impl<T, I: InquireExt<T>> InquireBuilder<T, I> {
    pub fn new(inquire: I) -> Self {
        Self {
            test: None,
            inquire,
            _p: PhantomData,
        }
    }
    pub fn new_with_test(test: Option<<I as InquireExt<T>>::Output>, inquire: I) -> Self {
        Self {
            test,
            inquire,
            _p: PhantomData,
        }
    }
    pub fn with<T2, B: InquireExt<T2>>(
        self,
        next_inquire: B,
    ) -> InquireBuilder2<T, T2, InquireBuilder<T, I>, B> {
        InquireBuilder2 {
            test: None,
            branch: self,
            leaf: next_inquire,
            _p1: PhantomData,
            _p2: PhantomData,
        }
    }
    pub fn with_test<T2, B: InquireExt<T2>>(
        self,
        test: Option<<B as InquireExt<T2>>::Output>,
        next_inquire: B,
    ) -> InquireBuilder2<T, T2, InquireBuilder<T, I>, B> {
        InquireBuilder2 {
            test,
            branch: self,
            leaf: next_inquire,
            _p1: PhantomData,
            _p2: PhantomData,
        }
    }
    pub fn prompt(self) -> InquireResult<<I as InquireExt<T>>::Output> {
        if let Some(test) = self.test {
            Ok(test)
        } else {
            self.inquire.prompt()
        }
    }
}
impl<T, I: InquireExt<T>> InquireExt<T> for InquireBuilder<T, I> {
    type Output = <I as InquireExt<T>>::Output;

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}

impl<T, T2, I: InquireExt<T>, B: InquireExt<T2>> InquireBuilder2<T, T2, I, B> {
    pub fn with<T3, B2: InquireExt<T3>>(
        self,
        next_inquire: B2,
    ) -> InquireBuilder2<(T, T2), T3, InquireBuilder2<T, T2, I, B>, B2> {
        InquireBuilder2 {
            test: None,
            branch: self,
            leaf: next_inquire,
            _p1: PhantomData,
            _p2: PhantomData,
        }
    }
    pub fn with_test<T3, B2: InquireExt<T3>>(
        self,
        test: Option<<B2 as InquireExt<T3>>::Output>,
        next_inquire: B2,
    ) -> InquireBuilder2<(T, T2), T3, InquireBuilder2<T, T2, I, B>, B2> {
        InquireBuilder2 {
            test,
            branch: self,
            leaf: next_inquire,
            _p1: PhantomData,
            _p2: PhantomData,
        }
    }
    pub fn prompt(
        self,
    ) -> InquireResult<(<I as InquireExt<T>>::Output, <B as InquireExt<T2>>::Output)> {
        let t = self.branch.prompt()?;

        let t2 = if let Some(test) = self.test {
            test
        } else {
            self.leaf.prompt()?
        };

        Ok((t, t2))
    }
}
impl<T, T2, I: InquireExt<T>, B: InquireExt<T2>> InquireExt<(T, T2)>
    for InquireBuilder2<T, T2, I, B>
{
    type Output = (<I as InquireExt<T>>::Output, <B as InquireExt<T2>>::Output);

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}

impl<'a, T: Display> InquireExt<T> for inquire::Select<'a, T> {
    type Output = T;

    fn prompt(self) -> InquireResult<Self::Output> {
        inquire::Select::prompt(self)
    }
}

impl<'a, T: Display + Clone> InquireExt<T> for inquire::CustomType<'a, T> {
    type Output = T;

    fn prompt(self) -> InquireResult<Self::Output> {
        inquire::CustomType::prompt(self)
    }
}

impl<'a, T: Display> InquireExt<T> for inquire::MultiSelect<'a, T> {
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
    pub fn prompt(self) -> InquireResult<<Self as InquireExt<T>>::Output> {
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
impl<'a, T: Display + Clone + PartialEq> InquireExt<T> for SelectToIdx<'a, T> {
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
    pub fn prompt(self) -> InquireResult<<Self as InquireExt<T>>::Output> {
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
impl<'a, T: Display + Clone + PartialEq> InquireExt<T> for MultiSelectToIdx<'a, T> {
    type Output = Vec<usize>;

    fn prompt(self) -> InquireResult<Self::Output> {
        self.prompt()
    }
}
