use super::FfiError;
use crate::client::{AdminClient, ClientConfig};
use crate::command::admin::{
    PendingDevice, RegisteredDevice, SendPowerCommandArg, SetStatusArg, StoredCameraPicture,
};
use crate::command::device::{DeviceEvent, StatusReply};
use crate::command::server::PowerCommand;
use crate::crypto::RootKeys;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Mutex;
use tokio::runtime::Runtime;

pub struct AdminClientFfi {
    rt: Runtime,
    client: Mutex<AdminClient>,
}

impl AdminClientFfi {
    pub fn new(config: &ClientConfig, keys: &RootKeys) -> Result<Self, FfiError> {
        let rt = Runtime::new().unwrap();
        let client = rt.block_on(AdminClient::new(config, keys))?.into();
        Ok(Self { rt, client })
    }

    fn do_request<R: DeserializeOwned>(
        &self,
        route: &str,
        arg: impl Serialize,
    ) -> Result<R, FfiError> {
        let mut client = self.client.lock().expect("Poisoned lock");
        self.rt
            .block_on(async { client.do_request(route, arg).await })
            .map_err(FfiError::Error)
    }

    pub fn list_pending(&self) -> Result<Vec<PendingDevice>, FfiError> {
        self.do_request("list_pending_devices", ())
    }

    pub fn delete_pending(&self, name: String) -> Result<(), FfiError> {
        self.do_request("delete_pending_device", name)
    }

    pub fn confirm_pending(&self, name: String) -> Result<(), FfiError> {
        self.do_request("confirm_pending_device", name)
    }

    pub fn list_registered(&self) -> Result<Vec<RegisteredDevice>, FfiError> {
        self.do_request("list_registered_devices", ())
    }

    pub fn delete_registered(&self, name: String) -> Result<(), FfiError> {
        self.do_request("delete_registered_device", name)
    }

    pub fn set_status(&self, arg: SetStatusArg) -> Result<StatusReply, FfiError> {
        self.do_request("set_status", arg)
    }

    pub fn delete_device_camera_pictures(&self, dev_name: String) -> Result<(), FfiError> {
        self.do_request("delete_device_camera_pictures", dev_name)
    }

    pub fn get_device_camera_pictures(
        &self,
        dev_name: String,
    ) -> Result<Vec<StoredCameraPicture>, FfiError> {
        self.do_request("get_device_camera_pictures", dev_name)
    }

    pub fn send_power_command(&self, dev_name: String, cmd: PowerCommand) -> Result<(), FfiError> {
        self.do_request(
            "send_power_command",
            SendPowerCommandArg {
                dev_name,
                command: cmd,
            },
        )
    }

    pub fn delete_device_events(&self, dev_name: String) -> Result<(), FfiError> {
        self.do_request("delete_device_events", dev_name)
    }

    pub fn get_device_events(&self, dev_name: String) -> Result<Vec<DeviceEvent>, FfiError> {
        self.do_request("get_device_events", dev_name)
    }
}

#[cfg(test)]
mod test {
    use super::AdminClientFfi;
    use std::marker::PhantomData;

    #[test]
    fn sync_admin_client_is_send_sync() {
        struct Test<T: Send + Sync>(PhantomData<T>);
        let _ = Test::<AdminClientFfi>(Default::default());
    }
}
