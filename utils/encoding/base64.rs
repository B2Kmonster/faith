use base64::{Engine as _, engine::general_purpose};

pub struct Base64Encoder;

impl Base64Encoder {
    pub fn encode(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(data)
    }
    
    pub fn decode(data: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        general_purpose::STANDARD.decode(data)
            .map_err(|e| e.into())
    }
    
    pub fn encode_url_safe(data: &[u8]) -> String {
        general_purpose::URL_SAFE.encode(data)
    }
    
    pub fn decode_url_safe(data: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        general_purpose::URL_SAFE.decode(data)
            .map_err(|e| e.into())
    }
    
    pub fn encode_with_line_break(data: &[u8], line_length: usize) -> String {
        let encoded = Self::encode(data);
        encoded.as_bytes()
            .chunks(line_length)
            .map(|chunk| String::from_utf8_lossy(chunk))
            .collect::<Vec<_>>()
            .join("\n")
    }
}