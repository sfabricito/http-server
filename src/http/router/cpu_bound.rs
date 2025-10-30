use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    router::router::{PooledHandler, QueryParam},
    request::HttpRequest,
    response::{Response, OK, ACCEPTED, SERVICE_UNAVAILABLE},
    errors::ServerError,
};

use crate::jobs::{
    best_effort::{self, BestEffortError, BestEffortOutcome},
    job::Priority,
    manager::JobManager,
};
use crate::worker_pool::{self, ThreadPool};

use crate::utils::{
    cpu::{
        is_prime::{self, PrimeMethod},
        factor::factorize,
        matrixmul::matrixmul,
        mandelbrot::mandelbrot
    },
};

/// /isprime?n=NUM
pub struct IsPrimeHandler {
    pub job_manager: Arc<JobManager>,
}

fn queue_full_response(retry_after_ms: u64) -> Response {
    let retry_after_secs = ((retry_after_ms + 999) / 1000).max(1);
    Response::new(SERVICE_UNAVAILABLE)
        .set_header("Content-Type", "application/json")
        .set_header("Retry-After", &retry_after_secs.to_string())
        .with_body(format!("{{\"retry_after_ms\":{}}}", retry_after_ms))
}

fn with_worker_pid(resp: Response, worker_pid: Option<i32>) -> Response {
    let pid_opt = worker_pid.or_else(|| worker_pool::current_worker_pid());
    if let Some(pid) = pid_opt {
        resp.set_header("X-Worker-Pid", &pid.to_string())
    } else {
        resp
    }
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
        let method_name = match method {
            PrimeMethod::Trial => "trial",
            _ => "miller-rabin",
        }
        .to_string();

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(500);

        let mut params = HashMap::new();
        params.insert("n".into(), n.to_string());
        params.insert("method".into(), method_name.clone());

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "isprime",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let start = Instant::now();
                let result = is_prime::is_prime(n, method);
                let elapsed = start.elapsed().as_millis();
                Ok(format!(
                    "{{\"n\": {}, \"is_prime\": {}, \"method\": \"{}\", \"elapsed_ms\": {}}}",
                    n, result, method_name, elapsed
                ))
            },
        );

        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(BestEffortError::QueueFull { retry_after_ms }) => {
                return Ok(queue_full_response(retry_after_ms));
            }
            Err(BestEffortError::HandlerFailed(err)) => {
                return Err(ServerError::Internal(err));
            }
            Err(BestEffortError::Internal(err)) => {
                return Err(ServerError::Internal(err));
            }
        };

        match outcome {
            BestEffortOutcome::Completed { json, worker_pid } => {
                let resp = Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json);
                Ok(with_worker_pid(resp, worker_pid))
            }
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"n\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    n, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
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
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(500);

        let mut params = HashMap::new();
        params.insert("n".into(), n.to_string());

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "factor",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let start = Instant::now();
                let factors = factorize(n);
                let elapsed = start.elapsed().as_millis();
                let factors_json = factors
                    .iter()
                    .map(|(p, c)| format!("[{},{}]", p, c))
                    .collect::<Vec<_>>()
                    .join(",");
                Ok(format!(
                    "{{\"n\": {}, \"factors\": [{}], \"elapsed_ms\": {}}}",
                    n, factors_json, elapsed
                ))
            },
        );

        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(BestEffortError::QueueFull { retry_after_ms }) => {
                return Ok(queue_full_response(retry_after_ms));
            }
            Err(BestEffortError::HandlerFailed(err)) => {
                return Err(ServerError::Internal(err));
            }
            Err(BestEffortError::Internal(err)) => {
                return Err(ServerError::Internal(err));
            }
        };

        match outcome {
            BestEffortOutcome::Completed { json, worker_pid } => {
                let resp = Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json);
                Ok(with_worker_pid(resp, worker_pid))
            }
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"n\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    n, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

