use ed25519_dalek::{ed25519::signature::SignerMut, SigningKey};
use rand_core::OsRng;
use sha2::{Digest, Sha512};
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Default)]
pub struct Key {
    pub public: [u8; 32],
    pub private: [u8; 32]
}

pub struct PreKey {
    pub key: Key,
    pub id: u32,
    pub signature: [u8; 64]
}

impl Key {
    pub fn new() -> Self {
        let private = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&private);

        Self {
            private: private.to_bytes(),
            public: public.to_bytes(),
        }
    }

    pub fn create_signed_pre_key(&self, key_id: u32) -> PreKey {
        let mut new_key = PreKey::new(key_id);
        
        new_key.signature = self.sign(&new_key.key);
        new_key
    }

    pub fn sign(&self, key_to_sign: &Key) -> [u8; 64] {
        let mut signing_key = derive_ed25519_from_x25519(&self.private);

        let mut pub_key_for_signature = [0u8; 33];
        pub_key_for_signature[0] = 0x05; // DJB_TYPE
        pub_key_for_signature[1..].copy_from_slice(&key_to_sign.public);

        signing_key.sign(&pub_key_for_signature).to_bytes()
    }
}

impl PreKey {
    fn new(key_id: u32) -> Self {
        Self {
            key: Key::new(),
            id: key_id,
            signature: [0u8; 64]
        }
    }
}

fn derive_ed25519_from_x25519(x25519_priv: &[u8; 32]) -> SigningKey {
    let hash = Sha512::digest(x25519_priv);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hash[..32]);
    SigningKey::from_bytes(&seed)
}
