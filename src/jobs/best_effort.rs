use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

use crate::jobs::job::Priority;
use crate::jobs::manager::{JobError, JobManager};

pub enum BestEffortOutcome {
    Completed(String),
    Offloaded { job_id: String },
}

pub enum BestEffortError {
    HandlerFailed(String),
    QueueFull { retry_after_ms: u64 },
    Internal(String),
}

pub fn execute<F>(
    job_manager: Arc<JobManager>,
    task: &str,
    params: HashMap<String, String>,
    priority: Priority,
    timeout: Duration,
    exec: F,
) -> Result<BestEffortOutcome, BestEffortError>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    let (tx, rx) = sync_channel::<Result<String, String>>(1);
    let job_id_slot = Arc::new(Mutex::new(None::<String>));
    let job_id_for_thread = Arc::clone(&job_id_slot);
    let manager_for_thread = job_manager.clone();

    thread::spawn(move || {
        let result = exec();
        if tx.send(result.clone()).is_err() {
            if let Some(job_id) = job_id_for_thread.lock().unwrap().clone() {
                match result {
                    Ok(json) => manager_for_thread.mark_job_done(&job_id, Ok(json)),
                    Err(err) => manager_for_thread.mark_job_done(&job_id, Err(err)),
                }
            }
        }
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(json)) => Ok(BestEffortOutcome::Completed(json)),
        Ok(Err(err)) => Err(BestEffortError::HandlerFailed(err)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let job = job_manager
                .register_inflight(task, params, priority)
                .map_err(|err| match err {
                    JobError::QueueFull { retry_after_ms } => {
                        BestEffortError::QueueFull { retry_after_ms }
                    }
                    JobError::Unknown(msg) => BestEffortError::Internal(msg),
                })?;

            let job_id = job.id.clone();
            *job_id_slot.lock().unwrap() = Some(job_id.clone());
            drop(rx);

            Ok(BestEffortOutcome::Offloaded { job_id })
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(BestEffortError::Internal(
                "Computation thread terminated unexpectedly".into(),
            ))
        }
    }
}
