use crate::utils::cpu::is_prime::{is_prime, PrimeMethod};
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let n = params
        .get("n")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or("Missing or invalid 'n' parameter")?;

    let method_str = params.get("method").cloned().unwrap_or("miller-rabin".into());
    let method = match method_str.to_lowercase().as_str() {
        "trial" | "sqrt" => PrimeMethod::Trial,
        _ => PrimeMethod::MillerRabin,
    };

    let result = is_prime(n, method);
    Ok(format!(
        "{{\"n\": {}, \"is_prime\": {}, \"method\": \"{}\"}}",
        n, result, method_str
    ))
}
