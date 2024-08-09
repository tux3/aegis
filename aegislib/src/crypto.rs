use anyhow::{bail, Result};
use ed25519_dalek::Digest;
use serde::{Deserialize, Serialize};
use signature::Signature;
use std::path::Path;

pub use ed25519_dalek::SigningKey;

// Random buffer prepended to the signature. Doesn't actually prevent any kind of replay!
// Mostly this serves to inject random *somewhere* (hey, we get websocket message IDs for free!)
const SIGNATURE_RANDOM_BUF_LEN: usize = 16;
const SIGNATURE_FULL_LEN: usize = SIGNATURE_RANDOM_BUF_LEN + ed25519_dalek::SIGNATURE_LENGTH;

pub fn randomized_signature(
    keypair: &ed25519_dalek::SigningKey,
    route: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let mut random_buf = [0u8; SIGNATURE_RANDOM_BUF_LEN];
    getrandom::getrandom(&mut random_buf).expect("Failed to get random");
    let mut hasher = ed25519_dalek::Sha512::new();
    hasher.update(random_buf);
    hasher.update(route);
    hasher.update(payload);
    let signature = keypair.sign_prehashed(hasher, None).unwrap();
    let mut result = random_buf.to_vec();
    result.extend_from_slice(&signature.to_bytes());

    debug_assert_eq!(result.len(), SIGNATURE_FULL_LEN);
    result
}

pub fn check_signature(
    public_key: &ed25519_dalek::VerifyingKey,
    randomized_signature: &[u8],
    route: &[u8],
    payload: &[u8],
) -> bool {
    if randomized_signature.len() != SIGNATURE_FULL_LEN {
        return false;
    }
    let random_buf = &randomized_signature[..SIGNATURE_RANDOM_BUF_LEN];
    let signature = match randomized_signature[SIGNATURE_RANDOM_BUF_LEN..].try_into() {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    let signature = ed25519_dalek::Signature::from_bytes(signature);

    let mut hasher = ed25519_dalek::Sha512::new();
    hasher.update(random_buf);
    hasher.update(route);
    hasher.update(payload);
    public_key
        .verify_prehashed(hasher, None, &signature)
        .is_ok()
}

pub fn random_sign_keypair() -> ed25519_dalek::SigningKey {
    let sk = &mut [0u8; ed25519_dalek::SECRET_KEY_LENGTH];
    getrandom::getrandom(sk).unwrap();
    SigningKey::from_bytes(sk)
}

pub fn sign_keypair_from_file(path: impl AsRef<Path>) -> Result<ed25519_dalek::SigningKey> {
    let key = &std::fs::read(path.as_ref())?.try_into();
    match key {
        Err(_) => bail!(
            "Invalid private signature key file: {}",
            path.as_ref().display()
        ),
        Ok(k) => Ok(ed25519_dalek::SigningKey::from_bytes(k)),
    }
}

#[derive(Serialize, Deserialize)]
pub struct RootKeys {
    pub sig: ed25519_dalek::SigningKey,
    pub enc: chacha20poly1305::Key,
}

impl RootKeys {
    #[cfg(feature = "ffi")]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        unreachable!() // UniFFI wants a default constructor...
    }

    #[cfg(feature = "ffi")]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::ffi::FfiError> {
        Ok(bincode::deserialize(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize: {}", e))?)
    }

    #[cfg(feature = "ffi")]
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn derive(password: &str) -> RootKeys {
        const SECRET_SIZE: usize =
            ed25519_dalek::SECRET_KEY_LENGTH + std::mem::size_of::<chacha20poly1305::Key>();
        let secret_buf = &mut [0u8; SECRET_SIZE];
        let salt = b"expand password into 32-byte key"; // Not much up my sleeve, promise!

        // Why does it take so much code to set the output size !? :(
        let mut params = argon2::ParamsBuilder::default();
        params.output_len(SECRET_SIZE).unwrap(); // NOTE: This is not a builder!
        let params = params.params().unwrap();
        let argon2 = argon2::Argon2::new(Default::default(), Default::default(), params);
        argon2
            .hash_password_into(password.as_bytes(), salt, secret_buf)
            .unwrap();

        let sig_skey = &secret_buf[..ed25519_dalek::SECRET_KEY_LENGTH]
            .try_into()
            .unwrap();
        let sig = ed25519_dalek::SigningKey::from_bytes(sig_skey);
        let enc =
            chacha20poly1305::Key::from_slice(&secret_buf[ed25519_dalek::SECRET_KEY_LENGTH..])
                .to_owned();

        RootKeys { sig, enc }
    }

    #[cfg(feature = "ffi")]
    pub fn matches_serializes_pubkey(&self, pubkey: &str) -> bool {
        use base64::prelude::*;

        let our_pubkey = self.sig.public;
        let our_pubkey = BASE64_URL_SAFE_NO_PAD.encode(our_pubkey.as_ref());
        our_pubkey == pubkey
    }
}
