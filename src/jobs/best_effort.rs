use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

use crate::jobs::job::Priority;
use crate::jobs::manager::{JobError, JobManager};
use crate::worker_pool;

#[derive(Clone)]
struct WorkerResult {
    json: String,
    worker_pid: Option<i32>,
}

pub enum BestEffortOutcome {
    Completed { json: String, worker_pid: Option<i32> },
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
    let (tx, rx) = sync_channel::<Result<WorkerResult, String>>(1);
    let job_id_slot = Arc::new(Mutex::new(None::<String>));
    let job_id_for_thread = Arc::clone(&job_id_slot);
    let manager_for_thread = job_manager.clone();

    thread::spawn(move || {
        worker_pool::register_worker_thread();
        let result = exec().map(|json| WorkerResult {
            worker_pid: worker_pool::current_worker_pid(),
            json,
        });
        if tx.send(result.clone()).is_err() {
            if let Some(job_id) = job_id_for_thread.lock().unwrap().clone() {
                match result {
                    Ok(data) => manager_for_thread.mark_job_done(&job_id, Ok(data.json)),
                    Err(err) => manager_for_thread.mark_job_done(&job_id, Err(err)),
                }
            }
        }
        worker_pool::clear_worker_thread();
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(result)) => Ok(BestEffortOutcome::Completed {
            json: result.json,
            worker_pid: result.worker_pid,
        }),
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
