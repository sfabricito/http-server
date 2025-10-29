use std::sync::Arc;
use std::collections::HashMap;

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    request::HttpRequest,
    response::{Response, OK},
    errors::ServerError,
    router::QueryParam,
};
use crate::jobs::manager::JobManager;
use crate::jobs::job::JobStatus;

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
                };

                let status_str = match &status {
                    JobStatus::Queued => "queued",
                    JobStatus::Running => "running",
                    JobStatus::Done => "done",
                    JobStatus::Error(_) => "error",
                    JobStatus::Canceled => "canceled",
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

        let mut params = HashMap::new();
        for pair in req.query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k != "task" {
                    params.insert(k.to_string(), v.to_string());
                }
            }
        }

        let job_id = self.job_manager.submit(task, params, true);

        let json = format!(
            "{{\"job_id\":\"{}\",\"status\":\"queued\"}}",
            job_id
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
}