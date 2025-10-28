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
}

#[derive(Clone)]
pub struct Job {
    pub id: String,
    pub task: String,
    pub params: HashMap<String, String>,
    pub priority: u8, // 0=low, 1=normal, 2=high
    pub status: Arc<Mutex<JobStatus>>,
    pub progress: Arc<Mutex<f32>>,
    pub result: Arc<Mutex<Option<String>>>,
    pub created_at: Instant,
    pub timeout: Duration,
}

impl Job {
    pub fn new(task: &str, params: HashMap<String, String>, timeout: Duration) -> Self {
        Job {
            id: Uuid::new_v4().to_string(),
            task: task.to_string(),
            params,
            priority: 1,
            status: Arc::new(Mutex::new(JobStatus::Queued)),
            progress: Arc::new(Mutex::new(0.0)),
            result: Arc::new(Mutex::new(None)),
            created_at: Instant::now(),
            timeout,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }
}
