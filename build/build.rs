pub struct XorCipher {
    key: Vec<u8>,
}

impl XorCipher {
    pub fn new(key: &[u8]) -> Self {
        Self {
            key: key.to_vec(),
        }
    }
    
    pub fn from_string(key: &str) -> Self {
        Self::new(key.as_bytes())
    }
    
    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ self.key[i % self.key.len()])
            .collect()
    }
    
    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        // XOR is symmetric
        self.encrypt(data)
    }
    
    pub fn encrypt_in_place(&self, data: &mut [u8]) {
        for (i, b) in data.iter_mut().enumerate() {
            *b ^= self.key[i % self.key.len()];
        }
    }
    
    pub fn rotate_key(&mut self, rotation: usize) {
        if !self.key.is_empty() {
            let rotation = rotation % self.key.len();
            self.key.rotate_left(rotation);
        }
    }
}