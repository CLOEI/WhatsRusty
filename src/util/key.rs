use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, Scalar};
use rand_core::OsRng;

#[derive(Default)]
pub struct Key {
    pub public: [u8; 32],
    pub private: [u8; 32]
}

pub struct PreKey {
    key: Key,
    id: u32,
    signature: [u8; 64]
}

impl Key {
    pub fn new() -> Self {
        let private = Scalar::random(&mut OsRng);
        let public = (ED25519_BASEPOINT_POINT * private).to_montgomery();
        
        Self { 
            public: public.to_bytes(),
            private: private.to_bytes()
        }
    }

    pub fn create_signed_pre_key(&self, key_id: u32) {
        let mut new_key = PreKey::new(key_id);
        
        new_key.signature = self.sign(&new_key.key_pair);
        new_key
    }

    pub fn sign(key: Key) {
        
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