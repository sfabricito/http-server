use std::{
    collections::VecDeque,
    fs::File,
    io::{self, Read, Write, ErrorKind},
    os::fd::{FromRawFd, RawFd},
    sync::{
        Arc,
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use libc::{
    self, c_int, sockaddr, sockaddr_in, socklen_t,
    AF_INET, SOCK_STREAM, SOL_SOCKET, SO_REUSEADDR,
};

use crate::{
    errors::ServerError,
    http::{
        handler::Dispatcher,
        request::{HttpRequest, HttpMethod},
        response::{
            Status, Response,
            BAD_REQUEST, NOT_FOUND, CONFLICT,
            TOO_MANY_REQUESTS, INTERNAL_SERVER_ERROR, SERVICE_UNAVAILABLE,
        },
    },
};


pub struct ServerConfig { pub bind_addr: String, pub max_connections: usize, pub rate_limit_per_sec: usize }
impl Default for ServerConfig { fn default() -> Self { Self{ bind_addr: "127.0.0.1:8080".into(), max_connections: 64, rate_limit_per_sec: 200 } } }

pub struct HttpServer {
    pub cfg: ServerConfig,
    pub dispatcher: Arc<Dispatcher>,
    active: Arc<AtomicUsize>,
    window: Arc<Mutex<VecDeque<Instant>>>,
}

impl HttpServer {
    pub fn new(cfg: ServerConfig) -> Self { Self::with_dispatcher(cfg, Dispatcher::new()) }

    pub fn with_dispatcher(cfg: ServerConfig, dispatcher: Dispatcher) -> Self {
        Self {
            cfg,
            dispatcher: Arc::new(dispatcher),
            active: Arc::new(AtomicUsize::new(0)),
            window: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn run(&self) -> io::Result<()> {
        let (ip, port) = parse_ipv4_addr(&self.cfg.bind_addr)?;
        let listen_fd = create_listen_socket(ip, port)?;
        println!("🚀 Listening on {}", self.cfg.bind_addr);

        loop {
            let client_fd = match Self::accept_client(listen_fd) {
                Ok(fd) => fd,
                Err(e) => {
                    eprintln!("Accept error: {e}");
                    continue;
                }
            };

            if self.active.load(Ordering::SeqCst) >= self.cfg.max_connections {
                Self::reject_client(client_fd, SERVICE_UNAVAILABLE, "Service Unavailable: too many connections");
                continue;
            }

            if self.is_rate_limited() {
                Self::reject_client(client_fd, TOO_MANY_REQUESTS, "Too Many Requests");
                continue;
            }

            self.active.fetch_add(1, Ordering::SeqCst);
            let dispatcher = Arc::clone(&self.dispatcher);
            let active = Arc::clone(&self.active);

            thread::spawn(move || {
                if let Err(e) = Self::serve_client(client_fd, dispatcher) {
                    eprintln!("Error handling connection: {e}");
                }
                active.fetch_sub(1, Ordering::SeqCst);
            });
        }
    }

    fn accept_client(listen_fd: i32) -> io::Result<i32> {
        let mut addr: sockaddr_in = unsafe { std::mem::zeroed() };
        let mut addr_len = std::mem::size_of::<sockaddr_in>() as socklen_t;

        let fd = unsafe {
            libc::accept(
                listen_fd,
                (&mut addr as *mut sockaddr_in).cast::<sockaddr>(),
                &mut addr_len,
            )
        };

        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    }

    fn serve_client(fd: i32, dispatcher: Arc<Dispatcher>) -> Result<(), ServerError> {
        let mut stream = unsafe { File::from_raw_fd(fd) };
        handle_connection(&mut stream, &dispatcher)
    }

    fn reject_client(fd: i32, status: Status, message: &str) {
        unsafe {
            let mut stream = std::fs::File::from_raw_fd(fd);
            let response = Response::new(status).with_body(message);
            let _ = stream.write_all(&response.to_bytes(false));
            let _ = stream.flush();
        }
    }

    fn is_rate_limited(&self) -> bool {
        let now = Instant::now();
        let mut window = self.window.lock().expect("rate limiter mutex");

        while let Some(&front) = window.front() {
            if now.duration_since(front) > Duration::from_secs(1) {
                window.pop_front();
            } else {
                break;
            }
        }

        if window.len() >= self.cfg.rate_limit_per_sec {
            true
        } else {
            window.push_back(now);
            false
        }
    }
}

fn handle_connection<RW: Read + Write>(
    rw: &mut RW,
    dispatcher: &Dispatcher
) -> Result<(), ServerError> {
    match HttpRequest::parse(rw) {
        Ok(req) => {
            let is_head = matches!(req.method, HttpMethod::HEAD);

            let resp = match dispatcher.dispatch(&req) {
                Ok(r) => r,
                Err(err) => {
                    let status = match err {
                        ServerError::BadRequest(_) => BAD_REQUEST,
                        ServerError::NotFound => NOT_FOUND,
                        ServerError::Conflict(_) => CONFLICT,
                        ServerError::TooManyRequests => TOO_MANY_REQUESTS,
                        ServerError::ServiceUnavailable => SERVICE_UNAVAILABLE,
                        ServerError::Internal(_) | ServerError::Io(_) => INTERNAL_SERVER_ERROR,
                    };

                    let json_body = format!("{{\"error\": \"{}\"}}", err.to_string());
                    Response::new(status)
                        .set_header("Content-Type", "application/json")
                        .with_body(json_body)
                }
            };

            let bytes = resp.to_bytes(is_head);
            let _ = rw.write_all(&bytes);
            let _ = rw.flush();
            Ok(())
        }

        Err(e) => {
            let status = match e {
                ServerError::BadRequest(_) => BAD_REQUEST,
                ServerError::NotFound => NOT_FOUND,
                ServerError::Conflict(_) => CONFLICT,
                ServerError::TooManyRequests => TOO_MANY_REQUESTS,
                ServerError::ServiceUnavailable => SERVICE_UNAVAILABLE,
                ServerError::Internal(_) | ServerError::Io(_) => INTERNAL_SERVER_ERROR,
            };

            let json_body = format!("{{\"error\": \"{}\"}}", e.to_string());
            let resp = Response::new(status)
                .set_header("Content-Type", "application/json")
                .with_body(json_body);

            let _ = rw.write_all(&resp.to_bytes(false));
            let _ = rw.flush();
            Ok(())
        }
    }
}

pub fn create_listen_socket(ip_host: u32, port_host: u16) -> io::Result<RawFd> {
    let fd = unsafe { libc::socket(AF_INET, SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    // Allow immediate reuse of port
    let opt: c_int = 1;
    unsafe {
        libc::setsockopt(
            fd,
            SOL_SOCKET,
            SO_REUSEADDR,
            (&opt as *const c_int).cast(),
            std::mem::size_of_val(&opt) as socklen_t,
        );
    }

    let mut addr: sockaddr_in = unsafe { std::mem::zeroed() };
    addr.sin_family = AF_INET as u16;
    addr.sin_port = port_host.to_be();     // convert port to network byte order
    addr.sin_addr.s_addr = ip_host;        // use ip as-is (already parsed)

    let rc = unsafe {
        libc::bind(
            fd,
            (&addr as *const sockaddr_in).cast::<sockaddr>(),
            std::mem::size_of::<sockaddr_in>() as socklen_t,
        )
    };
    if rc < 0 {
        let e = io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(e);
    }

    let rc = unsafe { libc::listen(fd, 128) };
    if rc < 0 {
        let e = io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(e);
    }

    Ok(fd)
}

fn create_parse_error(msg: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, msg)
}

fn parse_ipv4_addr(addr: &str) -> io::Result<(u32, u16)> {
    let split = addr.trim();

    let (host_str, port_str) = split.rsplit_once(':')
        .ok_or_else(|| create_parse_error("Address format must be 'HOST:PORT'"))?;
    
    let host_str = host_str.trim();
    let port_str = port_str.trim();

    let port: u16 = port_str.parse()
        .map_err(|_| create_parse_error(&format!("Invalid port value: '{}'", port_str)))?;

    let final_host_str = match host_str {
        "*" | "0.0.0.0" => {
            return Ok((0u32, port.to_be()));
        }
        host if host.eq_ignore_ascii_case("localhost") => "127.0.0.1",
        host => host,
    };

    let mut octets: [u8; 4] = [0; 4];
    
    for (i, part) in final_host_str.split('.').enumerate() {
        if i >= 4 { 
            return Err(create_parse_error(&format!("Invalid IPv4 format: '{}' has too many octets", final_host_str)));
        }

        let octet_val = part.parse::<u8>()
            .map_err(|_| create_parse_error(&format!("Invalid octet value: '{}'", part)))?;
        
        octets[i] = octet_val;
    }

    if final_host_str.split('.').count() != 4 {
        return Err(create_parse_error(&format!("Invalid IPv4 format: '{}' must have 4 octets", final_host_str)));
    }

    let ip_host: u32 = u32::from_ne_bytes(octets);
    let port_host: u16 = port;

    Ok((ip_host, port_host))
}   