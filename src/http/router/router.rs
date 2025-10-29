use std::env;
use std::sync::Arc;
use std::time::SystemTime;

use crate::{
    http::{
        errors::ServerError,
        handler::{RequestHandlerStrategy, Dispatcher},
        request::HttpRequest,
        response::{Response, OK},
        router::{command, jobs, cpu_bound, io_bound}
    },
    jobs::{
        manager::JobManager,
    },
    
};

pub struct SimpleHandler<F>(pub F);

impl<F> RequestHandlerStrategy for SimpleHandler<F>
where
    F: Fn(&HttpRequest) -> Result<Response, ServerError> + Send + Sync + 'static,
{
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        (self.0)(req)
    }
}


pub fn build_routes(job_manager: Arc<JobManager>) -> Dispatcher {
    let mut builder = Dispatcher::builder();


    // Routes from other modules
    builder = command::register(builder);
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
