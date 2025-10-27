# HTTP-Server

**A Multithreaded Raw HTTP/1.0 Server for Concurrent CPU & IO-Bound Task Processing**

**Course:** Principles of Operating Systems  
**Professor:** Kenneth Obando  
**Authors:** Fabricio Solis Alpizar, Pavel Zamora Araya 
**License:** MIT

---

## Overview

This project implements a fully-functional **HTTP/1.0 server from scratch** using raw TCP sockets, compliant request parsing, and manual response serialization. Developed in Rust, a reliable systems programming language, that ensures memory safety and high performance.
The project is designed as a learning exercise for Operating Systems concepts, focusing on concurrency, synchronization, scheduling, and workload management.
It demonstrates fundamental Operating System concepts including:

- Process/thread concurrency  
- Synchronization primitives  
- Scheduling and queueing  
- Rate limiting and backpressure  
- Worker pools per command type  
- CPU-bound and IO-bound workloads  
- Job abstraction (async polling model)   
- Performance metrics and observability  

> No HTTP frameworks, embedded servers, or high-level libraries are used — all network operations are implemented manually.

The server is scalable, supports parallel execution of workloads, and exposes an extensible routing layer for dispatching commands across worker pools.

---

## Features Summary

✅ Raw HTTP/1.0 protocol  
✅ Multi-client concurrency (thread-per-connection)  
✅ Rate limiting via sliding-window  
✅ Connection limit enforcement  
✅ Routing and handler dispatch  
✅ Worker pools per command  
✅ Backpressure on overload  
✅ Priority job scheduling  
✅ CPU-bound tasks  
✅ IO-bound tasks  
✅ Real-time metrics reporting  
✅ `/jobs/*` asynchronous execution model  
✅ CLI & environment configuration

---

## Project Structure

```
HTTP-Server/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── data/
│   ├── jobs.db
│   └── test_files/
├── src/
│   ├── main.rs
│   ├── server.rs
│   ├── http/
│   │   ├── handler.rs
│   │   ├── request.rs
│   │   ├── response.rs
│   │   └── router.rs
│   ├── util/
│   │   ├── math.rs
│   │   ├── file.rs
│   │   ├── text.rs
│   │   ├── hash.rs
│   │   └── time.rs
│   ├── workers/
│   │   ├── pool.rs
│   │   └── mod.rs
│   ├── jobs/
│   │   ├── mod.rs
│   │   ├── queue.rs
│   │   ├── job.rs
│   │   └── api.rs
│   ├── metrics/
│   │   ├── mod.rs
│   │   └── reporter.rs
│   └── trace.rs
└── tests/
    ├── basic.rs
    ├── concurrency.rs
    ├── jobs.rs
    └── io.rs
```

---

## Requirements

- Linux (Ubuntu recommended)  
- Rust stable toolchain  
- Build tools (`build-essential`)  
- Bash/Zsh terminal  
- Curl or Postman

---

## Build & Run

```bash
cargo build --release
./target/release/HTTP-Server
```

or with environment variables:

```bash
BIND_ADDRESS="0.0.0.0:8080" MAX_CONNECTIONS=128 RUNTIME_STATS=1 \
./target/release/HTTP-Server
```

---

##  HTTP Endpoints

###  Short CPU Tasks

| Endpoint | Description |
|-----------|--------------|
| `/fibonacci?num=N` | Computes Fibonacci(N) |
| `/isprime?n=N` | Primality test |
| `/factor?n=N` | Prime decomposition |
| `/pi?digits=D` | Computes π to D digits |
| `/matrixmul?size=N&seed=S` | Matrix multiplication with SHA verification |
| `/mandelbrot?...` | Generates fractal iteration map |

---

### Text Utilities

| Endpoint | Description |
|-----------|--------------|
| `/reverse?text=abc` | Reverse string |
| `/toupper?text=rust` | Uppercase conversion |

---

### IO-Bound Commands

| Endpoint | Description |
|-----------|--------------|
| `/createfile?name=x&content=y&repeat=z` | File generation |
| `/deletefile?name=x` | File removal |
| `/sortfile?name=f&algo=merge` | External sort |
| `/wordcount?name=f` | Lines, words, bytes |
| `/grep?name=f&pattern=…` | Regex match |
| `/compress?name=f&codec=gzip` | Compression |
| `/hashfile?name=f&algo=sha256` | Hash large files |

---

### ⏱ Timing & Simulation

| Endpoint | Description |
|-----------|--------------|
| `/timestamp` | UNIX epoch timestamp |
| `/sleep?seconds=S` | Artificial delay |
| `/simulate?seconds=S&task=T` | Simulates job work |

---

### Load Testing

