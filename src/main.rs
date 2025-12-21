mod http;
mod threads;

use http::{Response, Router, Server, ServerConfig};

fn main() {
    let router = Router::new()
        .get("/", |_| Response::html("<h1>Welcome to RustServe!</h1>"))
        .get("/health", |_| (200, r#"{"status": "ok"}"#))
        .post("/echo", |req| match req.body() {
            Some(body) => Response::ok(body.to_vec()),
            None => Response::bad_request(),
        });

    let config = ServerConfig::new("127.0.0.1", 7878).threads(20);

    match Server::new(config) {
        Ok(server) => server.run(router),
        Err(e) => eprintln!("Failed to start server: {}", e),
    }
}
