use std::any::Any;
use std::sync::Arc;
use std::collections::HashMap;
use super::response::{Response, OK};
use crate::http::errors::ServerError;
use super::request::{HttpMethod, HttpRequest};

pub trait RequestHandlerStrategy: Any + Send + Sync + 'static {
    fn handle(&self, req: &HttpRequest) -> Result<Response, ServerError>;
}

impl dyn RequestHandlerStrategy {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
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
    get_default: Arc<dyn RequestHandlerStrategy>,
    head_default: Arc<dyn RequestHandlerStrategy>,
    post_default: Arc<dyn RequestHandlerStrategy>,
    get_routes: HashMap<String, Arc<dyn RequestHandlerStrategy>>,
    head_routes: HashMap<String, Arc<dyn RequestHandlerStrategy>>,
    post_routes: HashMap<String, Arc<dyn RequestHandlerStrategy>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            get_default: Arc::new(GetHandler),
            head_default: Arc::new(HeadHandler),
            post_default: Arc::new(PostHandler),
            get_routes: HashMap::new(),
            head_routes: HashMap::new(),
            post_routes: HashMap::new(),
        }
    }
    pub fn builder() -> DispatcherBuilder { DispatcherBuilder::default() }
    pub fn dispatch(&self, req: &HttpRequest) -> Result<Response, ServerError> {
        match req.method {
            HttpMethod::GET => {
                if self.get_routes.is_empty() {
                    self.get_default.handle(req)
                } else if let Some(handler) = self.get_routes.get(&req.path) {
                    handler.handle(req)
                } else {
                    Err(ServerError::NotFound)
                }
            }
            HttpMethod::HEAD => {
                if self.head_routes.is_empty() {
                    self.head_default.handle(req)
                } else if let Some(handler) = self.head_routes.get(&req.path) {
                    handler.handle(req)
                } else {
                    Err(ServerError::NotFound)
                }
            }
            HttpMethod::POST => {
                if self.post_routes.is_empty() {
                    self.post_default.handle(req)
                } else if let Some(handler) = self.post_routes.get(&req.path) {
                    handler.handle(req)
                } else {
                    Err(ServerError::NotFound)
                }
            }
            HttpMethod::Unsupported(ref m) => {
                Err(ServerError::BadRequest(format!("Unsupported method: {}", m)))
            }
        }
    }

    pub fn routes(&self) -> Vec<(String, Arc<dyn RequestHandlerStrategy>)> {
        self.get_routes
            .iter()
            .map(|(path, handler)| (path.clone(), Arc::clone(handler)))
            .collect()
    }
}

#[derive(Default)]
#[derive(Clone)]
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
        Dispatcher {
            get_default: Arc::new(GetHandler),
            head_default: Arc::new(HeadHandler),
            post_default: Arc::new(PostHandler),
            get_routes: self.get_map,
            head_routes: self.head_map,
            post_routes: self.post_map,
        }
    }
}
