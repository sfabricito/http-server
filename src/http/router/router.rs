use std::sync::Arc;
use std::sync::mpsc;

use crate::{
    http::{
        errors::ServerError,
        handler::{RequestHandlerStrategy, Dispatcher},
        request::HttpRequest,
        response::Response,
        router::{command, jobs, cpu_bound, io_bound},
        server::HttpServer, 
    },
    jobs::manager::JobManager,
};

use crate::worker_pool::ThreadPool;

pub struct SimpleHandler<F>(pub F);

impl<F> RequestHandlerStrategy for SimpleHandler<F>
where
    F: Fn(&HttpRequest) -> Result<Response, ServerError> + Send + Sync + 'static,
{
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        (self.0)(req)
    }
}

pub struct PooledHandler {
    pool: ThreadPool,
    handler: Arc<dyn RequestHandlerStrategy>,
}

impl PooledHandler {
    pub fn new(
        pool: ThreadPool,
        handler: Arc<dyn RequestHandlerStrategy>,
    ) -> Self {
        Self { pool, handler }
    }

    pub fn pool(&self) -> &ThreadPool {
        &self.pool
    }
}

impl RequestHandlerStrategy for PooledHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        let (tx, rx) = mpsc::channel();
        let handler = self.handler.clone();
        let request = req.clone();

        self.pool.execute(move || {
            let outcome = handler.handle(&request);
            let _ = tx.send(outcome);
        });

        rx.recv().unwrap_or_else(|_| {
            Err(ServerError::Internal(
                "Worker pool dropped the response channel".into(),
            ))
        })
    }
}


pub fn build_routes(server: Arc<HttpServer>, job_manager: Arc<JobManager>) -> Dispatcher {
    let mut builder = Dispatcher::builder();

    // Routes from other modules
    builder = command::register(builder, server.clone(), job_manager.clone());
    builder = jobs::register(builder, job_manager.clone());
    builder = cpu_bound::register(builder, job_manager.clone());
    builder = io_bound::register(builder, job_manager.clone());

    builder.build()
}

pub trait QueryParam {
    fn query_param(&self, key: &str) -> Option<&str>;
}

impl QueryParam for HttpRequest {
    fn query_param(&self, key: &str) -> Option<&str> {
        if self.query.is_empty() {
            return None;
        }
        for pair in self.query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }
}
