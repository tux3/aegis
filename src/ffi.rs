mod admin_client_ffi;
use admin_client_ffi::AdminClientFfi;
mod error;
pub(crate) use error::FfiError;

use crate::client::*;
use crate::command::admin::*;
use crate::crypto::RootKeys;

uniffi_macros::include_scaffolding!("client");
