use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use anyhow::anyhow;

use super::GenericError;

pub struct GenericAnyhowedError(pub(crate) anyhow::Error, pub(crate) Option<String>);

impl GenericAnyhowedError {
    pub fn new(err: GenericError) -> Self {
        match err {
            GenericError::Context { context, source } => {
                GenericAnyhowedError(anyhow!(*source).context(context.clone()), Some(context))
            },
            other => GenericAnyhowedError(anyhow::Error::new(other), None),
        }
    }

    pub fn unwind(self) -> GenericError {
        let err: GenericError = self.0.downcast().expect("Shouldn't be created manually");

        match self.1 {
            None => err,
            Some(context) => GenericError::Context {
                context,
                source: Box::new(err),
            },
        }
    }
}

impl Debug for GenericAnyhowedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for GenericAnyhowedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for GenericAnyhowedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Error::source(&*self.0)
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        Error::backtrace(&*self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Context;

    #[test]
    fn unwind() {
        let err = GenericError::StrumParseError(strum::ParseError::VariantNotFound);

        assert!(matches!(
            err,
            GenericError::StrumParseError(strum::ParseError::VariantNotFound),
        ));

        let err = GenericAnyhowedError::new(err).unwind();

        assert!(matches!(
            err,
            GenericError::StrumParseError(strum::ParseError::VariantNotFound),
        ));
    }

    #[test]
    fn unwind_context() {
        let err = GenericError::StrumParseError(strum::ParseError::VariantNotFound);
        let res: Result<(), _> = Err(err);

        let err = res.context("add context").unwrap_err();

        assert!(matches!(
            err,
            GenericError::Context {
                context: _,
                source: box GenericError::StrumParseError(strum::ParseError::VariantNotFound),
            },
        ));

        let err = GenericAnyhowedError::new(err).unwind();

        assert!(matches!(
            err,
            GenericError::Context {
                context: _,
                source: box GenericError::StrumParseError(strum::ParseError::VariantNotFound),
            },
        ));
    }
}
