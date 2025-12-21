use std::net::TcpListener;
use std::sync::Arc;

use super::{Request, Router};
use crate::threads::ThreadPool;

pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    pub thread_count: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
            thread_count: 4,
        }
    }
}

impl ServerConfig {
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        ServerConfig {
            address: address.into(),
            port,
            thread_count: 4,
        }
    }

    pub fn threads(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }
}

pub struct Server {
    listener: TcpListener,
    pool: ThreadPool,
}

impl Server {
    pub fn new(config: ServerConfig) -> std::io::Result<Self> {
        let addr = format!("{}:{}", config.address, config.port);
        let listener = TcpListener::bind(&addr)?;
        let pool = ThreadPool::new(config.thread_count);

        Ok(Server { listener, pool })
    }

    pub fn run(self, router: Router) {
        let router = Arc::new(router);

        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let router = Arc::clone(&router);

                    self.pool.execute(move || match Request::parse(&stream) {
                        Ok(request) => {
                            let response = router.handle(&request);
                            if let Err(e) = response.write_to_stream(&mut stream) {
                                eprintln!("Error writing response: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error parsing request: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }
}
