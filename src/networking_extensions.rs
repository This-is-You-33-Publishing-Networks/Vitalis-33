//! v86 Networking extensions: TLS-ready sockets, framed protocols, and timeout/cancellation integration.

use crate::concurrency::ConcurrencyError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsMode {
    Disabled,
    Opportunistic,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsConfig {
    pub mode: TlsMode,
    pub sni: Option<String>,
    pub alpn_protocols: Vec<String>,
    pub min_tls_version: u16,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            mode: TlsMode::Opportunistic,
            sni: None,
            alpn_protocols: vec!["http/1.1".to_string()],
            min_tls_version: 0x0303,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketEndpoint {
    pub host: String,
    pub port: u16,
    pub tls: TlsConfig,
}

impl SocketEndpoint {
    pub fn new(host: &str, port: u16, tls: TlsConfig) -> Self {
        Self {
            host: host.to_string(),
            port,
            tls,
        }
    }

    pub fn validate(&self) -> Result<(), ConcurrencyError> {
        if self.host.trim().is_empty() {
            return Err(ConcurrencyError::InvalidOperation("endpoint host is empty".to_string()));
        }
        if self.port == 0 {
            return Err(ConcurrencyError::InvalidOperation("endpoint port must be non-zero".to_string()));
        }
        if matches!(self.tls.mode, TlsMode::Required) && self.tls.min_tls_version < 0x0303 {
            return Err(ConcurrencyError::InvalidOperation(
                "TLS required endpoints must use TLS 1.2+".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub stream_id: u32,
    pub kind: u8,
    pub payload_len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFrame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl ProtocolFrame {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(9 + self.payload.len());
        out.extend_from_slice(&self.header.stream_id.to_be_bytes());
        out.push(self.header.kind);
        out.extend_from_slice(&self.header.payload_len.to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ConcurrencyError> {
        if bytes.len() < 9 {
            return Err(ConcurrencyError::InvalidOperation("frame too short".to_string()));
        }
        let stream_id = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let kind = bytes[4];
        let payload_len = u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);

        let expected = 9usize + payload_len as usize;
        if bytes.len() != expected {
            return Err(ConcurrencyError::InvalidOperation(format!(
                "frame length mismatch: expected {}, got {}",
                expected,
                bytes.len()
            )));
        }

        Ok(Self {
            header: FrameHeader {
                stream_id,
                kind,
                payload_len,
            },
            payload: bytes[9..].to_vec(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationToken {
    cancelled: bool,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self { cancelled: false }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutController {
    deadline_ms: u64,
}

impl TimeoutController {
    pub fn new(deadline_ms: u64) -> Self {
        Self { deadline_ms }
    }

    pub fn check(&self, now_ms: u64) -> Result<(), ConcurrencyError> {
        if now_ms > self.deadline_ms {
            Err(ConcurrencyError::Timeout)
        } else {
            Ok(())
        }
    }
}

pub fn send_with_control(
    endpoint: &SocketEndpoint,
    frame: &ProtocolFrame,
    token: &CancellationToken,
    timeout: &TimeoutController,
    now_ms: u64,
) -> Result<usize, ConcurrencyError> {
    endpoint.validate()?;
    if token.is_cancelled() {
        return Err(ConcurrencyError::TaskCancelled);
    }
    timeout.check(now_ms)?;

    // Simulated send path returns encoded byte count.
    Ok(frame.encode().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_required_endpoint_validation() {
        let endpoint = SocketEndpoint::new(
            "api.example",
            443,
            TlsConfig {
                mode: TlsMode::Required,
                sni: Some("api.example".to_string()),
                alpn_protocols: vec!["h2".to_string()],
                min_tls_version: 0x0303,
            },
        );
        assert!(endpoint.validate().is_ok());
    }

    #[test]
    fn test_tls_required_rejects_old_version() {
        let endpoint = SocketEndpoint::new(
            "api.example",
            443,
            TlsConfig {
                mode: TlsMode::Required,
                sni: None,
                alpn_protocols: vec!["http/1.1".to_string()],
                min_tls_version: 0x0301,
            },
        );
        assert!(endpoint.validate().is_err());
    }

    #[test]
    fn test_protocol_frame_roundtrip() {
        let frame = ProtocolFrame {
            header: FrameHeader {
                stream_id: 7,
                kind: 2,
                payload_len: 4,
            },
            payload: vec![1, 2, 3, 4],
        };

        let encoded = frame.encode();
        let decoded = ProtocolFrame::decode(&encoded).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn test_protocol_frame_length_mismatch() {
        let bytes = vec![0, 0, 0, 1, 1, 0, 0, 0, 3, 9, 9];
        assert!(ProtocolFrame::decode(&bytes).is_err());
    }

    #[test]
    fn test_send_with_control_cancelled() {
        let endpoint = SocketEndpoint::new("svc", 8080, TlsConfig::default());
        let frame = ProtocolFrame {
            header: FrameHeader {
                stream_id: 1,
                kind: 1,
                payload_len: 1,
            },
            payload: vec![42],
        };
        let mut token = CancellationToken::new();
        token.cancel();
        let timeout = TimeoutController::new(1000);

        let result = send_with_control(&endpoint, &frame, &token, &timeout, 100);
        assert_eq!(result, Err(ConcurrencyError::TaskCancelled));
    }

    #[test]
    fn test_send_with_control_timeout() {
        let endpoint = SocketEndpoint::new("svc", 8080, TlsConfig::default());
        let frame = ProtocolFrame {
            header: FrameHeader {
                stream_id: 1,
                kind: 1,
                payload_len: 1,
            },
            payload: vec![42],
        };
        let token = CancellationToken::new();
        let timeout = TimeoutController::new(1000);

        let result = send_with_control(&endpoint, &frame, &token, &timeout, 1001);
        assert_eq!(result, Err(ConcurrencyError::Timeout));
    }

    #[test]
    fn test_property_like_frame_decode_variants() {
        // Property-style sweep over multiple payload sizes and stream IDs.
        for stream_id in [1u32, 2, 17, 1024] {
            for payload_size in [0usize, 1, 3, 8, 32] {
                let payload: Vec<u8> = (0..payload_size).map(|i| (i % 251) as u8).collect();
                let frame = ProtocolFrame {
                    header: FrameHeader {
                        stream_id,
                        kind: 9,
                        payload_len: payload.len() as u32,
                    },
                    payload,
                };
                let encoded = frame.encode();
                let decoded = ProtocolFrame::decode(&encoded).unwrap();
                assert_eq!(decoded, frame);
            }
        }
    }
}
