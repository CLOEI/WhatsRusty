use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, Scalar};
use rand_core::OsRng;

pub struct Key {
    pub public: [u8; 32],
    pub private: [u8; 32]
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
}