use crate::utils::io::word_count::word_count;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let name = params.get("name").cloned().unwrap_or_default();

    match word_count(&name) {
        Ok((counts, elapsed, path)) => {
            let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            Ok(format!(
                "{{\"file\":\"{}\",\"lines\":{},\"words\":{},\"bytes\":{},\"elapsed_ms\":{}}}",
                file, counts.lines, counts.words, counts.bytes, elapsed
            ))
        }
        Err(e) => Err(format!("Word count failed: {}", e)),
    }
}
