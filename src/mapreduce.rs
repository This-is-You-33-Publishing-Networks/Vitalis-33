//! MapReduce — v388
//! MapReduce framework with mapper, shuffler, reducer, and combiner phases.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

/// A key-value pair in the MapReduce pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct KvPair {
    pub key: String,
    pub value: i64,
}

/// MapReduce job with all phases.
#[derive(Debug)]
pub struct MapReduceJob {
    pub id: i64,
    input: Vec<i64>,
    mapped: Vec<KvPair>,
    shuffled: HashMap<String, Vec<i64>>,
    reduced: HashMap<String, i64>,
    completed: bool,
}

impl MapReduceJob {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            input: Vec::new(),
            mapped: Vec::new(),
            shuffled: HashMap::new(),
            reduced: HashMap::new(),
            completed: false,
        }
    }

    pub fn add_input(&mut self, value: i64) {
        self.input.push(value);
    }

    /// Map phase: classify each input into buckets.
    pub fn map_phase(&mut self) {
        self.mapped.clear();
        for &val in &self.input {
            // Default mapper: group by even/odd and by magnitude.
            let key = if val % 2 == 0 { "even" } else { "odd" };
            self.mapped.push(KvPair { key: key.to_string(), value: val });

            let mag_key = if val.abs() < 10 {
                "small"
            } else if val.abs() < 100 {
                "medium"
            } else {
                "large"
            };
            self.mapped.push(KvPair { key: mag_key.to_string(), value: val });
        }
    }

    /// Custom map with a user-defined function.
    pub fn map_with<F: Fn(i64) -> Vec<KvPair>>(&mut self, mapper: F) {
        self.mapped.clear();
        for &val in &self.input {
            self.mapped.extend(mapper(val));
        }
    }

    /// Shuffle phase: group mapped values by key.
    pub fn shuffle_phase(&mut self) {
        self.shuffled.clear();
        for pair in &self.mapped {
            self.shuffled
                .entry(pair.key.clone())
                .or_default()
                .push(pair.value);
        }
    }

    /// Reduce phase: aggregate each group.
    pub fn reduce_phase(&mut self) {
        self.reduced.clear();
        for (key, values) in &self.shuffled {
            let sum: i64 = values.iter().sum();
            self.reduced.insert(key.clone(), sum);
        }
        self.completed = true;
    }

    /// Custom reduce with a user-defined function.
    pub fn reduce_with<F: Fn(&[i64]) -> i64>(&mut self, reducer: F) {
        self.reduced.clear();
        for (key, values) in &self.shuffled {
            self.reduced.insert(key.clone(), reducer(values));
        }
        self.completed = true;
    }

    /// Run all phases in sequence.
    pub fn execute(&mut self) {
        self.map_phase();
        self.shuffle_phase();
        self.reduce_phase();
    }

    pub fn get_result(&self, key: &str) -> Option<i64> {
        self.reduced.get(key).copied()
    }

    pub fn result_keys(&self) -> Vec<String> {
        self.reduced.keys().cloned().collect()
    }

    pub fn partition_count(&self) -> usize {
        self.shuffled.len()
    }

    pub fn total_mapped(&self) -> usize {
        self.mapped.len()
    }

    pub fn is_completed(&self) -> bool {
        self.completed
    }

    pub fn reset(&mut self) {
        self.mapped.clear();
        self.shuffled.clear();
        self.reduced.clear();
        self.completed = false;
    }
}

/// Word count MapReduce: count occurrences of words in strings.
pub fn word_count(texts: &[&str]) -> HashMap<String, i64> {
    let mut counts = HashMap::new();
    for text in texts {
        for word in text.split_whitespace() {
            let w = word.to_lowercase();
            *counts.entry(w).or_insert(0) += 1;
        }
    }
    counts
}

/// Inverted index MapReduce: map words to document IDs.
pub fn inverted_index(docs: &[(usize, &str)]) -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for &(doc_id, text) in docs {
        for word in text.split_whitespace() {
            let w = word.to_lowercase();
            let entry = index.entry(w).or_default();
            if !entry.contains(&doc_id) {
                entry.push(doc_id);
            }
        }
    }
    index
}

