use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use crate::http::handler::{RequestHandlerStrategy, DispatcherBuilder};
use crate::http::router::router::{SimpleHandler, QueryParam};
use crate::http::request::HttpRequest;
use crate::http::response::{Response, OK};
use crate::http::errors::ServerError;
use crate::jobs::job::Priority;
use crate::jobs::manager::JobManager;

use crate::utils::io::{
    sort_file::sort_file,
    word_count::word_count,
    grep::grep_file,
    hash_file::hash_file,
    compress::compress_file
};
use crate::utils::timeout::run_with_timeout;

/// /sortfile?name=FILE&algo=merge|quick
pub struct SortFileHandler {
    pub job_manager: Arc<JobManager>,
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

        let name_clone = name.to_string();
        let algo_clone = algo.clone();

        if let Some((result, _total_elapsed)) = run_with_timeout(timeout_ms, move || sort_file(&name_clone, &algo_clone)) {
            match result {
                Ok((out_path, count, elapsed)) => {
                    let file = out_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                    let json = format!(
                        "{{\"file\":\"{}\",\"algo\":\"{}\",\"sorted_file\":\"{}\",\"count\":{},\"elapsed_ms\":{}}}",
                        name, algo, file, count, elapsed
                    );
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) => Err(ServerError::Internal(format!("Sort failed: {}", e))),
            }
        } else {
            let mut params = HashMap::new();
            params.insert("name".into(), name.to_string());
            params.insert("algo".into(), algo.clone());

            let job_id = self.job_manager.submit("sortfile", params, false, Priority::Normal)
                .map_err(|e| ServerError::Internal(format!("Job submit failed: {}", e)))?;

            let json = format!(
                "{{\"file\":\"{}\",\"algo\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                name, algo, timeout_ms, job_id
            );
            Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json))
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

        let name_clone = name.to_string();

        if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || word_count(&name_clone)) {
            match result {
                Ok((counts, elapsed, path)) => {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                    let json = format!(
                        "{{\"file\":\"{}\",\"lines\":{},\"words\":{},\"bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                        filename, counts.lines, counts.words, counts.bytes, elapsed, total_elapsed
                    );
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) => Err(ServerError::Internal(format!("Wordcount failed: {}", e))),
            }
        } else {
            let mut params = HashMap::new();
            params.insert("name".into(), name.clone().to_string());

            let job_id = self.job_manager.submit("wordcount", params, true, Priority::Normal)
                .map_err(|e| ServerError::Internal(format!("Job submit failed: {}", e)))?;

            let json = format!(
                "{{\"file\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                name, timeout_ms, job_id
            );
            Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json))
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

        let name_clone = name.to_string();
        let pattern_clone = pattern.to_string();

        if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || grep_file(&name_clone, &pattern_clone)) {
            match result {
                Ok(res) => {
                    let lines_json = res.matched_lines
                        .iter()
                        .map(|l| format!("\"{}\"", l.replace('"', "\\\"")))
                        .collect::<Vec<_>>()
                        .join(",");
                    let json = format!(
                        "{{\"file\":\"{}\",\"pattern\":\"{}\",\"matches\":{},\"lines\":[{}],\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                        name, pattern, res.total_matches, lines_json, res.elapsed_ms, total_elapsed
                    );
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) => Err(ServerError::Internal(format!("Grep failed: {}", e))),
            }
        } else {
            let mut params = HashMap::new();
            params.insert("name".into(), name.to_string());
            params.insert("pattern".into(), pattern.to_string());
            let job_id = self.job_manager.submit("grep", params, true, Priority::Normal)
                .map_err(|e| ServerError::Internal(format!("Job submit failed: {}", e)))?;

            let json = format!(
                "{{\"file\":\"{}\",\"pattern\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                name, pattern, timeout_ms, job_id
            );
            Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json))
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

        let name_clone = name.to_string();
        let codec_clone = codec.to_string();

        if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || compress_file(&name_clone, &codec_clone)) {
            match result {
                Ok(res) => {
                    let out_name = res.output_file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                    let json = format!(
                        "{{\"file\":\"{}\",\"codec\":\"{}\",\"output\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                        name, codec, out_name, res.compressed_size, res.elapsed_ms, total_elapsed
                    );
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) => Err(ServerError::Internal(format!("Compression failed: {}", e))),
            }
        } else {
            let mut params = HashMap::new();
            params.insert("name".into(), name.to_string());
            params.insert("codec".into(), codec.to_string());
            let job_id = self.job_manager.submit("compress", params, true, Priority::Normal)
                .map_err(|e| ServerError::Internal(format!("Job submit failed: {}", e)))?;

            let json = format!(
                "{{\"file\":\"{}\",\"codec\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                name, codec, timeout_ms, job_id
            );
            Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json))
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

        let name_clone = name.to_string();
        let algo_clone = algo.to_string();

        if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || hash_file(&name_clone, &algo_clone)) {
            match result {
                Ok(res) => {
                    let json = format!(
                        "{{\"file\":\"{}\",\"algorithm\":\"{}\",\"hash\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                        name, algo, res.hash_hex, res.file_size, res.elapsed_ms, total_elapsed
                    );
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) => Err(ServerError::Internal(format!("Hashing failed: {}", e))),
            }
        } else {
            let mut params = HashMap::new();
            params.insert("name".into(), name.to_string());
            params.insert("algo".into(), algo.to_string());
            let job_id = self.job_manager.submit("hashfile", params, true, Priority::Normal)
                .map_err(|e| ServerError::Internal(format!("Job submit failed: {}", e)))?;

            let json = format!(
                "{{\"file\":\"{}\",\"algo\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                name, algo, timeout_ms, job_id
            );
            Ok(Response::new(OK)
                .set_header("Content-Type", "application/json")
                .with_body(json))
        }
    }
}

pub fn register(builder: DispatcherBuilder, job_manager: Arc<JobManager>) -> DispatcherBuilder {
    builder
        .get("/sortfile", Arc::new(SortFileHandler { job_manager: job_manager.clone() }))
        .get("/wordcount", Arc::new(WordCountHandler { job_manager: job_manager.clone() }))
        .get("/grep", Arc::new(GrepHandler { job_manager: job_manager.clone() }))
        .get("/compress", Arc::new(CompressHandler { job_manager: job_manager.clone() }))
        .get("/hashfile", Arc::new(HashFileHandler { job_manager }))
}
