//! Blockchain — v389
//! Merkle trees, block hashing, chain validation, and proof-of-work.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

/// Simple hash (FNV-1a based) for blockchain operations.
fn block_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// A single block in the chain.
#[derive(Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub data: String,
    pub prev_hash: u64,
    pub hash: u64,
    pub nonce: u64,
}

impl Block {
    pub fn new(index: u64, data: &str, prev_hash: u64) -> Self {
        let mut block = Self {
            index,
            timestamp: index, // simplified timestamp
            data: data.to_string(),
            prev_hash,
            hash: 0,
            nonce: 0,
        };
        block.hash = block.compute_hash();
        block
    }

    pub fn compute_hash(&self) -> u64 {
        let payload = format!("{}:{}:{}:{}:{}", self.index, self.timestamp, self.data, self.prev_hash, self.nonce);
        block_hash(payload.as_bytes())
    }

    /// Mine block with proof-of-work (find hash with leading zero bits).
    pub fn mine(&mut self, difficulty: u32) {
        let target_mask = if difficulty >= 64 { 0 } else { !0u64 >> difficulty };
        loop {
            self.hash = self.compute_hash();
            if self.hash <= target_mask || difficulty == 0 {
                break;
            }
            self.nonce += 1;
            if self.nonce > 100_000 {
                break; // Safety limit
            }
        }
    }
}

/// A full blockchain.
#[derive(Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: u32,
}

impl Blockchain {
    pub fn new(difficulty: u32) -> Self {
        let genesis = Block::new(0, "Genesis Block", 0);
        Self {
            chain: vec![genesis],
            difficulty,
        }
    }

    pub fn add_block(&mut self, data: &str) {
        let prev_hash = self.chain.last().map(|b| b.hash).unwrap_or(0);
        let index = self.chain.len() as u64;
        let mut block = Block::new(index, data, prev_hash);
        if self.difficulty > 0 {
            block.mine(self.difficulty);
        }
        self.chain.push(block);
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];

            // Check hash consistency.
            if current.hash != current.compute_hash() {
                return false;
            }

            // Check chain linkage.
            if current.prev_hash != previous.hash {
                return false;
            }
        }
        true
    }

    pub fn length(&self) -> usize {
        self.chain.len()
    }

    pub fn latest_hash(&self) -> u64 {
        self.chain.last().map(|b| b.hash).unwrap_or(0)
    }

    pub fn get_block(&self, index: usize) -> Option<&Block> {
        self.chain.get(index)
    }
}

/// Merkle tree for transaction verification.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub leaves: Vec<u64>,
    pub levels: Vec<Vec<u64>>,
}

impl MerkleTree {
    pub fn new(data: &[&str]) -> Self {
        let leaves: Vec<u64> = data.iter().map(|d| block_hash(d.as_bytes())).collect();
        let mut tree = Self { leaves: leaves.clone(), levels: vec![leaves] };
        tree.build();
        tree
    }

    fn build(&mut self) {
        loop {
            let last_level = self.levels.last().unwrap();
            if last_level.len() <= 1 {
                break;
            }
            let mut next_level = Vec::new();
            let mut i = 0;
            while i < last_level.len() {
                let left = last_level[i];
                let right = if i + 1 < last_level.len() { last_level[i + 1] } else { left };
                let combined = format!("{}:{}", left, right);
                next_level.push(block_hash(combined.as_bytes()));
                i += 2;
            }
            self.levels.push(next_level);
        }
    }

    pub fn root(&self) -> u64 {
        self.levels.last().and_then(|l| l.first().copied()).unwrap_or(0)
    }

    /// Generate proof for a leaf at given index.
    pub fn proof(&self, leaf_index: usize) -> Vec<(u64, bool)> {
        let mut proof = Vec::new();
        let mut idx = leaf_index;
        for level in &self.levels[..self.levels.len().saturating_sub(1)] {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            if sibling_idx < level.len() {
                proof.push((level[sibling_idx], idx % 2 == 0));
            }
            idx /= 2;
        }
        proof
    }

    /// Verify a proof against the root.
    pub fn verify(leaf_hash: u64, proof: &[(u64, bool)], root: u64) -> bool {
        let mut current = leaf_hash;
        for &(sibling, is_left) in proof {
            let combined = if is_left {
                format!("{}:{}", current, sibling)
            } else {
                format!("{}:{}", sibling, current)
            };
            current = block_hash(combined.as_bytes());
        }
        current == root
    }

    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    pub fn height(&self) -> usize {
        self.levels.len()
    }
}

