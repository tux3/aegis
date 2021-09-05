use signature::Signature;
use sodiumoxide::crypto::sign;

const SIGNATURE_RANDOM_BUF_LEN: usize = 16; // Random buffer prepended to the signature
const SIGNATURE_FULL_LEN: usize = SIGNATURE_RANDOM_BUF_LEN + sign::SIGNATUREBYTES;

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
