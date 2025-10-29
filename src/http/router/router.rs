use std::env;
use std::sync::Arc;
use std::time::SystemTime;

use crate::{
    http::{
        errors::ServerError,
        handler::{RequestHandlerStrategy, Dispatcher},
        request::HttpRequest,
        response::{Response, OK},
        router::{jobs, cpu_bound}
    },
    jobs::job::Priority,
    utils::{
        math, 
        text, 
        hash, 
        file, 
        time,
        timeout::run_with_timeout,
        io::{
            sort_file::sort_file,
            word_count::word_count,
            grep::grep_file,
            hash_file::hash_file,
            compress::compress_file
        }
    },
    jobs::{
        manager::JobManager,
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

    // /sortfile?name=FILE&algo=merge|quick
    builder = builder.clone().get("/sortfile", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let name = req
                .query_param("name")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?
                .to_string();

            let algo = req.query_param("algo").unwrap_or("merge").to_string();

            if !["merge", "quick"].contains(&algo.as_str()) {
                return Err(ServerError::BadRequest(format!(
                    "Invalid algorithm '{}'. Must be 'merge' or 'quick'.", algo
                )));
            }

            let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10_000);

            // 🧠 Try immediate execution (best effort)
            let name_clone = name.clone();
            let algo_clone = algo.clone();
            if let Some((result, _total_elapsed)) = run_with_timeout(timeout_ms, move || sort_file(&name_clone, &algo_clone)) {
                match result {
                    Ok((out_path, count, sort_elapsed)) => {
                        let sorted_name = out_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                        let json = format!(
                            "{{\"file\":\"{}\",\"algo\":\"{}\",\"sorted_file\":\"{}\",\"count\":{},\"elapsed_ms\":{}}}",
                            name, algo, sorted_name, count, sort_elapsed
                        );
                        return Ok(Response::new(OK)
                            .set_header("Content-Type", "application/json")
                            .with_body(json));
                    }
                    Err(e) => {
                        let json = format!(
                            "{{\"file\":\"{}\",\"algo\":\"{}\",\"error\":\"{}\"}}",
                            name, algo, e
                        );
                        return Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                            .set_header("Content-Type", "application/json")
                            .with_body(json));
                    }
                }
            }

            // ⏳ Otherwise, enqueue as IO job
            let mut params = std::collections::HashMap::new();
            params.insert("name".into(), name.clone());
            params.insert("algo".into(), algo.clone());

            match job_manager.submit("sortfile", params, false, Priority::Normal) {
                Ok(job_id) => {
                    let json = format!(
                        "{{\"file\":\"{}\",\"algo\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                        name, algo, timeout_ms, job_id
                    );
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) if e == "QueueFull" => {
                    let retry_after = 5000; // ms
                    let json = format!(
                        "{{\"error\":\"queue full\",\"retry_after_ms\":{}}}",
                        retry_after
                    );
                    Ok(Response::new(crate::http::response::SERVICE_UNAVAILABLE)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                Err(e) => Err(ServerError::Internal(format!("Job submission failed: {}", e))),
            }
        }
    })));

    // /wordcount?name=FILE
    builder = builder.get("/wordcount", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let name = req.query_param("name")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;

            if name.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'name' cannot be empty".into()));
            }

            let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10_000);

            let name = name.to_string();
            let name_for_timeout = name.clone();
            if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || {
                word_count(&name_for_timeout)
            }) {
                match result {
                    Ok((counts, elapsed, path)) => {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");

                        let json = format!(
                            "{{\"file\":\"{}\",\"lines\":{},\"words\":{},\"bytes\":{},\"elapsed_ms\": {}, \"total_elapsed_ms\": {}}}",
                            filename, counts.lines, counts.words, counts.bytes, elapsed, total_elapsed
                        );

                        Ok(Response::new(OK)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                    Err(e) => {
                        let json = format!(
                            "{{\"file\":\"{}\",\"error\":\"{}\"}}",
                            name, e
                        );
                        Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                }
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("name".into(), name.clone().to_string());

                let job_id = job_manager.submit("wordcount", params, true, Priority::Normal)
                    .map_err(|e| ServerError::Internal(format!("Failed to submit job: {}", e)))?;

                let json = format!(
                    "{{\"file\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /grep?name=FILE&pattern=REGEX
    builder = builder.get("/grep", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let name = req.query_param("name")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
            let pattern = req.query_param("pattern")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'pattern'".into()))?;

            if name.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'name' cannot be empty".into()));
            }
            if pattern.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'pattern' cannot be empty".into()));
            }

            let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10_000);

            let name_str = name.to_string();
            let pattern_str = pattern.to_string();
            let name_for_timeout = name_str.clone();
            let pattern_for_timeout = pattern_str.clone();

            if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || {
                grep_file(&name_for_timeout, &pattern_for_timeout)
            }) {
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
                    Err(e) => {
                        let json = format!(
                            "{{\"file\":\"{}\",\"error\":\"{}\"}}",
                            name, e
                        );
                        Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                }
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("name".into(), name.to_string());
                params.insert("pattern".into(), pattern.to_string());

                let job_id = job_manager.submit("grep", params, true, Priority::Normal)
                    .map_err(|e| ServerError::Internal(format!("Failed to submit job: {}", e)))?;

                let json = format!(
                    "{{\"file\":\"{}\",\"pattern\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name_str, pattern_str, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /compress?name=FILE&codec=gzip|xz
    builder = builder.get("/compress", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            // ---- Parse and validate query params ----
            let name = req.query_param("name")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
            let codec = req.query_param("codec")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'codec'".into()))?;

            if name.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'name' cannot be empty".into()));
            }

            if !["gzip", "xz"].contains(&codec) {
                return Err(ServerError::BadRequest(format!(
                    "Invalid codec '{}'. Must be 'gzip' or 'xz'.",
                    codec
                )));
            }

            let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(20_000);

            let name_str = name.to_string();
            let codec_str = codec.to_string();
            let name_for_timeout = name_str.clone();
            let codec_for_timeout = codec_str.clone();

            if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || {
                compress_file(&name_for_timeout, &codec_for_timeout)
            }) {
                match result {
                    Ok(res) => {
                        let out_name = res.output_file.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");

                        let json = format!(
                            "{{\"file\":\"{}\",\"codec\":\"{}\",\"output\":\"{}\",\"size_bytes\":{},\"elapsed_ms\":{},\"total_elapsed_ms\":{}}}",
                            name, codec, out_name, res.compressed_size, res.elapsed_ms, total_elapsed
                        );

                        Ok(Response::new(OK)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                    Err(e) => {
                        let json = format!(
                            "{{\"file\":\"{}\",\"error\":\"{}\"}}",
                            name, e
                        );
                        Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                }
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("name".into(), name.to_string());
                params.insert("codec".into(), codec.to_string());

                let job_id = job_manager.submit("compress", params, true, Priority::Normal)
                    .map_err(|e| ServerError::Internal(format!("Failed to submit job: {}", e)))?;

                let json = format!(
                    "{{\"file\":\"{}\",\"codec\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name_str, codec_str, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // /hashfile?name=FILE&algo=sha256
    builder = builder.get("/hashfile", Arc::new(SimpleHandler({
        let job_manager = job_manager.clone();
        move |req: &HttpRequest| {
            let name = req.query_param("name")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'name'".into()))?;
            let algo = req.query_param("algo")
                .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'algo'".into()))?;

            if name.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'name' cannot be empty".into()));
            }
            if algo.trim().is_empty() {
                return Err(ServerError::BadRequest("Parameter 'algo' cannot be empty".into()));
            }

            if algo.to_lowercase() != "sha256" {
                return Err(ServerError::BadRequest(format!(
                    "Unsupported algorithm '{}'. Only 'sha256' is supported.",
                    algo
                )));
            }

            let timeout_ms = env::var("BEST_EFFORT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10_000);

            let name_str = name.to_string();
            let algo_str = algo.to_string();
            let name_for_timeout = name_str.clone();
            let algo_for_timeout = algo_str.clone();

            if let Some((result, total_elapsed)) = run_with_timeout(timeout_ms, move || {
                hash_file(&name_for_timeout, &algo_for_timeout)
            }) {
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
                    Err(e) => {
                        let json = format!(
                            "{{\"file\":\"{}\",\"error\":\"{}\"}}",
                            name, e
                        );
                        Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                }
            } else {
                let mut params = std::collections::HashMap::new();
                params.insert("name".into(), name.to_string());
                params.insert("algo".into(), algo.to_string());

                let job_id = job_manager.submit("hashfile", params, true, Priority::Normal)
                    .map_err(|e| ServerError::Internal(format!("Failed to submit job: {}", e)))?;

                let json = format!(
                    "{{\"file\":\"{}\",\"algo\":\"{}\",\"status\":\"queued\",\"timeout_ms\":{},\"job_id\":\"{}\"}}",
                    name_str, algo_str, timeout_ms, job_id
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    })));

    // Routes from other modules
    builder = jobs::register(builder, job_manager.clone());
    builder = cpu_bound::register(builder, job_manager.clone());
    builder.build()
}


pub trait QueryParam {
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
