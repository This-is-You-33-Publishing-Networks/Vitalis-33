//! Database primitives: page cache, B+tree-like index API, and transactional log.

use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub page_id: u64,
    pub data: Vec<u8>,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub struct PageCache {
    capacity: usize,
    pages: BTreeMap<u64, Page>,
    lru: VecDeque<u64>,
}

impl PageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            pages: BTreeMap::new(),
            lru: VecDeque::new(),
        }
    }

    fn touch(&mut self, page_id: u64) {
        if let Some(pos) = self.lru.iter().position(|id| *id == page_id) {
            self.lru.remove(pos);
        }
        self.lru.push_front(page_id);
    }

    fn evict_if_needed(&mut self) -> Option<Page> {
        if self.pages.len() < self.capacity {
            return None;
        }
        let evicted_id = self.lru.pop_back()?;
        self.pages.remove(&evicted_id)
    }

    pub fn put(&mut self, page_id: u64, data: Vec<u8>, dirty: bool) -> Option<Page> {
        let evicted = if !self.pages.contains_key(&page_id) {
            self.evict_if_needed()
        } else {
            None
        };

        self.pages.insert(
            page_id,
            Page {
                page_id,
                data,
                dirty,
            },
        );
        self.touch(page_id);
        evicted
    }

    pub fn get(&mut self, page_id: u64) -> Option<&Page> {
        if self.pages.contains_key(&page_id) {
            self.touch(page_id);
        }
        self.pages.get(&page_id)
    }

    pub fn mark_dirty(&mut self, page_id: u64) {
        if let Some(page) = self.pages.get_mut(&page_id) {
            page.dirty = true;
            self.touch(page_id);
        }
    }

    pub fn flush_dirty_pages(&mut self) -> Vec<u64> {
        let mut flushed = Vec::new();
        for (id, page) in &mut self.pages {
            if page.dirty {
                page.dirty = false;
                flushed.push(*id);
            }
        }
        flushed.sort_unstable();
        flushed
    }
}

#[derive(Debug, Clone)]
pub struct BPlusIndex {
    // Backed by a sorted map to provide deterministic API behavior.
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl BPlusIndex {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.entries.insert(key, value);
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.entries.get(key).map(|v| v.as_slice())
    }

