use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusArg {}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusReply {
    pub vt_locked: bool,
    pub ssh_locked: bool,
}
