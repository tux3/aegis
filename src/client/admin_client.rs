use crate::client::{ApiClient, ClientConfig, RestClient};
use crate::command::admin::{PendingDevice, RegisteredDevice};
use crate::crypto::{randomized_signature, RootKeys};
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct AdminClient {
    client: RestClient,
    keys: RootKeys,
}

impl AdminClient {
    pub async fn new(config: &ClientConfig, keys: RootKeys) -> Result<Self> {
        let client = RestClient::new_client(config).await?;
        Ok(AdminClient { client, keys })
    }

    async fn do_request<R: DeserializeOwned>(
        &mut self,
        route: &str,
        arg: impl Serialize,
    ) -> Result<R> {
        let route = format!("/admin/{}", route);
        let payload = bincode::serialize(&arg)?;
        let signature = randomized_signature(&self.keys.sig, route.as_bytes(), &payload);
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
}
