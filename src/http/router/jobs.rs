use std::sync::Arc;

use crate::http::{
    handler::{RequestHandlerStrategy, DispatcherBuilder},
    request::HttpRequest,
    response::{Response, OK},
    errors::ServerError,
};
use crate::jobs::manager::JobManager;
use crate::jobs::job::JobStatus;

pub struct JobResultHandler {
    pub job_manager: Arc<JobManager>,
}

impl RequestHandlerStrategy for JobResultHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        // Validate query param (parse from raw query string here so this module
        // is independent of router-local helpers)
        let id = if req.query.is_empty() {
            return Err(ServerError::BadRequest("Missing query parameter 'id'".into()));
        } else {
            let mut found: Option<&str> = None;
            for pair in req.query.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    if k == "id" {
                        found = Some(v);
                        break;
                    }
                }
            }
            found.ok_or_else(|| ServerError::BadRequest("Missing query parameter 'id'".into()))?
        };

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

pub fn register(builder: DispatcherBuilder, job_manager: Arc<JobManager>) -> DispatcherBuilder {
    builder.get("/jobs/result", Arc::new(JobResultHandler { job_manager }))
}
