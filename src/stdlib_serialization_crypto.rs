//! v89 Stdlib Expansion II: serialization traits, parser combinators, and crypto-safe helpers.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerdeError {
    UnexpectedEof,
    InvalidFormat(String),
}

pub trait BinarySerializable: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(input: &[u8]) -> Result<Self, SerdeError>;
}

pub trait TextSerializable: Sized {
    fn to_text(&self) -> String;
    fn from_text(input: &str) -> Result<Self, SerdeError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireMessage {
    pub id: u32,
    pub kind: u8,
    pub payload: Vec<u8>,
}

impl BinarySerializable for WireMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(9 + self.payload.len());
        out.extend_from_slice(&self.id.to_be_bytes());
        out.push(self.kind);
        out.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    fn from_bytes(input: &[u8]) -> Result<Self, SerdeError> {
        if input.len() < 9 {
            return Err(SerdeError::UnexpectedEof);
        }
        let id = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);
        let kind = input[4];
        let len = u32::from_be_bytes([input[5], input[6], input[7], input[8]]) as usize;
        if input.len() != 9 + len {
            return Err(SerdeError::InvalidFormat("payload length mismatch".to_string()));
        }
        Ok(Self {
            id,
            kind,
            payload: input[9..].to_vec(),
        })
    }
}

impl TextSerializable for WireMessage {
    fn to_text(&self) -> String {
        format!("{}:{}:{}", self.id, self.kind, to_hex(&self.payload))
    }

    fn from_text(input: &str) -> Result<Self, SerdeError> {
        let parts: Vec<&str> = input.split(':').collect();
        if parts.len() != 3 {
            return Err(SerdeError::InvalidFormat("expected 3 fields".to_string()));
        }
        let id = parts[0]
            .parse::<u32>()
            .map_err(|_| SerdeError::InvalidFormat("invalid id".to_string()))?;
        let kind = parts[1]
            .parse::<u8>()
            .map_err(|_| SerdeError::InvalidFormat("invalid kind".to_string()))?;
        let payload = from_hex(parts[2])?;
        Ok(Self { id, kind, payload })
    }
}

pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

pub fn from_hex(s: &str) -> Result<Vec<u8>, SerdeError> {
    if !s.len().is_multiple_of(2) {
        return Err(SerdeError::InvalidFormat("hex length must be even".to_string()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let pair = std::str::from_utf8(&bytes[i..i + 2])
            .map_err(|_| SerdeError::InvalidFormat("invalid utf8 hex".to_string()))?;
        let b = u8::from_str_radix(pair, 16)
            .map_err(|_| SerdeError::InvalidFormat("invalid hex byte".to_string()))?;
        out.push(b);
        i += 2;
    }
    Ok(out)
}

#[derive(Clone)]
pub struct Cursor<'a> {
    data: &'a [u8],
    pub pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn read_u8(&mut self) -> Result<u8, SerdeError> {
        if self.pos >= self.data.len() {
            return Err(SerdeError::UnexpectedEof);
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub fn read_u16_be(&mut self) -> Result<u16, SerdeError> {
        let hi = self.read_u8()? as u16;
        let lo = self.read_u8()? as u16;
        Ok((hi << 8) | lo)
    }

    pub fn expect_tag(&mut self, tag: u8) -> Result<(), SerdeError> {
        let got = self.read_u8()?;
        if got == tag {
            Ok(())
        } else {
            Err(SerdeError::InvalidFormat(format!(
                "tag mismatch: expected {}, got {}",
                tag, got
            )))
        }
    }
}

pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Deterministic key derivation for local utility usage.
pub fn derive_key(material: &[u8], context: &str, out_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(out_len);
    let mut counter = 0u64;
    while out.len() < out_len {
        let mut block = material.to_vec();
        block.extend_from_slice(context.as_bytes());
        block.extend_from_slice(&counter.to_be_bytes());
        let digest = fnv1a64(&block).to_be_bytes();
        out.extend_from_slice(&digest);
        counter = counter.wrapping_add(1);
    }
    out.truncate(out_len);
    out
}

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerdeError::UnexpectedEof => write!(f, "unexpected eof"),
            SerdeError::InvalidFormat(msg) => write!(f, "invalid format: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_roundtrip() {
        let m = WireMessage {
            id: 7,
            kind: 2,
            payload: vec![1, 2, 3, 4],
        };
        let bytes = m.to_bytes();
        let decoded = WireMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, m);
    }

    #[test]
    fn test_text_roundtrip_hex_fixture() {
        let fixture = "42:9:deadbeef";
        let msg = WireMessage::from_text(fixture).unwrap();
        assert_eq!(msg.id, 42);
        assert_eq!(msg.kind, 9);
        assert_eq!(msg.payload, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(msg.to_text(), fixture);
    }

    #[test]
    fn test_parser_cursor_combinators() {
        let mut c = Cursor::new(&[0xaa, 0x01, 0x02]);
        c.expect_tag(0xaa).unwrap();
        let val = c.read_u16_be().unwrap();
        assert_eq!(val, 0x0102);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }

    #[test]
    fn test_derive_key_deterministic() {
        let k1 = derive_key(b"master", "vitalis", 24);
        let k2 = derive_key(b"master", "vitalis", 24);
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 24);
    }

    #[test]
    fn test_hex_decode_rejects_invalid() {
        assert!(from_hex("abc").is_err());
        assert!(from_hex("zz").is_err());
    }
}
