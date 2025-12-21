mod http;
mod threads;

use http::{Response, Router, Server, ServerConfig};

fn main() {
    let v1 = Router::prefix("/v1")
        .get("/users", |_| Response::ok("List of users from v1"))
        .get("/posts", |_| Response::ok("List of posts from v1"));

    let v2 = Router::prefix("/v2").get("/users", |_| Response::ok("List of users from v2"));

    let api = Router::prefix("/api").nested(v1).nested(v2);

    let router = Router::new()
        .get("/", |_| Response::html("<h1>Welcome to RustServe!</h1>"))
        .nested(api);

    let config = ServerConfig::new("127.0.0.1", 7878).threads(20);

    println!("Starting server at http://127.0.0.1:7878");

    match Server::new(config) {
        Ok(server) => server.run(router),
        Err(e) => eprintln!("Failed to start server: {}", e),
    }
}
