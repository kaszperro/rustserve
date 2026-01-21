use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe statistics for the file server
#[derive(Default)]
pub struct Stats {
    pub active_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub total_bytes_sent: AtomicU64,
    pub files_downloaded: AtomicU64,
}

impl Stats {
    pub fn new() -> Self {
        Stats::default()
    }

    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn request_served(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bytes_sent(&self, bytes: u64) {
        self.total_bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn file_downloaded(&self) {
        self.files_downloaded.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn get_total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    pub fn get_total_bytes_sent(&self) -> u64 {
        self.total_bytes_sent.load(Ordering::Relaxed)
    }

    pub fn get_files_downloaded(&self) -> u64 {
        self.files_downloaded.load(Ordering::Relaxed)
    }

    /// Format bytes into human-readable string (e.g., "1.2 GB")
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}
