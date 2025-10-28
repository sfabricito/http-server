use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub fn run_with_timeout<T, F>(timeout_ms: u64, func: F) -> Option<(T, u128)>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let start = Instant::now();
        let result = func();
        let elapsed = start.elapsed().as_millis();
        let _ = tx.send((result, elapsed));
    });

    rx.recv_timeout(Duration::from_millis(timeout_ms)).ok()
}
