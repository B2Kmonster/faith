use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::Payload;

pub struct AesGcmCipher {
    cipher: Aes256Gcm,
}

impl AesGcmCipher {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key).expect("Invalid key size");
        Self { cipher }
    }
    
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }
    
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = self.cipher.encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        
        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if ciphertext.len() < 12 {
            return Err("Ciphertext too short".into());
        }
        
        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let encrypted = &ciphertext[12..];
        
        self.cipher.decrypt(nonce, encrypted)
            .map_err(|e| format!("Decryption failed: {:?}", e).into())
    }
    
    pub fn encrypt_with_aad(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let payload = Payload {
            msg: plaintext,
            aad,
        };
        
        let ciphertext = self.cipher.encrypt(nonce, payload)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
}