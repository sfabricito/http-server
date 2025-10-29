use crate::utils::cpu::factor::factorize;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let n = params
        .get("n")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or("Missing or invalid 'n' parameter")?;

    let factors = factorize(n);
    let factors_json = factors
        .iter()
        .map(|(p, c)| format!("[{},{}]", p, c))
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        "{{\"n\": {}, \"factors\": [{}]}}",
        n, factors_json
    ))
}