    pub fn range(&self, start: &[u8], end: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.entries
            .range(start.to_vec()..end.to_vec())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRecord {
    Begin { tx_id: u64 },
    Put { tx_id: u64, key: Vec<u8>, value: Vec<u8> },
    Delete { tx_id: u64, key: Vec<u8> },
    Commit { tx_id: u64 },
}

#[derive(Debug, Clone, Default)]
pub struct TransactionLog {
    records: Vec<LogRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadKind {
    Sequential,
    Random,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadBenchmarkResult {
    pub workload: WorkloadKind,
    pub operations: usize,
    pub hits: usize,
    pub misses: usize,
    pub write_ops: usize,
}

impl WorkloadBenchmarkResult {
    pub fn hit_rate(&self) -> f64 {
        if self.operations == 0 {
            0.0
        } else {
            self.hits as f64 / self.operations as f64
        }
    }
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Deterministic benchmark workload for page-cache and index access patterns.
pub fn benchmark_workload(
    kind: WorkloadKind,
    operations: usize,
    cache_capacity: usize,
    key_space: u64,
) -> WorkloadBenchmarkResult {
    let mut cache = PageCache::new(cache_capacity);
    let mut index = BPlusIndex::new();

    for i in 0..key_space {
        let key = i.to_be_bytes().to_vec();
        index.insert(key, vec![(i % 251) as u8]);
    }

    let mut hits = 0usize;
    let mut misses = 0usize;
    let mut write_ops = 0usize;
    let mut rng = 0x4d595df4d0f33173u64;

    for i in 0..operations {
        let page_id = match kind {
            // Sequential workload models locality over a bounded hot set.
            WorkloadKind::Sequential => {
                let hot_set = (cache_capacity as u64 / 2).max(1).min(key_space);
                (i as u64) % hot_set
            }
            WorkloadKind::Random => lcg_next(&mut rng) % key_space,
        };

        if cache.get(page_id).is_some() {
            hits += 1;
        } else {
            misses += 1;
            let value = index
                .get(&page_id.to_be_bytes())
                .map(|v| v.to_vec())
                .unwrap_or_else(|| vec![0]);
            cache.put(page_id, value, false);
            write_ops += 1;
        }
    }

    WorkloadBenchmarkResult {
        workload: kind,
        operations,
        hits,
        misses,
        write_ops,
    }
}

impl TransactionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, record: LogRecord) {
        self.records.push(record);
    }

    pub fn checkpoint(&self) -> usize {
        self.records.len()
    }

    pub fn replay(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut state: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        let mut active: BTreeMap<u64, Vec<LogRecord>> = BTreeMap::new();

        for record in &self.records {
            match record {
                LogRecord::Begin { tx_id } => {
                    active.insert(*tx_id, vec![record.clone()]);
                }
                LogRecord::Put { tx_id, .. } | LogRecord::Delete { tx_id, .. } => {
                    if let Some(buffer) = active.get_mut(tx_id) {
                        buffer.push(record.clone());
                    }
                }
                LogRecord::Commit { tx_id } => {
                    if let Some(buffer) = active.remove(tx_id) {
                        for op in buffer {
                            match op {
                                LogRecord::Put { key, value, .. } => {
                                    state.insert(key, value);
                                }
                                LogRecord::Delete { key, .. } => {
                                    state.remove(&key);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_cache_eviction_lru() {
        let mut cache = PageCache::new(2);
        assert!(cache.put(1, vec![1], false).is_none());
        assert!(cache.put(2, vec![2], false).is_none());
        cache.get(1);

        let evicted = cache.put(3, vec![3], false).unwrap();
        assert_eq!(evicted.page_id, 2);
        assert!(cache.get(2).is_none());
        assert!(cache.get(1).is_some());
        assert!(cache.get(3).is_some());
    }

    #[test]
    fn test_page_cache_flush_dirty_sorted() {
        let mut cache = PageCache::new(3);
        cache.put(10, vec![0], false);
        cache.put(2, vec![0], true);
        cache.put(7, vec![0], true);
        cache.mark_dirty(10);

        let flushed = cache.flush_dirty_pages();
        assert_eq!(flushed, vec![2, 7, 10]);
        assert!(cache.flush_dirty_pages().is_empty());
    }

    #[test]
    fn test_bplus_index_insert_get_range() {
        let mut idx = BPlusIndex::new();
        idx.insert(b"a".to_vec(), b"1".to_vec());
        idx.insert(b"c".to_vec(), b"3".to_vec());
        idx.insert(b"b".to_vec(), b"2".to_vec());

        assert_eq!(idx.len(), 3);
        assert_eq!(idx.get(b"b"), Some(&b"2"[..]));

        let range = idx.range(b"a", b"d");
        let keys: Vec<Vec<u8>> = range.into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_transaction_log_replay_committed_only() {
        let mut wal = TransactionLog::new();
        wal.append(LogRecord::Begin { tx_id: 1 });
        wal.append(LogRecord::Put {
            tx_id: 1,
            key: b"k1".to_vec(),
            value: b"v1".to_vec(),
        });
        wal.append(LogRecord::Commit { tx_id: 1 });

        wal.append(LogRecord::Begin { tx_id: 2 });
        wal.append(LogRecord::Put {
            tx_id: 2,
            key: b"k2".to_vec(),
            value: b"v2".to_vec(),
        });
        // tx_id 2 never commits and must not be visible after recovery.

        let recovered = wal.replay();
        assert_eq!(recovered.get(b"k1".as_slice()), Some(&b"v1".to_vec()));
        assert!(recovered.get(b"k2".as_slice()).is_none());
    }

    #[test]
    fn test_transaction_log_delete_on_commit() {
        let mut wal = TransactionLog::new();
        wal.append(LogRecord::Begin { tx_id: 10 });
        wal.append(LogRecord::Put {
            tx_id: 10,
            key: b"x".to_vec(),
            value: b"1".to_vec(),
        });
        wal.append(LogRecord::Commit { tx_id: 10 });

        wal.append(LogRecord::Begin { tx_id: 11 });
        wal.append(LogRecord::Delete {
            tx_id: 11,
            key: b"x".to_vec(),
        });
        wal.append(LogRecord::Commit { tx_id: 11 });

        let recovered = wal.replay();
        assert!(recovered.get(b"x".as_slice()).is_none());
    }

    #[test]
    fn test_workload_benchmark_sequential_vs_random() {
        let sequential = benchmark_workload(WorkloadKind::Sequential, 4_000, 256, 1024);
        let random = benchmark_workload(WorkloadKind::Random, 4_000, 256, 1024);

        assert_eq!(sequential.operations, 4_000);
        assert_eq!(random.operations, 4_000);
        assert!(sequential.hit_rate() > random.hit_rate());
    }

    #[test]
    fn test_workload_benchmark_against_baseline() {
        // Deterministic baseline thresholds for regression detection.
        let seq = benchmark_workload(WorkloadKind::Sequential, 5_000, 256, 1024);
        let rnd = benchmark_workload(WorkloadKind::Random, 5_000, 256, 1024);

        assert!(seq.hit_rate() >= 0.90, "sequential hit-rate below baseline: {}", seq.hit_rate());
        assert!(rnd.hit_rate() <= 0.10, "random hit-rate above baseline window: {}", rnd.hit_rate());
        assert!(seq.hit_rate() - rnd.hit_rate() >= 0.50, "sequential-vs-random gap regressed");
    }
}
