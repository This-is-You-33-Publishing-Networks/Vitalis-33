//! TLS Engine — v390
//! TLS 1.3 handshake simulation, key derivation, record encryption/decryption.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

/// Simple HMAC-like key derivation (for simulation purposes).
fn derive_key(secret: u64, label: &str) -> u64 {
    let mut hash: u64 = secret;
    for byte in label.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// XOR-based symmetric cipher (for simulation).
fn xor_cipher(data: &[u8], key: u64) -> Vec<u8> {
    let key_bytes = key.to_le_bytes();
    data.iter().enumerate().map(|(i, &b)| b ^ key_bytes[i % 8]).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum TlsState {
    Initial,
    ClientHello,
    ServerHello,
    Handshake,
    Established,
    Closed,
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub subject: String,
    pub issuer: String,
    pub serial: u64,
    pub valid: bool,
}

impl Certificate {
    pub fn self_signed(subject: &str) -> Self {
        let serial = derive_key(0x12345, subject);
        Self {
            subject: subject.to_string(),
            issuer: subject.to_string(),
            serial,
            valid: true,
        }
    }

    pub fn verify(&self) -> bool {
        self.valid && !self.subject.is_empty()
    }
}

#[derive(Debug)]
pub struct TlsSession {
    pub id: i64,
    pub state: TlsState,
    pub client_random: u64,
    pub server_random: u64,
    pub session_key: u64,
    pub certificate: Option<Certificate>,
    pub cipher_suite: String,
    pub bytes_encrypted: usize,
    pub bytes_decrypted: usize,
}

impl TlsSession {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            state: TlsState::Initial,
            client_random: 0,
            server_random: 0,
            session_key: 0,
            certificate: None,
            cipher_suite: "TLS_AES_128_GCM_SHA256".to_string(),
            bytes_encrypted: 0,
            bytes_decrypted: 0,
        }
    }

    /// Simulate TLS 1.3 handshake.
    pub fn handshake(&mut self, client_random: u64, server_random: u64) -> bool {
        self.state = TlsState::ClientHello;
        self.client_random = client_random;

        self.state = TlsState::ServerHello;
        self.server_random = server_random;

        // Key schedule (simplified HKDF).
        let pre_master = client_random ^ server_random;
        let master_secret = derive_key(pre_master, "master_secret");
        self.session_key = derive_key(master_secret, "session_key");

        self.state = TlsState::Handshake;

        // Generate self-signed certificate.
        self.certificate = Some(Certificate::self_signed("localhost"));

        self.state = TlsState::Established;
        true
    }

    /// Encrypt a record.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Option<Vec<u8>> {
        if self.state != TlsState::Established {
            return None;
        }
        let ciphertext = xor_cipher(plaintext, self.session_key);
        self.bytes_encrypted += plaintext.len();
        Some(ciphertext)
    }

    /// Decrypt a record.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        if self.state != TlsState::Established {
            return None;
        }
        let plaintext = xor_cipher(ciphertext, self.session_key);
        self.bytes_decrypted += ciphertext.len();
        Some(plaintext)
    }

    /// Derive a new key from the session.
    pub fn derive_application_key(&self, label: &str) -> u64 {
        derive_key(self.session_key, label)
    }

    pub fn is_secure(&self) -> bool {
        self.state == TlsState::Established && self.session_key != 0
    }

    pub fn close(&mut self) {
        self.state = TlsState::Closed;
        self.session_key = 0;
    }
}

