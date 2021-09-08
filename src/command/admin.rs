use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::sign;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisteredDevice {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub pubkey: sign::PublicKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PendingDevice {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub pubkey: sign::PublicKey,
}
