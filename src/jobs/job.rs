use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error(String),
    Canceled,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Low,
    Normal,
    High,
}

#[derive(Clone)]
pub struct Job {
    pub id: String,
    pub task: String,
    pub params: HashMap<String, String>,
    pub priority: Priority,
    pub status: Arc<Mutex<JobStatus>>,
    pub progress: Arc<Mutex<f32>>,
    pub result: Arc<Mutex<Option<String>>>,
    pub created_at: Instant,
    pub started_at: Arc<Mutex<Option<Instant>>>,
    pub finished_at: Arc<Mutex<Option<Instant>>>,
    pub timeout: Duration,
    pub cancel_flag: Arc<Mutex<bool>>,
}

impl Job {
    pub fn new(task: &str, params: HashMap<String, String>, timeout: Duration) -> Self {
        Job {
            id: Uuid::new_v4().to_string(),
            task: task.to_string(),
            params,
            priority: Priority::Normal,
            status: Arc::new(Mutex::new(JobStatus::Queued)),
            progress: Arc::new(Mutex::new(0.0)),
            result: Arc::new(Mutex::new(None)),
            created_at: Instant::now(),
            started_at: Arc::new(Mutex::new(None)),
            finished_at: Arc::new(Mutex::new(None)),
            timeout,
            cancel_flag: Arc::new(Mutex::new(false)),
        }
    }

    pub fn with_priority(task: &str, params: HashMap<String, String>, priority: Priority, timeout: Duration) -> Self {
        let mut job = Self::new(task, params, timeout);
        job.priority = priority;
        job
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }

    pub fn from_saved(
        id: &str,
        task: &str,
        params: HashMap<String, String>,
        priority: Priority,
        status: JobStatus,
        timeout: Duration,
        result: Option<String>,
    ) -> Self {
        let mut j = Job::with_priority(task, params, priority, timeout);
        j.id = id.to_string();
        *j.status.lock().unwrap() = status;
        *j.result.lock().unwrap() = result;
        j
    }

}
