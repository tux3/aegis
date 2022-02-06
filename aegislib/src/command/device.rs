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

#[derive(Serialize, Deserialize, Debug)]
pub struct StoreCameraPictureArg {
    pub jpeg_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StoreCameraPictureReply {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceEvent {
    pub timestamp: u64,
    pub level: EventLogLevel,
    pub message: String,
}
