//! Reactive Streams — v383
//! Observable/subscriber pattern with operators: map, filter, reduce, merge, take.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum StreamItem {
    Int(i64),
    Float(f64),
    Text(String),
    End,
}

#[derive(Debug, Clone)]
pub enum Operator {
    Map(fn(i64) -> i64),
    Filter(fn(i64) -> bool),
    Take(usize),
    Skip(usize),
    Scan(i64), // accumulator seed
}

#[derive(Debug)]
pub struct Observable {
    pub id: i64,
    items: Vec<i64>,
    operators: Vec<StoredOp>,
    subscribers: Vec<i64>,
    emitted_count: usize,
}

#[derive(Debug, Clone)]
enum StoredOp {
    MapAdd(i64),
    MapMul(i64),
    FilterGt(i64),
    FilterLt(i64),
    FilterEq(i64),
    Take(usize),
    Skip(usize),
    Distinct,
}

impl Observable {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            items: Vec::new(),
            operators: Vec::new(),
            subscribers: Vec::new(),
            emitted_count: 0,
        }
    }

    pub fn from_iter(id: i64, items: Vec<i64>) -> Self {
        Self {
            id,
            items,
            operators: Vec::new(),
            subscribers: Vec::new(),
            emitted_count: 0,
        }
    }

    pub fn emit(&mut self, value: i64) {
        self.items.push(value);
        self.emitted_count += 1;
    }

    pub fn add_map_add(&mut self, offset: i64) {
        self.operators.push(StoredOp::MapAdd(offset));
    }

    pub fn add_filter_gt(&mut self, threshold: i64) {
        self.operators.push(StoredOp::FilterGt(threshold));
    }

    pub fn add_take(&mut self, n: usize) {
        self.operators.push(StoredOp::Take(n));
    }

    pub fn subscribe(&mut self, subscriber_id: i64) {
        self.subscribers.push(subscriber_id);
    }

    /// Apply all operators to items and return resulting values.
    pub fn collect(&self) -> Vec<i64> {
        let mut result: Vec<i64> = self.items.clone();
        for op in &self.operators {
            result = match op {
                StoredOp::MapAdd(offset) => result.into_iter().map(|x| x + offset).collect(),
                StoredOp::MapMul(factor) => result.into_iter().map(|x| x * factor).collect(),
                StoredOp::FilterGt(t) => result.into_iter().filter(|x| *x > *t).collect(),
                StoredOp::FilterLt(t) => result.into_iter().filter(|x| *x < *t).collect(),
                StoredOp::FilterEq(t) => result.into_iter().filter(|x| *x == *t).collect(),
                StoredOp::Take(n) => result.into_iter().take(*n).collect(),
                StoredOp::Skip(n) => result.into_iter().skip(*n).collect(),
                StoredOp::Distinct => {
                    let mut seen = std::collections::HashSet::new();
                    result.into_iter().filter(|x| seen.insert(*x)).collect()
                }
            };
        }
        result
    }

    pub fn reduce(&self) -> i64 {
        self.collect().into_iter().sum()
    }

    pub fn count(&self) -> usize {
        self.collect().len()
    }
}

/// Merge two sorted streams.
pub fn merge_sorted(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut result = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            result.push(a[i]);
            i += 1;
        } else {
            result.push(b[j]);
            j += 1;
        }
    }
    result.extend_from_slice(&a[i..]);
    result.extend_from_slice(&b[j..]);
    result
}

