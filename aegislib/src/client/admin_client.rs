use crate::client::{ApiClient, ClientConfig, RestClient};
use crate::command::admin::{
    PendingDevice, RegisteredDevice, SendPowerCommandArg, SetStatusArg, StoredCameraPicture,
};
use crate::command::device::StatusReply;
use crate::command::server::PowerCommand;
use crate::crypto::{randomized_signature, RootKeys};
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct AdminClient {
    client: RestClient,
    key: ed25519_dalek::Keypair,
}

impl AdminClient {
    pub async fn new(config: &ClientConfig, keys: &RootKeys) -> Result<Self> {
        let client = RestClient::new_client(config).await;
        Ok(AdminClient {
            client,
            // No Clone, because let's frustrate people until they decide to use libsodium instead :(
            // Yes, we make a copy of a key. Hope no one dumps my ram before both copies get zeroed...
            key: ed25519_dalek::Keypair::from_bytes(&keys.sig.to_bytes()).unwrap(),
        })
    }

    pub(crate) async fn do_request<R: DeserializeOwned>(
        &mut self,
        route: &str,
        arg: impl Serialize,
    ) -> Result<R> {
        let route = format!("/admin/{}", route);
        let payload = bincode::serialize(&arg)?;
        let signature = randomized_signature(&self.key, route.as_bytes(), &payload);
        let reply = self.client.request(&route, &signature, payload).await?;
        Ok(bincode::deserialize(&reply)?)
    }

    pub async fn list_pending(&mut self) -> Result<Vec<PendingDevice>> {
        self.do_request("list_pending_devices", ()).await
    }

    pub async fn delete_pending(&mut self, name: String) -> Result<()> {
        self.do_request("delete_pending_device", name).await
    }

    pub async fn confirm_pending(&mut self, name: String) -> Result<()> {
        self.do_request("confirm_pending_device", name).await
    }

    pub async fn list_registered(&mut self) -> Result<Vec<RegisteredDevice>> {
        self.do_request("list_registered_devices", ()).await
    }

    pub async fn delete_registered(&mut self, name: String) -> Result<()> {
        self.do_request("delete_registered_device", name).await
    }

    pub async fn set_status(&mut self, arg: SetStatusArg) -> Result<StatusReply> {
        self.do_request("set_status", arg).await
    }

    pub async fn delete_device_camera_pictures(&mut self, dev_name: String) -> Result<()> {
        self.do_request("delete_device_camera_pictures", dev_name)
            .await
    }

    pub async fn get_device_camera_pictures(
        &mut self,
        dev_name: String,
    ) -> Result<Vec<StoredCameraPicture>> {
        self.do_request("get_device_camera_pictures", dev_name)
            .await
    }

    pub async fn send_power_command(&mut self, dev_name: String, cmd: PowerCommand) -> Result<()> {
        self.do_request(
            "send_power_command",
            SendPowerCommandArg {
                dev_name,
                command: cmd,
            },
        )
        .await
    }
}

#[cfg(test)]
mod test {
    use crate::client::AdminClient;
    use std::marker::PhantomData;

    #[test]
    fn admin_client_is_send_sync() {
        struct Test<T: Send + Sync>(PhantomData<T>);
        let _ = Test::<AdminClient>(Default::default());
    }
}
