use flate2::write::{GzEncoder, GzDecoder};
use flate2::Compression;
use std::io::{Write, Read};
use zip::{ZipWriter, write::FileOptions};
use std::fs::File;
use std::path::Path;

pub struct CompressionEngine;

impl CompressionEngine {
    pub fn compress_gzip(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        Ok(compressed)
    }
    
    pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    pub fn create_encrypted_zip(files: &[&Path], password: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let buffer = Vec::new();
        let mut zip = ZipWriter::new(std::io::Cursor::new(buffer));
        
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(9))
            .unix_permissions(0o755);
        
        for file in files {
            let file_name = file.file_name()
                .unwrap_or_default()
                .to_string_lossy();
            
            zip.start_file(file_name, options)?;
            
            let mut f = File::open(file)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            
            zip.write_all(&buffer)?;
        }
        
        let result = zip.finish()?;
        // Encrypt with AES-256
        // Implementation would use aes-gcm
        
        Ok(result.into_inner())
    }
    
    pub fn chunk_data(data: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
        data.chunks(chunk_size)
            .map(|c| c.to_vec())
            .collect()
    }
}