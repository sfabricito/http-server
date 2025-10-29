use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::jobs::{
    job::{Job, JobStatus},
    manager::JobManager,
    queue::JobQueue,
};

pub struct WorkerMetrics {
    pub active_workers: Arc<Mutex<usize>>,
    pub total_workers: usize,
    pub avg_wait: Arc<Mutex<Duration>>,
    pub avg_exec: Arc<Mutex<Duration>>,
}

impl WorkerMetrics {
    pub fn new(total: usize) -> Self {
        Self {
            active_workers: Arc::new(Mutex::new(0)),
            total_workers: total,
            avg_wait: Arc::new(Mutex::new(Duration::ZERO)),
            avg_exec: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }
}

pub fn spawn_workers(
    tag: &str,
    pool_size: usize,
    queue: Arc<JobQueue>,
    manager: Arc<JobManager>,
) -> Arc<WorkerMetrics> {
    let metrics = Arc::new(WorkerMetrics::new(pool_size));

    for idx in 0..pool_size {
        let queue = queue.clone();
        let manager = manager.clone();
        let metrics = metrics.clone();
        let tag = tag.to_string();

        thread::spawn(move || loop {
            let job = queue.dequeue();

            if matches!(*job.status.lock().unwrap(), JobStatus::Canceled) {
                continue;
            }

            {
                let mut active = metrics.active_workers.lock().unwrap();
                *active += 1;
            }

            let start_wait = job.created_at.elapsed();
            {
                let mut avg_wait = metrics.avg_wait.lock().unwrap();
                *avg_wait = (*avg_wait + start_wait) / 2;
            }

            {
                *job.started_at.lock().unwrap() = Some(Instant::now());
                *job.status.lock().unwrap() = JobStatus::Running;
            }

            let exec_start = Instant::now();
            let job_id = job.id.clone();
            let task_name = job.task.clone();
            let result = std::panic::catch_unwind(|| {
                manager.execute_job(job.clone());
            });

            let exec_time = exec_start.elapsed();

            {
                let mut avg_exec = metrics.avg_exec.lock().unwrap();
                *avg_exec = (*avg_exec + exec_time) / 2;
            }

            match result {
                Ok(_) => {
                    *job.finished_at.lock().unwrap() = Some(Instant::now());
                }
                Err(_) => {
                    *job.status.lock().unwrap() = JobStatus::Error("panic".into());
                }
            }

            {
                let mut active = metrics.active_workers.lock().unwrap();
                if *active > 0 {
                    *active -= 1;
                }
            }

            println!(
                "[{} worker {}] finished job {} (task {}) in {:?}",
                tag, idx, job_id, task_name, exec_time
            );
        });
    }

    metrics
}
