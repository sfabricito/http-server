use std::sync::Arc;
use crate::jobs::{queue::JobQueue, manager::JobManager};
use super::worker::{spawn_workers, WorkerMetrics};

pub struct IoPool {
    pub queue: Arc<JobQueue>,
    pub metrics: Arc<WorkerMetrics>,
}
impl IoPool {
    pub fn new(size: usize, manager: Arc<JobManager>) -> Self {
        let queue = Arc::new(JobQueue::new());
        let metrics = spawn_workers("IO", size, queue.clone(), manager);
        Self { queue, metrics }
    }

    pub fn queue_lengths(&self) -> (usize, usize, usize) {
        self.queue.len_by_priority()
    }
}

impl IoPool {
    pub fn empty() -> Self {
        let queue = Arc::new(JobQueue::new());
        let dummy_metrics = Arc::new(WorkerMetrics::new(0));
        Self { queue, metrics: dummy_metrics }
    }
}
