use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use signature::Signature;
use sodiumoxide::crypto::{aead, generichash, pwhash, sign};
use sodiumoxide::randombytes::randombytes;
use std::path::Path;

// Random buffer prepended to the signature. Doesn't actually prevent any kind of replay!
// Mostly this serves to inject random *somewhere* (hey, we get websocket message IDs for free!)
const SIGNATURE_RANDOM_BUF_LEN: usize = 16;
const SIGNATURE_FULL_LEN: usize = SIGNATURE_RANDOM_BUF_LEN + sign::SIGNATUREBYTES;

pub fn randomized_signature(
    private_key: &sign::SecretKey,
    route: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let random_buf = randombytes(SIGNATURE_RANDOM_BUF_LEN);
    let mut signer = sign::State::init();
    signer.update(&random_buf);
    signer.update(route);
    signer.update(payload);
    let mut result = random_buf;
    result.extend_from_slice(signer.finalize(private_key).as_bytes());

    debug_assert_eq!(result.len(), SIGNATURE_FULL_LEN);
    result
}

pub fn check_signature(
    public_key: &sign::PublicKey,
    randomized_signature: &[u8],
    route: &[u8],
    payload: &[u8],
) -> bool {
    if randomized_signature.len() != SIGNATURE_FULL_LEN {
        return false;
    }
    let random_buf = &randomized_signature[..SIGNATURE_RANDOM_BUF_LEN];
    let signature = &randomized_signature[SIGNATURE_RANDOM_BUF_LEN..];
    let signature = match sign::Signature::from_bytes(signature) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    let mut verifier = sign::State::init();
    verifier.update(random_buf);
    verifier.update(route);
    verifier.update(payload);
    verifier.verify(&signature, public_key)
}

pub fn priv_sign_key_from_file(path: impl AsRef<Path>) -> Result<sign::SecretKey> {
    let key = std::fs::read(path.as_ref())?;
    match sign::SecretKey::from_slice(&key) {
        None => bail!(
            "Invalid private signature key file: {}",
            path.as_ref().display()
        ),
        Some(k) => Ok(k),
    }
}

#[derive(Serialize, Deserialize)]
pub struct RootKeys {
    pub sig: sign::SecretKey,
    pub enc: aead::Key,
}

impl RootKeys {
    #[cfg(feature = "ffi")]
    pub fn new() -> Self {
        unreachable!() // UniFFI wants a default constructor...
    }

    pub fn derive(password: &str) -> RootKeys {
        let secret_buf = &mut [0u8; 64];
        let salt = b"expand password into 64-byte key"; // Not much up my sleeve, promise!
        let salt = pwhash::Salt::from_slice(salt.as_ref()).unwrap();
        pwhash::derive_key_interactive(secret_buf, password.as_bytes(), &salt)
            .expect("failed to derive root keys (unknown libsodium error)");

        let sig_seed = generichash::hash(
            secret_buf,
            Some(sign::SEEDBYTES),
            Some(b"root signature key seed".as_ref()),
        )
        .expect("failed to derive root sig key seed (unknown libsodium error)");
        let sig_seed = sign::Seed::from_slice(sig_seed.as_ref()).unwrap();
        let sig = sign::keypair_from_seed(&sig_seed).1;

        let enc = generichash::hash(
            secret_buf,
            Some(aead::KEYBYTES),
            Some(b"root encryption key".as_ref()),
        )
        .expect("failed to derive root enc key (unknown libsodium error)");
        let enc = aead::Key::from_slice(enc.as_ref()).unwrap();

        RootKeys { sig, enc }
    }

    #[cfg(feature = "ffi")]
    pub fn matches_serializes_pubkey(&self, pubkey: &str) -> bool {
        use sodiumoxide::base64;
        let our_pubkey = self.sig.public_key();
        let our_pubkey = base64::encode(our_pubkey.as_ref(), base64::Variant::UrlSafeNoPadding);
        our_pubkey == pubkey
    }
}
