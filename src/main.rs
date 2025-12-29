use rustserve::http::{Filter, get};
use rustserve::http::{Response, Server, ServerConfig};

fn main() {
    println!("See examples/routes.rs for more comprehensive examples.");
    println!("Starting basic server...");

    let hello = get("/hello").map(|_| Response::ok("Hello from rustserve main!"));

    let config = ServerConfig::new("127.0.0.1", 7878).threads(20);
    println!("Listening on http://127.0.0.1:7878");

    if let Ok(server) = Server::new(config) {
        server.run(hello);
    }
}
