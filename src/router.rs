use std::sync::Arc;

use crate::{
    errors::ServerError,
    http::{
        handler::{RequestHandlerStrategy, Dispatcher},
        request::HttpRequest,
        response::{Response, OK},
    },
    utils::{
        math, 
        text, 
        hash, 
        file, 
        time,
        timeout::run_with_timeout,
        cpu::is_prime,
    },
    jobs::{
        manager::JobManager,
        job::JobStatus,
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

    // /fibonacci?num=N
    builder = builder.get("/fibonacci", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let n_str = req.query_param("num")
            .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'num'".into()))?;

        let n = n_str
            .parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("Invalid integer value for 'num': {}", n_str)))?;

        if n > 93 {
            return Err(ServerError::BadRequest("Value too large — risk of overflow".into()));
        }

        let fib = math::fibonacci(n);

        let json_body = format!("{{\"num\": {}, \"fibonacci\": {}}}", n, fib);
        Ok(Response::new(OK)
            .set_header("Content-Type", "application/json")
            .with_body(json_body))
    })));

    // /toupper?text=abcd
    builder = builder.get("/toupper", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let text = req.query_param("text").unwrap_or("");
        Ok(Response::new(OK).with_body(text::to_upper(text)))
    })));

    // /reverse?text=abcdef
    builder = builder.get("/reverse", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let text = req.query_param("text")  .unwrap_or("");
        Ok(Response::new(OK).with_body(text::reverse(text)))
    })));

    // /hash?text=someinput
    builder = builder.get("/hash", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let text = req.query_param("text").unwrap_or("");
        Ok(Response::new(OK).with_body(hash::hash_text(text)))
    })));

    // /timestamp
    builder = builder.get("/timestamp", Arc::new(SimpleHandler(|_req: &HttpRequest| {
        Ok(Response::new(OK).with_body(time::timestamp()))
    })));

    // /simulate?seconds=s&task=name
    builder = builder.get("/simulate", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let secs = req.query_param("seconds").unwrap_or("1").parse::<u64>().unwrap_or(1);
        let task = req.query_param("task").unwrap_or("demo");
        Ok(Response::new(OK).with_body(time::simulate(secs, task)))
    })));

    // /createfile?name=filename&content=text&repeat=x
    builder = builder.get("/createfile", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let name = req.query_param("name").unwrap_or("output.txt");
        let content = req.query_param("content").unwrap_or("Hello");
        let repeat = req.query_param("repeat").unwrap_or("1").parse::<usize>().unwrap_or(1);
        file::create_file(name, content, repeat)?;
        Ok(Response::new(OK).with_body(format!("File '{}' created", name)))
    })));

    // /deletefile?name=filename
    builder = builder.get("/deletefile", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let name = req.query_param("name").unwrap_or("output.txt");
        file::delete_file(name)?;
        Ok(Response::new(OK).with_body(format!("File '{}' deleted", name)))
    })));

    // /random?count=n&min=a&max=b
    builder = builder.get("/random", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let count = req.query_param("count").unwrap_or("5").parse::<usize>().unwrap_or(5);
        let min = req.query_param("min").unwrap_or("0").parse::<i32>().unwrap_or(0);
        let max = req.query_param("max").unwrap_or("100").parse::<i32>().unwrap_or(100);
        let nums = math::random(count, min, max);
        Ok(Response::new(OK).with_body(format!("{:?}", nums)))
    })));

    // /sleep?seconds=s
    builder = builder.get("/sleep", Arc::new(SimpleHandler(|req: &HttpRequest| {
        let secs = req.query_param("seconds").unwrap_or("1").parse::<u64>().unwrap_or(1);
        time::sleep(secs);
        Ok(Response::new(OK).with_body(format!("Slept {} seconds", secs)))
    })));

    //help
    builder = builder.get("/help", Arc::new(SimpleHandler(|_req: &HttpRequest| {
        Ok(Response::new(OK).with_body(text::help()))
    })));

    // /status
    builder = builder.get("/status", Arc::new(SimpleHandler(|_req: &HttpRequest| {
        Ok(Response::new(OK).with_body("Server running OK"))
    })));


    println!("✅ Router loaded!");

    builder.build()
}


trait QueryParam {
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
