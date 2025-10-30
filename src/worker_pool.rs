use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{
    mpsc, Arc, Mutex,
    atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
    OnceLock,
};
use std::thread;

thread_local! {
    static CURRENT_WORKER_PID: Cell<i32> = Cell::new(0);
}

#[cfg(target_os = "linux")]
fn gettid() -> i32 {
    // SAFETY: direct syscall; returns thread id (TID) on Linux
    unsafe { libc::syscall(libc::SYS_gettid) as i32 }
}
#[cfg(not(target_os = "linux"))]
fn gettid() -> i32 {
    // Fallback: use process id if no gettid (not unique per thread, but deterministic)
    std::process::id() as i32
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Job),
    Shutdown,
}

#[derive(Default)]
struct WorkerInfo {
    tid: AtomicI32,      // OS thread id (TID on Linux, PID fallback elsewhere)
    busy: AtomicBool,    // true when executing a job
    name: String,        // thread name for debugging
}

struct ThreadPoolInner {
    name: String,
    sender: mpsc::Sender<Message>,
    workers: Mutex<Vec<Option<thread::JoinHandle<()>>>>,
    infos: Vec<Arc<WorkerInfo>>, // one per worker
    total_workers: usize,
    active_workers: AtomicUsize,
}

static GLOBAL_POOLS: OnceLock<Mutex<HashMap<String, ThreadPool>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, ThreadPool>> {
    GLOBAL_POOLS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn set_current_worker_pid(pid: i32) {
    CURRENT_WORKER_PID.with(|cell| cell.set(pid));
}

pub fn register_worker_thread() {
    let pid = gettid();
    set_current_worker_pid(pid);
}

pub fn clear_worker_thread() {
    set_current_worker_pid(0);
}

pub fn current_worker_pid() -> Option<i32> {
    let pid = CURRENT_WORKER_PID.with(|cell| cell.get());
    if pid > 0 { Some(pid) } else { None }
}

#[derive(Clone)]
pub struct ThreadPool {
    inner: Arc<ThreadPoolInner>,
}

impl Drop for ThreadPoolInner {
    fn drop(&mut self) {
        let worker_len = self.workers.lock().unwrap().len();

        // Tell everyone to shut down
        for _ in 0..worker_len {
            let _ = self.sender.send(Message::Shutdown);
        }

        // Join them
        let mut workers = self.workers.lock().unwrap();
        for handle_opt in workers.iter_mut() {
            if let Some(handle) = handle_opt.take() {
                let _ = handle.join();
            }
        }

        // Remove from registry
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
        let mut infos = Vec::with_capacity(size);

        let inner = Arc::new(ThreadPoolInner {
            name: name.to_string(),
            sender: tx,
            workers: Mutex::new(Vec::new()),
            infos: Vec::new(),
            total_workers: size,
            active_workers: AtomicUsize::new(0),
        });

        for idx in 0..size {
            let rx = Arc::clone(&receiver);
            let inner_cloned = inner.clone();
            let thread_name = format!("{}-worker-{}", name, idx);

            let info = Arc::new(WorkerInfo {
                tid: AtomicI32::new(0),
                busy: AtomicBool::new(false),
                name: thread_name.clone(),
            });
            infos.push(info.clone());

            let handle = thread::Builder::new()
                .name(thread_name.clone())
                .spawn(move || {
                    register_worker_thread();
                    // record OS thread id
                    let tid = gettid();
                    info.tid.store(tid, Ordering::SeqCst);

                    loop {
                        let message = {
                            let guard = rx.lock().expect("worker receiver lock poisoned");
                            guard.recv()
                        };

                        match message {
                            Ok(Message::Run(job)) => {
                                info.busy.store(true, Ordering::SeqCst);
                                inner_cloned.active_workers.fetch_add(1, Ordering::SeqCst);

                                job();

                                inner_cloned.active_workers.fetch_sub(1, Ordering::SeqCst);
                                info.busy.store(false, Ordering::SeqCst);
                            }
                            Ok(Message::Shutdown) | Err(_) => {
                                // best effort: mark idle on exit
                                info.busy.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                    }

                    clear_worker_thread();
                })
                .expect("Failed to spawn worker thread");

            workers.push(Some(handle));
        }

        {
            let mut guard = inner.workers.lock().unwrap();
            *guard = workers;
        }
        {
            // store infos
            let inner_mut = Arc::as_ptr(&inner) as *mut ThreadPoolInner;
            unsafe {
                (*inner_mut).infos = infos;
            }
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

    pub fn per_worker_snapshots(&self) -> Vec<WorkerSnapshot> {
        self.inner
            .infos
            .iter()
            .map(|info| WorkerSnapshot {
                name: info.name.clone(),
                pid: info.tid.load(Ordering::SeqCst),
                state: if info.busy.load(Ordering::SeqCst) { "busy" } else { "idle" }.to_string(),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct EndpointPoolMetrics {
    pub total: usize,
    pub active: usize,
}

#[derive(Clone, Debug)]
pub struct WorkerSnapshot {
    pub name: String,
    pub pid: i32,
    pub state: String,
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

pub fn all_endpoint_workers_detail() -> HashMap<String, Vec<WorkerSnapshot>> {
    let reg = registry().lock().unwrap();
    let mut m = HashMap::new();
    for (name, pool) in reg.iter() {
        m.insert(name.clone(), pool.per_worker_snapshots());
    }
    m
}

pub fn all_pools() -> Vec<(String, ThreadPool)> {
    let reg = registry().lock().unwrap();
    reg.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
