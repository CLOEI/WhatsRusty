use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::{Aead, Payload};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};
use crate::constant;
use crate::socket::noise_socket::NoiseSocket;
use crate::utils::gcm;

#[derive(Default)]
pub struct NoiseHandShake {
    hash: Vec<u8>,
    salt: Vec<u8>,
    counter: usize,
    key: Option<Aes256Gcm>
}

impl NoiseHandShake {
    pub fn start(&mut self, header: Vec<u8>) {
        let mut data = vec![];
        data.extend_from_slice(constant::NOISE_PATTERN.as_bytes());
        if data.len() == 32 {
            self.hash = data;
        } else {
            self.hash = self.compute_hash(data);
        }

        self.salt = self.hash.clone();
        self.key = Some(gcm::prepare(self.hash.clone()));
        self.authenticate(&header);
    }

    pub fn authenticate(&mut self, data: &Vec<u8>) {
        self.hash = self.compute_hash([self.hash.clone(), data.clone()].concat());
    }

    pub fn mix_shared_secret(&mut self, private_key: [u8; 32], public_key: [u8; 32]) {
        let secret = StaticSecret::from(private_key);
        let public = PublicKey::from(public_key);
        let shared_secret = secret.diffie_hellman(&public);
        self.mix_into_key(shared_secret.as_bytes());
    }

    pub fn decrypt(&mut self, cipher: &[u8]) -> Vec<u8> {
        self.counter += 1;
        let iv = NoiseSocket::generate_iv(self.counter - 1);
        let nonce = Nonce::from_slice(&iv);
        let plaintext = self.key.as_ref().unwrap().decrypt(nonce, Payload {
            msg: cipher,
            aad: &self.hash,
        }).expect("Failed to decrypt");
        self.authenticate(&cipher.to_vec());
        plaintext
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        self.counter += 1;
        let iv = NoiseSocket::generate_iv(self.counter - 1);
        let nonce = Nonce::from_slice(&iv);
        let ciphertext = self.key.as_ref().unwrap().encrypt(nonce, Payload {
            msg: plaintext,
            aad: &self.hash,
        }).expect("Failed to decrypt");
        self.authenticate(&ciphertext.to_vec());
        ciphertext
    }

    fn mix_into_key(&mut self, data: &[u8]) {
        self.counter = 0;
        let (write, read) = self.extract_and_expand(Some(data));

        self.salt = write.to_vec();
        self.key = Some(gcm::prepare(read.to_vec()))
    }

    fn compute_hash(&mut self, data: Vec<u8>) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    pub fn extract_and_expand(&self, data: Option<&[u8]>) -> (Vec<u8>, Vec<u8>) {
        let hk = Hkdf::<Sha256>::new(Some(&self.salt), data.unwrap_or(&[]));
        let mut okm = [0u8; 64];
        hk.expand(&[], &mut okm).expect("HKDF expand failed");

        let (write, read) = okm.split_at(32);
        (write.to_vec(), read.to_vec())
    }
}