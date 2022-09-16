use anyhow::{bail, Result};
use ed25519_dalek::Digest;
use serde::{Deserialize, Serialize};
use signature::Signature;
use std::path::Path;

pub use ed25519_dalek::Keypair;

// Random buffer prepended to the signature. Doesn't actually prevent any kind of replay!
// Mostly this serves to inject random *somewhere* (hey, we get websocket message IDs for free!)
const SIGNATURE_RANDOM_BUF_LEN: usize = 16;
const SIGNATURE_FULL_LEN: usize = SIGNATURE_RANDOM_BUF_LEN + ed25519_dalek::SIGNATURE_LENGTH;

pub fn randomized_signature(
    keypair: &ed25519_dalek::Keypair,
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
    result.extend_from_slice(signature.as_bytes());

    debug_assert_eq!(result.len(), SIGNATURE_FULL_LEN);
    result
}

pub fn check_signature(
    public_key: &ed25519_dalek::PublicKey,
    randomized_signature: &[u8],
    route: &[u8],
    payload: &[u8],
) -> bool {
    if randomized_signature.len() != SIGNATURE_FULL_LEN {
        return false;
    }
    let random_buf = &randomized_signature[..SIGNATURE_RANDOM_BUF_LEN];
    let signature = &randomized_signature[SIGNATURE_RANDOM_BUF_LEN..];
    let signature = match ed25519_dalek::Signature::from_bytes(signature) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    let mut hasher = ed25519_dalek::Sha512::new();
    hasher.update(random_buf);
    hasher.update(route);
    hasher.update(payload);
    public_key
        .verify_prehashed(hasher, None, &signature)
        .is_ok()
}

pub fn random_sign_keypair() -> ed25519_dalek::Keypair {
    let sk = &mut [0u8; ed25519_dalek::SECRET_KEY_LENGTH];
    getrandom::getrandom(sk).unwrap();
    let sk = ed25519_dalek::SecretKey::from_bytes(sk).unwrap();
    ed25519_dalek::Keypair {
        public: ed25519_dalek::PublicKey::from(&sk),
        secret: sk,
    }
}

pub fn sign_keypair_from_file(path: impl AsRef<Path>) -> Result<ed25519_dalek::Keypair> {
    let key = std::fs::read(path.as_ref())?;
    match ed25519_dalek::Keypair::from_bytes(&key) {
        Err(_) => bail!(
            "Invalid private signature key file: {}",
            path.as_ref().display()
        ),
        Ok(k) => Ok(k),
    }
}

#[derive(Serialize, Deserialize)]
pub struct RootKeys {
    pub sig: ed25519_dalek::Keypair,
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

        let sig_skey =
            ed25519_dalek::SecretKey::from_bytes(&secret_buf[..ed25519_dalek::SECRET_KEY_LENGTH])
                .unwrap();
        let sig = ed25519_dalek::Keypair {
            public: ed25519_dalek::PublicKey::from(&sig_skey),
            secret: sig_skey,
        };
        let enc =
            chacha20poly1305::Key::from_slice(&secret_buf[ed25519_dalek::SECRET_KEY_LENGTH..])
                .to_owned();

        RootKeys { sig, enc }
    }

    #[cfg(feature = "ffi")]
    pub fn matches_serializes_pubkey(&self, pubkey: &str) -> bool {
        let our_pubkey = self.sig.public;
        let our_pubkey = base64::encode_config(our_pubkey.as_ref(), base64::URL_SAFE_NO_PAD);
        our_pubkey == pubkey
    }
}
