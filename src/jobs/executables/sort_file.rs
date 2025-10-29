use crate::utils::io::sort_file::sort_file;
use std::path::PathBuf;

pub fn run(params: &std::collections::HashMap<String, String>) -> Result<String, String> {
    let name = params.get("name").cloned().unwrap_or_default();
    let algo = params.get("algo").cloned().unwrap_or("merge".into());

    match sort_file(&name, &algo) {
        Ok((out_path, count, sort_elapsed)) => {
            let sorted_name = out_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            Ok(format!(
                "{{\"file\":\"{}\",\"algo\":\"{}\",\"sorted_file\":\"{}\",\"count\":{},\"elapsed_ms\":{}}}",
                name, algo, sorted_name, count, sort_elapsed
            ))
        }
        Err(e) => Err(format!("Error sorting file: {}", e)),
    }
}