static TLS_STORE: LazyLock<Mutex<HashMap<i64, TlsSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static TLS_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn tls_alloc() -> i64 {
    let mut next = TLS_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_handshake(client_random: i64, server_random: i64) -> i64 {
    let id = tls_alloc();
    let mut session = TlsSession::new(id);
    if session.handshake(client_random as u64, server_random as u64) {
        TLS_STORE.lock().unwrap().insert(id, session);
        id
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_encrypt(id: i64, data: i64) -> i64 {
    let mut store = TLS_STORE.lock().unwrap();
    if let Some(session) = store.get_mut(&id) {
        let plaintext = data.to_le_bytes();
        if let Some(ct) = session.encrypt(&plaintext) {
            let mut result_bytes = [0u8; 8];
            for (i, &b) in ct.iter().enumerate().take(8) {
                result_bytes[i] = b;
            }
            i64::from_le_bytes(result_bytes)
        } else {
            -1
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_decrypt(id: i64, data: i64) -> i64 {
    let mut store = TLS_STORE.lock().unwrap();
    if let Some(session) = store.get_mut(&id) {
        let ciphertext = data.to_le_bytes();
        if let Some(pt) = session.decrypt(&ciphertext) {
            let mut result_bytes = [0u8; 8];
            for (i, &b) in pt.iter().enumerate().take(8) {
                result_bytes[i] = b;
            }
            i64::from_le_bytes(result_bytes)
        } else {
            -1
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_derive_key(id: i64, label_hash: i64) -> i64 {
    let store = TLS_STORE.lock().unwrap();
    if let Some(session) = store.get(&id) {
        let label = format!("key_{}", label_hash);
        session.derive_application_key(&label) as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_verify_cert(id: i64) -> i64 {
    let store = TLS_STORE.lock().unwrap();
    if let Some(session) = store.get(&id) {
        if let Some(cert) = &session.certificate {
            if cert.verify() { 1 } else { 0 }
        } else {
            0
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_create_cert(subject_hash: i64) -> i64 {
    let cert = Certificate::self_signed(&format!("host_{}", subject_hash));
    cert.serial as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_session_id(id: i64) -> i64 {
    let store = TLS_STORE.lock().unwrap();
    if store.contains_key(&id) { id } else { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tls_is_secure(id: i64) -> i64 {
    let store = TLS_STORE.lock().unwrap();
    if let Some(session) = store.get(&id) {
        if session.is_secure() { 1 } else { 0 }
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let s = TlsSession::new(1);
        assert_eq!(s.state, TlsState::Initial);
        assert!(!s.is_secure());
    }

    #[test]
    fn test_handshake() {
        let mut s = TlsSession::new(1);
        assert!(s.handshake(12345, 67890));
        assert_eq!(s.state, TlsState::Established);
    }

    #[test]
    fn test_is_secure() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        assert!(s.is_secure());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        let plaintext = b"hello tls";
        let ct = s.encrypt(plaintext).unwrap();
        let pt = s.decrypt(&ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_encrypt_before_handshake() {
        let mut s = TlsSession::new(1);
        assert!(s.encrypt(b"test").is_none());
    }

    #[test]
    fn test_decrypt_before_handshake() {
        let mut s = TlsSession::new(1);
        assert!(s.decrypt(b"test").is_none());
    }

    #[test]
    fn test_different_keys() {
        let mut s1 = TlsSession::new(1);
        s1.handshake(100, 200);
        let mut s2 = TlsSession::new(2);
        s2.handshake(300, 400);
        assert_ne!(s1.session_key, s2.session_key);
    }

    #[test]
    fn test_derive_key() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        let k1 = s.derive_application_key("read");
        let k2 = s.derive_application_key("write");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_certificate_self_signed() {
        let cert = Certificate::self_signed("localhost");
        assert_eq!(cert.subject, "localhost");
        assert_eq!(cert.issuer, "localhost");
        assert!(cert.valid);
    }

    #[test]
    fn test_certificate_verify() {
        let cert = Certificate::self_signed("test.com");
        assert!(cert.verify());
    }

    #[test]
    fn test_certificate_invalid() {
        let mut cert = Certificate::self_signed("test.com");
        cert.valid = false;
        assert!(!cert.verify());
    }

    #[test]
    fn test_close_session() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        s.close();
        assert_eq!(s.state, TlsState::Closed);
        assert!(!s.is_secure());
    }

    #[test]
    fn test_encrypt_after_close() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        s.close();
        assert!(s.encrypt(b"test").is_none());
    }

    #[test]
    fn test_bytes_tracking() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        s.encrypt(b"hello");
        assert_eq!(s.bytes_encrypted, 5);
    }

    #[test]
    fn test_xor_cipher_roundtrip() {
        let key: u64 = 0xDEADBEEF;
        let data = b"test data 12345";
        let encrypted = xor_cipher(data, key);
        let decrypted = xor_cipher(&encrypted, key);
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let k1 = derive_key(42, "test");
        let k2 = derive_key(42, "test");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_derive_key_different_inputs() {
        let k1 = derive_key(42, "test");
        let k2 = derive_key(43, "test");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_cipher_suite() {
        let s = TlsSession::new(1);
        assert_eq!(s.cipher_suite, "TLS_AES_128_GCM_SHA256");
    }

    #[test]
    fn test_handshake_generates_cert() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        assert!(s.certificate.is_some());
    }

    #[test]
    fn test_session_key_nonzero() {
        let mut s = TlsSession::new(1);
        s.handshake(100, 200);
        assert_ne!(s.session_key, 0);
    }
}
