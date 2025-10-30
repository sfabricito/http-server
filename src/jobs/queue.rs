use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar};
use std::time::Duration;
use crate::jobs::job::{Job, Priority};

pub struct JobQueue {
    high: Mutex<VecDeque<Arc<Job>>>,
    normal: Mutex<VecDeque<Arc<Job>>>,
    low: Mutex<VecDeque<Arc<Job>>>,
    cv: Condvar,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            high: Mutex::new(VecDeque::new()),
            normal: Mutex::new(VecDeque::new()),
            low: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
        }
    }

    /// Add a job to the appropriate priority queue and wake a waiting worker.
    pub fn enqueue(&self, job: Arc<Job>) {
        {
            let queue = match job.priority {
                Priority::High => &self.high,
                Priority::Normal => &self.normal,
                Priority::Low => &self.low,
            };
            queue.lock().unwrap().push_back(job);
        }
        self.cv.notify_one();
    }

    /// Try to enqueue without blocking if total queue is full.
    pub fn try_enqueue(&self, job: Arc<Job>) -> Result<(), String> {
        let max = std::env::var("JOB_QUEUE_MAX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(100);
        if self.total_len() >= max {
            return Err("QueueFull".into());
        }
        self.enqueue(job);
        Ok(())
    }

    /// Dequeue with priority order: High → Normal → Low.
    pub fn dequeue(&self) -> Arc<Job> {
        loop {
            // Try high first
            if let Some(job) = self.high.lock().unwrap().pop_front() {
                return job;
            }
            // Then normal
            if let Some(job) = self.normal.lock().unwrap().pop_front() {
                return job;
            }
            // Then low
            if let Some(job) = self.low.lock().unwrap().pop_front() {
                return job;
            }

            // Wait for new job if all empty
            let lock = self.high.lock().unwrap();
            let _ = self.cv.wait_timeout(lock, Duration::from_millis(500)).unwrap();
        }
    }

    /// Return the number of jobs in each priority queue.
    pub fn len_by_priority(&self) -> (usize, usize, usize) {
        (
            self.high.lock().unwrap().len(),
            self.normal.lock().unwrap().len(),
            self.low.lock().unwrap().len(),
        )
    }

    /// Return total number of queued jobs.
    pub fn total_len(&self) -> usize {
        let (h, n, l) = self.len_by_priority();
        h + n + l
    }

    /// Return a copy of queue sizes as (high, normal, low).
    pub fn queue_lengths(&self) -> (usize, usize, usize) {
        self.len_by_priority()
    }
}
