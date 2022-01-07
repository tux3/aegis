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
}