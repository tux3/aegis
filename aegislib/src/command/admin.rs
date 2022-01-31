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
