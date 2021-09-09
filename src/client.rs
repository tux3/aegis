#[cfg(feature = "ffi")]
use crate::crypto::RootKeys;

#[cfg(feature = "ffi")]
uniffi_macros::include_scaffolding!("client");

#[cfg(feature = "ffi")]
mod sync_admin_client;
#[cfg(feature = "ffi")]
use sync_admin_client::SyncAdminClient;

mod api_client;
pub use api_client::*;

mod dev_client;
pub use dev_client::*;

mod admin_client;
pub use admin_client::*;

mod rest_client;
pub use rest_client::*;

mod ws_client;
pub use ws_client::*;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_addr: String,
    pub use_tls: bool,
    pub use_rest: bool,
}
