use crate::utils::cpu::pi::pi_number;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    println!("Running pi calculation with params: {:?}", params);
    let digits = params
        .get("digits")
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("Missing or invalid 'digits' parameter")?;

    if digits == 0 {
        return Err("Digits must be greater than 0".into());
    }

    let start = std::time::Instant::now();
    let result = pi_number(digits); 
    let elapsed = start.elapsed().as_millis();

    Ok(format!(
        "{{\"digits\": {}, \"algo\": \"chudnovsky\", \"result\": \"{}\", \"elapsed_ms\": {}}}",
        digits, result, elapsed
    ))
}
