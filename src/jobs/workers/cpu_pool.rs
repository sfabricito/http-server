use std::sync::{mpsc, Arc, Mutex};
use super::worker::WorkerPool;
use crate::jobs::job::Job;

pub struct CpuPool {
    pub sender: mpsc::Sender<Arc<Job>>,
    pub pool: WorkerPool,
}

impl CpuPool {
    pub fn new(worker_count: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Arc<Job>>();
        let pool = WorkerPool::new("CPU", Arc::new(Mutex::new(rx)), worker_count);
        Self { sender: tx, pool }
    }
}
