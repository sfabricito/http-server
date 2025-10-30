use std::sync::Arc;
use std::collections::HashMap;

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    request::HttpRequest,
    response::{Response, OK, SERVICE_UNAVAILABLE},
    errors::ServerError,
    router::router::QueryParam,
    
};
use crate::jobs::manager::{JobError, JobManager};
use crate::jobs::job::{JobStatus, Priority};
use crate::worker_pool;

pub struct JobResultHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for JobResultHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let id = req.query_param("id")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'id'".into()))?;

        if id.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'id' cannot be empty".into()));
        }

        match self.job_manager.status(id) {
            Some(status) => match status {
                JobStatus::Done => {
                    if let Some(output) = self.job_manager.result(id) {
                        let json = format!("{{\"id\":\"{}\",\"output\":{}}}", id, output);
                        Ok(Response::new(OK)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    } else {
                        let json = format!("{{\"id\":\"{}\",\"error\":\"Job finished but no output available\"}}", id);
                        Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                            .set_header("Content-Type", "application/json")
                            .with_body(json))
                    }
                }
                JobStatus::Error(err_msg) => {
                    let json = format!("{{\"id\":\"{}\",\"error\":\"{}\"}}", id, err_msg);
                    Ok(Response::new(crate::http::response::INTERNAL_SERVER_ERROR)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
                other => {
                    let status_str = format!("{:?}", other);
                    let json = format!("{{\"id\":\"{}\",\"status\":\"{}\"}}", id, status_str);
                    Ok(Response::new(OK)
                        .set_header("Content-Type", "application/json")
                        .with_body(json))
                }
            },
            None => {
                let json = format!("{{\"id\":\"{}\",\"error\":\"Job not found\"}}", id);
                Ok(Response::new(crate::http::response::NOT_FOUND)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

pub struct JobStatusHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for JobStatusHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let id = req.query_param("id")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'id'".into()))?;

        if id.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'id' cannot be empty".into()));
        }

        match self.job_manager.status(id) {
            Some(status) => {
                let (progress, eta) = match status {
                    JobStatus::Queued => (0, "unknown"),
                    JobStatus::Running => (50, "estimating"),
                    JobStatus::Done => (100, "0s"),
                    JobStatus::Error(_) => (100, "n/a"),
                    JobStatus::Canceled => (0, "n/a"),
                    JobStatus::Timeout => (100, "n/a"),
                };

                let status_str = match &status {
                    JobStatus::Queued => "queued",
                    JobStatus::Running => "running",
                    JobStatus::Done => "done",
                    JobStatus::Error(_) => "error",
                    JobStatus::Canceled => "canceled",
                    JobStatus::Timeout => "timeout",
                };

                let json = format!(
                    "{{\"id\":\"{}\",\"status\":\"{}\",\"progress\":{},\"eta\":\"{}\"}}",
                    id, status_str, progress, eta
                );

                Ok(Response::new(OK)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
            None => {
                let json = format!("{{\"id\":\"{}\",\"error\":\"Job not found\"}}", id);
                Ok(Response::new(crate::http::response::NOT_FOUND)
                    .set_header("Content-Type", "application/json")
                    .with_body(json))
            }
        }
    }
}

pub struct JobSubmitHandler {
    pub job_manager: Arc<JobManager>,
}

fn queue_full_response(retry_after_ms: u64) -> Response {
    let retry_after_secs = ((retry_after_ms + 999) / 1000).max(1);
    Response::new(SERVICE_UNAVAILABLE)
        .set_header("Content-Type", "application/json")
        .set_header("Retry-After", &retry_after_secs.to_string())
        .with_body(format!("{{\"retry_after_ms\":{}}}", retry_after_ms))
}

impl RequestHandlerStrategy for JobSubmitHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let task = req.query_param("task")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'task'".into()))?;

        if task.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'task' cannot be empty".into()));
        }

        let priority_str = req.query_param("priority").unwrap_or("normal");
        let priority = match priority_str.to_lowercase().as_str() {
            "low" => Priority::Low,
            "high" => Priority::High,
            "normal" | _ => Priority::Normal,
        };

        let mut params = HashMap::new();
        for pair in req.query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k != "task" && k != "priority" {
                    params.insert(k.to_string(), v.to_string());
                }
            }
        }

        let job_id = match self.job_manager.submit(task, params, priority) {
            Ok(id) => id,
            Err(JobError::QueueFull { retry_after_ms }) => {
                return Ok(queue_full_response(retry_after_ms));
            }
            Err(JobError::Unknown(err)) => {
                return Err(ServerError::Internal(format!("Failed to submit job: {}", err)));
            }
        };

        let json = format!(
            "{{\"job_id\":\"{}\",\"status\":\"queued\",\"priority\":\"{}\"}}",
            job_id, priority_str
        );

        println!("Job submitted: id='{}', task='{}' ", job_id, task);

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    }
}

