use crate::utils::io::hash_file::hash_file;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let name = params.get("name").cloned().unwrap_or_default();
    let algo = params.get("algo").cloned().unwrap_or("sha256".into());

    match hash_file(&name, &algo) {
        Ok(result) => Ok(format!(
            "{{\"file\":\"{}\",\"algorithm\":\"{}\",\"hash\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{}}}",
            name, algo, result.hash_hex, result.file_size, result.elapsed_ms
        )),
        Err(e) => Err(format!("Hashing failed: {}", e)),
    }
}
