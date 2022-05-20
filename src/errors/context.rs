use std::backtrace::Backtrace;
use std::convert::Infallible;
use std::fmt::Display;

use super::GenericError;

pub trait Context<T, E> {
    fn context<C>(self, context: C) -> Result<T, GenericError>
    where
        C: Display + Send + Sync + 'static;
}

impl<T, E> Context<T, E> for Result<T, E>
where
    E: Into<GenericError> + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T, GenericError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|error| GenericError::Context {
            source: Box::new(error.into()),
            context: context.to_string(),
        })
    }
}

impl<T> Context<T, Infallible> for Option<T> {
    fn context<C>(self, context: C) -> Result<T, GenericError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| GenericError::Context {
            source: Box::new(GenericError::Basic {
                message: format!("{}", context),
                backtrace: Backtrace::capture(),
            }),
            context: context.to_string(),
        })
    }
}
