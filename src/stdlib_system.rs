//! v88 Stdlib Expansion I: systems-oriented file/path/process/time APIs,
//! plus collection and iterator helpers.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdlibError {
    NotFound(String),
    InvalidInput(String),
    PermissionDenied(String),
    ProcessFailed(i32),
}

impl std::fmt::Display for StdlibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StdlibError::NotFound(p) => write!(f, "not found: {}", p),
            StdlibError::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
            StdlibError::PermissionDenied(p) => write!(f, "permission denied: {}", p),
            StdlibError::ProcessFailed(code) => write!(f, "process failed with code {}", code),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub path: String,
    pub size: usize,
    pub is_readonly: bool,
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryFs {
    files: BTreeMap<String, (Vec<u8>, bool)>, // path -> (content, readonly)
}

impl InMemoryFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), StdlibError> {
        if path.trim().is_empty() {
            return Err(StdlibError::InvalidInput("empty path".to_string()));
        }
        if let Some((_, readonly)) = self.files.get(path)
            && *readonly
        {
            return Err(StdlibError::PermissionDenied(path.to_string()));
        }
        self.files
            .insert(normalize_path(path), (content.to_vec(), false));
        Ok(())
    }

    pub fn set_readonly(&mut self, path: &str, readonly: bool) -> Result<(), StdlibError> {
        let key = normalize_path(path);
        let entry = self
            .files
            .get_mut(&key)
            .ok_or_else(|| StdlibError::NotFound(key.clone()))?;
        entry.1 = readonly;
        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, StdlibError> {
        let key = normalize_path(path);
        self.files
            .get(&key)
            .map(|(v, _)| v.clone())
            .ok_or(StdlibError::NotFound(key))
    }

    pub fn metadata(&self, path: &str) -> Result<FileMetadata, StdlibError> {
        let key = normalize_path(path);
        self.files
            .get(&key)
            .map(|(v, ro)| FileMetadata {
                path: key.clone(),
                size: v.len(),
                is_readonly: *ro,
            })
            .ok_or(StdlibError::NotFound(key))
    }
}

pub fn normalize_path(path: &str) -> String {
    let mut out = path.replace('\\', "/");
    while out.contains("//") {
        out = out.replace("//", "/");
    }
    if out.ends_with('/') && out.len() > 1 {
        out.pop();
    }
    out
}

pub fn join_path(base: &str, segment: &str) -> Result<String, StdlibError> {
    if segment.trim().is_empty() {
        return Err(StdlibError::InvalidInput("empty segment".to_string()));
    }
    let base = normalize_path(base);
    let seg = normalize_path(segment);
    Ok(normalize_path(&format!("{}/{}", base, seg)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn simulate_process(spec: &ProcessSpec) -> Result<ProcessOutput, StdlibError> {
    if spec.program.trim().is_empty() {
        return Err(StdlibError::InvalidInput("empty program".to_string()));
    }

    if spec.program == "false" {
        return Err(StdlibError::ProcessFailed(1));
    }

    let rendered_args = spec.args.join(" ");
    let env_count = spec.env.len();
    Ok(ProcessOutput {
        exit_code: 0,
        stdout: format!("exec:{} args:[{}] env:{}", spec.program, rendered_args, env_count),
        stderr: String::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonotonicClock {
    now_ms: u64,
}

impl MonotonicClock {
    pub fn new(start_ms: u64) -> Self {
        Self { now_ms: start_ms }
    }

    pub fn now(&self) -> u64 {
        self.now_ms
    }

    pub fn advance(&mut self, delta_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(delta_ms);
    }
}

/// Stable map backed by sorted keys for deterministic iteration.
#[derive(Debug, Clone)]
pub struct StableMap<V> {
    inner: BTreeMap<String, V>,
}

impl<V: Clone> StableMap<V> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: V) {
        self.inner.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.inner.get(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }
}

pub fn chunked<T: Clone>(items: &[T], size: usize) -> Result<Vec<Vec<T>>, StdlibError> {
    if size == 0 {
        return Err(StdlibError::InvalidInput("chunk size must be > 0".to_string()));
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < items.len() {
        let end = (i + size).min(items.len());
        out.push(items[i..end].to_vec());
        i = end;
    }
    Ok(out)
}

pub fn windows<T: Clone>(items: &[T], size: usize) -> Result<Vec<Vec<T>>, StdlibError> {
    if size == 0 {
        return Err(StdlibError::InvalidInput("window size must be > 0".to_string()));
    }
    if size > items.len() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for i in 0..=(items.len() - size) {
        out.push(items[i..i + size].to_vec());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_normalize_and_join() {
        assert_eq!(normalize_path("a\\b//c/"), "a/b/c");
        assert_eq!(join_path("a/b", "c").unwrap(), "a/b/c");
    }

    #[test]
    fn test_inmemory_fs_read_write_metadata() {
        let mut fs = InMemoryFs::new();
        fs.write_file("/tmp/a.txt", b"hello").unwrap();
        let bytes = fs.read_file("/tmp/a.txt").unwrap();
        assert_eq!(bytes, b"hello");
        let meta = fs.metadata("/tmp/a.txt").unwrap();
        assert_eq!(meta.size, 5);
        assert!(!meta.is_readonly);
    }

    #[test]
    fn test_inmemory_fs_readonly_guard() {
        let mut fs = InMemoryFs::new();
        fs.write_file("/tmp/a.txt", b"x").unwrap();
        fs.set_readonly("/tmp/a.txt", true).unwrap();
        let result = fs.write_file("/tmp/a.txt", b"y");
        assert!(matches!(result, Err(StdlibError::PermissionDenied(_))));
    }

    #[test]
    fn test_process_simulation_ok_and_fail() {
        let spec = ProcessSpec {
            program: "echo".to_string(),
            args: vec!["hi".to_string()],
            env: BTreeMap::new(),
        };
        let out = simulate_process(&spec).unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("exec:echo"));

        let bad = ProcessSpec {
            program: "false".to_string(),
            args: vec![],
            env: BTreeMap::new(),
        };
        assert!(matches!(simulate_process(&bad), Err(StdlibError::ProcessFailed(1))));
    }

    #[test]
    fn test_monotonic_clock() {
        let mut c = MonotonicClock::new(100);
        c.advance(25);
        c.advance(5);
        assert_eq!(c.now(), 130);
    }

    #[test]
    fn test_stable_map_order() {
        let mut m = StableMap::new();
        m.insert("z", 1);
        m.insert("a", 2);
        m.insert("m", 3);
        assert_eq!(m.keys(), vec!["a".to_string(), "m".to_string(), "z".to_string()]);
        assert_eq!(m.get("m"), Some(&3));
    }

    #[test]
    fn test_chunked_and_windows() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(chunked(&items, 2).unwrap(), vec![vec![1, 2], vec![3, 4], vec![5]]);
        assert_eq!(windows(&items, 3).unwrap(), vec![vec![1, 2, 3], vec![2, 3, 4], vec![3, 4, 5]]);
    }
}
