use actix_web::http::StatusCode;
///! Defines an newtype wrapper around anyhow::Error
///! Required because the orphan rule prevent us from having From<sqlx::Error> for actix_web::Error
use actix_web::ResponseError;
use anyhow::anyhow;
use std::fmt::{Debug, Display, Formatter};

/// Replaces anyhow::bail! in functions that must return a crate::error::Error
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err(crate::error::Error::Anyhow(anyhow::anyhow!($msg)))
    };
    ($err:expr $(,)?) => ({
        return Err(crate::error::Error::Anyhow(anyhow::anyhow!($err)))
    });
    ($fmt:expr, $($arg:tt)*) => {
        return Err(crate::error::Error::Anyhow(anyhow::anyhow!($fmt, $($arg)*)))
    };
}
pub(crate) use bail;

#[derive(Debug)]
pub enum Error {
    Anyhow(anyhow::Error),
    Actix(actix_web::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Actix(e) => e.as_response_error().status_code(),
            Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anyhow(e) => write!(f, "{}", e),
            Self::Actix(e) => write!(f, "{}", e),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}

impl From<actix_web::Error> for Error {
    fn from(e: actix_web::Error) -> Self {
        Self::Actix(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Anyhow(anyhow!("Database error: {}", e))
    }
}
