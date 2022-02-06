use crate::command::server::PowerCommand;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisteredDevice {
    pub id: i32,
    pub created_at: SystemTime,
    pub name: String,
    pub pubkey: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PendingDevice {
    pub created_at: SystemTime,
    pub name: String,
    pub pubkey: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetStatusArg {
    pub dev_name: String,
    pub vt_locked: Option<bool>,
    pub ssh_locked: Option<bool>,
    pub draw_decoy: Option<bool>,
    // NOTE: update is_no_op if you add a field
}

impl SetStatusArg {
    pub fn is_no_op(&self) -> bool {
        // Destructure to cause build error if we add a field
        self.vt_locked.is_none() && self.ssh_locked.is_none() && self.draw_decoy.is_none()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCameraPicturesArg {
    pub dev_id: i32,
    pub pictures: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StoredCameraPicture {
    pub created_at_timestamp: u64,
    pub jpeg_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendPowerCommandArg {
    pub dev_name: String,
    pub command: PowerCommand,
}