| Endpoint | Description |
|-----------|--------------|
| `/loadtest?tasks=N&sleep=X` | Spawns N concurrent tasks sleeping X seconds |

> Demonstrates concurrency: total runtime ≈ X seconds regardless of N.

---

## Job Execution Model (Long Tasks)

| Endpoint | Description |
|-----------|--------------|
| `/jobs/submit?task=...` | Enqueue long-running job |
| `/jobs/status?id=UUID` | Poll status/progress |
| `/jobs/result?id=UUID` | Fetch result |
| `/jobs/cancel?id=UUID` | Cancel job |

Jobs survive graceful restart via **ephemeral journal** in `data/jobs.db`.

---

## Metrics & Observability

| Endpoint | Description |
|-----------|--------------|
| `/metrics` | p50/p95/p99 latency, worker occupancy, queue depth |
| `/status` | uptime, PID, active connections, worker state |

---

## Concurrency Model

- **Thread-per-connection**
- **Worker pools per command type**
- **FIFO queues with priority classes:** low, normal, high  
- **Backpressure** applied when queue depth > threshold:  
  - server responds **503 Service Unavailable**
  - includes `retry_after_ms` hint  
- Atomic counters track active connections

---

## Synchronization

- `Arc<Mutex<…>>`  
- `Arc<AtomicUsize>`  
- Lock-free counters where appropriate  
- No busy-waiting  
- No sleeping for synchronization

---

## Backpressure Behavior

When load is too high:

- Queue accumulation triggers **HTTP 429**
- Connection limit triggers **HTTP 503**
- Job timeout triggers graceful cancellation

---

## Example Usage (curl)

Some short tasks:

```bash
curl "http://127.0.0.1:8080/reverse?text=hello"
curl "http://127.0.0.1:8080/random?count=5&min=1&max=50"
curl "http://127.0.0.1:8080/sleep?seconds=3"
curl "http://127.0.0.1:8080/fibonacci?num=40"
```

Submit job:

```bash
curl "http://127.0.0.1:8080/jobs/submit?task=isprime&n=9007199254740881"
```

Poll status:

```bash
curl "http://127.0.0.1:8080/jobs/status?id=<uuid>"
```

---

## Architecture Diagram (Simplified)

```
   +----------------------+
   | Incoming TCP Clients |
   +----------+-----------+
              |
              v
      +-------+--------+
      |   Accept Loop  |
      +-------+--------+
              |
              v
      +-------+--------+
      |     Router     |
      +-------+--------+
              |
        +-----+-----+
        |           |
        v           v
 +------+------+ +--+------------+
 | Short Tasks | | Job Manager   |
 +------+------+ +------+--------+
        |              |
        v              v
 +------+-----+   +----+-------+
 | WorkerPool |   | Queue/Prio |
 +------+-----+   +------------+
        |
        v
    +---+------+
    | Execution|
    +----------+
```

---

## Testing

Run unit + integration tests:

```bash
cargo test -- --nocapture
```

Load testing scripts:

```bash
./scripts/loadtest.sh
```

Large file operations rely on:

```bash
./scripts/gen_bigfile.rs
```

> Coverage target ≥ **90%**

---

## Error Handling

| Code | Meaning |
|------|----------|
| 200 | OK |
| 400 | Bad Request |
| 404 | Not Found |
| 409 | Conflict |
| 429 | Too Many Requests |
| 500 | Server Error |
| 503 | Service Unavailable |

> Response bodies always include machine-readable messages.

---

## Troubleshooting

**Nothing returned?**  
Your shell may pipe output — add `-0` (HTTP/1.0):

```bash
curl -0 ...
```

**Too many concurrent connections?**  
Increase:
```bash
MAX_CONNECTIONS=256
```

**Overloaded workers?**  
Use:
```bash
/jobs/status
/metrics
```

---

## Security Notes

- No HTTPS (spec requirement)
- No user isolation
- Trust boundary assumed local/private network
- Input validation applied on params

---

## License

MIT License — see `LICENSE` file.

---

## Conclusion

This HTTP/1.0 server demonstrates:

- OS-level concurrency  
- Network programming  
- Workload scheduling  
- CPU and IO performance handling  
- Asynchronous job management  
- Observability instrumentation  

It provides a solid foundation for systems-level performance experimentation and **horizontal scalability**.
Feel free to explore, modify, and extend! 
A investigation paper detailing design decisions and performance analysis is avaiable at:  
[Design and Performance Analysis of a Multithreaded HTTP/1.0 Server for Concurrent CPU and IO-Bound Task Processing](https://example.com/research-paper)

You can reach us at:  
- Fabricio Solis Alpizar: 
- Pavel Zamora Araya: 