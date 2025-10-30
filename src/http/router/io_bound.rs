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

use crate::utils::{
    io::{
        sort_file::sort_file,
        word_count::word_count,
        grep::grep_file,
        hash_file::hash_file,
        compress::compress_file
    },
};

use crate::worker_pool::ThreadPool;

/// /sortfile?name=FILE&algo=merge|quick
pub struct SortFileHandler {
    pub job_manager: Arc<JobManager>,
}

fn queue_full_response(retry_after_ms: u64) -> Response {
    let retry_after_secs = ((retry_after_ms + 999) / 1000).max(1);
    Response::new(SERVICE_UNAVAILABLE)
        .set_header("Content-Type", "application/json")
        .set_header("Retry-After", &retry_after_secs.to_string())
        .with_body(format!("{{\"retry_after_ms\":{}}}", retry_after_ms))
}

impl RequestHandlerStrategy for SortFileHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
        let algo = req.query_param("algo").unwrap_or("merge").to_string();

        if !["merge", "quick"].contains(&algo.as_str()) {
            return Err(ServerError::BadRequest(format!("Invalid algorithm '{}'", algo)));
        }

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT").ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10_000);

        let mut params = HashMap::new();
        params.insert("name".into(), name.to_string());
        params.insert("algo".into(), algo.clone());

        let file_name = name.to_string();
        let algo_for_exec = algo.clone();

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "sortfile",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || match sort_file(&file_name, &algo_for_exec) {
                Ok((out_path, count, elapsed)) => {
                    let sorted = out_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Ok(format!(
                        "{{\"file\":\"{}\",\"algo\":\"{}\",\"sorted_file\":\"{}\",\"count\":{},\"elapsed_ms\":{}}}",
                        file_name, algo_for_exec, sorted, count, elapsed
                    ))
                }
                Err(e) => Err(format!("Sort failed: {}", e)),
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
            BestEffortOutcome::Completed(json) => Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json)),
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"file\":\"{}\",\"algo\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name, algo, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

/// /wordcount?name=FILE
pub struct WordCountHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for WordCountHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT").ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10_000);

        let mut params = HashMap::new();
        params.insert("name".into(), name.to_string());

        let file_name = name.to_string();

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "wordcount",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let total_start = Instant::now();
                match word_count(&file_name) {
                    Ok((counts, elapsed, path)) => {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let total_elapsed = total_start.elapsed().as_millis();
                        Ok(format!(
                            "{{\"file\":\"{}\",\"lines\":{},\"words\":{},\"bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                            filename, counts.lines, counts.words, counts.bytes, elapsed, total_elapsed
                        ))
                    }
                    Err(e) => Err(format!("Wordcount failed: {}", e)),
                }
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
            BestEffortOutcome::Completed(json) => Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json)),
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"file\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

/// /grep?name=FILE&pattern=REGEX
pub struct GrepHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for GrepHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
        let pattern = req.query_param("pattern")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'pattern'".into()))?;

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT").ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10_000);

        let mut params = HashMap::new();
        params.insert("name".into(), name.to_string());
        params.insert("pattern".into(), pattern.to_string());

        let file_name = name.to_string();
        let pattern_value = pattern.to_string();

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "grep",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || match grep_file(&file_name, &pattern_value) {
                Ok(res) => {
                    let lines_json = res
                        .matched_lines
                        .iter()
                        .map(|l| format!("\"{}\"", l.replace('"', "\\\"")))
                        .collect::<Vec<_>>()
                        .join(",");
                    Ok(format!(
                        "{{\"file\":\"{}\",\"pattern\":\"{}\",\"matches\":{},\"lines\":[{}],\"elapsed_ms\":{}}}",
                        file_name, pattern_value, res.total_matches, lines_json, res.elapsed_ms
                    ))
                }
                Err(e) => Err(format!("Grep failed: {}", e)),
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
            BestEffortOutcome::Completed(json) => Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json)),
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"file\":\"{}\",\"pattern\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name, pattern, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

