use std::sync::Arc;
use std::time::SystemTime;

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    request::HttpRequest,
    response::{Response, OK},
    errors::ServerError,
    router::router::{PooledHandler, SimpleHandler, QueryParam},
};

use crate::utils::{math, text, hash, file, time};
use crate::worker_pool::ThreadPool;

// /fibonacci?num=N
fn fibonacci_handler(req: &HttpRequest) -> Result<Response, ServerError> {
    let n_str = req.query_param("num")
        .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'num'".into()))?;
    let n = n_str
        .parse::<u64>()
        .map_err(|_| ServerError::BadRequest(format!("Invalid integer for 'num': {}", n_str)))?;

    if n > 93 {
        return Err(ServerError::BadRequest("Value too large — risk of overflow".into()));
    }

    let fib = math::fibonacci(n);
    let json = format!("{{\"num\": {}, \"fibonacci\": {}}}", n, fib);
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /toupper?text=abcd
fn toupper_handler(req: &HttpRequest) -> Result<Response, ServerError> {
    let text = req.query_param("text")
        .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'text'".into()))?;

    if text.trim().is_empty() {
        return Err(ServerError::BadRequest("Parameter 'text' cannot be empty".into()));
    }

    let upper = text::to_upper(text);
    let json = format!("{{\"original\": \"{}\", \"upper\": \"{}\"}}", text, upper);
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /reverse?text=abcdef
fn reverse_handler(req: &HttpRequest) -> Result<Response, ServerError> {
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
}

// /hash?text=someinput
fn hash_handler(req: &HttpRequest) -> Result<Response, ServerError> {
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
}

// /timestamp
fn timestamp_handler(_req: &HttpRequest) -> Result<Response, ServerError> {
    let ts = time::timestamp();
    let json = format!("{{\"timestamp\": \"{}\"}}", ts);
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /simulate?seconds=s&task=name
fn simulate_handler(req: &HttpRequest) -> Result<Response, ServerError> {
    let secs_str = req.query_param("seconds")
        .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'seconds'".into()))?;
    let secs = secs_str
        .parse::<u64>()
        .map_err(|_| ServerError::BadRequest(format!("Invalid integer for 'seconds': {}", secs_str)))?;

    let task = req.query_param("task").unwrap_or("demo");
    let result = time::simulate(secs, task);

    let json = format!(
        "{{\"task\": \"{}\", \"duration_seconds\": {}, \"result\": \"{}\"}}",
        task, secs, result
    );
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /createfile?name=filename&content=text&repeat=x
fn createfile_handler(req: &HttpRequest) -> Result<Response, ServerError> {
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
        .map_err(|_| ServerError::BadRequest(format!("Invalid integer for 'repeat': {}", repeat_str)))?;
    if repeat == 0 {
        return Err(ServerError::BadRequest("Parameter 'repeat' must be greater than 0".into()));
    }

    file::create_file(name, content, repeat)?;
    let json = format!("{{\"file\": \"{}\", \"content\": \"{}\", \"repeat\": {}}}", name, content, repeat);
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /deletefile?name=filename
fn deletefile_handler(req: &HttpRequest) -> Result<Response, ServerError> {
    let name = req.query_param("name")
        .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;

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
}

// /random?count=n&min=a&max=b
fn random_handler(req: &HttpRequest) -> Result<Response, ServerError> {
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
    let json = format!(
        "{{\"count\": {}, \"min\": {}, \"max\": {}, \"values\": {:?}}}",
        count, min, max, nums
    );
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /sleep?seconds=s
fn sleep_handler(req: &HttpRequest) -> Result<Response, ServerError> {
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
}

// /help
fn help_handler(_req: &HttpRequest) -> Result<Response, ServerError> {
    let help_text = text::help();
    let json = format!(
        "{{\"endpoint\": \"/help\", \"description\": \"Available commands and usage information.\", \"details\": \"{}\"}}",
        help_text.replace('"', "'")
    );
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

// /status
fn status_handler(_req: &HttpRequest) -> Result<Response, ServerError> {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
    let json = format!(
        "{{\"status\": \"running\", \"uptime\": {}, \"message\": \"Server running OK\"}}",
        now
    );
    Ok(Response::new(OK)
        .set_header("Content-Type", "application/json")
        .with_body(json))
}

pub fn register(builder: DispatcherBuilder) -> DispatcherBuilder {
    let make = |name: &'static str,
                env_var: &'static str,
                default: usize,
                handler_fn: fn(&HttpRequest) -> Result<Response, ServerError>|
     -> Arc<dyn RequestHandlerStrategy> {
        let inner: Arc<dyn RequestHandlerStrategy> = Arc::new(SimpleHandler(handler_fn));
        let pooled = PooledHandler::new(ThreadPool::from_env(name, env_var, default), inner);
        Arc::new(pooled)
    };

    builder
        .get("/fibonacci", make("fibonacci", "WORKERS_FIBONACCI", 2, fibonacci_handler))
        .get("/toupper", make("toupper", "WORKERS_TOUPPER", 2, toupper_handler))
        .get("/reverse", make("reverse", "WORKERS_REVERSE", 2, reverse_handler))
        .get("/hash", make("hash", "WORKERS_HASH", 2, hash_handler))
        .get("/timestamp", make("timestamp", "WORKERS_TIMESTAMP", 1, timestamp_handler))
        .get("/simulate", make("simulate", "WORKERS_SIMULATE", 2, simulate_handler))
        .get("/createfile", make("createfile", "WORKERS_CREATEFILE", 2, createfile_handler))
        .get("/deletefile", make("deletefile", "WORKERS_DELETEFILE", 2, deletefile_handler))
        .get("/random", make("random", "WORKERS_RANDOM", 2, random_handler))
        .get("/sleep", make("sleep", "WORKERS_SLEEP", 2, sleep_handler))
        .get("/help", make("help", "WORKERS_HELP", 1, help_handler))
        .get("/status", make("status", "WORKERS_STATUS", 1, status_handler))
}
