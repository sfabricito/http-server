use std::collections::HashMap;
use std::sync::{
    mpsc, Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    OnceLock,
};
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
    total_workers: usize,
    active_workers: AtomicUsize,
}

static GLOBAL_POOLS: OnceLock<Mutex<HashMap<String, ThreadPool>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, ThreadPool>> {
    GLOBAL_POOLS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct ThreadPool {
    inner: Arc<ThreadPoolInner>,
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

        if let Ok(mut reg) = registry().lock() {
            reg.remove(&self.name);
        }
    }
}

impl ThreadPool {
    pub fn new(name: &str, size: usize) -> Self {
        assert!(size > 0, "ThreadPool '{}' must have at least one worker", name);

        let (tx, rx) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(size);

        let inner = Arc::new(ThreadPoolInner {
            name: name.to_string(),
            sender: tx,
            workers: Mutex::new(Vec::new()),
            total_workers: size,
            active_workers: AtomicUsize::new(0),
        });

        for idx in 0..size {
            let rx = Arc::clone(&receiver);
            let inner_cloned = inner.clone();
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
                            inner_cloned.active_workers.fetch_add(1, Ordering::SeqCst);
                            job();
                            inner_cloned.active_workers.fetch_sub(1, Ordering::SeqCst);
                        }
                        Ok(Message::Shutdown) | Err(_) => {
                            break;
                        }
                    }
                })
                .expect("Failed to spawn worker thread");

            workers.push(Some(handle));
        }

        {
            let mut guard = inner.workers.lock().unwrap();
            *guard = workers;
        }

        let pool = ThreadPool { inner };

        {
            let mut reg = registry().lock().unwrap();
            reg.insert(pool.name().to_string(), pool.clone());
        }

        pool
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

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn total_workers(&self) -> usize {
        self.inner.total_workers
    }

    pub fn active_workers(&self) -> usize {
        self.inner.active_workers.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EndpointPoolMetrics {
    pub total: usize,
    pub active: usize,
}

pub fn all_endpoint_metrics() -> HashMap<String, EndpointPoolMetrics> {
    let reg = registry().lock().unwrap();
    let mut m = HashMap::new();
    for (name, pool) in reg.iter() {
        m.insert(
            name.clone(),
            EndpointPoolMetrics {
                total: pool.total_workers(),
                active: pool.active_workers(),
            },
        );
    }
    m
}

pub fn all_pools() -> Vec<(String, ThreadPool)> {
    let reg = registry().lock().unwrap();
    reg.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
