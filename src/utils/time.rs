
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(dur) => dur.as_secs().to_string(),
        Err(_) => "Error getting timestamp".to_string(),
    }
}

pub fn sleep(seconds: u64) {
    thread::sleep(Duration::from_secs(seconds));
}

pub fn simulate(seconds: u64, task: &str) -> String {
    println!("Starting simulation: {}", task);
    thread::sleep(Duration::from_secs(seconds));
    format!("Simulation '{}' completed after {} seconds", task, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_format() {
        let ts = timestamp();
        assert!(ts.parse::<u64>().is_ok());
    }

    #[test]
    fn test_simulate() {
        let msg = simulate(1, "demo");
        assert!(msg.contains("completed"));
    }
}
