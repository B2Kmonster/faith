#!/usr/bin/env python3
"""
String obfuscation for AcademicPhantom
Replaces sensitive strings with encrypted/encoded versions
"""

import sys
import re
import os
import random
import string

def xor_encrypt(data: bytes, key: bytes) -> bytes:
    return bytes([b ^ key[i % len(key)] for i, b in enumerate(data)])

def generate_key(length: int = 16) -> bytes:
    return bytes([random.randint(0, 255) for _ in range(length)])

def obfuscate_string(s: str) -> str:
    """Convert string to XOR encrypted byte array"""
    key = generate_key()
    encrypted = xor_encrypt(s.encode('utf-8'), key)
    
    key_str = ','.join(f'0x{b:02x}' for b in key)
    data_str = ','.join(f'0x{b:02x}' for b in encrypted)
    
    return f'obf_decrypt(&[{data_str}], &[{key_str}])'

def process_file(filepath: str):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Find string literals (simplified)
    pattern = r'"([^"]{10,})"'  # Strings longer than 10 chars
    
    def replace_string(match):
        original = match.group(1)
        # Skip format strings and obvious safe strings
        if original.startswith('{') or original in ['rust', 'true', 'false']:
            return match.group(0)
        return obfuscate_string(original)
    
    obfuscated = re.sub(pattern, replace_string, content)
    
    # Add decryption helper if not present
    if 'obf_decrypt' not in content:
        helper = '''
fn obf_decrypt(data: &[u8], key: &[u8]) -> String {
    String::from_utf8(
        data.iter().enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect()
    ).unwrap_or_default()
}
'''
        obfuscated = helper + obfuscated
    
    with open(filepath, 'w') as f:
        f.write(obfuscated)

def main():
    if len(sys.argv) < 2:
        print("Usage: obfuscate.py <source_directory>")
        sys.exit(1)
    
    src_dir = sys.argv[1]
    
    for root, dirs, files in os.walk(src_dir):
        for file in files:
            if file.endswith('.rs'):
                filepath = os.path.join(root, file)
                print(f"Obfuscating {filepath}")
                process_file(filepath)

if __name__ == '__main__':
    main()