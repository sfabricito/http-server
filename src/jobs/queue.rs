use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar};
use std::time::Duration;

use crate::jobs::job::{Job, Priority};

pub struct JobQueue {
    inner: Mutex<JobQueueInner>,
    cv: Condvar,
}

struct JobQueueInner {
    high: VecDeque<Arc<Job>>,
    normal: VecDeque<Arc<Job>>,
    low: VecDeque<Arc<Job>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(JobQueueInner {
                high: VecDeque::new(),
                normal: VecDeque::new(),
                low: VecDeque::new(),
            }),
            cv: Condvar::new(),
        }
    }

    pub fn try_enqueue(&self, job: Arc<Job>, max: usize) -> Result<(), String> {
        let mut q = self.inner.lock().unwrap();

        let total = q.high.len() + q.normal.len() + q.low.len();
        if total >= max {
            return Err("QueueFull".into());
        }

        match job.priority {
            Priority::High => q.high.push_back(job),
            Priority::Normal => q.normal.push_back(job),
            Priority::Low => q.low.push_back(job),
        }

        self.cv.notify_one();
        Ok(())
    }

    pub fn enqueue(&self, job: Arc<Job>) {
        let mut q = self.inner.lock().unwrap();
        match job.priority {
            Priority::High => q.high.push_back(job),
            Priority::Normal => q.normal.push_back(job),
            Priority::Low => q.low.push_back(job),
        }
        self.cv.notify_one();
    }

    pub fn dequeue(&self) -> Arc<Job> {
        let mut q = self.inner.lock().unwrap();

        loop {
            if let Some(job) = q.high.pop_front() {
                return job;
            }
            if let Some(job) = q.normal.pop_front() {
                return job;
            }
            if let Some(job) = q.low.pop_front() {
                return job;
            }

            q = self.cv.wait_timeout(q, Duration::from_millis(500)).unwrap().0;
        }
    }

    pub fn len_by_priority(&self) -> (usize, usize, usize) {
        let q = self.inner.lock().unwrap();
        (q.high.len(), q.normal.len(), q.low.len())
    }

    pub fn total_len(&self) -> usize {
        let (h, n, l) = self.len_by_priority();
        h + n + l
    }

    pub fn queue_lengths(&self) -> (usize, usize, usize) {
        self.len_by_priority()
    }
}
