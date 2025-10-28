use std::{fs, path::Path};
use crate::jobs::job::Job;
use serde_json;

pub fn save_job_state(job: &Job, path: &Path) {
    let json = serde_json::json!({
        "id": job.id,
        "task": job.task,
        "status": format!("{:?}", *job.status.lock().unwrap())
    });
    fs::write(path, json.to_string()).ok();
}
