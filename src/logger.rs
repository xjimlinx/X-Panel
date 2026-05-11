use chrono::Local;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Info => "",
            LogLevel::Warn => "",
            LogLevel::Error => "",
        }
    }
}

pub struct Logger {
    file: Option<File>,
    buffer: Vec<String>,
    max_entries: usize,
}

impl Logger {
    pub fn new(max_entries: usize) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let log_dir = PathBuf::from(home).join(".local/share/x-panel");
        let log_path = log_dir.join("x-panel.log");

        let _ = create_dir_all(&log_dir);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .ok();

        Logger {
            file,
            buffer: Vec::with_capacity(max_entries + 1),
            max_entries,
        }
    }

    pub fn log(&mut self, level: LogLevel, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let line = format!("[{}] [{}] {}", timestamp, level.as_str(), message);

        if self.buffer.len() >= self.max_entries {
            self.buffer.remove(0);
        }
        self.buffer.push(line.clone());

        if let Some(ref mut file) = self.file {
            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
        }
    }

    pub fn info(&mut self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&mut self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&mut self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn recent(&self, n: usize) -> Vec<(&str, LogLevel)> {
        self.buffer.iter().rev().take(n).map(|s| {
            let level = if s.contains("[ERROR]") { LogLevel::Error }
                       else if s.contains("[WARN]") { LogLevel::Warn }
                       else { LogLevel::Info };
            (s.as_str(), level)
        }).collect()
    }
}
