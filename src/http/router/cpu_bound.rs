use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    router::router::QueryParam,
    request::HttpRequest,
    response::{Response, OK, SERVICE_UNAVAILABLE},
    errors::ServerError,
};

use crate::jobs::{
    job::Priority,
    manager::JobManager,
};

use crate::utils::{
    cpu::{
        is_prime::{self, PrimeMethod},
        factor::factorize,
        matrixmul::matrixmul,
        pi::pi_number,
        mandelbrot::mandelbrot
    },
    timeout::run_with_timeout
};

const SU_PREFIX: &str = "SERVICE_UNAVAILABLE:";

/// /isprime?n=NUM
pub struct IsPrimeHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for IsPrimeHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let n_str = req.query_param("n")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'n'".into()))?;

        let n = n_str.parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer: {}", n_str)))?;

        let method_env = env::var("PRIME_NUMBER_METHOD").unwrap_or_else(|_| "MILLER_RABIN".into());
        let method = match method_env.trim().to_uppercase().as_str() {
            "TRIAL" | "SQRT" => PrimeMethod::Trial,
            _ => PrimeMethod::MillerRabin,
        };
        let method_name = match method { PrimeMethod::Trial => "trial", _ => "miller-rabin" };

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(500);

        // Try immediate execution
        if let Some((result, elapsed)) = run_with_timeout(timeout_ms, move || is_prime::is_prime(n, method)) {
            let json = format!(
                "{{\"n\": {}, \"is_prime\": {}, \"method\": \"{}\", \"elapsed_ms\": {}}}",
                n, result, method_name, elapsed
            );
            return Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json));
        }

        // Otherwise enqueue as job
        let mut params = HashMap::new();
        params.insert("n".into(), n.to_string());
        params.insert("method".into(), method_name.to_string());

        match self.job_manager.submit("isprime", params, Priority::Normal) {
            Ok(job_id) => {
                let json = format!(
                    "{{\"n\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    n, timeout_ms, job_id
                );
                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            Err(e) if e.starts_with(SU_PREFIX) => {
                let body = &e[SU_PREFIX.len()..];
                Ok(Response::new(SERVICE_UNAVAILABLE)
                    .set_header("Content-Type", "application/json")
                    .set_header("Retry-After", "1")
                    .with_body(body.to_string()))
            }
            Err(e) => Err(ServerError::Internal(format!("Job submit failed: {}", e))),
        }
    }
}


/// /factor?n=NUM
pub struct FactorHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for FactorHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let n_str = req.query_param("n")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'n'".into()))?;

        let n = n_str.parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer: {}", n_str)))?;

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(500);

        if let Some((factors, elapsed)) = run_with_timeout(timeout_ms, move || factorize(n)) {
            let factors_json = factors
                .iter()
                .map(|(p, c)| format!("[{},{}]", p, c))
                .collect::<Vec<_>>()
                .join(",");

            let json = format!(
                "{{\"n\": {}, \"factors\": [{}], \"elapsed_ms\": {}}}",
                n, factors_json, elapsed
            );
            return Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json));
        }

        let mut params = HashMap::new();
        params.insert("n".into(), n.to_string());
        match self.job_manager.submit("factor", params, Priority::Normal) {
            Ok(job_id) => {
                let json = format!(
                    "{{\"n\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    n, timeout_ms, job_id
                );
                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            Err(e) if e.starts_with(SU_PREFIX) => {
                let body = &e[SU_PREFIX.len()..];
                Ok(Response::new(SERVICE_UNAVAILABLE)
                    .set_header("Content-Type", "application/json")
                    .set_header("Retry-After", "1")
                    .with_body(body.to_string()))
            }
            Err(e) => Err(ServerError::Internal(format!("Job submit failed: {}", e))),
        }
    }
}

/// /pi?digits=D
pub struct PiHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for PiHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        // Extract query parameter
        let digits_str = req.query_param("digits")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'digits'".into()))?;

        let digits = digits_str
            .parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid digits value: {}", digits_str)))?;

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5000);

        // Try direct computation (best effort)
        if let Some((result, elapsed)) = run_with_timeout(timeout_ms, move || pi_number(digits)) {
            let json = format!(
                "{{\"digits\": {}, \"pi\": \"{}\", \"elapsed_ms\": {}}}",
                digits, result, elapsed
            );

            return Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json));
        }

        // Fallback: submit as async job
        let mut params = HashMap::new();
        params.insert("digits".into(), digits.to_string());
        params.insert("algo".into(), "chudnovsky".into());

        match self.job_manager.submit("pi", params, Priority::Normal) {
            Ok(job_id) => {
                let json = format!(
                    "{{\"digits\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    digits, timeout_ms, job_id
                );
                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            Err(e) if e.starts_with(SU_PREFIX) => {
                let body = &e[SU_PREFIX.len()..];
                Ok(Response::new(SERVICE_UNAVAILABLE)
                    .set_header("Content-Type", "application/json")
                    .set_header("Retry-After", "1")
                    .with_body(body.to_string()))
            }
            Err(e) => Err(ServerError::Internal(format!("Job submit failed: {}", e))),
        }
    }
}

