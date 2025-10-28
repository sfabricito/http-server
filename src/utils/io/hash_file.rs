use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use std::time::Instant;

use sha2::{Sha256, Digest};

#[derive(Debug)]
pub struct HashResult {
    pub hash_hex: String,
    pub elapsed_ms: u128,
    pub file_size: u64,
}

pub fn hash_file(name: &str, algo: &str) -> io::Result<HashResult> {
    let base = env::var("FILE_STORAGE_PATH").unwrap_or_else(|_| "./data/files".to_string());
    let path = PathBuf::from(&base).join(name);

    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "File not found"));
    }

    let start = Instant::now();
    let mut file = BufReader::new(File::open(&path)?);
    let mut buffer = [0u8; 8192];

    match algo {
        "sha256" => {
            let mut hasher = Sha256::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            let hash = hasher.finalize();
            let hash_hex = format!("{:x}", hash);
            let elapsed = start.elapsed().as_millis();
            let size = std::fs::metadata(&path)?.len();
            Ok(HashResult {
                hash_hex,
                elapsed_ms: elapsed,
                file_size: size,
            })
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported hash algorithm")),
    }
}

/// JSON-friendly wrapper for HTTP response
pub fn hash_json(name: &str, algo: &str) -> String {
    match hash_file(name, algo) {
        Ok(res) => format!(
            r#"{{"algorithm":"{}","hash":"{}","size_bytes":{},"elapsed_ms":{}}}"#,
            algo, res.hash_hex, res.file_size, res.elapsed_ms
        ),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn make_test_file(name: &str, content: &str) -> PathBuf {
        let base = "./data/files";
        std::fs::create_dir_all(base).unwrap();
        let path = PathBuf::from(base).join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_basic_sha256() {
        let path = make_test_file("test_hash1.txt", "Rust is great!");
        let result = hash_file("test_hash1.txt", "sha256").unwrap();
        assert_eq!(
            result.hash_hex,
            "d7b0e3d21a5f5a26c2b8d6c4e9df46e326ea4ac88a5de9142543e14b4e9ed5c1"
        );
        assert!(result.file_size > 0);
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_invalid_algorithm() {
        let path = make_test_file("test_hash_invalid.txt", "test");
        let result = hash_file("test_hash_invalid.txt", "md5");
        assert!(result.is_err());
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_nonexistent_file() {
        let result = hash_file("no_such_file.txt", "sha256");
        assert!(result.is_err());
    }

    #[test]
    fn test_large_file_performance() {
        let path = make_test_file("test_hash_large.txt", &"A".repeat(1_000_000));
        let res = hash_file("test_hash_large.txt", "sha256").unwrap();
        assert!(res.file_size > 100_000);
        assert!(res.elapsed_ms < 2000, "Hashing too slow");
        fs::remove_file(path).ok();
    }
}