/// Zip two streams with addition.
pub fn zip_with_add(a: &[i64], b: &[i64]) -> Vec<i64> {
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

static RX_STORE: LazyLock<Mutex<HashMap<i64, Observable>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static RX_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn rx_alloc() -> i64 {
    let mut next = RX_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_create(initial_value: i64) -> i64 {
    let id = rx_alloc();
    let mut obs = Observable::new(id);
    if initial_value != 0 {
        obs.emit(initial_value);
    }
    RX_STORE.lock().unwrap().insert(id, obs);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_map(id: i64, offset: i64) -> i64 {
    let mut store = RX_STORE.lock().unwrap();
    if let Some(obs) = store.get_mut(&id) {
        obs.add_map_add(offset);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_filter(id: i64, threshold: i64) -> i64 {
    let mut store = RX_STORE.lock().unwrap();
    if let Some(obs) = store.get_mut(&id) {
        obs.add_filter_gt(threshold);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_reduce(id: i64) -> i64 {
    let store = RX_STORE.lock().unwrap();
    if let Some(obs) = store.get(&id) {
        obs.reduce()
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_merge(a: i64, b: i64) -> i64 {
    let store = RX_STORE.lock().unwrap();
    let items_a = store.get(&a).map(|o| o.collect()).unwrap_or_default();
    let items_b = store.get(&b).map(|o| o.collect()).unwrap_or_default();
    drop(store);
    let merged = merge_sorted(&items_a, &items_b);
    let new_id = rx_alloc();
    let obs = Observable::from_iter(new_id, merged);
    RX_STORE.lock().unwrap().insert(new_id, obs);
    new_id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_take(id: i64, n: i64) -> i64 {
    let mut store = RX_STORE.lock().unwrap();
    if let Some(obs) = store.get_mut(&id) {
        obs.add_take(n.max(0) as usize);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_subscribe(id: i64, subscriber: i64) -> i64 {
    let mut store = RX_STORE.lock().unwrap();
    if let Some(obs) = store.get_mut(&id) {
        obs.subscribe(subscriber);
        1
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_rx_count(id: i64) -> i64 {
    let store = RX_STORE.lock().unwrap();
    if let Some(obs) = store.get(&id) {
        obs.count() as i64
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observable_new() {
        let obs = Observable::new(1);
        assert_eq!(obs.id, 1);
        assert!(obs.items.is_empty());
    }

    #[test]
    fn test_emit() {
        let mut obs = Observable::new(1);
        obs.emit(10);
        obs.emit(20);
        assert_eq!(obs.items, vec![10, 20]);
    }

    #[test]
    fn test_collect_no_operators() {
        let obs = Observable::from_iter(1, vec![1, 2, 3]);
        assert_eq!(obs.collect(), vec![1, 2, 3]);
    }

    #[test]
    fn test_map_add() {
        let mut obs = Observable::from_iter(1, vec![1, 2, 3]);
        obs.add_map_add(10);
        assert_eq!(obs.collect(), vec![11, 12, 13]);
    }

    #[test]
    fn test_filter_gt() {
        let mut obs = Observable::from_iter(1, vec![1, 5, 3, 7, 2]);
        obs.add_filter_gt(3);
        assert_eq!(obs.collect(), vec![5, 7]);
    }

    #[test]
    fn test_take() {
        let mut obs = Observable::from_iter(1, vec![10, 20, 30, 40, 50]);
        obs.add_take(3);
        assert_eq!(obs.collect(), vec![10, 20, 30]);
    }

    #[test]
    fn test_chained_operators() {
        let mut obs = Observable::from_iter(1, vec![1, 2, 3, 4, 5]);
        obs.add_map_add(10);
        obs.add_filter_gt(12);
        assert_eq!(obs.collect(), vec![13, 14, 15]);
    }

    #[test]
    fn test_reduce_sum() {
        let obs = Observable::from_iter(1, vec![1, 2, 3, 4]);
        assert_eq!(obs.reduce(), 10);
    }

    #[test]
    fn test_count() {
        let obs = Observable::from_iter(1, vec![1, 2, 3]);
        assert_eq!(obs.count(), 3);
    }

    #[test]
    fn test_count_after_filter() {
        let mut obs = Observable::from_iter(1, vec![1, 2, 3, 4, 5]);
        obs.add_filter_gt(3);
        assert_eq!(obs.count(), 2);
    }

    #[test]
    fn test_subscribe() {
        let mut obs = Observable::new(1);
        obs.subscribe(42);
        assert_eq!(obs.subscribers, vec![42]);
    }

    #[test]
    fn test_merge_sorted() {
        let a = vec![1, 3, 5];
        let b = vec![2, 4, 6];
        assert_eq!(merge_sorted(&a, &b), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_merge_empty() {
        let a = vec![1, 2, 3];
        let b: Vec<i64> = vec![];
        assert_eq!(merge_sorted(&a, &b), vec![1, 2, 3]);
    }

    #[test]
    fn test_zip_with_add() {
        let a = vec![1, 2, 3];
        let b = vec![10, 20, 30];
        assert_eq!(zip_with_add(&a, &b), vec![11, 22, 33]);
    }

    #[test]
    fn test_emitted_count() {
        let mut obs = Observable::new(1);
        obs.emit(1);
        obs.emit(2);
        obs.emit(3);
        assert_eq!(obs.emitted_count, 3);
    }

    #[test]
    fn test_from_iter() {
        let obs = Observable::from_iter(1, vec![10, 20]);
        assert_eq!(obs.collect(), vec![10, 20]);
    }

    #[test]
    fn test_empty_collect() {
        let obs = Observable::new(1);
        assert!(obs.collect().is_empty());
    }

    #[test]
    fn test_reduce_empty() {
        let obs = Observable::new(1);
        assert_eq!(obs.reduce(), 0);
    }

    #[test]
    fn test_distinct() {
        let mut obs = Observable::from_iter(1, vec![1, 2, 2, 3, 1, 4]);
        obs.operators.push(StoredOp::Distinct);
        assert_eq!(obs.collect(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_skip() {
        let mut obs = Observable::from_iter(1, vec![1, 2, 3, 4, 5]);
        obs.operators.push(StoredOp::Skip(2));
        assert_eq!(obs.collect(), vec![3, 4, 5]);
    }

    #[test]
    fn test_map_mul() {
        let mut obs = Observable::from_iter(1, vec![2, 3, 4]);
        obs.operators.push(StoredOp::MapMul(10));
        assert_eq!(obs.collect(), vec![20, 30, 40]);
    }
}
