mod server;
mod errors;
mod http;
mod router; 
mod utils;  
use std::env;
use dotenv::dotenv;
use server::{HttpServer, ServerConfig};
use router::build_routes; 


fn main() {
    dotenv().ok();

    let parse_env_var = |name: &str, default: usize| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    let bind_addr = env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let max_conns = parse_env_var("MAX_CONNECTIONS", 64);
    let rate_limit = parse_env_var("RATE_LIMIT_PER_SEC", 200);

    let cfg = ServerConfig {
        bind_addr,
        max_connections: max_conns,
        rate_limit_per_sec: rate_limit,
    };

    // ⬇️ Build Dispatcher from your router.rs
    let dispatcher = build_routes();

    // ⬇️ Plug dispatcher into server
    let server = HttpServer::with_dispatcher(cfg, dispatcher);

    if let Err(e) = server.run() {
        eprintln!("🛑 Server encountered a fatal error: {}", e);
    }
}
