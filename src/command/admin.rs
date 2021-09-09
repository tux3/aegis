use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisteredDevice {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub pubkey: ed25519_dalek::PublicKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PendingDevice {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub pubkey: ed25519_dalek::PublicKey,
}
