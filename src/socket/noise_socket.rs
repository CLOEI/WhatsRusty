pub struct NoiseSocket {
    pub tes: String
}

impl NoiseSocket {
    pub fn new(pubkey: [u8; 32], privkey: [u8; 32]) -> Self {
        Self {
            tes: "test".to_string()
        }
    }
}