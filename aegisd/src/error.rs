///! Defines an newtype wrapper around anyhow::Error
///! Required because the orphan rule prevent us from having From<sqlx::Error> for actix_web::Error
use anyhow::anyhow;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

/// Replaces anyhow::bail! in functions that must return a crate::error::Error
#[allow(unused_macro_rules)]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err(crate::error::Error::Anyhow(anyhow::anyhow!($msg)))
    };
    ($err:expr $(,)?) => ({
        return Err(crate::error::Error::Anyhow(anyhow::anyhow!($err)))
    });
    (StatusCode::$code:ident, $msg:literal $(,)?) => ({
        return Err(crate::error::Error::Response(StatusCode::$code, $msg.to_owned()))
    });
    (StatusCode::$code:ident, $err:expr $(,)?) => ({
        return Err(crate::error::Error::Response(StatusCode::$code, String::from($err)))
    });
    ($fmt:expr, $($arg:tt)*) => {
        return Err(crate::error::Error::Anyhow(anyhow::anyhow!($fmt, $($arg)*)))
    };
}
pub(crate) use bail;

#[derive(Debug)]
pub enum Error {
    Anyhow(anyhow::Error),
    Axum(axum::Error),
    Response(StatusCode, String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let code = match self {
            Self::Axum(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Response(code, body) => return (code, body).into_response(),
        };
        (code, self.to_string()).into_response()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anyhow(e) => write!(f, "{e}"),
            Self::Response(_code, body) => write!(f, "{body}"),
            Self::Axum(e) => write!(f, "{e}"),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}

impl From<(StatusCode, String)> for Error {
    fn from((code, body): (StatusCode, String)) -> Self {
        Self::Response(code, body)
    }
}

impl From<axum::Error> for Error {
    fn from(e: axum::Error) -> Self {
        Self::Axum(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Anyhow(anyhow!("Database error: {}", e))
    }
}

impl From<Error> for Infallible {
    fn from(_: Error) -> Infallible {
        unreachable!()
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
