use crate::utils::io::grep::grep_file;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let name = params.get("name").cloned().unwrap_or_default();
    let pattern = params.get("pattern").cloned().unwrap_or_default();

    match grep_file(&name, &pattern) {
        Ok(result) => {
            let lines_json = result
                .matched_lines
                .iter()
                .map(|l| format!("\"{}\"", l.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",");

            Ok(format!(
                "{{\"file\":\"{}\",\"pattern\":\"{}\",\"matches\":{},\"lines\":[{}],\"elapsed_ms\":{}}}",
                name, pattern, result.total_matches, lines_json, result.elapsed_ms
            ))
        }
        Err(e) => Err(format!("Grep failed: {}", e)),
    }
}
