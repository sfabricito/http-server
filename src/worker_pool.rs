use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Job),
    Shutdown,
}

struct ThreadPoolInner {
    name: String,
    sender: mpsc::Sender<Message>,
    workers: Mutex<Vec<Option<thread::JoinHandle<()>>>>,
}

impl Drop for ThreadPoolInner {
    fn drop(&mut self) {
        let worker_len = self.workers.lock().unwrap().len();

        for _ in 0..worker_len {
            let _ = self.sender.send(Message::Shutdown);
        }

        let mut workers = self.workers.lock().unwrap();
        for handle_opt in workers.iter_mut() {
            if let Some(handle) = handle_opt.take() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Clone)]
pub struct ThreadPool {
    inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    pub fn new(name: &str, size: usize) -> Self {
        assert!(size > 0, "ThreadPool '{}' must have at least one worker", name);

        let (tx, rx) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(size);

        for idx in 0..size {
            let rx = Arc::clone(&receiver);
            let thread_name = format!("{}-worker-{}", name, idx);

            let handle = thread::Builder::new()
                .name(thread_name.clone())
                .spawn(move || loop {
                    let message = {
                        let guard = rx.lock().expect("worker receiver lock poisoned");
                        guard.recv()
                    };

                    match message {
                        Ok(Message::Run(job)) => {
                            job();
                        }
                        Ok(Message::Shutdown) | Err(_) => {
                            break;
                        }
                    }
                })
                .expect("Failed to spawn worker thread");

            workers.push(Some(handle));
        }

        ThreadPool {
            inner: Arc::new(ThreadPoolInner {
                name: name.to_string(),
                sender: tx,
                workers: Mutex::new(workers),
            }),
        }
    }

    pub fn from_env(name: &str, env_var: &str, default: usize) -> Self {
        let size = std::env::var(env_var)
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(default);
        Self::new(name, size)
    }

    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Err(err) = self.inner.sender.send(Message::Run(Box::new(job))) {
            eprintln!(
                "ThreadPool {}: worker channel closed, job dropped ({})",
                self.inner.name, err
            );
        }
    }
}
