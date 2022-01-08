use derive_more::From;
use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusUpdate {
    pub vt_locked: bool,
    pub ssh_locked: bool,
}

#[derive(Serialize, Deserialize, Debug, From, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ServerCommand {
    StatusUpdate(StatusUpdate),
}
