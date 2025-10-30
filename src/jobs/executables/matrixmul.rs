use crate::utils::cpu::matrixmul::matrixmul;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let size = params
        .get("size")
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("Missing or invalid 'size' parameter")?;

    let seed = params
        .get("seed")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(123);

    if size == 0 || size > 1000 {
        return Err("Matrix size must be between 1 and 1000".into());
    }

    let (hash, elapsed_calc) = matrixmul(size, seed);

    Ok(format!(
        "{{\"size\": {}, \"seed\": {}, \"result_sha256\": \"{}\", \"elapsed_ms\": {}}}",
        size, seed, hash, elapsed_calc
    ))
}