/// /matrixmul?size=N&seed=S
pub struct MatrixMulHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for MatrixMulHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let size_str = req.query_param("size")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'size'".into()))?;
        let seed_str = req.query_param("seed").unwrap_or("123");

        let size = size_str.parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid size: {}", size_str)))?;
        let seed = seed_str.parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid seed: {}", seed_str)))?;

        if size == 0 || size > 1000 {
            return Err(ServerError::BadRequest("Matrix size must be 1–1000".into()));
        }

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5000);

        if let Some(((hash, elapsed_calc), total_elapsed)) = run_with_timeout(timeout_ms, move || matrixmul(size, seed)) {
            let json = format!(
                "{{\"size\": {}, \"seed\": {}, \"result_sha256\": \"{}\", \"elapsed_ms\": {}, \"total_elapsed_ms\": {}}}",
                size, seed, hash, elapsed_calc, total_elapsed
            );
            return Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json));
        }

        let mut params = HashMap::new();
        params.insert("size".into(), size.to_string());
        params.insert("seed".into(), seed.to_string());

        match self.job_manager.submit("matrixmul", params, Priority::Normal) {
            Ok(job_id) => {
                let json = format!(
                    "{{\"size\": {}, \"seed\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    size, seed, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            Err(e) if e.starts_with(SU_PREFIX) => {
                let body = &e[SU_PREFIX.len()..];
                Ok(Response::new(SERVICE_UNAVAILABLE)
                    .set_header("Content-Type", "application/json")
                    .set_header("Retry-After", "1")
                    .with_body(body.to_string()))
            }
            Err(e) => Err(ServerError::Internal(format!("Job submit failed: {}", e))),
        }
    }
}

/// /mandelbrot?width=W&height=H&max_iter=I
pub struct MandelbrotHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for MandelbrotHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let width_str = req.query_param("width")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'width'".into()))?;
        let height_str = req.query_param("height")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'height'".into()))?;
        let iter_str = req.query_param("max_iter")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'max_iter'".into()))?;

        let width = width_str.parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid width: {}", width_str)))?;
        let height = height_str.parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid height: {}", height_str)))?;
        let max_iter = iter_str.parse::<u32>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid max_iter: {}", iter_str)))?;

        if width == 0 || height == 0 {
            return Err(ServerError::BadRequest("Width and height must be > 0".into()));
        }

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(500);

        if let Some(result) = run_with_timeout(timeout_ms, move || mandelbrot(width, height, max_iter, None)) {
            let ((map, mandelbrot_elapsed), _) = result;

            let rows_json = map.iter()
                .map(|row| format!("[{}]", row.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")))
                .collect::<Vec<_>>()
                .join(",");

            let json = format!(
                "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"elapsed_ms\": {}, \"map\": [{}]}}",
                width, height, max_iter, mandelbrot_elapsed, rows_json
            );
            return Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json));
        }

        let mut params = HashMap::new();
        params.insert("width".into(), width.to_string());
        params.insert("height".into(), height.to_string());
        params.insert("max_iter".into(), max_iter.to_string());

        match self.job_manager.submit("mandelbrot", params, Priority::Normal) {
            Ok(job_id) => {
                let json = format!(
                    "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    width, height, max_iter, timeout_ms, job_id
                );
                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            Err(e) if e.starts_with(SU_PREFIX) => {
                let body = &e[SU_PREFIX.len()..];
                Ok(Response::new(SERVICE_UNAVAILABLE)
                    .set_header("Content-Type", "application/json")
                    .set_header("Retry-After", "1")
                    .with_body(body.to_string()))
            }
            Err(e) => Err(ServerError::Internal(format!("Job submit failed: {}", e))),
        }
    }
}

pub fn register(builder: DispatcherBuilder, job_manager: Arc<JobManager>) -> DispatcherBuilder {
    builder
        .get("/isprime", Arc::new(IsPrimeHandler { job_manager: job_manager.clone() }))
        .get("/factor", Arc::new(FactorHandler { job_manager: job_manager.clone() }))
        .get("/pi", Arc::new(PiHandler { job_manager: job_manager.clone() }))
        .get("/matrixmul", Arc::new(MatrixMulHandler { job_manager: job_manager.clone() }))
        .get("/mandelbrot", Arc::new(MandelbrotHandler { job_manager: job_manager.clone() }))
}
