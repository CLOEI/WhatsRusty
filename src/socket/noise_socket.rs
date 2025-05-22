use aes_gcm::{aead::Aead, Aes256Gcm, Nonce};
use aes_gcm::aead::Payload;
use paris::error;

pub struct NoiseSocket {
    pub write_key: Aes256Gcm,
    pub read_key: Aes256Gcm,
    pub read_counter: usize,
    pub write_counter: usize,
}

impl NoiseSocket {
    pub fn new(write_key: Aes256Gcm, read_key: Aes256Gcm) -> Self {
        Self {
            write_key,
            read_key,
            read_counter: 0,
            write_counter: 0
        }
    }

    pub fn receive_encrypted_frame(&mut self, ciphertext: &[u8]) -> Vec<u8> {
        let counter = self.read_counter;
        self.read_counter += 1;

        let iv = Self::generate_iv(counter);
        let nonce = Nonce::from_slice(&iv);

        match self.read_key.decrypt(nonce, Payload {
            msg: ciphertext,
            aad: b"",
        }) {
            Ok(plaintext) => {
                plaintext
            }
            Err(e) => {
                error!("Failed to decrypt frame: {:?}", e);
                panic!("Failed to decrypt frame");
            }
        }
    }

    pub fn make_frame(&mut self, plaintext: Vec<u8>) -> Vec<u8> {
        let iv = Self::generate_iv(self.write_counter);
        let nonce = Nonce::from_slice(&iv);
        let cipher_text = self.write_key.encrypt(nonce, plaintext.as_slice());
        self.write_counter += 1;
        cipher_text.unwrap()
    }

    pub fn generate_iv(counter: usize) -> [u8; 12] {
        let mut iv = [0u8; 12];
        iv[8] = (counter >> 24) as u8;
        iv[9] = (counter >> 16) as u8;
        iv[10] = (counter >> 8) as u8;
        iv[11] = counter as u8;
        iv
    }
}