use std::backtrace::Backtrace;
use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use thiserror::Error;
use tokio::sync::AcquireError;

pub type GenericResult<T, E = GenericError> = Result<T, E>;

/// All errors from everywhere!
#[derive(Error, Debug)]
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
        AcquireError,
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

impl GenericError {
    /// Returns error without context
    #[allow(dead_code)] // TODO Remove
    pub fn unwrap(self) -> Self {
        match self {
            Self::Context { source, .. } => *source,
            _ => self,
        }
    }
}

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
