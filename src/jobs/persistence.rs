use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::Mutex,
};
use crate::jobs::job::{Job, JobStatus, Priority};
use serde_json::{json, Value, Map};
use lazy_static::lazy_static;

// Global file lock (prevents concurrent reads/writes)
lazy_static! {
    static ref FILE_LOCK: Mutex<()> = Mutex::new(());
}

/// --------------------------------------------------------------------
///  Save Job State (Thread-Safe)
/// --------------------------------------------------------------------
pub fn save_job_state(job: &Job, path: &Path) {
    let _guard = FILE_LOCK.lock().unwrap(); // 🔒 Prevent data races

    let snapshot = json!({
        "id": job.id,
        "task": job.task,
        "priority": format!("{:?}", job.priority),
        "status": format!("{:?}", *job.status.lock().unwrap()),
        "progress": *job.progress.lock().unwrap(),
        "result": job.result.lock().unwrap().clone().unwrap_or_default(),
        "params": job.params,
        "created_at_ms": job.created_at.elapsed().as_millis(),
        "started_at": job.started_at.lock().unwrap().map(|t| t.elapsed().as_millis()),
        "finished_at": job.finished_at.lock().unwrap().map(|t| t.elapsed().as_millis()),
        "timeout_secs": job.timeout.as_secs(),
        "cancel_flag": *job.cancel_flag.lock().unwrap(),
    });

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Rebuild state file excluding any previous entry for this job
    let mut existing = Vec::new();
    if path.exists() {
        if let Ok(file) = fs::File::open(path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Ok(val) = serde_json::from_str::<Value>(&line) {
                    if val.get("id").and_then(|v| v.as_str()) != Some(job.id.as_str()) {
                        existing.push(val);
                    }
                }
            }
        }
    }

    existing.push(snapshot);

    // Atomic replace write
    let tmp_path = path.with_extension("tmp");
    if let Ok(mut file) = OpenOptions::new().create(true).write(true).truncate(true).open(&tmp_path)
    {
        for val in &existing {
            if let Err(e) = writeln!(file, "{}", val) {
                eprintln!("[persistence] failed to write job {}: {}", job.id, e);
            }
        }
        if let Err(e) = fs::rename(&tmp_path, path) {
            eprintln!("[persistence] failed to replace state file: {}", e);
        }
    }
}

/// --------------------------------------------------------------------
///  SavedJob struct — represents stored JSON state
/// --------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct SavedJob {
    pub id: String,
    pub task: String,
    pub priority: Priority,
    pub status: JobStatus,
    pub params: Option<Map<String, Value>>,
    pub result: Option<String>,
}

/// --------------------------------------------------------------------
///  Load Persistent Jobs (Thread-Safe)
/// --------------------------------------------------------------------
pub fn load_job_states(path: &Path) -> Vec<SavedJob> {
    let _guard = FILE_LOCK.lock().unwrap(); // 🔒 Prevent race with writers
    let mut restored = Vec::new();

    if !path.exists() {
        return restored;
    }

    if let Ok(file) = fs::File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            if let Ok(val) = serde_json::from_str::<Value>(&line) {
                if let (Some(id), Some(task), Some(status)) =
                    (val.get("id"), val.get("task"), val.get("status"))
                {
                    let id = id.as_str().unwrap_or("").to_string();
                    let task = task.as_str().unwrap_or("").to_string();

                    let priority = match val.get("priority").and_then(|v| v.as_str()) {
                        Some("High") => Priority::High,
                        Some("Low") => Priority::Low,
                        _ => Priority::Normal,
                    };

                    let status = match status.as_str().unwrap_or("") {
                        "Queued" => JobStatus::Queued,
                        "Running" => JobStatus::Queued,
                        "Done" => JobStatus::Done,
                        "Canceled" => JobStatus::Canceled,
                        s if s.starts_with("Error") => JobStatus::Error(s.to_string()),
                        _ => JobStatus::Queued,
                    };

                    let result = val.get("result")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string());

                    restored.push(SavedJob {
                        id,
                        task,
                        priority,
                        status,
                        params: val.get("params").and_then(|p| p.as_object()).cloned(),
                        result,
                    });
                }
            }
        }
    }

    restored
}
