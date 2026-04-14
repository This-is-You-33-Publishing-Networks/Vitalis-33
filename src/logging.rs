//! Logging — v376
//! Structured logging with levels, sinks, contextual fields, and rotation.

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub fn from_i64(v: i64) -> Self {
        match v {
            0 => Self::Trace,
            1 => Self::Debug,
            2 => Self::Info,
            3 => Self::Warn,
            4 => Self::Error,
            5 => Self::Fatal,
            _ => Self::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: u64,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Logger {
    entries: VecDeque<LogEntry>,
    min_level: LogLevel,
    max_entries: usize,
    counter: u64,
    level_counts: [u64; 6],
}

impl Logger {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            min_level: LogLevel::Debug,
            max_entries: 10000,
            counter: 0,
            level_counts: [0; 6],
        }
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }

    pub fn log(&mut self, level: LogLevel, message: &str) {
        if level < self.min_level {
            return;
        }
        self.counter += 1;
        self.level_counts[level as usize] += 1;
        let entry = LogEntry {
            level,
            message: message.to_string(),
            timestamp: self.counter,
            fields: Vec::new(),
        };
        self.entries.push_back(entry);
        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn log_with_fields(&mut self, level: LogLevel, message: &str, fields: Vec<(String, String)>) {
        if level < self.min_level {
            return;
        }
        self.counter += 1;
        self.level_counts[level as usize] += 1;
        let entry = LogEntry {
            level,
            message: message.to_string(),
            timestamp: self.counter,
            fields,
        };
        self.entries.push_back(entry);
        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn trace(&mut self, msg: &str) { self.log(LogLevel::Trace, msg); }
    pub fn debug(&mut self, msg: &str) { self.log(LogLevel::Debug, msg); }
    pub fn info(&mut self, msg: &str) { self.log(LogLevel::Info, msg); }
    pub fn warn(&mut self, msg: &str) { self.log(LogLevel::Warn, msg); }
    pub fn error(&mut self, msg: &str) { self.log(LogLevel::Error, msg); }
    pub fn fatal(&mut self, msg: &str) { self.log(LogLevel::Fatal, msg); }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn count_level(&self, level: LogLevel) -> u64 {
        self.level_counts[level as usize]
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.level_counts = [0; 6];
        self.counter = 0;
    }

    pub fn last_n(&self, n: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    pub fn filter_level(&self, level: LogLevel) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.level == level).collect()
    }

    pub fn search(&self, pattern: &str) -> Vec<&LogEntry> {
        let pattern = pattern.to_lowercase();
        self.entries.iter()
            .filter(|e| e.message.to_lowercase().contains(&pattern))
            .collect()
    }

    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max;
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static LOGGER: LazyLock<Mutex<Logger>> = LazyLock::new(|| Mutex::new(Logger::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_init() -> i64 {
    *LOGGER.lock().unwrap() = Logger::new();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_debug(msg_hash: i64) -> i64 {
    LOGGER.lock().unwrap().debug(&format!("debug_{}", msg_hash));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_info(msg_hash: i64) -> i64 {
    LOGGER.lock().unwrap().info(&format!("info_{}", msg_hash));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_warn(msg_hash: i64) -> i64 {
    LOGGER.lock().unwrap().warn(&format!("warn_{}", msg_hash));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_error(msg_hash: i64) -> i64 {
    LOGGER.lock().unwrap().error(&format!("error_{}", msg_hash));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_set_level(level: i64) -> i64 {
    LOGGER.lock().unwrap().set_level(LogLevel::from_i64(level));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_count() -> i64 {
    LOGGER.lock().unwrap().count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_log_clear() -> i64 {
    LOGGER.lock().unwrap().clear();
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_logger() {
        let logger = Logger::new();
        assert_eq!(logger.count(), 0);
    }

    #[test]
    fn test_log_info() {
        let mut logger = Logger::new();
        logger.info("test message");
        assert_eq!(logger.count(), 1);
    }

    #[test]
    fn test_log_levels() {
        let mut logger = Logger::new();
        logger.set_level(LogLevel::Trace);
        logger.trace("t");
        logger.debug("d");
        logger.info("i");
        logger.warn("w");
        logger.error("e");
        logger.fatal("f");
        assert_eq!(logger.count(), 6);
    }

    #[test]
    fn test_level_filter() {
        let mut logger = Logger::new();
        logger.set_level(LogLevel::Warn);
        logger.debug("should skip");
        logger.info("should skip");
        logger.warn("should include");
        logger.error("should include");
        assert_eq!(logger.count(), 2);
    }

    #[test]
    fn test_level_count() {
        let mut logger = Logger::new();
        logger.info("a");
        logger.info("b");
        logger.error("c");
        assert_eq!(logger.count_level(LogLevel::Info), 2);
        assert_eq!(logger.count_level(LogLevel::Error), 1);
    }

    #[test]
    fn test_clear() {
        let mut logger = Logger::new();
        logger.info("test");
        logger.clear();
        assert_eq!(logger.count(), 0);
    }

    #[test]
    fn test_last_n() {
        let mut logger = Logger::new();
        logger.info("first");
        logger.info("second");
        logger.info("third");
        let last = logger.last_n(2);
        assert_eq!(last.len(), 2);
        assert_eq!(last[0].message, "third");
        assert_eq!(last[1].message, "second");
    }

    #[test]
    fn test_filter_level() {
        let mut logger = Logger::new();
        logger.info("a");
        logger.warn("b");
        logger.info("c");
        let infos = logger.filter_level(LogLevel::Info);
        assert_eq!(infos.len(), 2);
    }

    #[test]
    fn test_search() {
        let mut logger = Logger::new();
        logger.info("hello world");
        logger.info("goodbye world");
        logger.info("hello rust");
        let results = logger.search("hello");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut logger = Logger::new();
        logger.info("Hello World");
        let results = logger.search("hello");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_max_entries() {
        let mut logger = Logger::new();
        logger.set_max_entries(3);
        for i in 0..10 {
            logger.info(&format!("msg {}", i));
        }
        assert_eq!(logger.count(), 3);
    }

    #[test]
    fn test_log_with_fields() {
        let mut logger = Logger::new();
        logger.log_with_fields(
            LogLevel::Info,
            "request",
            vec![("method".into(), "GET".into()), ("path".into(), "/api".into())],
        );
        assert_eq!(logger.count(), 1);
        assert_eq!(logger.entries[0].fields.len(), 2);
    }

    #[test]
    fn test_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_level_from_i64() {
        assert_eq!(LogLevel::from_i64(0), LogLevel::Trace);
        assert_eq!(LogLevel::from_i64(2), LogLevel::Info);
        assert_eq!(LogLevel::from_i64(4), LogLevel::Error);
        assert_eq!(LogLevel::from_i64(99), LogLevel::Info);
    }

    #[test]
    fn test_level_as_str() {
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_last_n_empty() {
        let logger = Logger::new();
        assert!(logger.last_n(5).is_empty());
    }

    #[test]
    fn test_search_no_match() {
        let mut logger = Logger::new();
        logger.info("hello");
        assert!(logger.search("xyz").is_empty());
    }

    #[test]
    fn test_timestamp_increments() {
        let mut logger = Logger::new();
        logger.info("a");
        logger.info("b");
        assert!(logger.entries[1].timestamp > logger.entries[0].timestamp);
    }

    #[test]
    fn test_set_level_change() {
        let mut logger = Logger::new();
        logger.set_level(LogLevel::Error);
        logger.info("skip");
        assert_eq!(logger.count(), 0);
        logger.error("include");
        assert_eq!(logger.count(), 1);
    }
}
