use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::jobs::{
    job::{Job, JobStatus},
    manager::JobManager,
    queue::JobQueue,
};

const METRIC_WINDOW: usize = 1000; // number of samples used for std. deviation

#[derive(Default)]
pub struct WorkerMetrics {
    pub active_workers: Arc<Mutex<usize>>,
    pub total_workers: usize,
    pub total_jobs: Arc<Mutex<u64>>,
    pub avg_wait: Arc<Mutex<Duration>>,
    pub avg_exec: Arc<Mutex<Duration>>,
    pub avg_total: Arc<Mutex<Duration>>,
    pub wait_samples: Arc<Mutex<VecDeque<Duration>>>,
    pub exec_samples: Arc<Mutex<VecDeque<Duration>>>,
}

impl WorkerMetrics {
    pub fn new(total: usize) -> Self {
        Self {
            active_workers: Arc::new(Mutex::new(0)),
            total_workers: total,
            total_jobs: Arc::new(Mutex::new(0)),
            avg_wait: Arc::new(Mutex::new(Duration::ZERO)),
            avg_exec: Arc::new(Mutex::new(Duration::ZERO)),
            avg_total: Arc::new(Mutex::new(Duration::ZERO)),
            wait_samples: Arc::new(Mutex::new(VecDeque::new())),
            exec_samples: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn std_dev_ms(samples: &VecDeque<Duration>) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let n = samples.len() as f64;
        let mean = samples.iter().map(|d| d.as_millis() as f64).sum::<f64>() / n;
        let var = samples
            .iter()
            .map(|d| {
                let diff = d.as_millis() as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / n;
        var.sqrt()
    }

    pub fn std_wait_ms(&self) -> f64 {
        Self::std_dev_ms(&self.wait_samples.lock().unwrap())
    }

    pub fn std_exec_ms(&self) -> f64 {
        Self::std_dev_ms(&self.exec_samples.lock().unwrap())
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

            let wait_time = job.created_at.elapsed();
            {
                let mut avg_wait = metrics.avg_wait.lock().unwrap();
                *avg_wait = ((*avg_wait * 9) + wait_time) / 10;
            }

            {
                let mut samples = metrics.wait_samples.lock().unwrap();
                samples.push_back(wait_time);
                if samples.len() > METRIC_WINDOW {
                    samples.pop_front();
                }
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
                *avg_exec = ((*avg_exec * 9) + exec_time) / 10;
            }

            {
                let mut samples = metrics.exec_samples.lock().unwrap();
                samples.push_back(exec_time);
                if samples.len() > METRIC_WINDOW {
                    samples.pop_front();
                }
            }

            let total_time = wait_time + exec_time;
            {
                let mut avg_total = metrics.avg_total.lock().unwrap();
                *avg_total = ((*avg_total * 9) + total_time) / 10;
            }

            {
                let mut total_jobs = metrics.total_jobs.lock().unwrap();
                *total_jobs += 1;
            }

            if result.is_err() {
                *job.status.lock().unwrap() = JobStatus::Error("panic".into());
            } else {
                *job.finished_at.lock().unwrap() = Some(Instant::now());
            }

            {
                let mut active = metrics.active_workers.lock().unwrap();
                if *active > 0 {
                    *active -= 1;
                }
            }
        });
    }

    metrics
}
