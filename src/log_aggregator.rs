//! Log Aggregator — v395
//! Log collection, filtering, pattern matching, and structured query.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: u64,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
    pub timestamp: u64,
    pub fields: HashMap<String, String>,
}

impl LogEntry {
    pub fn new(id: u64, level: LogLevel, message: &str, source: &str) -> Self {
        Self {
            id,
            level,
            message: message.to_string(),
            source: source.to_string(),
            timestamp: id, // simplified
            fields: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct LogAggregator {
    entries: Vec<LogEntry>,
    next_id: u64,
    max_entries: usize,
}

impl LogAggregator {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
            max_entries,
        }
    }

    pub fn ingest(&mut self, level: LogLevel, message: &str, source: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = LogEntry::new(id, level, message, source);
        self.entries.push(entry);
        // Evict oldest if over capacity.
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        id
    }

    pub fn ingest_with_fields(&mut self, level: LogLevel, message: &str, source: &str, fields: HashMap<String, String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut entry = LogEntry::new(id, level, message, source);
        entry.fields = fields;
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        id
    }

    pub fn query(&self, pattern: &str) -> Vec<&LogEntry> {
        let pattern_lower = pattern.to_lowercase();
        self.entries.iter()
            .filter(|e| e.message.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    pub fn filter_level(&self, min_level: &LogLevel) -> Vec<&LogEntry> {
        self.entries.iter()
            .filter(|e| e.level >= *min_level)
            .collect()
    }

    pub fn filter_source(&self, source: &str) -> Vec<&LogEntry> {
        self.entries.iter()
            .filter(|e| e.source == source)
            .collect()
    }

    pub fn last_n(&self, n: usize) -> Vec<&LogEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].iter().collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn count_by_level(&self, level: &LogLevel) -> usize {
        self.entries.iter().filter(|e| e.level == *level).count()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn pattern_match(&self, pattern: &str) -> Vec<&LogEntry> {
        // Simple glob-like pattern matching: * matches any sequence.
        let parts: Vec<&str> = pattern.split('*').collect();
        self.entries.iter().filter(|entry| {
            let msg = entry.message.to_lowercase();
            if parts.len() == 1 {
                return msg.contains(&parts[0].to_lowercase());
            }
            let mut pos = 0;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() { continue; }
                let lower_part = part.to_lowercase();
                if let Some(found) = msg[pos..].find(&lower_part) {
                    if i == 0 && found != 0 {
                        return false; // Must start with first part.
                    }
                    pos += found + lower_part.len();
                } else {
                    return false;
                }
            }
            true
        }).collect()
    }

    pub fn stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        for entry in &self.entries {
            let level_str = format!("{:?}", entry.level);
            *stats.entry(level_str).or_insert(0) += 1;
        }
        stats
    }
}

static AGG_STORE: LazyLock<Mutex<HashMap<i64, LogAggregator>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static AGG_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn agg_alloc() -> i64 {
    let mut next = AGG_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_create(max_entries: i64) -> i64 {
    let id = agg_alloc();
    let agg = LogAggregator::new(max_entries.max(10) as usize);
    AGG_STORE.lock().unwrap().insert(id, agg);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_ingest(id: i64, level: i64, msg_hash: i64) -> i64 {
    let mut store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get_mut(&id) {
        let msg = format!("log_message_{}", msg_hash);
        agg.ingest(LogLevel::from_i64(level), &msg, "default") as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_query(id: i64, pattern_hash: i64) -> i64 {
    let store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get(&id) {
        let pattern = format!("{}", pattern_hash);
        agg.query(&pattern).len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_count(id: i64) -> i64 {
    let store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get(&id) {
        agg.count() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_filter_level(id: i64, min_level: i64) -> i64 {
    let store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get(&id) {
        agg.filter_level(&LogLevel::from_i64(min_level)).len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_last_n(id: i64, n: i64) -> i64 {
    let store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get(&id) {
        agg.last_n(n.max(0) as usize).len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_clear(id: i64) -> i64 {
    let mut store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get_mut(&id) {
        agg.clear();
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_agg_pattern_match(id: i64, pattern_hash: i64) -> i64 {
    let store = AGG_STORE.lock().unwrap();
    if let Some(agg) = store.get(&id) {
        let pattern = format!("*{}*", pattern_hash);
        agg.pattern_match(&pattern).len() as i64
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregator_new() {
        let agg = LogAggregator::new(1000);
        assert_eq!(agg.count(), 0);
    }

    #[test]
    fn test_ingest() {
        let mut agg = LogAggregator::new(1000);
        let id = agg.ingest(LogLevel::Info, "hello", "test");
        assert_eq!(id, 1);
        assert_eq!(agg.count(), 1);
    }

    #[test]
    fn test_ingest_multi() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "msg1", "src1");
        agg.ingest(LogLevel::Error, "msg2", "src2");
        assert_eq!(agg.count(), 2);
    }

    #[test]
    fn test_query() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "hello world", "test");
        agg.ingest(LogLevel::Info, "goodbye world", "test");
        let results = agg.query("hello");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_case_insensitive() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "Hello World", "test");
        let results = agg.query("hello");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_level() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Debug, "debug", "src");
        agg.ingest(LogLevel::Info, "info", "src");
        agg.ingest(LogLevel::Error, "error", "src");
        let results = agg.filter_level(&LogLevel::Info);
        assert_eq!(results.len(), 2); // Info + Error
    }

    #[test]
    fn test_filter_source() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "msg1", "src_a");
        agg.ingest(LogLevel::Info, "msg2", "src_b");
        agg.ingest(LogLevel::Info, "msg3", "src_a");
        let results = agg.filter_source("src_a");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_last_n() {
        let mut agg = LogAggregator::new(1000);
        for i in 0..10 {
            agg.ingest(LogLevel::Info, &format!("msg_{}", i), "test");
        }
        let last3 = agg.last_n(3);
        assert_eq!(last3.len(), 3);
    }

    #[test]
    fn test_last_n_more_than_total() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "msg", "test");
        let last10 = agg.last_n(10);
        assert_eq!(last10.len(), 1);
    }

    #[test]
    fn test_count_by_level() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "a", "s");
        agg.ingest(LogLevel::Error, "b", "s");
        agg.ingest(LogLevel::Error, "c", "s");
        assert_eq!(agg.count_by_level(&LogLevel::Error), 2);
        assert_eq!(agg.count_by_level(&LogLevel::Info), 1);
    }

    #[test]
    fn test_clear() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "msg", "src");
        agg.clear();
        assert_eq!(agg.count(), 0);
    }

    #[test]
    fn test_max_entries() {
        let mut agg = LogAggregator::new(3);
        for i in 0..5 {
            agg.ingest(LogLevel::Info, &format!("msg_{}", i), "test");
        }
        assert_eq!(agg.count(), 3);
        // First 2 entries should be evicted.
        let results = agg.query("msg_0");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_pattern_match() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "user logged in", "auth");
        agg.ingest(LogLevel::Info, "user logged out", "auth");
        agg.ingest(LogLevel::Info, "server started", "system");
        let results = agg.pattern_match("user*logged*");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_pattern_match_no_match() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "hello", "test");
        let results = agg.pattern_match("xyz*abc");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_stats() {
        let mut agg = LogAggregator::new(1000);
        agg.ingest(LogLevel::Info, "a", "s");
        agg.ingest(LogLevel::Info, "b", "s");
        agg.ingest(LogLevel::Error, "c", "s");
        let stats = agg.stats();
        assert_eq!(stats.get("Info"), Some(&2));
        assert_eq!(stats.get("Error"), Some(&1));
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error > LogLevel::Info);
        assert!(LogLevel::Debug < LogLevel::Warn);
        assert!(LogLevel::Fatal > LogLevel::Error);
    }

    #[test]
    fn test_ingest_with_fields() {
        let mut agg = LogAggregator::new(1000);
        let mut fields = HashMap::new();
        fields.insert("user".into(), "admin".into());
        agg.ingest_with_fields(LogLevel::Info, "login", "auth", fields);
        assert_eq!(agg.count(), 1);
    }

    #[test]
    fn test_entry_fields() {
        let mut entry = LogEntry::new(1, LogLevel::Info, "test", "src");
        entry.fields.insert("key".into(), "value".into());
        assert_eq!(entry.fields.get("key").unwrap(), "value");
    }

    #[test]
    fn test_log_level_from_i64() {
        assert_eq!(LogLevel::from_i64(0), LogLevel::Trace);
        assert_eq!(LogLevel::from_i64(4), LogLevel::Error);
        assert_eq!(LogLevel::from_i64(99), LogLevel::Info); // default
    }

    #[test]
    fn test_query_empty() {
        let agg = LogAggregator::new(1000);
        assert_eq!(agg.query("anything").len(), 0);
    }
}
