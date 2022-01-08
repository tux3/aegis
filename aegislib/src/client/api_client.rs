use async_trait::async_trait;
use bytes::Bytes;
use crate::client::ClientError;

#[async_trait]
pub trait ApiClient {
    async fn request(&mut self, handler: &str, signature: &[u8], payload: Vec<u8>)
        -> Result<Bytes, ClientError>;
}
