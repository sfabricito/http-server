use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use super::job::Job;

pub struct JobQueue {
    queue: Arc<(Mutex<VecDeque<Arc<Job>>>, Condvar)>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
        }
    }

    pub fn enqueue(&self, job: Arc<Job>) {
        let (lock, cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();
        q.push_back(job);
        cvar.notify_one();
    }

    pub fn dequeue(&self) -> Arc<Job> {
        let (lock, cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();
        loop {
            if let Some(job) = q.pop_front() {
                return job;
            }
            q = cvar.wait(q).unwrap();
        }
    }

    pub fn len(&self) -> usize {
        self.queue.0.lock().unwrap().len()
    }
}