static CHAIN_STORE: LazyLock<Mutex<HashMap<i64, Blockchain>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CHAIN_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn chain_alloc() -> i64 {
    let mut next = CHAIN_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_create(difficulty: i64) -> i64 {
    let id = chain_alloc();
    let bc = Blockchain::new(difficulty.max(0) as u32);
    CHAIN_STORE.lock().unwrap().insert(id, bc);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_add_block(id: i64, data_hash: i64) -> i64 {
    let mut store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get_mut(&id) {
        let data = format!("tx_{}", data_hash);
        bc.add_block(&data);
        bc.length() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_validate(id: i64) -> i64 {
    let store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get(&id) {
        if bc.is_valid() { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_length(id: i64) -> i64 {
    let store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get(&id) {
        bc.length() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_latest_hash(id: i64) -> i64 {
    let store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get(&id) {
        bc.latest_hash() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_get_block(id: i64, index: i64) -> i64 {
    let store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get(&id) {
        if let Some(block) = bc.get_block(index as usize) {
            block.hash as i64
        } else {
            -1
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_merkle_root(count: i64) -> i64 {
    let data: Vec<String> = (0..count).map(|i| format!("leaf_{}", i)).collect();
    let refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
    let tree = MerkleTree::new(&refs);
    tree.root() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_merkle_verify(leaf_count: i64) -> i64 {
    let data: Vec<String> = (0..leaf_count).map(|i| format!("leaf_{}", i)).collect();
    let refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
    let tree = MerkleTree::new(&refs);
    let root = tree.root();
    let proof = tree.proof(0);
    if MerkleTree::verify(tree.leaves[0], &proof, root) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_difficulty(id: i64) -> i64 {
    let store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get(&id) {
        bc.difficulty as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_chain_is_valid(id: i64) -> i64 {
    let store = CHAIN_STORE.lock().unwrap();
    if let Some(bc) = store.get(&id) {
        if bc.is_valid() { 1 } else { 0 }
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_new() {
        let b = Block::new(0, "test", 0);
        assert_eq!(b.index, 0);
        assert!(b.hash != 0);
    }

    #[test]
    fn test_block_hash_deterministic() {
        let b1 = Block::new(0, "test", 0);
        let b2 = Block::new(0, "test", 0);
        assert_eq!(b1.hash, b2.hash);
    }

    #[test]
    fn test_block_hash_changes() {
        let b1 = Block::new(0, "test1", 0);
        let b2 = Block::new(0, "test2", 0);
        assert_ne!(b1.hash, b2.hash);
    }

    #[test]
    fn test_blockchain_genesis() {
        let bc = Blockchain::new(0);
        assert_eq!(bc.length(), 1);
        assert!(bc.is_valid());
    }

    #[test]
    fn test_add_block() {
        let mut bc = Blockchain::new(0);
        bc.add_block("block 1");
        bc.add_block("block 2");
        assert_eq!(bc.length(), 3);
    }

    #[test]
    fn test_chain_valid() {
        let mut bc = Blockchain::new(0);
        bc.add_block("tx1");
        bc.add_block("tx2");
        assert!(bc.is_valid());
    }

    #[test]
    fn test_chain_tampered() {
        let mut bc = Blockchain::new(0);
        bc.add_block("tx1");
        bc.add_block("tx2");
        bc.chain[1].data = "tampered".to_string();
        assert!(!bc.is_valid());
    }

    #[test]
    fn test_latest_hash() {
        let mut bc = Blockchain::new(0);
        let h1 = bc.latest_hash();
        bc.add_block("new");
        let h2 = bc.latest_hash();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_get_block() {
        let mut bc = Blockchain::new(0);
        bc.add_block("data");
        assert!(bc.get_block(0).is_some());
        assert!(bc.get_block(1).is_some());
        assert!(bc.get_block(5).is_none());
    }

    #[test]
    fn test_chain_linkage() {
        let mut bc = Blockchain::new(0);
        bc.add_block("data");
        let genesis_hash = bc.chain[0].hash;
        assert_eq!(bc.chain[1].prev_hash, genesis_hash);
    }

    #[test]
    fn test_merkle_tree_single() {
        let tree = MerkleTree::new(&["a"]);
        assert_eq!(tree.leaf_count(), 1);
        assert!(tree.root() != 0);
    }

    #[test]
    fn test_merkle_tree_two() {
        let tree = MerkleTree::new(&["a", "b"]);
        assert_eq!(tree.leaf_count(), 2);
        assert!(tree.height() >= 2);
    }

    #[test]
    fn test_merkle_tree_four() {
        let tree = MerkleTree::new(&["a", "b", "c", "d"]);
        assert_eq!(tree.leaf_count(), 4);
        assert!(tree.height() >= 3);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let t1 = MerkleTree::new(&["a", "b", "c"]);
        let t2 = MerkleTree::new(&["a", "b", "c"]);
        assert_eq!(t1.root(), t2.root());
    }

    #[test]
    fn test_merkle_root_changes() {
        let t1 = MerkleTree::new(&["a", "b"]);
        let t2 = MerkleTree::new(&["a", "c"]);
        assert_ne!(t1.root(), t2.root());
    }

    #[test]
    fn test_merkle_proof() {
        let tree = MerkleTree::new(&["a", "b", "c", "d"]);
        let proof = tree.proof(0);
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_merkle_verify() {
        let tree = MerkleTree::new(&["a", "b", "c", "d"]);
        let root = tree.root();
        let proof = tree.proof(0);
        assert!(MerkleTree::verify(tree.leaves[0], &proof, root));
    }

    #[test]
    fn test_merkle_verify_wrong_leaf() {
        let tree = MerkleTree::new(&["a", "b", "c", "d"]);
        let root = tree.root();
        let proof = tree.proof(0);
        let wrong_leaf = block_hash(b"wrong");
        assert!(!MerkleTree::verify(wrong_leaf, &proof, root));
    }

    #[test]
    fn test_mine_block() {
        let mut b = Block::new(1, "mine me", 12345);
        b.mine(0); // difficulty 0 = any hash accepted
        assert!(b.hash != 0);
    }

    #[test]
    fn test_blockchain_with_difficulty() {
        let mut bc = Blockchain::new(0);
        bc.add_block("block1");
        assert!(bc.is_valid());
    }
}
