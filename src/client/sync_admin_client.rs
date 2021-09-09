use crate::client::{AdminClient, ClientConfig};
use crate::command::admin::{PendingDevice, RegisteredDevice};
use crate::crypto::RootKeys;
use anyhow::Result;
use futures::executor::block_on;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct SyncAdminClient {
    client: AdminClient,
}

impl SyncAdminClient {
    pub fn new() -> Self {
        unreachable!() // UniFFI wants a default constructor...
    }

    pub fn ffi_new(config: &ClientConfig, keys: &RootKeys) -> Self {
        block_on(async {
            Self {
                client: AdminClient::new(config, keys).await.unwrap(),
            }
        })
    }

    fn do_request<R: DeserializeOwned>(&mut self, route: &str, arg: impl Serialize) -> Result<R> {
        block_on(async { self.client.do_request(route, arg).await })
    }

    pub fn list_pending(&mut self) -> Result<Vec<PendingDevice>> {
        self.do_request("list_pending_devices", ())
    }

    pub fn delete_pending(&mut self, name: String) -> Result<()> {
        self.do_request("delete_pending_device", name)
    }

    pub fn confirm_pending(&mut self, name: String) -> Result<()> {
        self.do_request("confirm_pending_device", name)
    }

    pub fn list_registered(&mut self) -> Result<Vec<RegisteredDevice>> {
        self.do_request("list_registered_devices", ())
    }

    pub fn delete_registered(&mut self, name: String) -> Result<()> {
        self.do_request("delete_registered_device", name)
    }
}

#[cfg(test)]
mod test {
    use super::SyncAdminClient;
    use std::marker::PhantomData;

    #[test]
    fn sync_admin_client_is_send_sync() {
        struct Test<T: Send + Sync>(PhantomData<T>);
        let _ = Test::<SyncAdminClient>(Default::default());
    }
}
