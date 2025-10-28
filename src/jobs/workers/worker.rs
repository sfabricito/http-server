use std::sync::{Arc, Mutex, mpsc::Receiver};
use std::thread;
use std::time::Instant;
use crate::jobs::job::{Job, JobStatus};

pub struct WorkerPool {
    threads: Vec<thread::JoinHandle<()>>,
}

impl WorkerPool {
    pub fn new(name: &str, receiver: Arc<Mutex<Receiver<Arc<Job>>>>, worker_count: usize) -> Self {
        let mut threads = Vec::new();

        for i in 0..worker_count {
            let rx = receiver.clone();
            let tag = name.to_string();
            threads.push(thread::spawn(move || {
                loop {
                    let job = {
                        let guard = rx.lock().unwrap();
                        guard.recv().unwrap()
                    };

                    {
                        let mut status = job.status.lock().unwrap();
                        *status = JobStatus::Running;
                    }

                    let start = Instant::now();
                    // Execute job (placeholder: replace with dispatcher)
                    let result = format!("Task {} executed by {}", job.task, tag);
                    *job.result.lock().unwrap() = Some(result);
                    *job.progress.lock().unwrap() = 100.0;

                    let elapsed = start.elapsed();
                    *job.status.lock().unwrap() = if job.is_expired() {
                        JobStatus::Error("timeout".into())
                    } else {
                        JobStatus::Done
                    };
                    println!("[{} Worker {}] Finished job {:?} in {:?}", tag, i, job.id, elapsed);
                }
            }));
        }

        Self { threads }
    }
}