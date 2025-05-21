use aes_gcm::{Aes256Gcm, Key, KeyInit};

pub fn prepare(secret: Vec<u8>) -> Aes256Gcm {
    let key = Key::<Aes256Gcm>::from_slice(&secret[..]);
    Aes256Gcm::new(key)
}