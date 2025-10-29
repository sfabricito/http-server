use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use super::job::{Job, Priority};

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

    pub fn enqueue(&self, job: Arc<Job>) {
        let queue = match job.priority {
            Priority::High => &self.high,
            Priority::Normal => &self.normal,
            Priority::Low => &self.low,
        };
        let (lock, cvar) = &**queue;
        let mut q = lock.lock().unwrap();
        q.push_back(job);
        cvar.notify_one();
    }

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

            // wait until some queue gets work
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

    pub fn len_by_priority(&self) -> (usize, usize, usize) {
        (
            self.high.0.lock().unwrap().len(),
            self.normal.0.lock().unwrap().len(),
            self.low.0.lock().unwrap().len(),
        )
    }

    pub fn total_len(&self) -> usize {
        let (h, n, l) = self.len_by_priority();
        h + n + l
    }
}
