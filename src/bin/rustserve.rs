use std::env;
use std::fs;
use std::io;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rustserve::html::generate_index_html;
use rustserve::http::get;
use rustserve::http::Filter;
use rustserve::http::Response;
use rustserve::http::Server;
use rustserve::http::ServerConfig;
use rustserve::stats::Stats;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let program_name = args.remove(0);

    let (directory, port) = match args.len() {
        0 => (".".to_string(), 8080u16),
        1 => (args[0].clone(), 8080u16),
        2 => {
            let port = args[1].parse().unwrap_or_else(|_| {
                eprintln!("Invalid port: {}", args[1]);
                std::process::exit(1);
            });
            (args[0].clone(), port)
        }
        _ => {
            eprintln!("Usage: {} [directory] [port]", program_name);
            eprintln!("  directory: Path to serve (default: current directory)");
            eprintln!("  port: Port number (default: 8080)");
            std::process::exit(1);
        }
    };

    let root_path = PathBuf::from(&directory)
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("Error: Directory '{}' not found", directory);
            std::process::exit(1);
        });

    if !root_path.is_dir() {
        eprintln!("Error: '{}' is not a directory", directory);
        std::process::exit(1);
    }

    let stats = Arc::new(Stats::new());
    let stats_display = Arc::clone(&stats);

    // Clone for stats display thread
    let root_for_display = root_path.clone();

    // Build routes
    let root_for_index = root_path.clone();
    let root_for_browse = root_path.clone();
    let root_for_api = root_path;

    let stats_for_index = Arc::clone(&stats);
    let stats_for_files = Arc::clone(&stats);
    let stats_for_browse = Arc::clone(&stats);
    let stats_for_api = Arc::clone(&stats);

    // GET / - Main UI
    let index = get("/").map(move |_| {
        stats_for_index.request_served();
        let html = generate_index_html(&root_for_index, "");
        let bytes = html.len() as u64;
        stats_for_index.bytes_sent(bytes);
        Response::html(html)
    });

    // GET /browse/* - Browse subdirectories
    let value = root_for_browse.clone();
    let browse = get("/browse")
        .param_slashes::<String>()
        .map(move |(sub_path,)| {
            stats_for_browse.request_served();
            // Extract path from request - for now, serve root
            let html = generate_index_html(&value, &sub_path);
            let bytes = html.len() as u64;
            stats_for_browse.bytes_sent(bytes);
            Response::html(html)
        });

    // GET /download/* - File downloads
    let value = root_for_browse.clone();
    let download = get("/download")
        .param_slashes::<String>()
        .map(move |(path,)| {
            stats_for_files.request_served();
            let file_path = value.join(&path);
            let file_content = fs::read(file_path).unwrap();
            Response::file(&file_content)
        });

    // GET /api/files - JSON directory listing
    let api_files = get("§").map(move |_| {
        stats_for_api.request_served();
        match list_directory_json(&root_for_api) {
            Ok(json) => {
                let bytes = json.len() as u64;
                stats_for_api.bytes_sent(bytes);
                Response::json(json)
            }
            Err(e) => Response::internal_error().body(format!("Error: {}", e)),
        }
    });

    // Combine routes
    let routes = index.or(browse).or(download).or(api_files);

    let config = ServerConfig::new("0.0.0.0", port).threads(20);

    let server = match Server::new(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    };

    println!("Starting rustserve file server...\n");

    // Start stats display thread
    thread::spawn(move || loop {
        print_stats(&stats_display, &root_for_display, port);
        thread::sleep(Duration::from_millis(500));
    });

    server.run(routes);
}

fn print_stats(stats: &Stats, root_path: &Path, port: u16) {
    let active = stats.get_active_connections();
    let requests = stats.get_total_requests();
    let downloads = stats.get_files_downloaded();
    let bytes = stats.get_total_bytes_sent();
    let bytes_str = Stats::format_bytes(bytes);

    // Move cursor to top and clear
    print!("\x1B[H");

    let dir_name = root_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| root_path.to_string_lossy().to_string());

    let local_ip = get_local_ip().unwrap_or_else(|| "unknown".to_string());
    let local_url = format!("http://{}:{}", local_ip, port);

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  📁 rustserve - File Server                                  ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  📂 Serving: {:<48} ║", truncate_string(&dir_name, 48));
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Local:     http://127.0.0.1:{:<32} ║", port);
    println!("║  Network:   {:<48} ║", truncate_string(&local_url, 48));
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  👥 Active connections: {:<37} ║", active);
    println!("║  📊 Total requests: {:<41} ║", requests);
    println!("║  📥 Files downloaded: {:<39} ║", downloads);
    println!("║  📤 Data sent: {:<46} ║", bytes_str);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Share the Network URL with others on your local network!");
    println!("Press Ctrl+C to stop the server");
}

/// Get the local IP address by connecting a UDP socket to an external address.
/// This doesn't actually send any data, but allows the OS to choose the correct
/// local IP for reaching external networks.
fn get_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    // Connect to Google's DNS - doesn't actually send packets, just sets up routing
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

fn list_directory_json(path: &Path) -> io::Result<String> {
    let entries = fs::read_dir(path)?;
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();
        let size = if is_dir {
            0
        } else {
            path.metadata().map(|m| m.len()).unwrap_or(0)
        };

        files.push(format!(
            r#"{{"name":"{}","isDir":{},"size":{}}}"#,
            json_escape(&name),
            is_dir,
            size
        ));
    }

    Ok(format!("[{}]", files.join(",")))
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
