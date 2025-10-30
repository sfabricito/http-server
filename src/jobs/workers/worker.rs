use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crate::jobs::{
    job::JobStatus,
    manager::JobManager,
    queue::JobQueue,
};

#[derive(Default, Clone, Copy)]
struct RunningStats {
    count: u64,
    mean: f64,
    m2: f64,
}

impl RunningStats {
    fn update(&mut self, sample: f64) {
        self.count += 1;
        let delta = sample - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = sample - self.mean;
        self.m2 += delta * delta2;
    }

    fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.mean
        }
    }

    fn std_dev(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            (self.m2 / (self.count - 1) as f64).sqrt()
        }
    }
}

pub struct MetricsSnapshot {
    pub avg_wait_ms: f64,
    pub avg_exec_ms: f64,
    pub std_dev_wait_ms: f64,
    pub std_dev_exec_ms: f64,
    pub samples: u64,
}

pub struct WorkerMetrics {
    pub active_workers: Arc<Mutex<usize>>,
    pub total_workers: usize,
    wait_stats: Arc<Mutex<RunningStats>>,
    exec_stats: Arc<Mutex<RunningStats>>,
}

impl WorkerMetrics {
    pub fn new(total: usize) -> Self {
        Self {
            active_workers: Arc::new(Mutex::new(0)),
            total_workers: total,
            wait_stats: Arc::new(Mutex::new(RunningStats::default())),
            exec_stats: Arc::new(Mutex::new(RunningStats::default())),
        }
    }

    pub fn record_wait(&self, duration: Duration) {
        let mut stats = self.wait_stats.lock().unwrap();
        stats.update(duration.as_secs_f64() * 1000.0);
    }

    pub fn record_exec(&self, duration: Duration) {
        let mut stats = self.exec_stats.lock().unwrap();
        stats.update(duration.as_secs_f64() * 1000.0);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let wait = *self.wait_stats.lock().unwrap();
        let exec = *self.exec_stats.lock().unwrap();
        MetricsSnapshot {
            avg_wait_ms: wait.mean(),
            avg_exec_ms: exec.mean(),
            std_dev_wait_ms: wait.std_dev(),
            std_dev_exec_ms: exec.std_dev(),
            samples: wait.count,
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
            metrics.record_wait(start_wait);

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
            metrics.record_exec(exec_time);

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
