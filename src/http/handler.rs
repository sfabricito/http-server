use std::collections::HashMap;
use std::sync::Arc;

use crate::errors::ServerError;
use super::request::{HttpMethod, HttpRequest};
use super::response::{Response, OK};

pub trait RequestHandlerStrategy: Send + Sync + 'static {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError>;
}

pub struct GetHandler;
impl RequestHandlerStrategy for GetHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        match req.path.as_str() {
            "/" => Ok(Response::new(OK)
                .set_header("Content-Type", "text/html; charset=utf-8")
                .with_body("<html><body><h1>GET /</h1><p>Hello from GET!</p></body></html>")),
            "/conflict" => Err(ServerError::Conflict("Simulated conflict".into())),
            _ => Err(ServerError::NotFound),
        }
    }
}

pub struct HeadHandler;
impl RequestHandlerStrategy for HeadHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        match req.path.as_str() {
            "/" => Ok(Response::new(OK)
                .set_header("Content-Type", "text/html; charset=utf-8")
                .with_body("<html><body><h1>HEAD /</h1><p>Headers only</p></body></html>")),
            _ => Err(ServerError::NotFound),
        }
    }
}

pub struct PostHandler;
impl RequestHandlerStrategy for PostHandler {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        if req.headers.get("Content-Type").map(|v| v.starts_with("text/plain")).unwrap_or(true) {
            let body = String::from_utf8_lossy(&req.body).into_owned();
            Ok(Response::new(OK)
                .set_header("Content-Type", "text/plain; charset=utf-8")
                .with_body(format!("You POSTed: {}", body)))
        } else {
            Err(ServerError::BadRequest("Only text/plain supported".into()))
        }
    }
}

pub struct Dispatcher {
    get: Arc<dyn RequestHandlerStrategy>,
    head: Arc<dyn RequestHandlerStrategy>,
    post: Arc<dyn RequestHandlerStrategy>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self { get: Arc::new(GetHandler), head: Arc::new(HeadHandler), post: Arc::new(PostHandler) }
    }
    pub fn builder() -> DispatcherBuilder { DispatcherBuilder::default() }
    pub fn dispatch(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        match req.method { HttpMethod::GET=>self.get.handle(req), HttpMethod::HEAD=>self.head.handle(req), HttpMethod::POST=>self.post.handle(req), HttpMethod::Unsupported(ref m)=>Err(ServerError::BadRequest(format!("Unsupported method: {}", m))) }
    }
}

#[derive(Default)]
pub struct DispatcherBuilder {
    get_map: HashMap<String, Arc<dyn RequestHandlerStrategy>>,
    head_map: HashMap<String, Arc<dyn RequestHandlerStrategy>>,
    post_map: HashMap<String, Arc<dyn RequestHandlerStrategy>>,
}

impl DispatcherBuilder {
    pub fn get(mut self, path: &str, handler: Arc<dyn RequestHandlerStrategy>) -> Self { self.get_map.insert(path.to_string(), handler); self }
    pub fn head(mut self, path: &str, handler: Arc<dyn RequestHandlerStrategy>) -> Self { self.head_map.insert(path.to_string(), handler); self }
    pub fn post(mut self, path: &str, handler: Arc<dyn RequestHandlerStrategy>) -> Self { self.post_map.insert(path.to_string(), handler); self }

    pub fn build(self) -> Dispatcher {
        // Build a dispatcher that first checks explicit maps; fallback to defaults
        struct MapHandler { map: HashMap<String, Arc<dyn RequestHandlerStrategy>> }
        impl RequestHandlerStrategy for MapHandler {
            fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError> {
                if let Some(h) = self.map.get(&req.path) { h.handle(req) } else { Err(ServerError::NotFound) }
            }
        }

        let get: Arc<dyn RequestHandlerStrategy> = if self.get_map.is_empty() {
            Arc::new(GetHandler)
        } else {
            Arc::new(MapHandler { map: self.get_map })
        };
        let head: Arc<dyn RequestHandlerStrategy> = if self.head_map.is_empty() {
            Arc::new(HeadHandler)
        } else {
            Arc::new(MapHandler { map: self.head_map })
        };
        let post: Arc<dyn RequestHandlerStrategy> = if self.post_map.is_empty() {
            Arc::new(PostHandler)
        } else {
            Arc::new(MapHandler { map: self.post_map })
        };

        Dispatcher { get, head, post }
    }
}
