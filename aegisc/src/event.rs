pub enum ClientEvent {
    WebcamPicture(Vec<u8>),
    InputWhileLockedWithoutWebcam,
}
