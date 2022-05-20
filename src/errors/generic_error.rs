use std::backtrace::Backtrace;
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use super::GenericAnyhowedError;

pub type GenericResult<T, E = GenericError> = Result<T, E>;

/// All errors from everywhere!
#[derive(thiserror::Error)]
pub enum GenericError {
    // RSpotify
    #[error(transparent)]
    RspotifyClientError(
        #[from]
        #[backtrace]
        rspotify::ClientError,
    ),

    #[error(transparent)]
    RspotifyIdError(
        #[from]
        #[backtrace]
        rspotify::model::IdError,
    ),

    // SeaORM + DB
    #[error(transparent)]
    SeaORM(
        #[from]
        #[backtrace]
        sea_orm::DbErr,
    ),

    #[error(transparent)]
    SqlxError(
        #[from]
        #[backtrace]
        sqlx::Error,
    ),

    #[error(transparent)]
    SqlxMigrateError(
        #[from]
        #[backtrace]
        sqlx::migrate::MigrateError,
    ),
    // Teloxide
    #[error(transparent)]
    TeloxideRequestError(
        #[from]
        #[backtrace]
        teloxide::RequestError,
    ),

    #[error(transparent)]
    TeloxideCommandParseError(
        #[from]
        #[backtrace]
        teloxide::utils::command::ParseError,
    ),

    // Reqwest + Url
    #[error(transparent)]
    Reqwest(
        #[from]
        #[backtrace]
        reqwest::Error,
    ),

    #[error(transparent)]
    ReqwestInvalidHeaderValue(
        #[from]
        #[backtrace]
        reqwest::header::InvalidHeaderValue,
    ),

    #[error(transparent)]
    ReqwestHeaderToStrError(
        #[from]
        #[backtrace]
        reqwest::header::ToStrError,
    ),

    #[error(transparent)]
    UrlParseError(
        #[from]
        #[backtrace]
        url::ParseError,
    ),

    // Influx
    #[error(transparent)]
    InfluxdbError(
        #[from]
        #[backtrace]
        influxdb::Error,
    ),

    //Tracing
    #[error(transparent)]
    TracingLokiError(
        #[from]
        #[backtrace]
        tracing_loki::Error,
    ),

    #[error(transparent)]
    TracingParseLevelError(
        #[from]
        #[backtrace]
        tracing::metadata::ParseLevelError,
    ),

    #[error(transparent)]
    TracingTryInitError(
        #[from]
        #[backtrace]
        tracing_subscriber::util::TryInitError,
    ),

    /////////////////////
    #[error(transparent)]
    Serde(
        #[from]
        #[backtrace]
        serde_json::Error,
    ),

    #[error(transparent)]
    StrumParseError(
        #[from]
        #[backtrace]
        strum::ParseError,
    ),

    #[error(transparent)]
    DotenvError(
        #[from]
        #[backtrace]
        dotenv::Error,
    ),

    //////////////////////
    #[error(transparent)]
    AcquireError(
        #[from]
        #[backtrace]
        tokio::sync::AcquireError,
    ),
    #[error(transparent)]
    FromUtf8Error(
        #[from]
        #[backtrace]
        FromUtf8Error,
    ),

    #[error(transparent)]
    ParseIntError(
        #[from]
        #[backtrace]
        ParseIntError,
    ),

    #[error(transparent)]
    Anyhow(
        #[from]
        #[backtrace]
        anyhow::Error,
    ),

    #[error("{message}\n\n{backtrace:?}")]
    Basic {
        message: String,
        backtrace: Backtrace,
    },

    #[error("{context}\n\n{source:?}")]
    Context {
        context: String,
        #[backtrace]
        source: Box<Self>,
    },
}

impl Debug for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let ptr = self as *const Self;
            let result = core::ptr::read(ptr);

            let any = GenericAnyhowedError::new(result);

            let res = Debug::fmt(&any, f);

            let ptr = ptr as *mut Self;

            core::ptr::write(ptr, any.unwind());

            res
        }
    }
}

impl From<GenericAnyhowedError> for GenericError {
    fn from(err: GenericAnyhowedError) -> Self {
        err.unwind()
    }
}

impl GenericError {
    /// Returns error and context as separate variables
    pub fn unwind(self) -> (Self, String) {
        let mut err = self;
        let mut contexts = vec![];

        loop {
            err = match err {
                Self::Context { source, context } => {
                    contexts.push(context);
                    *source
                },
                err => return (err, contexts.join("\n")),
            }
        }
    }

    pub fn context<C>(self, context: C) -> Self
    where
        C: Display + Send + Sync + 'static,
    {
        Self::Context {
            source: Box::new(self),
            context: context.to_string(),
        }
    }
}
