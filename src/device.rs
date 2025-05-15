use rand_core::{OsRng, RngCore};

use crate::util::key::Key;

pub struct Device {
    pub noise_key: Key,
    pub identity_key: Key,
    pub registration_id: u32,
    pub adv_secret_key: [u8; 32]
}

impl Device {
    pub fn new() -> Self {
        let mut random_byte = [0u8; 32];
        OsRng.fill_bytes(&mut random_byte);
        Self {
            noise_key: Key::new(),
            identity_key: Key::new(),
            registration_id: OsRng.next_u32(),
            adv_secret_key: random_byte
        }
    } 
}