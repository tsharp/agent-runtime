use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Simple file logger for workflow debugging
pub struct FileLogger {
    file: Arc<Mutex<std::fs::File>>,
}

impl FileLogger {
    /// Create a new file logger
    pub fn new(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }
    
    /// Log a message
    pub fn log(&self, message: impl AsRef<str>) {
        if let Ok(mut file) = self.file.lock() {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message.as_ref());
        }
    }
    
    /// Log with a specific level
    pub fn log_level(&self, level: &str, message: impl AsRef<str>) {
        if let Ok(mut file) = self.file.lock() {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, message.as_ref());
        }
    }
}

impl Clone for FileLogger {
    fn clone(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
        }
    }
}
