use rsa::{RsaPrivateKey, RsaPublicKey, PaddingScheme, PublicKey, PrivateKey};
use rsa::pkcs8::{EncodePrivateKey, DecodePrivateKey, EncodePublicKey, DecodePublicKey};
use sha2::{Sha256, Digest};

pub struct RsaCrypto {
    private_key: Option<RsaPrivateKey>,
    public_key: RsaPublicKey,
}

impl RsaCrypto {
    pub fn generate(bits: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, bits)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        Ok(Self {
            private_key: Some(private_key),
            public_key,
        })
    }
    
    pub fn from_public_key_pem(pem: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let public_key = RsaPublicKey::from_public_key_pem(pem)?;
        
        Ok(Self {
            private_key: None,
            public_key,
        })
    }
    
    pub fn from_private_key_pem(pem: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(pem)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        Ok(Self {
            private_key: Some(private_key),
            public_key,
        })
    }
    
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut rng = rand::thread_rng();
        let padding = PaddingScheme::new_pkcs1v15_encrypt();
        
        self.public_key.encrypt(&mut rng, padding, data)
            .map_err(|e| e.into())
    }
    
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let private_key = self.private_key.as_ref()
            .ok_or("No private key available")?;
        
        let padding = PaddingScheme::new_pkcs1v15_encrypt();
        private_key.decrypt(padding, data)
            .map_err(|e| e.into())
    }
    
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let private_key = self.private_key.as_ref()
            .ok_or("No private key available")?;
        
        // Hash data first
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        
        let padding = PaddingScheme::new_pkcs1v15_sign::<Sha256>();
        private_key.sign(padding, &hash)
            .map_err(|e| e.into())
    }
    
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        
        let padding = PaddingScheme::new_pkcs1v15_sign::<Sha256>();
        
        match self.public_key.verify(padding, &hash, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    pub fn public_key_to_pem(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.public_key.to_public_key_pem(Default::default())
            .map_err(|e| e.into())
    }
    
    pub fn private_key_to_pem(&self) -> Result<String, Box<dyn std::error::Error>> {
        let private_key = self.private_key.as_ref()
            .ok_or("No private key available")?;
        
        private_key.to_pkcs8_pem(Default::default())
            .map(|s| s.to_string())
            .map_err(|e| e.into())
    }
}