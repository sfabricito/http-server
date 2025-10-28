use std::time::Instant;

pub fn run_with_timeout<T, F>(timeout_ms: u64, func: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = func();
        let _ = tx.send(result);
    });

    let start = Instant::now();
    loop {
        if let Ok(result) = rx.try_recv() {
            return Some(result);
        }
        if start.elapsed().as_millis() as u64 > timeout_ms {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
