use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;
use sha2::{Sha512, Digest};
use x25519_dalek::{PublicKey, StaticSecret};

pub struct Key {
    pub public: PublicKey,
    pub private: StaticSecret,
}

pub struct PreKey {
    pub key: Key,
    pub id: u32,
    pub signature: [u8; 64]
}

impl Key {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let private = StaticSecret::random_from_rng(&mut csprng);
        let public = PublicKey::from(&private);

        Self {
            public,
            private,
        }
    }

    pub fn create_signed_pre_key(&self, key_id: u32) -> PreKey {
        let mut new_key = PreKey::new(key_id);

        new_key.signature = self.sign(&new_key.key);
        new_key
    }

    pub fn sign(&self, key_to_sign: &Key) -> [u8; 64] {
        let hash = Sha512::digest(self.private.as_bytes());
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&hash[..32]);
        let mut signing_key = SigningKey::from(seed);

        let mut pub_key_for_signature = [0u8; 33];
        pub_key_for_signature[0] = 0x05; // DJB_TYPE
        pub_key_for_signature[1..].copy_from_slice(key_to_sign.public.as_bytes());

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