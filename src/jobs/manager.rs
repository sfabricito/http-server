use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::jobs::{
    job::{Job, JobStatus},
    workers::{cpu_pool::CpuPool, io_pool::IoPool},
};

pub struct JobManager {
    pub cpu_pool: CpuPool,
    pub io_pool: IoPool,
    pub jobs: Arc<Mutex<HashMap<String, Arc<Job>>>>,
}

impl JobManager {
    pub fn new(cpu_workers: usize, io_workers: usize) -> Self {
        Self {
            cpu_pool: CpuPool::new(cpu_workers),
            io_pool: IoPool::new(io_workers),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn submit(&self, task: &str, params: HashMap<String, String>, is_cpu: bool) -> String {
        let timeout = if is_cpu {
            Duration::from_secs(60)
        } else {
            Duration::from_secs(120)
        };
        let job = Arc::new(Job::new(task, params, timeout));
        let id = job.id.clone();

        self.jobs.lock().unwrap().insert(id.clone(), job.clone());

        if is_cpu {
            self.cpu_pool.sender.send(job).unwrap();
        } else {
            self.io_pool.sender.send(job).unwrap();
        }

        id
    }

    pub fn status(&self, id: &str) -> Option<JobStatus> {
        self.jobs.lock().unwrap().get(id).map(|j| j.status.lock().unwrap().clone())
    }

    pub fn result(&self, id: &str) -> Option<String> {
        self.jobs.lock().unwrap().get(id)
            .and_then(|j| j.result.lock().unwrap().clone())
    }

    pub fn cancel(&self, id: &str) -> bool {
        if let Some(job) = self.jobs.lock().unwrap().get(id) {
            *job.status.lock().unwrap() = JobStatus::Canceled;
            return true;
        }
        false
    }
}
