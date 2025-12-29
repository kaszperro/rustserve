use rustserve::http::{Filter, get, header, param, post};
use rustserve::http::{Response, Server, ServerConfig};

fn main() {
    println!("Defining routes...");

    // Example 1: Basic route with header extraction
    // GET /hello
    // Optionally checks for "user-agent" header
    let hello = get("/hello")
        .maybe(header("user-agent"))
        .map(|(agent,)| {
            let msg = debug_agent(agent);
            format!("Hello from Rust! {}", msg)
        })
        .map(|s| Response::ok(s));

    // Example 2: Path parameters
    // GET /users/<u32>
    let users = get("/users")
        .and(param::<String>())
        .map(|(id_str,)| {
            let id = id_str.parse::<u32>().unwrap_or(0);
            format!("Getting user with ID: {}", id)
        })
        .map(|s| Response::ok(s));

    // Example 3: Multiple path segments and params
    // GET /api/items/<String>
    let items = get("/api")
        .path("items")
        .and(param::<String>())
        .map(|(item_id,)| {
            // Using simulated JSON response
            Response::json(format!(r#"{{"item_id": "{}"}}"#, item_id))
        });

    // Example 4: POST request
    // POST /submit
    let submit = post("/submit").map(|_| Response::created().body("Submission received!"));

    // Combine all routes using `.or()`
    let routes = hello.or(users).or(items).or(submit);

    let config = ServerConfig::new("127.0.0.1", 7878).threads(20);

    println!("Starting server at http://127.0.0.1:7878");

    match Server::new(config) {
        Ok(server) => server.run(routes),
        Err(e) => eprintln!("Failed to start server: {}", e),
    }
}

fn debug_agent(agent: Option<String>) -> String {
    match agent {
        Some(a) => format!("User-Agent: {}", a),
        None => "No User-Agent provided".to_string(),
    }
}
