use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::job::{Job, JobStatus, Priority};

pub struct JobQueue {
    high: Arc<(Mutex<VecDeque<Arc<Job>>>, Condvar)>,
    normal: Arc<(Mutex<VecDeque<Arc<Job>>>, Condvar)>,
    low: Arc<(Mutex<VecDeque<Arc<Job>>>, Condvar)>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            high: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
            normal: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
            low: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
        }
    }

    /// Enqueue a job respecting JOB_QUEUE_MAX
    pub fn enqueue(&self, job: Arc<Job>) {
        use std::env;

        let queue_max = env::var("JOB_QUEUE_MAX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(100);

        // optional retry mechanism (useful under burst load)
        let mut attempts = 0;
        while self.total_len() >= queue_max && attempts < 3 {
            eprintln!(
                "[queue] ⚠️ Queue limit reached ({} >= {}), retrying attempt {}/3...",
                self.total_len(),
                queue_max,
                attempts + 1
            );
            std::thread::sleep(Duration::from_millis(200));
            attempts += 1;
        }

        if self.total_len() >= queue_max {
            eprintln!(
                "[queue] ❌ Queue overflow: job '{}' canceled (limit: {}).",
                job.id, queue_max
            );
            *job.status.lock().unwrap() = JobStatus::Canceled;
            return;
        }

        // Determine priority queue
        let queue = match job.priority {
            Priority::High => &self.high,
            Priority::Normal => &self.normal,
            Priority::Low => &self.low,
        };

        let (lock, cvar) = &**queue;
        let mut q = lock.lock().unwrap();
        q.push_back(job);
        cvar.notify_all();
    }

    /// Blocking dequeue (priority-based)
    pub fn dequeue(&self) -> Arc<Job> {
        loop {
            if let Some(job) = self.try_pop(&self.high) {
                return job;
            }
            if let Some(job) = self.try_pop(&self.normal) {
                return job;
            }
            if let Some(job) = self.try_pop(&self.low) {
                return job;
            }

            // Wait for a job to appear
            let (lock, cvar) = &*self.normal;
            let q = lock.lock().unwrap();
            let _unused = cvar.wait(q).unwrap();
        }
    }
    

    fn try_pop(&self, queue: &Arc<(Mutex<VecDeque<Arc<Job>>>, Condvar)>) -> Option<Arc<Job>> {
        let (lock, _) = &**queue;
        let mut q = lock.lock().unwrap();
        q.pop_front()
    }

    /// Returns (high, normal, low) queue lengths
    pub fn len_by_priority(&self) -> (usize, usize, usize) {
        (
            self.high.0.lock().unwrap().len(),
            self.normal.0.lock().unwrap().len(),
            self.low.0.lock().unwrap().len(),
        )
    }

    /// Returns total jobs in all queues
    pub fn total_len(&self) -> usize {
        let (h, n, l) = self.len_by_priority();
        h + n + l
    }
}
