use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusArg {}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StatusReply {
    pub updated_at_timestamp: u64,
    pub is_connected: bool,
    pub vt_locked: bool,
    pub ssh_locked: bool,
    pub draw_decoy: bool,
}
