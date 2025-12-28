mod http;
mod threads;

use http::{Response, Server, ServerConfig};

use crate::http::{Filter, get, header};

fn main() {
    let h = get("/hello")
        .maybe(header("def"))
        .map(|(a,)| format!("hello world {:?}", a))
        .map(|s| Response::ok(s));

    let config = ServerConfig::new("127.0.0.1", 7878).threads(20);

    println!("Starting server at http://127.0.0.1:7878");

    match Server::new(config) {
        Ok(server) => server.run(h),
        Err(e) => eprintln!("Failed to start server: {}", e),
    }
}
