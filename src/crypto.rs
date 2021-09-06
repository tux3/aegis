use anyhow::{bail, Result};
use signature::Signature;
use sodiumoxide::crypto::sign;
use sodiumoxide::randombytes::randombytes;
use std::path::Path;

// Random buffer prepended to the signature. Doesn't actually prevent any kind of replay!
// Mostly this serves to inject random *somewhere* (hey, we get websocket message IDs for free!)
const SIGNATURE_RANDOM_BUF_LEN: usize = 16;
const SIGNATURE_FULL_LEN: usize = SIGNATURE_RANDOM_BUF_LEN + sign::SIGNATUREBYTES;

pub fn randomized_signature(private_key: &sign::SecretKey, payload: &[u8]) -> Vec<u8> {
    let random_buf = randombytes(SIGNATURE_RANDOM_BUF_LEN);
    let mut signer = sign::State::init();
    signer.update(&random_buf);
    signer.update(payload);
    let mut result = random_buf;
    result.extend_from_slice(signer.finalize(private_key).as_bytes());
    debug_assert_eq!(result.len(), SIGNATURE_FULL_LEN);
    result
}

pub fn check_signature(
    public_key: &sign::PublicKey,
    randomized_signature: &[u8],
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
