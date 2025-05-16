use aes_gcm::{aead::{Aead, Payload}, Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::{constant};

use super::gcm;

#[derive(Default)]
pub struct NoiseHandShake {
    hash: Vec<u8>,
    salt: Vec<u8>,
    counter: u32,
    key: Option<Aes256Gcm>
}

impl NoiseHandShake {
    pub fn start(&mut self, header: Vec<u8>) {
        let mut data = vec![];
        data.extend_from_slice(constant::NOISE_PATTERN.as_bytes());
        if data.len() == 32 {
            self.hash = data;
        } else {
            self.hash = self.compute_sha256(data);
        }
        self.salt = self.hash.clone();
        self.key = Some(gcm::prepare(self.hash.clone()));
        self.authenticate(header);
    }

    pub fn authenticate(&mut self, data: Vec<u8>) {
        self.hash = self.compute_sha256([self.hash.clone(), data].concat())
    }

    fn compute_sha256(&self, data: Vec<u8>) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
    pub fn mix_shared_secret(&mut self, private_key: [u8; 32], public_key: [u8; 32]) {
        let secret = StaticSecret::from(private_key);
        let public = PublicKey::from(public_key);
        let shared_secret = secret.diffie_hellman(&public);
        self.mix_into_key(shared_secret.as_bytes());
    }

    pub fn decrypt(&mut self, cipher: &[u8]) -> Vec<u8> {
        self.counter += 1; 
        let iv = NoiseHandShake::generate_iv(self.counter - 1);
        let nonce = Nonce::from_slice(&iv);
        let plaintext = self.key.as_ref().unwrap().decrypt(nonce, Payload {
            msg: cipher,
            aad: &self.hash,
        }).expect("Failed to decrypt");
        self.authenticate(cipher.to_vec());
        plaintext
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        self.counter += 1; 
        let iv = NoiseHandShake::generate_iv(self.counter - 1);
        let nonce = Nonce::from_slice(&iv);
        let ciphertext = self.key.as_ref().unwrap().encrypt(nonce, Payload {
            msg: plaintext,
            aad: &self.hash,
        }).expect("Failed to decrypt");
        self.authenticate(ciphertext.to_vec());
        ciphertext
    }

    fn mix_into_key(&mut self, data: &[u8]) {
        self.counter = 0;
        let hk = Hkdf::<Sha256>::new(Some(&self.salt), data);
        let mut okm = [0u8; 64];
        hk.expand(&[], &mut okm).expect("HKDF expand failed");

        let (write, read) = okm.split_at(32);
        self.salt = write.to_vec();
        self.key = Some(gcm::prepare(read.to_vec()));
    }

    pub fn generate_iv(counter: u32) -> [u8; 12] {
        let mut iv = [0u8; 12];
        iv[8] = (counter >> 24) as u8;
        iv[9] = (counter >> 16) as u8;
        iv[10] = (counter >> 8) as u8;
        iv[11] = counter as u8;
        iv
    }
}