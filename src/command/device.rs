use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct StatusArg {}

#[derive(Serialize, Deserialize)]
pub struct StatusReply {
    pub vt_locked: bool,
    pub ssh_locked: bool,
}