pub struct JobCancelHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for JobCancelHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let id = req.query_param("id")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'id'".into()))?;

        if id.trim().is_empty() {
            return Err(ServerError::BadRequest("Parameter 'id' cannot be empty".into()));
        }

        let canceled = self.job_manager.cancel(id);

        let status_str = if canceled { "canceled".to_string() } else { "not_cancelable".to_string() };

        let json = format!(
            "{{\"id\":\"{}\",\"status\":\"{}\"}}",
            id, status_str
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    }
}

pub struct JobMetricsHandler {
    pub job_manager: Arc<JobManager>,
}


impl RequestHandlerStrategy for JobMetricsHandler {
    fn handle(&self, _req: &HttpRequest) -> Result<Response, ServerError> {
        let pools = self.job_manager.get_metrics();
        let endpoint_pools = worker_pool::all_endpoint_metrics();

        let mut system_json = Vec::new(); 
        let mut workers_json = Vec::new(); 

        let mut total_workers = 0;
        let mut busy_workers = 0;

        let ordered_names = ["cpu", "io"];
        for &name in &ordered_names {
            if let Some(metrics) = pools.get(name) {
                let queue = metrics.queue_lengths;
                let wm = &metrics.worker_metrics;

                let active = *wm.active_workers.lock().unwrap();
                let total = wm.total_workers;
                total_workers += total;
                busy_workers += active;

                let snapshot = wm.snapshot();
                let avg_wait_ms = snapshot.avg_wait_ms.round() as u64;
                let avg_exec_ms = snapshot.avg_exec_ms.round() as u64;
                let avg_total_ms = (snapshot.avg_wait_ms + snapshot.avg_exec_ms).round() as u64;

                system_json.push(format!(
                    r#""{}":{{
                        "queue_size": {{"high": {}, "normal": {}, "low": {}}},
                        "workers": {{"active": {}, "total": {}}},
                        "jobs": {{"samples": {}}}, 
                        "timings": {{
                            "avg_wait_ms": {},
                            "avg_exec_ms": {},
                            "avg_total_ms": {},
                            "std_dev_wait_ms": {:.2},
                            "std_dev_exec_ms": {:.2}
                        }}
                    }}"#,
                    name,
                    queue.0, queue.1, queue.2,
                    active, total,
                    snapshot.samples,
                    avg_wait_ms, avg_exec_ms, avg_total_ms,
                    snapshot.std_dev_wait_ms, snapshot.std_dev_exec_ms
                ));
            }
        }

        for (name, m) in endpoint_pools {
            total_workers += m.total;
            busy_workers += m.active;
            workers_json.push(format!(
                r#""{}":{{"active": {}, "total": {}}}"#,
                name, m.active, m.total
            ));
        }

        let json = format!(
            r#"{{
                "workers": {{
                    "total": {},
                    "busy": {}
                }},
                "system_pools": {{
                    {}
                }},
                "workers_detail": {{
                    {}
                }}
            }}"#,
            total_workers,
            busy_workers,
            system_json.join(","),
            workers_json.join(",")
        );

        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json))
    }
}


pub fn register(builder: DispatcherBuilder, job_manager: Arc<JobManager>) -> DispatcherBuilder {
    builder
        .get("/jobs/result", Arc::new(JobResultHandler { job_manager: job_manager.clone() }))
        .get("/jobs/status", Arc::new(JobStatusHandler { job_manager: job_manager.clone() }))
        .get("/jobs/submit", Arc::new(JobSubmitHandler { job_manager: job_manager.clone() }))
        .get("/jobs/cancel", Arc::new(JobCancelHandler { job_manager: job_manager.clone() }))
        .get("/metrics", Arc::new(JobMetricsHandler { job_manager }))
}
