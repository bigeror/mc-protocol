use std::sync::LazyLock;

use rsa::{RsaPrivateKey, RsaPublicKey, pkcs8::EncodePublicKey, rand_core::OsRng };

pub static ENCRYPT_KEY_PAIR: LazyLock<KeyPair> = LazyLock::new(|| {
    let private_key = RsaPrivateKey::new(&mut OsRng::default(), 1024)
        .expect("Failed to generate private key");
    let public_key = private_key.to_public_key();

    let public_key_der = public_key.to_public_key_der()
        .expect("Failed to encode public key to DER").into_vec();

    KeyPair { public_key, private_key, public_key_der }
});

pub struct KeyPair {
    pub public_key: RsaPublicKey,
    pub public_key_der: Vec<u8>,
    pub private_key: RsaPrivateKey
}
