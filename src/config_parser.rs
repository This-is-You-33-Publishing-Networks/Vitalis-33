//! Config Parser — v370
//! TOML-like configuration parsing, validation, merge, hierarchical keys.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<ConfigValue>),
    Table(HashMap<String, ConfigValue>),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub values: HashMap<String, ConfigValue>,
}

impl Config {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    /// Parse a simple key=value format (one per line).
    /// Supports: strings ("..."), integers, floats, booleans.
    pub fn parse(input: &str) -> Self {
        let mut values = HashMap::new();
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim().to_string();
                let val = val.trim();
                let parsed = Self::parse_value(val);
                values.insert(key, parsed);
            }
        }
        Self { values }
    }

    fn parse_value(val: &str) -> ConfigValue {
        if val == "true" {
            ConfigValue::Bool(true)
        } else if val == "false" {
            ConfigValue::Bool(false)
        } else if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            ConfigValue::Str(val[1..val.len()-1].to_string())
        } else if let Ok(i) = val.parse::<i64>() {
            ConfigValue::Int(i)
        } else if let Ok(f) = val.parse::<f64>() {
            ConfigValue::Float(f)
        } else {
            ConfigValue::Str(val.to_string())
        }
    }

    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        // Support dotted keys: "a.b.c"
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() == 1 {
            return self.values.get(key);
        }
        let mut current = &self.values;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                return current.get(*part);
            }
            match current.get(*part) {
                Some(ConfigValue::Table(t)) => current = t,
                _ => return None,
            }
        }
        None
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(ConfigValue::Int(i)) => Some(*i),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(ConfigValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key) {
            Some(ConfigValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: ConfigValue) {
        self.values.insert(key.to_string(), value);
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn keys_count(&self) -> usize {
        self.values.len()
    }

    pub fn merge(&mut self, other: &Config) {
        for (k, v) in &other.values {
            self.values.insert(k.clone(), v.clone());
        }
    }

    pub fn validate_required(&self, required: &[&str]) -> Vec<String> {
        required.iter()
            .filter(|k| !self.has(k))
            .map(|k| k.to_string())
            .collect()
    }

    pub fn to_string_repr(&self) -> String {
        let mut lines = Vec::new();
        let mut keys: Vec<_> = self.values.keys().collect();
        keys.sort();
        for key in keys {
            let val = &self.values[key];
            let val_str = match val {
                ConfigValue::Str(s) => format!("\"{}\"", s),
                ConfigValue::Int(i) => i.to_string(),
                ConfigValue::Float(f) => format!("{}", f),
                ConfigValue::Bool(b) => b.to_string(),
                ConfigValue::Array(_) => "[...]".to_string(),
                ConfigValue::Table(_) => "{...}".to_string(),
            };
            lines.push(format!("{} = {}", key, val_str));
        }
        lines.join("\n")
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static CFG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_parse(input_hash: i64) -> i64 {
    let mut cfg = CFG.lock().unwrap();
    *cfg = Config::new();
    cfg.set("_parsed", ConfigValue::Int(input_hash));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_get(key_hash: i64) -> i64 {
    let cfg = CFG.lock().unwrap();
    let key = format!("key_{}", key_hash);
    if cfg.has(&key) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_set(key_hash: i64, value: i64) -> i64 {
    let mut cfg = CFG.lock().unwrap();
    let key = format!("key_{}", key_hash);
    cfg.set(&key, ConfigValue::Int(value));
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_has(key_hash: i64) -> i64 {
    let cfg = CFG.lock().unwrap();
    let key = format!("key_{}", key_hash);
    if cfg.has(&key) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_keys() -> i64 {
    CFG.lock().unwrap().keys_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_merge() -> i64 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_validate() -> i64 {
    let cfg = CFG.lock().unwrap();
    let missing = cfg.validate_required(&[]);
    missing.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_config_to_string() -> i64 {
    let cfg = CFG.lock().unwrap();
    cfg.to_string_repr().len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let cfg = Config::parse("port = 8080");
        assert_eq!(cfg.get_int("port"), Some(8080));
    }

    #[test]
    fn test_parse_string() {
        let cfg = Config::parse("name = \"hello\"");
        assert_eq!(cfg.get_str("name"), Some("hello"));
    }

    #[test]
    fn test_parse_bool() {
        let cfg = Config::parse("debug = true\nrelease = false");
        assert_eq!(cfg.get_bool("debug"), Some(true));
        assert_eq!(cfg.get_bool("release"), Some(false));
    }

    #[test]
    fn test_parse_float() {
        let cfg = Config::parse("ratio = 3.14");
        match cfg.get("ratio") {
            Some(ConfigValue::Float(f)) => assert!((*f - 3.14).abs() < 0.001),
            _ => panic!("expected float"),
        }
    }

    #[test]
    fn test_comments_ignored() {
        let cfg = Config::parse("# comment\nkey = 1");
        assert_eq!(cfg.keys_count(), 1);
    }

    #[test]
    fn test_empty_lines_ignored() {
        let cfg = Config::parse("\n\nkey = 1\n\n");
        assert_eq!(cfg.keys_count(), 1);
    }

    #[test]
    fn test_set_get() {
        let mut cfg = Config::new();
        cfg.set("x", ConfigValue::Int(42));
        assert_eq!(cfg.get_int("x"), Some(42));
    }

    #[test]
    fn test_has() {
        let mut cfg = Config::new();
        assert!(!cfg.has("x"));
        cfg.set("x", ConfigValue::Int(1));
        assert!(cfg.has("x"));
    }

    #[test]
    fn test_merge() {
        let mut a = Config::parse("x = 1\ny = 2");
        let b = Config::parse("y = 3\nz = 4");
        a.merge(&b);
        assert_eq!(a.get_int("x"), Some(1));
        assert_eq!(a.get_int("y"), Some(3));
        assert_eq!(a.get_int("z"), Some(4));
    }

    #[test]
    fn test_validate_required() {
        let cfg = Config::parse("a = 1");
        let missing = cfg.validate_required(&["a", "b", "c"]);
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&"b".to_string()));
        assert!(missing.contains(&"c".to_string()));
    }

    #[test]
    fn test_validate_all_present() {
        let cfg = Config::parse("a = 1\nb = 2");
        let missing = cfg.validate_required(&["a", "b"]);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_keys_count() {
        let cfg = Config::parse("a = 1\nb = 2\nc = 3");
        assert_eq!(cfg.keys_count(), 3);
    }

    #[test]
    fn test_to_string_repr() {
        let cfg = Config::parse("name = \"vitalis\"\nversion = 400");
        let s = cfg.to_string_repr();
        assert!(s.contains("name"));
        assert!(s.contains("version"));
    }

    #[test]
    fn test_overwrite() {
        let mut cfg = Config::new();
        cfg.set("k", ConfigValue::Int(1));
        cfg.set("k", ConfigValue::Int(2));
        assert_eq!(cfg.get_int("k"), Some(2));
    }

    #[test]
    fn test_dotted_key() {
        let mut cfg = Config::new();
        let mut inner = HashMap::new();
        inner.insert("port".to_string(), ConfigValue::Int(3000));
        cfg.set("server", ConfigValue::Table(inner));
        assert_eq!(cfg.get_int("server.port"), Some(3000));
    }

    #[test]
    fn test_get_nonexistent() {
        let cfg = Config::new();
        assert!(cfg.get("nope").is_none());
    }

    #[test]
    fn test_empty_config() {
        let cfg = Config::parse("");
        assert_eq!(cfg.keys_count(), 0);
    }

    #[test]
    fn test_whitespace_handling() {
        let cfg = Config::parse("  key  =  42  ");
        assert_eq!(cfg.get_int("key"), Some(42));
    }

    #[test]
    fn test_negative_int() {
        let cfg = Config::parse("offset = -10");
        assert_eq!(cfg.get_int("offset"), Some(-10));
    }

    #[test]
    fn test_type_mismatch_returns_none() {
        let cfg = Config::parse("val = 42");
        assert!(cfg.get_str("val").is_none());
        assert!(cfg.get_bool("val").is_none());
    }
}
