use crate::utils::cpu::mandelbrot::mandelbrot;
use std::collections::HashMap;

pub fn run(params: &HashMap<String, String>) -> Result<String, String> {
    let width = params
        .get("width")
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("Missing or invalid 'width' parameter")?;

    let height = params
        .get("height")
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("Missing or invalid 'height' parameter")?;

    let max_iter = params
        .get("max_iter")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1000);

    if width == 0 || height == 0 {
        return Err("Width and height must be greater than 0".into());
    }

    let ((map, elapsed_calc), _) = mandelbrot(width, height, max_iter, None);

    let rows_json = map
        .iter()
        .map(|row| format!("[{}]", row.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")))
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"elapsed_ms\": {}, \"map\": [{}]}}",
        width, height, max_iter, elapsed_calc, rows_json
    ))
}
