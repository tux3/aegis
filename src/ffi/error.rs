use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum FfiError {
    Error(anyhow::Error),
}
impl std::error::Error for FfiError {}
impl Display for FfiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Error(e) => write!(f, "{}", e),
        }
    }
}
impl From<anyhow::Error> for FfiError {
    fn from(e: anyhow::Error) -> Self {
        Self::Error(e)
    }
}
