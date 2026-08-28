use crate::utils::crypto::{AesGcmCipher, RsaCrypto};
use sha2::{Sha256, Digest};

pub struct C2Crypto {
    session_key: [u8; 32],
    rsa_public: Option<RsaCrypto>,
}

impl C2Crypto {
    pub fn new() -> Self {
        Self {
            session_key: Self::derive_session_key(),
            rsa_public: None,
        }
    }
    
    pub fn with_rsa(server_pubkey_pem: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let rsa = RsaCrypto::from_public_key_pem(server_pubkey_pem)?;
        
        Ok(Self {
            session_key: Self::derive_session_key(),
            rsa_public: Some(rsa),
        })
    }
    
    pub fn encrypt_for_transit(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Use AES for bulk encryption
        let cipher = AesGcmCipher::new(&self.session_key);
        let encrypted = cipher.encrypt(data)?;
        
        // If RSA available, encrypt session key with it
        if let Some(ref rsa) = self.rsa_public {
            let encrypted_key = rsa.encrypt(&self.session_key)?;
            
            // Combine: [encrypted_key_len (4 bytes)][encrypted_key][encrypted_data]
            let mut result = Vec::new();
            result.extend_from_slice(&(encrypted_key.len() as u32).to_be_bytes());
            result.extend_from_slice(&encrypted_key);
            result.extend_from_slice(&encrypted);
            
            Ok(result)
        } else {
            // Just return AES encrypted with prepended key hash
            let mut result = Vec::new();
            let key_hash = Self::hash_key(&self.session_key);
            result.extend_from_slice(&key_hash[..8]); // First 8 bytes as ID
            result.extend_from_slice(&encrypted);
            Ok(result)
        }
    }
    
    pub fn decrypt_response(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cipher = AesGcmCipher::new(&self.session_key);
        cipher.decrypt(data)
    }
    
    pub fn rotate_session_key(&mut self) {
        self.session_key = Self::derive_session_key();
    }
    
    fn derive_session_key() -> [u8; 32] {
        use rand::Rng;
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        key
    }
    
    fn hash_key(key: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().into()
    }
    
    pub fn generate_beacon_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:016x}{:016x}", rng.gen::<u64>(), rng.gen::<u64>())
    }
}