/// /compress?name=FILE&codec=gzip|xz
pub struct CompressHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for CompressHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
        let codec = req.query_param("codec")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'codec'".into()))?;

        if !["gzip", "xz"].contains(&codec) {
            return Err(ServerError::BadRequest(format!("Invalid codec '{}'", codec)));
        }

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT").ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(20_000);

        let mut params = HashMap::new();
        params.insert("name".into(), name.to_string());
        params.insert("codec".into(), codec.to_string());

        let file_name = name.to_string();
        let codec_value = codec.to_string();

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "compress",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let total_start = Instant::now();
                match compress_file(&file_name, &codec_value) {
                    Ok(res) => {
                        let out_name = res
                            .output_file
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let total_elapsed = total_start.elapsed().as_millis();
                        Ok(format!(
                            "{{\"file\":\"{}\",\"codec\":\"{}\",\"output\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                            file_name, codec_value, out_name, res.compressed_size, res.elapsed_ms, total_elapsed
                        ))
                    }
                    Err(e) => Err(format!("Compression failed: {}", e)),
                }
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
            BestEffortOutcome::Completed(json) => Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json)),
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"file\":\"{}\",\"codec\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name, codec, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

/// /hashfile?name=FILE&algo=sha256
pub struct HashFileHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for HashFileHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let name = req.query_param("name")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
        let algo = req.query_param("algo").unwrap_or("sha256");

        if algo.to_lowercase() != "sha256" {
            return Err(ServerError::BadRequest(format!("Unsupported algorithm '{}'", algo)));
        }

        let timeout_ms = env::var("BEST_EFFORT_TIMEOUT").ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10_000);

        let mut params = HashMap::new();
        params.insert("name".into(), name.to_string());
        params.insert("algo".into(), algo.to_string());

        let file_name = name.to_string();
        let algo_value = algo.to_string();

        let outcome = best_effort::execute(
            self.job_manager.clone(),
            "hashfile",
            params,
            Priority::Normal,
            Duration::from_millis(timeout_ms),
            move || {
                let total_start = Instant::now();
                match hash_file(&file_name, &algo_value) {
                    Ok(res) => {
                        let total_elapsed = total_start.elapsed().as_millis();
                        Ok(format!(
                            "{{\"file\":\"{}\",\"algorithm\":\"{}\",\"hash\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                            file_name, algo_value, res.hash_hex, res.file_size, res.elapsed_ms, total_elapsed
                        ))
                    }
                    Err(e) => Err(format!("Hashing failed: {}", e)),
                }
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
            BestEffortOutcome::Completed(json) => Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json)),
            BestEffortOutcome::Offloaded { job_id } => {
                let json = format!(
                    "{{\"file\":\"{}\",\"algo\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name, algo, timeout_ms, job_id
                );
                Ok(Response::new(ACCEPTED)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

pub fn register(builder: DispatcherBuilder, job_manager: Arc<JobManager>) -> DispatcherBuilder {
    let sort_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(SortFileHandler { job_manager: job_manager.clone() });
    let wordcount_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(WordCountHandler { job_manager: job_manager.clone() });
    let grep_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(GrepHandler { job_manager: job_manager.clone() });
    let compress_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(CompressHandler { job_manager: job_manager.clone() });
    let hash_handler: Arc<dyn RequestHandlerStrategy> =
        Arc::new(HashFileHandler { job_manager: job_manager.clone() });

    let sort = Arc::new(PooledHandler::new(
        ThreadPool::from_env("sortfile", "WORKERS_SORTFILE", 2),
        sort_handler,
    ));
    let wordcount = Arc::new(PooledHandler::new(
        ThreadPool::from_env("wordcount", "WORKERS_WORDCOUNT", 2),
        wordcount_handler,
    ));
    let grep = Arc::new(PooledHandler::new(
        ThreadPool::from_env("grep", "WORKERS_GREP", 2),
        grep_handler,
    ));
    let compress = Arc::new(PooledHandler::new(
        ThreadPool::from_env("compress", "WORKERS_COMPRESS", 2),
        compress_handler,
    ));
    let hashfile = Arc::new(PooledHandler::new(
        ThreadPool::from_env("hashfile", "WORKERS_HASHFILE", 2),
        hash_handler,
    ));

    builder
        .get("/sortfile", sort)
        .get("/wordcount", wordcount)
        .get("/grep", grep)
        .get("/compress", compress)
        .get("/hashfile", hashfile)
}
