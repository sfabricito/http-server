use crate::utils::io::compress::compress_file;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let name = params.get("name").cloned().unwrap_or_default();
    let codec = params.get("codec").cloned().unwrap_or("gzip".into());

    match compress_file(&name, &codec) {
        Ok(result) => {
            let output = result.output_file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            Ok(format!(
                "{{\"file\":\"{}\",\"codec\":\"{}\",\"output\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{}}}",
                name, codec, output, result.compressed_size, result.elapsed_ms
            ))
        }
        Err(e) => Err(format!("Compression failed: {}", e)),
    }
}