//! Software Transactional Memory — v368
//! STM with TVars, atomic transactions, retry, and orElse composability.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_TVAR_ID: AtomicI64 = AtomicI64::new(1);

#[derive(Debug, Clone)]
pub struct TVar {
    pub id: i64,
    pub value: i64,
    pub version: u64,
}

impl TVar {
    pub fn new(value: i64) -> Self {
        let id = NEXT_TVAR_ID.fetch_add(1, Ordering::Relaxed);
        Self { id, value, version: 0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TxnState {
    Active,
    Committed,
    Aborted,
    Retrying,
}

#[derive(Debug)]
pub struct Transaction {
    pub read_set: HashMap<i64, (i64, u64)>,
    pub write_set: HashMap<i64, i64>,
    pub state: TxnState,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            read_set: HashMap::new(),
            write_set: HashMap::new(),
            state: TxnState::Active,
        }
    }

    pub fn read(&mut self, tvar: &TVar) -> i64 {
        if let Some(&val) = self.write_set.get(&tvar.id) {
            return val;
        }
        self.read_set.insert(tvar.id, (tvar.value, tvar.version));
        tvar.value
    }

    pub fn write(&mut self, tvar_id: i64, value: i64) {
        if self.state == TxnState::Active {
            self.write_set.insert(tvar_id, value);
        }
    }

    pub fn validate(&self, store: &StmStore) -> bool {
        for (&id, &(_, version)) in &self.read_set {
            if let Some(tvar) = store.vars.get(&id) {
                if tvar.version != version {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn commit(&mut self, store: &mut StmStore) -> bool {
        if self.state != TxnState::Active {
            return false;
        }
        if !self.validate(store) {
            self.state = TxnState::Aborted;
            return false;
        }
        for (&id, &val) in &self.write_set {
            if let Some(tvar) = store.vars.get_mut(&id) {
                tvar.value = val;
                tvar.version += 1;
            }
        }
        self.state = TxnState::Committed;
        true
    }

    pub fn abort(&mut self) {
        self.state = TxnState::Aborted;
    }

    pub fn retry(&mut self) {
        self.state = TxnState::Retrying;
    }
}

#[derive(Debug, Default)]
pub struct StmStore {
    pub vars: HashMap<i64, TVar>,
}

impl StmStore {
    pub fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    pub fn new_var(&mut self, value: i64) -> i64 {
        let tvar = TVar::new(value);
        let id = tvar.id;
        self.vars.insert(id, tvar);
        id
    }

    pub fn read_var(&self, id: i64) -> i64 {
        self.vars.get(&id).map(|v| v.value).unwrap_or(-1)
    }

    pub fn atomically<F>(&mut self, f: F) -> bool
    where
        F: Fn(&mut Transaction, &StmStore),
    {
        let max_retries = 10;
        for _ in 0..max_retries {
            let mut txn = Transaction::new();
            f(&mut txn, self);
            if txn.state == TxnState::Retrying {
                continue;
            }
            if txn.commit(self) {
                return true;
            }
        }
        false
    }

    pub fn or_else<F, G>(&mut self, first: F, second: G) -> bool
    where
        F: Fn(&mut Transaction, &StmStore),
        G: Fn(&mut Transaction, &StmStore),
    {
        let mut txn = Transaction::new();
        first(&mut txn, self);
        if txn.state == TxnState::Retrying {
            let mut txn2 = Transaction::new();
            second(&mut txn2, self);
            if txn2.state != TxnState::Retrying {
                return txn2.commit(self);
            }
            return false;
        }
        txn.commit(self)
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static STM_STORE: LazyLock<Mutex<StmStore>> = LazyLock::new(|| Mutex::new(StmStore::new()));

pub extern "C" fn slang_stm_new(value: i64) -> i64 {
    STM_STORE.lock().unwrap().new_var(value)
}

pub extern "C" fn slang_stm_read(id: i64) -> i64 {
    STM_STORE.lock().unwrap().read_var(id)
}

pub extern "C" fn slang_stm_write(id: i64, value: i64) -> i64 {
    let mut store = STM_STORE.lock().unwrap();
    if let Some(tvar) = store.vars.get_mut(&id) {
        tvar.value = value;
        tvar.version += 1;
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_stm_commit() -> i64 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_stm_abort() -> i64 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_stm_retry() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_stm_or_else() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_stm_atomically(var_id: i64, new_val: i64) -> i64 {
    let mut store = STM_STORE.lock().unwrap();
    let success = store.atomically(|txn, _store| {
        txn.write(var_id, new_val);
    });
    if success { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tvar_new() {
        let tv = TVar::new(42);
        assert_eq!(tv.value, 42);
        assert_eq!(tv.version, 0);
    }

    #[test]
    fn test_store_new_var() {
        let mut store = StmStore::new();
        let id = store.new_var(100);
        assert_eq!(store.read_var(id), 100);
    }

    #[test]
    fn test_transaction_read() {
        let tvar = TVar { id: 1, value: 55, version: 0 };
        let mut txn = Transaction::new();
        let val = txn.read(&tvar);
        assert_eq!(val, 55);
        assert!(txn.read_set.contains_key(&1));
    }

    #[test]
    fn test_transaction_write_read() {
        let tvar = TVar { id: 1, value: 10, version: 0 };
        let mut txn = Transaction::new();
        txn.write(1, 20);
        let val = txn.read(&tvar);
        assert_eq!(val, 20);
    }

    #[test]
    fn test_transaction_commit() {
        let mut store = StmStore::new();
        let id = store.new_var(10);
        let mut txn = Transaction::new();
        let _ = txn.read(store.vars.get(&id).unwrap());
        txn.write(id, 30);
        assert!(txn.commit(&mut store));
        assert_eq!(store.read_var(id), 30);
    }

    #[test]
    fn test_transaction_abort() {
        let mut txn = Transaction::new();
        txn.abort();
        assert_eq!(txn.state, TxnState::Aborted);
    }

    #[test]
    fn test_validation_success() {
        let mut store = StmStore::new();
        let id = store.new_var(10);
        let mut txn = Transaction::new();
        txn.read(store.vars.get(&id).unwrap());
        assert!(txn.validate(&store));
    }

    #[test]
    fn test_validation_failure() {
        let mut store = StmStore::new();
        let id = store.new_var(10);
        let mut txn = Transaction::new();
        txn.read(store.vars.get(&id).unwrap());
        store.vars.get_mut(&id).unwrap().version += 1;
        assert!(!txn.validate(&store));
    }

    #[test]
    fn test_atomically_success() {
        let mut store = StmStore::new();
        let id = store.new_var(0);
        let result = store.atomically(|txn, _| {
            txn.write(id, 42);
        });
        assert!(result);
        assert_eq!(store.read_var(id), 42);
    }

    #[test]
    fn test_retry_state() {
        let mut txn = Transaction::new();
        txn.retry();
        assert_eq!(txn.state, TxnState::Retrying);
    }

    #[test]
    fn test_commit_aborted_fails() {
        let mut store = StmStore::new();
        let mut txn = Transaction::new();
        txn.abort();
        assert!(!txn.commit(&mut store));
    }

    #[test]
    fn test_write_set_isolation() {
        let mut txn = Transaction::new();
        txn.write(1, 100);
        txn.write(2, 200);
        assert_eq!(txn.write_set.len(), 2);
        assert_eq!(txn.write_set[&1], 100);
        assert_eq!(txn.write_set[&2], 200);
    }

    #[test]
    fn test_multiple_vars() {
        let mut store = StmStore::new();
        let a = store.new_var(10);
        let b = store.new_var(20);
        let c = store.new_var(30);
        assert_eq!(store.read_var(a), 10);
        assert_eq!(store.read_var(b), 20);
        assert_eq!(store.read_var(c), 30);
    }

    #[test]
    fn test_read_nonexistent() {
        let store = StmStore::new();
        assert_eq!(store.read_var(999), -1);
    }

    #[test]
    fn test_version_increment() {
        let mut store = StmStore::new();
        let id = store.new_var(10);
        let mut txn = Transaction::new();
        txn.write(id, 20);
        txn.commit(&mut store);
        assert_eq!(store.vars[&id].version, 1);
    }

    #[test]
    fn test_or_else_first_succeeds() {
        let mut store = StmStore::new();
        let id = store.new_var(0);
        let result = store.or_else(
            |txn, _| { txn.write(id, 1); },
            |txn, _| { txn.write(id, 2); },
        );
        assert!(result);
        assert_eq!(store.read_var(id), 1);
    }

    #[test]
    fn test_or_else_first_retries() {
        let mut store = StmStore::new();
        let id = store.new_var(0);
        let result = store.or_else(
            |txn, _| { txn.retry(); },
            |txn, _| { txn.write(id, 99); },
        );
        assert!(result);
        assert_eq!(store.read_var(id), 99);
    }

    #[test]
    fn test_empty_transaction() {
        let mut store = StmStore::new();
        let mut txn = Transaction::new();
        assert!(txn.commit(&mut store));
        assert_eq!(txn.state, TxnState::Committed);
    }

    #[test]
    fn test_write_after_abort_ignored() {
        let mut txn = Transaction::new();
        txn.abort();
        txn.write(1, 42);
        assert!(txn.write_set.is_empty());
    }

    #[test]
    fn test_new_transaction_state() {
        let txn = Transaction::new();
        assert_eq!(txn.state, TxnState::Active);
        assert!(txn.read_set.is_empty());
        assert!(txn.write_set.is_empty());
    }
}
