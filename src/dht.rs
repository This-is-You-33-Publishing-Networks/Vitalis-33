//! Distributed Hash Table — v387
//! Consistent hashing with virtual nodes, key-value storage, and rebalancing.

use std::sync::{LazyLock, Mutex};
use std::collections::{BTreeMap, HashMap};

/// Simple hash function (FNV-1a inspired).
fn fnv_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// A node in the DHT ring.
#[derive(Debug, Clone)]
pub struct DhtNode {
    pub id: String,
    pub virtual_nodes: usize,
    pub items: HashMap<String, String>,
}

/// Consistent hash ring for distributed key-value storage.
#[derive(Debug)]
pub struct Dht {
    ring: BTreeMap<u64, String>,
    nodes: HashMap<String, DhtNode>,
    data: HashMap<String, String>,
    virtual_node_count: usize,
}

impl Dht {
    pub fn new(virtual_nodes: usize) -> Self {
        Self {
            ring: BTreeMap::new(),
            nodes: HashMap::new(),
            data: HashMap::new(),
            virtual_node_count: virtual_nodes,
        }
    }

    pub fn add_node(&mut self, node_id: &str) {
        let node = DhtNode {
            id: node_id.to_string(),
            virtual_nodes: self.virtual_node_count,
            items: HashMap::new(),
        };
        for i in 0..self.virtual_node_count {
            let vnode_key = format!("{}:{}", node_id, i);
            let hash = fnv_hash(vnode_key.as_bytes());
            self.ring.insert(hash, node_id.to_string());
        }
        self.nodes.insert(node_id.to_string(), node);
    }

    pub fn remove_node(&mut self, node_id: &str) {
        for i in 0..self.virtual_node_count {
            let vnode_key = format!("{}:{}", node_id, i);
            let hash = fnv_hash(vnode_key.as_bytes());
            self.ring.remove(&hash);
        }
        self.nodes.remove(node_id);
    }

    /// Find which node owns a given key.
    pub fn find_node(&self, key: &str) -> Option<&str> {
        if self.ring.is_empty() {
            return None;
        }
        let hash = fnv_hash(key.as_bytes());
        // Find the first node with hash >= key hash (clockwise on ring).
        if let Some((_, node_id)) = self.ring.range(hash..).next() {
            Some(node_id.as_str())
        } else {
            // Wrap around to the first node.
            self.ring.values().next().map(|s| s.as_str())
        }
    }

