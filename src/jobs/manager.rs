use std::{collections::HashMap, env, path::PathBuf, sync::{Arc, Mutex}, time::{Duration, Instant}};
use crate::jobs::{
    job::{Job, JobStatus, Priority},
    persistence::{save_job_state, load_job_states},
    workers::{cpu_pool::CpuPool, io_pool::IoPool, worker::WorkerMetrics},
};

use crate::utils::{
    io::{
        sort_file::sort_file,
    }
};

pub struct PoolMetrics {
    pub queue_lengths: (usize, usize, usize),
    pub worker_metrics: Arc<WorkerMetrics>,
}

pub struct JobManager {
    pub cpu_pool: Arc<CpuPool>,
    pub io_pool: Arc<IoPool>,
    pub jobs: Arc<Mutex<HashMap<String, Arc<Job>>>>,
    pub persist_path: PathBuf,
}

use crate::jobs::executables;

impl JobManager {
    pub fn new(cpu_workers: usize, io_workers: usize) -> Arc<Self> {
        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let persist_path = std::env::var("JOB_PERSIST_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/persistent/state.jsonl"));
            
        let manager = Arc::new_cyclic(|weak_self| JobManager {
            cpu_pool: Arc::new(CpuPool::empty()),
            io_pool: Arc::new(IoPool::empty()),
            jobs: jobs.clone(),
            persist_path: persist_path.clone(),
        });

        let cpu_pool = Arc::new(CpuPool::new(cpu_workers, manager.clone()));
        let io_pool = Arc::new(IoPool::new(io_workers, manager.clone()));

        unsafe {
            let ptr = Arc::as_ptr(&manager) as *mut JobManager;
            (*ptr).cpu_pool = cpu_pool;
            (*ptr).io_pool = io_pool;
        }

        Self::load_persistent_jobs(&manager);

        manager
    }


    pub fn submit(
        &self,
        task: &str,
        params: std::collections::HashMap<String, String>,
        is_cpu: bool,
        priority: Priority,
    ) -> Result<String, String> {
        let queue_max = env::var("JOB_QUEUE_MAX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(100);

        let queue = if is_cpu { &self.cpu_pool.queue } else { &self.io_pool.queue };
        if queue.total_len() >= queue_max {
            return Err("QueueFull".into());
        }

        let timeout_secs = if is_cpu {
            env::var("CPU_TIMEOUT").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(60)
        } else {
            env::var("IO_TIMEOUT").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(120)
        };

        let job = Arc::new(Job::with_priority(task, params, priority, std::time::Duration::from_secs(timeout_secs)));
        let id = job.id.clone();

        {
            let mut map = self.jobs.lock().unwrap();
            map.insert(id.clone(), job.clone());
        }

        save_job_state(&job, &self.persist_path);

        queue.enqueue(job);

        Ok(id)
    }

    pub fn execute_job(&self, job: Arc<Job>) {
        {
            *job.status.lock().unwrap() = JobStatus::Running;
            *job.started_at.lock().unwrap() = Some(Instant::now());
        }

        let out: Result<String, String> = match job.task.as_str() {
            // CPU-bound executables
            // "isprime" => executables::isprime::run(&job.params),
            // "factor" => executables::factor::run(&job.params),
            // "matrixmul" => executables::matrixmul::run(&job.params),
            // "mandelbrot" => executables::mandelbrot::run(&job.params),

            // IO-bound executables
            "sortfile" => executables::sort_file::run(&job.params),
            "wordcount" => executables::word_count::run(&job.params),
            "grep" => executables::grep::run(&job.params),
            "compress" => executables::compress::run(&job.params),
            "hashfile" => executables::hash_file::run(&job.params),

            // Unknown task
            _ => Err(format!("Unknown task '{}'", job.task)),
        };

        {
            *job.result.lock().unwrap() = out.clone().ok();
            *job.finished_at.lock().unwrap() = Some(Instant::now());

            *job.status.lock().unwrap() = match out {
                Ok(_) => {
                    if job.is_expired() {
                        JobStatus::Timeout
                    } else {
                        JobStatus::Done
                    }
                }
                Err(e) => JobStatus::Error(e),
            };
        }

        save_job_state(&job, &self.persist_path);
    }

    pub fn status(&self, id: &str) -> Option<JobStatus> {
        self.jobs.lock().unwrap().get(id).map(|j| j.status.lock().unwrap().clone())
    }

    pub fn result(&self, id: &str) -> Option<String> {
        self.jobs.lock().unwrap().get(id).and_then(|j| j.result.lock().unwrap().clone())
    }

    pub fn cancel(&self, id: &str) -> bool {
        let map = self.jobs.lock().unwrap();
        if let Some(job) = map.get(id) {
            let mut s = job.status.lock().unwrap();
            if *s == JobStatus::Queued {
                *s = JobStatus::Canceled;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn get_metrics(&self) -> HashMap<String, PoolMetrics> {
        let mut m = HashMap::new();
        m.insert(
            "cpu".into(),
            PoolMetrics {
                queue_lengths: self.cpu_pool.queue_lengths(),
                worker_metrics: self.cpu_pool.metrics.clone(),
            },
        );
        m.insert(
            "io".into(),
            PoolMetrics {
                queue_lengths: self.io_pool.queue_lengths(),
                worker_metrics: self.io_pool.metrics.clone(),
            },
        );
        m
    }

    fn load_persistent_jobs(manager: &Arc<JobManager>) {
        let persist_path = &manager.persist_path;
        let previous_jobs = load_job_states(persist_path);

        for record in previous_jobs {
            println!(
                "[restore] Found job {} (task='{}', state={:?})",
                record.id, record.task, record.status
            );

            let mut params = HashMap::new();
            if let Some(p) = record.params {
                for (k, v) in p {
                    if let Some(s) = v.as_str() {
                        params.insert(k.clone(), s.to_string());
                    }
                }
            }

            let timeout = Duration::from_secs(60);

            let job = Arc::new(Job::from_saved(
                &record.id,
                &record.task,
                params,
                record.priority,
                record.status.clone(),
                timeout,
                record.result.clone(),
            ));

            {
                let mut map = manager.jobs.lock().unwrap();
                map.insert(job.id.clone(), job.clone());
            }

            if matches!(record.status, JobStatus::Queued | JobStatus::Running) {
                let is_cpu = matches!(
                    record.task.as_str(),
                    "isprime" | "factor" | "matrixmul" | "mandelbrot"
                );

                if is_cpu {
                    manager.cpu_pool.queue.enqueue(job.clone());
                    println!("[restore] Re-queued job {} into CPU pool", record.id);
                } else {
                    manager.io_pool.queue.enqueue(job.clone());
                    println!("[restore] Re-queued job {} into IO pool", record.id);
                }
            } else {
                println!(
                    "[restore] Job {} restored in memory only (status = {:?})",
                    record.id, record.status
                );
            }
        }

        println!("[restore] Completed loading job persistence from {:?}", persist_path);
    }
}
