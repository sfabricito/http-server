use std::env;
use dotenv::dotenv;
use HTTP_Server::{
    http::{
        router::router::build_routes,
        server::{HttpServer, ServerConfig},
    },
    jobs::manager::JobManager,
};

fn main() {
    dotenv().ok();

    let parse_env_var = |name: &str, default: usize| {
        env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    let bind_addr = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let max_conns = parse_env_var("MAX_CONNECTIONS", 64);
    let rate_limit = parse_env_var("RATE_LIMIT_PER_SEC", 200);
    let cpu_workers = parse_env_var("CPU_WORKERS", 4);
    let io_workers = parse_env_var("IO_WORKERS", 2);

    let cfg = ServerConfig {
        bind_addr,
        max_connections: max_conns,
        rate_limit_per_sec: rate_limit,
    };

    let job_manager = JobManager::new(cpu_workers, io_workers);

    let dispatcher = build_routes(job_manager.clone());
    let server = HttpServer::with_dispatcher(cfg, dispatcher);

    if let Err(e) = server.run() {
        eprintln!("🛑 Server encountered a fatal error: {}", e);
    }
}