    pub fn put(&mut self, key: &str, value: &str) -> bool {
        if let Some(node_id) = self.find_node(key).map(|s| s.to_string()) {
            self.data.insert(key.to_string(), value.to_string());
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.items.insert(key.to_string(), value.to_string());
            }
            true
        } else {
            false
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    pub fn remove(&mut self, key: &str) -> bool {
        if let Some(node_id) = self.find_node(key).map(|s| s.to_string()) {
            self.data.remove(key);
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.items.remove(key);
            }
            true
        } else {
            false
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn hash_key(&self, key: &str) -> u64 {
        fnv_hash(key.as_bytes())
    }

    /// Rebalance data after a node addition/removal.
    pub fn rebalance(&mut self) -> usize {
        let mut reassigned = 0;
        let keys: Vec<String> = self.data.keys().cloned().collect();
        // Clear all node items.
        for node in self.nodes.values_mut() {
            node.items.clear();
        }
        // Re-assign all keys.
        for key in &keys {
            if let Some(node_id) = self.find_node(key).map(|s| s.to_string()) {
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    if let Some(val) = self.data.get(key) {
                        node.items.insert(key.clone(), val.clone());
                        reassigned += 1;
                    }
                }
            }
        }
        reassigned
    }
}

static DHT_STORE: LazyLock<Mutex<HashMap<i64, Dht>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static DHT_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn dht_alloc() -> i64 {
    let mut next = DHT_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_create(virtual_nodes: i64) -> i64 {
    let id = dht_alloc();
    let dht = Dht::new(virtual_nodes.max(1) as usize);
    // Add a default node.
    let mut dht = dht;
    dht.add_node("node_0");
    DHT_STORE.lock().unwrap().insert(id, dht);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_put(id: i64, key_hash: i64, value: i64) -> i64 {
    let mut store = DHT_STORE.lock().unwrap();
    if let Some(dht) = store.get_mut(&id) {
        let k = format!("key_{}", key_hash);
        let v = format!("{}", value);
        if dht.put(&k, &v) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_get(id: i64, key_hash: i64) -> i64 {
    let store = DHT_STORE.lock().unwrap();
    if let Some(dht) = store.get(&id) {
        let k = format!("key_{}", key_hash);
        if let Some(val) = dht.get(&k) {
            val.parse::<i64>().unwrap_or(-1)
        } else {
            -1
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_remove(id: i64, key_hash: i64) -> i64 {
    let mut store = DHT_STORE.lock().unwrap();
    if let Some(dht) = store.get_mut(&id) {
        let k = format!("key_{}", key_hash);
        if dht.remove(&k) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_contains(id: i64, key_hash: i64) -> i64 {
    let store = DHT_STORE.lock().unwrap();
    if let Some(dht) = store.get(&id) {
        let k = format!("key_{}", key_hash);
        if dht.contains(&k) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_size(id: i64) -> i64 {
    let store = DHT_STORE.lock().unwrap();
    if let Some(dht) = store.get(&id) {
        dht.size() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_hash(key_hash: i64) -> i64 {
    let k = format!("key_{}", key_hash);
    fnv_hash(k.as_bytes()) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_dht_rebalance(id: i64) -> i64 {
    let mut store = DHT_STORE.lock().unwrap();
    if let Some(dht) = store.get_mut(&id) {
        dht.rebalance() as i64
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_new() {
        let dht = Dht::new(3);
        assert_eq!(dht.size(), 0);
        assert_eq!(dht.node_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        assert_eq!(dht.node_count(), 1);
    }

    #[test]
    fn test_remove_node() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        dht.remove_node("node1");
        assert_eq!(dht.node_count(), 0);
    }

    #[test]
    fn test_put_get() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        dht.put("hello", "world");
        assert_eq!(dht.get("hello"), Some("world"));
    }

    #[test]
    fn test_get_missing() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        assert_eq!(dht.get("missing"), None);
    }

    #[test]
    fn test_contains() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        dht.put("key", "val");
        assert!(dht.contains("key"));
        assert!(!dht.contains("other"));
    }

    #[test]
    fn test_remove_key() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        dht.put("key", "val");
        dht.remove("key");
        assert!(!dht.contains("key"));
    }

    #[test]
    fn test_size() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        dht.put("a", "1");
        dht.put("b", "2");
        dht.put("c", "3");
        assert_eq!(dht.size(), 3);
    }

    #[test]
    fn test_find_node() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        assert!(dht.find_node("some_key").is_some());
    }

    #[test]
    fn test_find_node_empty() {
        let dht = Dht::new(3);
        assert!(dht.find_node("key").is_none());
    }

    #[test]
    fn test_multiple_nodes() {
        let mut dht = Dht::new(10);
        dht.add_node("node1");
        dht.add_node("node2");
        dht.add_node("node3");
        assert_eq!(dht.node_count(), 3);
    }

    #[test]
    fn test_distribution() {
        let mut dht = Dht::new(3);
        dht.add_node("alpha");
        dht.add_node("beta");
        dht.add_node("gamma");
        // With 3 nodes and diverse keys, at least 2 nodes should be reachable.
        let mut found_nodes = std::collections::HashSet::new();
        for i in 0..10000 {
            let key = format!("key_{}", i);
            if let Some(node) = dht.find_node(&key) {
                found_nodes.insert(node.to_string());
            }
        }
        assert!(found_nodes.len() >= 2);
    }

    #[test]
    fn test_hash_deterministic() {
        let h1 = fnv_hash(b"hello");
        let h2 = fnv_hash(b"hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different() {
        let h1 = fnv_hash(b"hello");
        let h2 = fnv_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_rebalance() {
        let mut dht = Dht::new(10);
        dht.add_node("node1");
        dht.put("a", "1");
        dht.put("b", "2");
        dht.add_node("node2");
        let reassigned = dht.rebalance();
        assert_eq!(reassigned, 2);
    }

    #[test]
    fn test_put_no_nodes() {
        let mut dht = Dht::new(3);
        assert!(!dht.put("key", "val"));
    }

    #[test]
    fn test_overwrite() {
        let mut dht = Dht::new(3);
        dht.add_node("node1");
        dht.put("key", "v1");
        dht.put("key", "v2");
        assert_eq!(dht.get("key"), Some("v2"));
    }

    #[test]
    fn test_virtual_nodes_in_ring() {
        let mut dht = Dht::new(5);
        dht.add_node("node1");
        assert_eq!(dht.ring.len(), 5);
    }

    #[test]
    fn test_hash_key() {
        let dht = Dht::new(3);
        let h = dht.hash_key("test");
        assert!(h != 0);
    }
}