static MR_STORE: LazyLock<Mutex<HashMap<i64, MapReduceJob>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static MR_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn mr_alloc() -> i64 {
    let mut next = MR_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_create() -> i64 {
    let id = mr_alloc();
    let job = MapReduceJob::new(id);
    MR_STORE.lock().unwrap().insert(id, job);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_add_input(id: i64, value: i64) -> i64 {
    let mut store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get_mut(&id) {
        job.add_input(value);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_map(id: i64) -> i64 {
    let mut store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get_mut(&id) {
        job.map_phase();
        job.total_mapped() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_shuffle(id: i64) -> i64 {
    let mut store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get_mut(&id) {
        job.shuffle_phase();
        job.partition_count() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_reduce(id: i64) -> i64 {
    let mut store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get_mut(&id) {
        job.reduce_phase();
        job.reduced.len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_result(id: i64) -> i64 {
    let store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get(&id) {
        // Return sum of all reduced values.
        job.reduced.values().sum()
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_partition_count(id: i64) -> i64 {
    let store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get(&id) {
        job.partition_count() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_mr_reset(id: i64) -> i64 {
    let mut store = MR_STORE.lock().unwrap();
    if let Some(job) = store.get_mut(&id) {
        job.reset();
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_new() {
        let job = MapReduceJob::new(1);
        assert!(!job.is_completed());
        assert_eq!(job.total_mapped(), 0);
    }

    #[test]
    fn test_add_input() {
        let mut job = MapReduceJob::new(1);
        job.add_input(10);
        job.add_input(20);
        assert_eq!(job.input.len(), 2);
    }

    #[test]
    fn test_map_phase() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.add_input(2);
        job.map_phase();
        assert!(job.total_mapped() > 0);
    }

    #[test]
    fn test_shuffle_phase() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.add_input(2);
        job.map_phase();
        job.shuffle_phase();
        assert!(job.partition_count() > 0);
    }

    #[test]
    fn test_reduce_phase() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.add_input(2);
        job.add_input(3);
        job.execute();
        assert!(job.is_completed());
    }

    #[test]
    fn test_even_odd_grouping() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.add_input(2);
        job.add_input(3);
        job.add_input(4);
        job.execute();
        // even: 2+4=6, odd: 1+3=4
        assert_eq!(job.get_result("even"), Some(6));
        assert_eq!(job.get_result("odd"), Some(4));
    }

    #[test]
    fn test_magnitude_grouping() {
        let mut job = MapReduceJob::new(1);
        job.add_input(5);
        job.add_input(50);
        job.add_input(500);
        job.execute();
        assert_eq!(job.get_result("small"), Some(5));
        assert_eq!(job.get_result("medium"), Some(50));
        assert_eq!(job.get_result("large"), Some(500));
    }

    #[test]
    fn test_custom_map() {
        let mut job = MapReduceJob::new(1);
        job.add_input(10);
        job.add_input(20);
        job.map_with(|v| vec![KvPair { key: "total".into(), value: v }]);
        job.shuffle_phase();
        job.reduce_phase();
        assert_eq!(job.get_result("total"), Some(30));
    }

    #[test]
    fn test_custom_reduce() {
        let mut job = MapReduceJob::new(1);
        job.add_input(3);
        job.add_input(7);
        job.add_input(5);
        job.map_with(|v| vec![KvPair { key: "all".into(), value: v }]);
        job.shuffle_phase();
        job.reduce_with(|vals| *vals.iter().max().unwrap_or(&0));
        assert_eq!(job.get_result("all"), Some(7));
    }

    #[test]
    fn test_reset() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.execute();
        job.reset();
        assert!(!job.is_completed());
        assert_eq!(job.total_mapped(), 0);
    }

    #[test]
    fn test_result_keys() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.add_input(2);
        job.execute();
        let keys = job.result_keys();
        assert!(!keys.is_empty());
    }

    #[test]
    fn test_missing_result() {
        let job = MapReduceJob::new(1);
        assert_eq!(job.get_result("nonexistent"), None);
    }

    #[test]
    fn test_word_count() {
        let texts = vec!["hello world", "hello rust", "world of rust"];
        let counts = word_count(&texts);
        assert_eq!(counts.get("hello"), Some(&2));
        assert_eq!(counts.get("world"), Some(&2));
        assert_eq!(counts.get("rust"), Some(&2));
        assert_eq!(counts.get("of"), Some(&1));
    }

    #[test]
    fn test_word_count_empty() {
        let texts: Vec<&str> = vec![];
        let counts = word_count(&texts);
        assert!(counts.is_empty());
    }

    #[test]
    fn test_inverted_index() {
        let docs = vec![(0, "hello world"), (1, "hello rust"), (2, "world rust")];
        let idx = inverted_index(&docs);
        assert_eq!(idx.get("hello").unwrap(), &vec![0, 1]);
        assert_eq!(idx.get("rust").unwrap(), &vec![1, 2]);
    }

    #[test]
    fn test_inverted_index_dedup() {
        let docs = vec![(0, "hello hello hello")];
        let idx = inverted_index(&docs);
        assert_eq!(idx.get("hello").unwrap(), &vec![0]);
    }

    #[test]
    fn test_empty_job_execute() {
        let mut job = MapReduceJob::new(1);
        job.execute();
        assert!(job.is_completed());
        assert_eq!(job.partition_count(), 0);
    }

    #[test]
    fn test_single_input() {
        let mut job = MapReduceJob::new(1);
        job.add_input(42);
        job.execute();
        assert_eq!(job.get_result("even"), Some(42));
    }

    #[test]
    fn test_negative_input() {
        let mut job = MapReduceJob::new(1);
        job.add_input(-5);
        job.add_input(-3);
        job.execute();
        assert_eq!(job.get_result("odd"), Some(-8));
    }

    #[test]
    fn test_partition_count_after_shuffle() {
        let mut job = MapReduceJob::new(1);
        job.add_input(1);
        job.add_input(2);
        job.map_phase();
        job.shuffle_phase();
        assert!(job.partition_count() >= 2);
    }
}
