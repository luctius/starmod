use std::{fmt::Display, marker::PhantomData};

mod sealed {
    use std::marker::PhantomData;

    pub trait InquireExt<T>: Sized {
        type Output;
        fn prompt(self) -> inquire::error::InquireResult<Self::Output>;
    }
    pub struct InquireBuilder2<T, T2, I: InquireExt<T>, B: InquireExt<T2>> {
        pub(super) inquire: I,
        pub(super) next_inquire: B,
        pub(super) _p1: PhantomData<T>,
        pub(super) _p2: PhantomData<T2>,
    }
}
use inquire::error::InquireResult;
use sealed::{InquireBuilder2, InquireExt};

pub struct InquireBuilder<T, I: InquireExt<T>> {
    test: Option<T>,
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
    pub fn new_with(test: Option<T>, inquire: I) -> Self {
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
            inquire: self,
            next_inquire,
            _p1: PhantomData,
            _p2: PhantomData,
        }
    }
    fn prompt(self) -> InquireResult<<I as InquireExt<T>>::Output> {
        self.test.map(Ok).unwrap_or_else(self.inquire.prompt())
    }
}
impl<T, I: InquireExt<T>> InquireExt<T> for InquireBuilder<T, I> {
    type Output = <I as InquireExt<T>>::Output;

    fn prompt(self) -> InquireResult<Self::Output> {
        self.inquire.prompt()
    }
}

impl<T, T2, I: InquireExt<T>, B: InquireExt<T2>> InquireBuilder2<T, T2, I, B> {
    pub fn with<T3, B2: InquireExt<T3>>(
        self,
        next_inquire: B2,
    ) -> InquireBuilder2<(T, T2), T3, InquireBuilder2<T, T2, I, B>, B2> {
        InquireBuilder2 {
            inquire: self,
            next_inquire,
            _p1: PhantomData,
            _p2: PhantomData,
        }
    }
    fn prompt(
        self,
    ) -> InquireResult<(<I as InquireExt<T>>::Output, <B as InquireExt<T2>>::Output)> {
        let t2 = self.next_inquire.prompt()?;
        let t = self.inquire.prompt()?;
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

impl<'a, T: Display> InquireExt<T> for inquire::MultiSelect<'a, T> {
    type Output = Vec<T>;

    fn prompt(self) -> InquireResult<Self::Output> {
        inquire::MultiSelect::prompt(self)
    }
}

#[cfg(test)]
mod tests {
    use inquire::{InquireError, Select};

    use super::InquireBuilder;

    #[test]
    pub fn test() {
        let options: Vec<&str> = vec!["Banana", "Apple"];

        let select = Select::new("What's your favorite fruit?", options);
        let ans: Result<&str, InquireError> = select.prompt();
    }

    #[test]
    pub fn test2() {
        let options: Vec<&str> = vec!["Banana", "Apple"];

        let select = InquireBuilder::new(Select::new("What's your favorite fruit?", options));
        let ans: Result<&str, InquireError> = select.prompt();
    }

    #[test]
    pub fn test3() {
        let options: Vec<&str> = vec!["Banana", "Apple"];

        let select =
            InquireBuilder::new(Select::new("What's your favorite fruit?", options.clone()))
                .with(Select::new("What's your favorite fruit?", options));
        let ans: Result<(&str, &str), InquireError> = select.prompt();
    }
}