/// /pi?digits=D
pub struct PiHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for PiHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        use crate::utils::cpu::pi::pi_number;

        // Extract query parameter
        let digits_str = req.query_param("digits")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'digits'".into()))?;

        let digits = digits_str
            .parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid digits value: {}", digits_str)))?;

        // if digits == 0 || digits > 5000 {
        //     return Err(ServerError::BadRequest("Digits must be between 1 and 5000".into()));
        // }

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5000);

        let mut params = HashMap::new();
        params.insert("digits".into(), digits.to_string());
        params.insert("algo".into(), "chudnovsky".into());

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "pi",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let start = Instant::now();
                let result = pi_number(digits);
                let elapsed = start.elapsed().as_millis();
                Ok(format!(
                    "{{\"digits\": {}, \"pi\": \"{}\", \"elapsed_ms\": {}}}",
                    digits, result, elapsed
                ))
            },
        );

        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(BestEffortError::QueueFull { retry_after_ms }) => {
                return Ok(queue_full_response(retry_after_ms));
            }
            Err(BestEffortError::HandlerFailed(err)) => {
                return Err(ServerError::Internal(err));
            }
            Err(BestEffortError::Internal(err)) => {
                return Err(ServerError::Internal(err));
            }
        };

        match outcome {
            BestEffortOutcome::Completed { json, worker_pid } => {
                let resp = Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json);
                Ok(with_worker_pid(resp, worker_pid))
            }
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"digits\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    digits, timeout_ms, job_id
                );

                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
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

        let mut params = HashMap::new();
        params.insert("size".into(), size.to_string());
        params.insert("seed".into(), seed.to_string());

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "matrixmul",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let total_start = Instant::now();
                let (hash, elapsed_calc) = matrixmul(size, seed);
                let total_elapsed = total_start.elapsed().as_millis();
                Ok(format!(
                    "{{\"size\": {}, \"seed\": {}, \"result_sha256\": \"{}\", \"elapsed_ms\": {}, \"total_elapsed_ms\": {}}}",
                    size, seed, hash, elapsed_calc, total_elapsed
                ))
            },
        );

        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(BestEffortError::QueueFull { retry_after_ms }) => {
                return Ok(queue_full_response(retry_after_ms));
            }
            Err(BestEffortError::HandlerFailed(err)) => {
                return Err(ServerError::Internal(err));
            }
            Err(BestEffortError::Internal(err)) => {
                return Err(ServerError::Internal(err));
            }
        };

        match outcome {
            BestEffortOutcome::Completed { json, worker_pid } => {
                let resp = Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json);
                Ok(with_worker_pid(resp, worker_pid))
            }
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"size\": {}, \"seed\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    size, seed, timeout_ms, job_id
                );

                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
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
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(500);

        let mut params = HashMap::new();
        params.insert("width".into(), width.to_string());
        params.insert("height".into(), height.to_string());
        params.insert("max_iter".into(), max_iter.to_string());

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "mandelbrot",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let (map, mandelbrot_elapsed) = mandelbrot(width, height, max_iter, None);
                let rows_json = map
                    .iter()
                    .map(|row| {
                        format!(
                            "[{}]",
                            row.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");

                Ok(format!(
                    "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"elapsed_ms\": {}, \"map\": [{}]}}",
                    width, height, max_iter, mandelbrot_elapsed, rows_json
                ))
            },
        );

        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(BestEffortError::QueueFull { retry_after_ms }) => {
                return Ok(queue_full_response(retry_after_ms));
            }
            Err(BestEffortError::HandlerFailed(err)) => {
                return Err(ServerError::Internal(err));
            }
            Err(BestEffortError::Internal(err)) => {
                return Err(ServerError::Internal(err));
            }
        };

        match outcome {
            BestEffortOutcome::Completed { json, worker_pid } => {
                let resp = Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json);
                Ok(with_worker_pid(resp, worker_pid))
            }
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    width, height, max_iter, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

pub fn register(builder: DispatcherBuilder, job_manager: Arc<JobManager>) -> DispatcherBuilder {
    let isprime_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(IsPrimeHandler { job_manager: job_manager.clone() });
    let factor_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(FactorHandler { job_manager: job_manager.clone() });
    let pi_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(PiHandler { job_manager: job_manager.clone() });
    let matrix_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(MatrixMulHandler { job_manager: job_manager.clone() });
    let mandelbrot_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(MandelbrotHandler { job_manager: job_manager.clone() });

    let isprime = Arc::new(PooledHandler::new(
        ThreadPool::from_env("isprime", "WORKERS_ISPRIME", 1),
        isprime_handler,
    ));
    let factor = Arc::new(PooledHandler::new(
        ThreadPool::from_env("factor", "WORKERS_FACTOR", 4),
        factor_handler,
    ));
    let pi = Arc::new(PooledHandler::new(
        ThreadPool::from_env("pi", "WORKERS_PI", 2),
        pi_handler,
    ));
    let matrix = Arc::new(PooledHandler::new(
        ThreadPool::from_env("matrixmul", "WORKERS_MATRIXMUL", 2),
        matrix_handler,
    ));
    let mandelbrot = Arc::new(PooledHandler::new(
        ThreadPool::from_env("mandelbrot", "WORKERS_MANDELBROT", 2),
        mandelbrot_handler,
    ));

    builder
        .get("/isprime", isprime)
        .get("/factor", factor)
        .get("/pi", pi)
        .get("/matrixmul", matrix)
        .get("/mandelbrot", mandelbrot)
}
