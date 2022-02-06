use crate::command::device::StatusReply;
use derive_more::From;
use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusUpdate {
    pub vt_locked: bool,
    pub ssh_locked: bool,
    pub draw_decoy: bool,
}

impl From<StatusReply> for StatusUpdate {
    fn from(reply: StatusReply) -> Self {
        Self {
            vt_locked: reply.vt_locked,
            ssh_locked: reply.ssh_locked,
            draw_decoy: reply.draw_decoy,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum PowerCommand {
    Reboot,
    Poweroff,
}

#[derive(Serialize, Deserialize, Debug, From, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ServerCommand {
    StatusUpdate(StatusUpdate),
    PowerCommand(PowerCommand),
}
