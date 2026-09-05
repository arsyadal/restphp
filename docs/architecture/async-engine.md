# Async Tokio & Axum Engine

The front-end HTTP server of RestPHP is built using **Tokio** and **Axum**, providing asynchronous I/O with near-zero connection overhead.

---

## High-Concurrency Connection Pooling

In `src/server.rs`:
- Axum accepts incoming HTTP/1.1 and HTTP/2 connections asynchronously.
- Non-blocking I/O allows a single process to hold open tens of thousands of idle client sockets without thread exhaustion.
- Incoming HTTP requests are converted into `WorkerJob` envelopes containing method, URI, query parameters, headers, cookies, and body bytes.

---

## Lock-Free Request Dispatch

Requests are queued to the worker thread pool using `crossbeam-channel`:
- Workers poll the bounded channel without mutual-exclusion lock contention.
- Once a worker completes request execution, it transmits the `PhpResponse` back to Axum's async task using a lightweight `tokio::sync::oneshot` channel.
- Axum streams the response status, headers, and body back to the HTTP client with zero duplicate allocations.
