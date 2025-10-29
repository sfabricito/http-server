// use crate::utils::cpu::pi::{spigot_pi, chudnovsky_pi};
// use std::collections::HashMap;

// pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
//     let digits = params
//         .get("digits")
//         .and_then(|v| v.parse::<usize>().ok())
//         .ok_or("Missing or invalid 'digits' parameter")?;

//     let algo = params
//         .get("algo")
//         .map(|s| s.to_lowercase())
//         .unwrap_or_else(|| "spigot".to_string());

//     if digits == 0 {
//         return Err("Digits must be greater than 0".into());
//     }

//     let start = std::time::Instant::now();
//     let result = match algo.as_str() {
//         "spigot" => spigot_pi(digits),
//         "chudnovsky" => chudnovsky_pi(digits),
//         _ => return Err(format!("Unknown algorithm '{}'", algo)),
//     };
//     let elapsed = start.elapsed().as_millis();

//     Ok(format!(
//         "{{\"digits\": {}, \"algo\": \"{}\", \"result\": \"{}\", \"elapsed_ms\": {}}}",
//         digits, algo, result, elapsed
//     ))
// }
