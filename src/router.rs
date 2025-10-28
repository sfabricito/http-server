use std::env;
use std::sync::Arc;
use std::time::Instant;

use crate::{
    errors::ServerError,
    http::{
        handler::{RequestHandlerStrategy, Dispatcher},
        request::HttpRequest,
        response::{Response, OK},
    },
    utils::{
        math, 
        text, 
        hash, 
        file, 
        time,
        timeout::run_with_timeout,
        cpu::{
            is_prime::{self, PrimeMethod},
            factor::factorize,
            matrixmul::matrixmul,
        },
    },
    jobs::{
        manager::JobManager,
        job::JobStatus,
    },
    
};

pub struct SimpleHandler<F>(pub F);

impl<F> RequestHandlerStrategy for SimpleHandler<F>
where
    F: Fn(&HttpRequest) -> Result<Response, ServerError> + Send + Sync + 'static,
{
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        (self.0)(req)
    }
}


pub fn build_routes(job_manager: Arc<JobManager>) -> Dispatcher {
    let mut builder = Dispatcher::builder();

    // /fibonacci?num=N
    builder = builder.get("/fibonacci", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let n_str = req.query_param("num")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'num'".into()))?;

        let n = n_str
            .parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'num': {}", n_str)))?;

        if n > 93 {
            return Err(ServerError::BadRequest("Value too large — risk of overflow".into()));
        }

        let fib = math::fibonacci(n);

        let json_body = format!("{{\"num\": {}, \"fibonacci\": {}}}", n, fib);
        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json_body))
    })));

    // /toupper?text=abcd
    builder = builder.get("/toupper", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let text = req.query_param("text")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'text'".into()))?;

        if text.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'text' cannot be empty".into()));
        }

        let upper = text::to_upper(text);

        let json_body = format!(
            "{{\"original\": \"{}\", \"upper\": \"{}\"}}",
            text, upper
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json_body))
    })));

    // /reverse?text=abcdef
    builder = builder.get("/reverse", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let text = req.query_param("text")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'text'".into()))?;

        if text.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'text' cannot be empty".into()));
        }

        let reversed = text::reverse(text);
        let json = format!("{{\"original\": \"{}\", \"reversed\": \"{}\"}}", text, reversed);

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));


    // /hash?text=someinput
    builder = builder.get("/hash", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let text = req.query_param("text")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'text'".into()))?;

        if text.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'text' cannot be empty".into()));
        }

        let hash_val = hash::hash_text(text);
        let json = format!("{{\"text\": \"{}\", \"sha256\": \"{}\"}}", text, hash_val);

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));


    // /timestamp
    builder = builder.get("/timestamp", Arc::new(SimpleHandler(|_req: &HttpRequest| {
        let ts = time::timestamp();
        let json = format!("{{\"timestamp\": \"{}\"}}", ts);
        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));


    // /simulate?seconds=s&task=name
    builder = builder.get("/simulate", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let secs_str = req.query_param("seconds")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'seconds'".into()))?;

        let secs = secs_str
            .parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'seconds': {}", secs_str)))?;

        let task = req.query_param("task").unwrap_or("demo");
        let result = time::simulate(secs, task);

        let json = format!(
            "{{\"task\": \"{}\", \"duration_seconds\": {}, \"result\": \"{}\"}}",
            task, secs, result
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));

    // /createfile?name=filename&content=text&repeat=x
    builder = builder.get("/createfile", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
        if name.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'name' cannot be empty".into()));
        }

        let content = req.query_param("content").unwrap_or("Hello");
        if content.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'content' cannot be empty".into()));
        }

        let repeat_str = req.query_param("repeat").unwrap_or("1");
        let repeat = repeat_str
            .parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'repeat': {}", repeat_str)))?;
        if repeat == 0 {
            return Err(ServerError::BadRequest("Parameter 'repeat' must be greater than 0".into()));
        }

        file::create_file(name, content, repeat)?;

        let json = format!(
            "{{\"file\": \"{}\", \"content\": \"{}\", \"repeat\": {}}}",
            name, content, repeat
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));

    // /deletefile?name=filename
    builder = builder.get("/deletefile", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;

        if name.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'name' cannot be empty".into()));
        }

        match file::delete_file(name) {
            Ok(msg) => {
                let json = format!("{{\"status\": \"ok\", \"message\": \"{}\"}}", msg);
                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            Err(e) => {
                let json = format!(
                    "{{\"status\": \"error\", \"message\": \"Failed to delete '{}': {}\"}}",
                    name, e
                );
                Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /random?count=n&min=a&max=b
    builder = builder.get("/random", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let count_str = req.query_param("count").unwrap_or("5");
        let min_str = req.query_param("min").unwrap_or("0");
        let max_str = req.query_param("max").unwrap_or("100");

        let count = count_str
            .parse::<usize>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid 'count': {}", count_str)))?;
        let min = min_str
            .parse::<i32>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid 'min': {}", min_str)))?;
        let max = max_str
            .parse::<i32>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid 'max': {}", max_str)))?;

        if min > max {
            return Err(ServerError::BadRequest("'min' cannot be greater than 'max'".into()));
        }

        let nums = math::random(count, min, max);
        let json = format!("{{\"count\": {}, \"min\": {}, \"max\": {}, \"values\": {:?}}}", count, min, max, nums);

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));


    // /sleep?seconds=s
    builder = builder.get("/sleep", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let secs_str = req.query_param("seconds")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'seconds'".into()))?;

        let secs = secs_str
            .parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer for 'seconds': {}", secs_str)))?;

        time::sleep(secs);

        let json = format!("{{\"slept_seconds\": {}}}", secs);
        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));

    // /help
    builder = builder.get("/help", Arc::new(SimpleHandler(|_req: &HttpRequest| {
        let help_text = text::help();

        let json = format!(
            "{{\"endpoint\": \"/help\", \"description\": \"Available commands and usage information.\", \"details\": \"{}\"}}",
            help_text.replace('"', "'") 
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));

    // /status
    builder = builder.get("/status", Arc::new(SimpleHandler(|_req: &HttpRequest| {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let json = format!(
            "{{\"status\": \"running\", \"uptime\": {} , \"message\": \"Server running OK\"}}",
            now
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    })));

    builder = builder.get("/isprime", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let n_str = req.query_param("n")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'n'".into()))?;

            if n_str.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'n' cannot be empty".into()));
            }

            let n = n_str
                .parse::<u64>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'n': {}", n_str)))?;

            let method_env = env::var("PRIME_NUMBER_METHOD")
                .unwrap_or_else(|_| "MILLER_RABIN".to_string());

            let method = match method_env.trim().to_uppercase().as_str() {
                "TRIAL" | "SQRT" => is_prime::PrimeMethod::Trial,
                _ => is_prime::PrimeMethod::MillerRabin,
            };

            let method_name = match method {
                is_prime::PrimeMethod::Trial => "trial",
                is_prime::PrimeMethod::MillerRabin => "miller-rabin",
            };

            let timeout_ms = env::var("TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(500);

            if let Some((result, elapsed)) = run_with_timeout(timeout_ms, move || {
                is_prime::is_prime(n, method)
            }) {
                let json = format!(
                    "{{\"n\": {}, \"is_prime\": {}, \"method\": \"{}\", \"elapsed_ms\": {}}}",
                    n, result, method_name, elapsed
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("n".into(), n.to_string());
                params.insert("method".into(), method_name.to_string());

                let job_id = job_manager.submit("isprime", params, true);

                let json = format!(
                    "{{\"n\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    n, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /factor?n=NUM
    builder = builder.get("/factor", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let n_str = req.query_param("n")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'n'".into()))?;

            if n_str.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'n' cannot be empty".into()));
            }

            let n = n_str
                .parse::<u64>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'n': {}", n_str)))?;

            let timeout_ms = env::var("TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(500);

            let start = Instant::now();

            if let Some((factors, elapsed)) = run_with_timeout(timeout_ms, move || {
                factorize(n)
            }) {
                let factors_json: String = factors
                    .iter()
                    .map(|(p, c)| format!("[{},{}]", p, c))
                    .collect::<Vec<_>>()
                    .join(",");

                let json = format!(
                    "{{\"n\": {}, \"factors\": [{}], \"elapsed_ms\": {}}}",
                    n, factors_json, elapsed
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("n".into(), n.to_string());

                let job_id = job_manager.submit("factor", params, true);

                let json = format!(
                    "{{\"n\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    n, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /mandelbrot?width=W&height=H&max_iter=I
    builder = builder.get("/mandelbrot", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let width_str = req.query_param("width")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'width'".into()))?;
            let height_str = req.query_param("height")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'height'".into()))?;
            let iter_str = req.query_param("max_iter")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'max_iter'".into()))?;

            let width = width_str.parse::<usize>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'width': {}", width_str)))?;
            let height = height_str.parse::<usize>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'height': {}", height_str)))?;
            let max_iter = iter_str.parse::<u32>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'max_iter': {}", iter_str)))?;

            if width == 0 || height == 0 {
                return Err(ServerError::BadRequest("Width and height must be greater than 0".into()));
            }
            if max_iter == 0 {
                return Err(ServerError::BadRequest("max_iter must be greater than 0".into()));
            }

            let timeout_ms = std::env::var("TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(500);

            if let Some(result) = run_with_timeout(timeout_ms, move || {
                crate::utils::cpu::mandelbrot::mandelbrot(width, height, max_iter, None)
            }) {
                let ((map, mandelbrot_elapsed), _wrapper_elapsed) = result;

                let rows_json = map
                    .iter()
                    .map(|row| {
                        let row_str = row.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
                        format!("[{}]", row_str)
                    })
                    .collect::<Vec<_>>()
                    .join(",");

                let json = format!(
                    "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"elapsed_ms\": {}, \"map\": [{}]}}",
                    width, height, max_iter, mandelbrot_elapsed, rows_json
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("width".into(), width.to_string());
                params.insert("height".into(), height.to_string());
                params.insert("max_iter".into(), max_iter.to_string());

                let job_id = job_manager.submit("mandelbrot", params, true);

                let json = format!(
                    "{{\"width\": {}, \"height\": {}, \"max_iter\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    width, height, max_iter, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /matrixmul?size=N&seed=S
    builder = builder.get("/matrixmul", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            // ---- Validate parameters ----
            let size_str = req.query_param("size")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'size'".into()))?;
            let seed_str = req.query_param("seed").unwrap_or("123");

            if size_str.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'size' cannot be empty".into()));
            }

            // ---- Parse parameters ----
            let size = size_str
                .parse::<usize>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'size': {}", size_str)))?;
            let seed = seed_str
                .parse::<u64>()
                .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'seed': {}", seed_str)))?;

            // ---- Range check ----
            if size == 0 {
                return Err(ServerError::BadRequest("Parameter 'size' must be greater than 0".into()));
            }
            if size > 1000 {
                return Err(ServerError::BadRequest("Matrix size too large (max 1000)".into()));
            }

            // ---- Timeout configuration ----
            let timeout_ms = std::env::var("TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(5000);

            // ---- Run with timeout ----
            if let Some(result) = run_with_timeout(timeout_ms, move || {
                crate::utils::cpu::matrixmul::matrixmul(size, seed)
            }) {
                // ✅ Double destructure the nested tuples
                let ((hash, calc_elapsed), timeout_elapsed) = result;

                let json = format!(
                    "{{\"size\": {}, \"seed\": {}, \"result_sha256\": \"{}\", \"elapsed_ms\": {}, \"total_elapsed_ms\": {}}}",
                    size, seed, hash, calc_elapsed, timeout_elapsed
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            } else {
                // ---- Timeout → background job ----
                let mut params = std::collections::HashMap::new();
                params.insert("size".into(), size.to_string());
                params.insert("seed".into(), seed.to_string());

                let job_id = job_manager.submit("matrixmul", params, true);

                let json = format!(
                    "{{\"size\": {}, \"seed\": {}, \"status\": \"queued\", \"timeout_ms\": {}, \"job_id\": \"{}\"}}",
                    size, seed, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));


    builder.build()
}


trait QueryParam {
    fn query_param(&self, key: &str) -> Option<&str>;
}

impl QueryParam for HttpRequest {
    fn query_param(&self, key: &str) -> Option<&str> {
        if self.query.is_empty() {
            return None;
        }
        for pair in self.query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }
}
