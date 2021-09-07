///! Defines an newtype wrapper around anyhow::Error
///! Required because the orphan rule prevent us from having From<sqlx::Error> for actix_web::Error
use actix_web::ResponseError;
use anyhow::anyhow;
use derive_more::Display;
use std::fmt::Debug;

/// Replaces anyhow::bail! in functions that must return a crate::error::Error
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err(crate::error::Error(anyhow::anyhow!($msg)))
    };
    ($err:expr $(,)?) => ({
        return Err(crate::error::Error(anyhow::anyhow!($err)))
    });
    ($fmt:expr, $($arg:tt)*) => {
        return Err(crate::error::Error(anyhow::anyhow!($fmt, $($arg)*)))
    };
}
pub(crate) use bail;

#[derive(Debug, Display)]
#[display(fmt = "{}", _0)]
pub struct Error(pub anyhow::Error);

impl ResponseError for Error {}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self(e)
    }
}

impl From<actix_web::Error> for Error {
    fn from(e: actix_web::Error) -> Self {
        Self(anyhow!("{}", e))
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self(anyhow!("Database error: {}", e))
    }
}
