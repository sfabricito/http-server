use std::sync::Arc;
use std::collections::HashMap;

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    request::HttpRequest,
    response::{Response, OK},
    errors::ServerError,
    router::router::QueryParam,
};
use crate::jobs::manager::JobManager;
use crate::jobs::job::{JobStatus, Priority};

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

        let job_id = self.job_manager
            .submit(task, params, priority)
            .map_err(|e| ServerError::Internal(format!("Failed to submit job: {}", e)))?;

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
        let mut pools_json = Vec::new();

        // --- Ensure CPU appears first ---
        let ordered_names = ["cpu", "io"];

        for &name in &ordered_names {
            if let Some(metrics) = pools.get(name) {
                let queue_lengths = metrics.queue_lengths;
                let wm = &metrics.worker_metrics;

                let active = *wm.active_workers.lock().unwrap();
                let total = wm.total_workers;
                let total_jobs = *wm.total_jobs.lock().unwrap();

                let avg_wait = wm.avg_wait.lock().unwrap().as_millis();
                let avg_exec = wm.avg_exec.lock().unwrap().as_millis();
                let avg_total = wm.avg_total.lock().unwrap().as_millis();

                let std_wait = wm.std_wait_ms();
                let std_exec = wm.std_exec_ms();

                pools_json.push(format!(
                    r#""{}":{{
                        "queue_size": {{"high": {}, "normal": {}, "low": {}}},
                        "workers": {{"active": {}, "total": {}}},
                        "jobs": {{"total": {}}},
                        "timings": {{
                            "avg_wait_ms": {},
                            "avg_exec_ms": {},
                            "avg_total_ms": {},
                            "std_dev_wait_ms": {:.2},
                            "std_dev_exec_ms": {:.2}
                        }}
                    }}"#,
                    name,
                    queue_lengths.0, queue_lengths.1, queue_lengths.2,
                    active, total,
                    total_jobs,
                    avg_wait, avg_exec, avg_total,
                    std_wait, std_exec
                ));
            }
        }

        let json = format!("{{\"pools\":{{{}}}}}", pools_json.join(","));

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