//! Debug Adapter Protocol (DAP) Support
//!
//! Implements the core DAP types and session logic for IDE debugging:
//! - Breakpoint management
//! - Variable inspection
//! - Call stack frames
//! - Step operations (in, over, out, continue)
//! - Expression evaluation during debug

use std::collections::HashMap;

/// Source location for breakpoints and stack frames
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Breakpoint state
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub location: SourceLocation,
    pub enabled: bool,
    pub condition: Option<String>,
    pub hit_count: u32,
}

/// A call stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub id: u32,
    pub name: String,
    pub location: SourceLocation,
    pub locals: HashMap<String, DebugValue>,
}

/// Debug value representation
#[derive(Debug, Clone)]
pub enum DebugValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<DebugValue>),
    Struct { name: String, fields: Vec<(String, DebugValue)> },
    Null,
}

impl std::fmt::Display for DebugValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugValue::Int(v) => write!(f, "{}", v),
            DebugValue::Float(v) => write!(f, "{:.6}", v),
            DebugValue::Bool(v) => write!(f, "{}", v),
            DebugValue::Str(v) => write!(f, "\"{}\"", v),
            DebugValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            DebugValue::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                for (i, (key, val)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", key, val)?;
                }
                write!(f, " }}")
            }
            DebugValue::Null => write!(f, "null"),
        }
    }
}

/// Debug execution state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionState {
    Running,
    Paused,
    Stopped,
    SteppingIn,
    SteppingOver,
    SteppingOut,
}

/// DAP event types
#[derive(Debug, Clone)]
pub enum DapEvent {
    Initialized,
    Stopped { reason: StopReason },
    Continued,
    Exited { code: i32 },
    Output { text: String, category: OutputCategory },
    Breakpoint { id: u32, verified: bool },
}

/// Reason execution stopped
#[derive(Debug, Clone)]
pub enum StopReason {
    Breakpoint(u32),
    Step,
    Pause,
    Exception(String),
    Entry,
}

/// Output category
#[derive(Debug, Clone)]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
}

/// The debug session
pub struct DebugSession {
    pub breakpoints: Vec<Breakpoint>,
    pub stack: Vec<StackFrame>,
    pub state: ExecutionState,
    pub event_log: Vec<DapEvent>,
    next_bp_id: u32,
    next_frame_id: u32,
    watched_expressions: Vec<String>,
}

impl DebugSession {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            stack: Vec::new(),
            state: ExecutionState::Stopped,
            event_log: Vec::new(),
            next_bp_id: 1,
            next_frame_id: 1,
            watched_expressions: Vec::new(),
        }
    }

    /// Add a breakpoint at a source location
    pub fn add_breakpoint(&mut self, file: &str, line: u32) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint {
            id,
            location: SourceLocation {
                file: file.to_string(),
                line,
                column: 0,
            },
            enabled: true,
            condition: None,
            hit_count: 0,
        });
        self.emit(DapEvent::Breakpoint { id, verified: true });
        id
    }

    /// Add a conditional breakpoint
    pub fn add_conditional_breakpoint(&mut self, file: &str, line: u32, condition: &str) -> u32 {
        let id = self.add_breakpoint(file, line);
        if let Some(bp) = self.breakpoints.iter_mut().find(|b| b.id == id) {
            bp.condition = Some(condition.to_string());
        }
        id
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        let len_before = self.breakpoints.len();
        self.breakpoints.retain(|bp| bp.id != id);
        self.breakpoints.len() < len_before
    }

    /// Toggle breakpoint enabled/disabled
    pub fn toggle_breakpoint(&mut self, id: u32) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|b| b.id == id) {
            bp.enabled = !bp.enabled;
            true
        } else {
            false
        }
    }

    /// Check if we should stop at the given location
    pub fn should_stop(&self, file: &str, line: u32) -> bool {
        self.breakpoints.iter().any(|bp| {
            bp.enabled && bp.location.file == file && bp.location.line == line
        })
    }

    /// Hit a breakpoint at a location
    pub fn hit_breakpoint(&mut self, file: &str, line: u32) -> Option<u32> {
        let bp = self.breakpoints.iter_mut()
            .find(|bp| bp.enabled && bp.location.file == file && bp.location.line == line)?;
        bp.hit_count += 1;
        let id = bp.id;
        self.state = ExecutionState::Paused;
        self.emit(DapEvent::Stopped { reason: StopReason::Breakpoint(id) });
        Some(id)
    }

    /// Push a stack frame
    pub fn push_frame(&mut self, name: &str, file: &str, line: u32) -> u32 {
        let id = self.next_frame_id;
        self.next_frame_id += 1;
        self.stack.push(StackFrame {
            id,
            name: name.to_string(),
            location: SourceLocation {
                file: file.to_string(),
                line,
                column: 0,
            },
            locals: HashMap::new(),
        });
        id
    }

    /// Pop the top stack frame
    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.stack.pop()
    }

    /// Set a local variable in the top frame
    pub fn set_local(&mut self, name: &str, value: DebugValue) {
        if let Some(frame) = self.stack.last_mut() {
            frame.locals.insert(name.to_string(), value);
        }
    }

    /// Get a local variable from the call stack (searches top-down)
    pub fn get_local(&self, name: &str) -> Option<&DebugValue> {
        for frame in self.stack.iter().rev() {
            if let Some(val) = frame.locals.get(name) {
                return Some(val);
            }
        }
        None
    }

    /// Continue execution
    pub fn continue_execution(&mut self) {
        self.state = ExecutionState::Running;
        self.emit(DapEvent::Continued);
    }

    /// Step into next function call
    pub fn step_in(&mut self) {
        self.state = ExecutionState::SteppingIn;
    }

    /// Step over current line
    pub fn step_over(&mut self) {
        self.state = ExecutionState::SteppingOver;
    }

    /// Step out of current function
    pub fn step_out(&mut self) {
        self.state = ExecutionState::SteppingOut;
    }

    /// Pause execution
    pub fn pause(&mut self) {
        self.state = ExecutionState::Paused;
        self.emit(DapEvent::Stopped { reason: StopReason::Pause });
    }

    /// Stop the debug session
    pub fn stop(&mut self, exit_code: i32) {
        self.state = ExecutionState::Stopped;
        self.stack.clear();
        self.emit(DapEvent::Exited { code: exit_code });
    }

    /// Add a watch expression
    pub fn add_watch(&mut self, expr: &str) {
        self.watched_expressions.push(expr.to_string());
    }

    /// Get all watch expressions
    pub fn watches(&self) -> &[String] {
        &self.watched_expressions
    }

    /// Log output
    pub fn output(&mut self, text: &str, category: OutputCategory) {
        self.emit(DapEvent::Output {
            text: text.to_string(),
            category,
        });
    }

    fn emit(&mut self, event: DapEvent) {
        self.event_log.push(event);
    }

    /// Get the number of events logged
    pub fn event_count(&self) -> usize {
        self.event_log.len()
    }

    /// Get current call depth
    pub fn call_depth(&self) -> usize {
        self.stack.len()
    }
}

// ─── DAP Wire Protocol ──────────────────────────────────────────────────

use std::io::{self, BufRead, Write};

/// A parsed DAP request message.
#[derive(Debug, Clone)]
pub struct DapRequest {
    pub seq: i64,
    pub command: String,
    pub arguments: String,
}

/// Read a single DAP message from a buffered reader (Content-Length framed).
pub fn dap_read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    let mut header_line = String::new();

    loop {
        header_line.clear();
        let n = reader.read_line(&mut header_line)?;
        if n == 0 { return Ok(None); }

        let trimmed = header_line.trim();
        if trimmed.is_empty() { break; }

        if let Some(val) = trimmed.strip_prefix("Content-Length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = Some(len);
            }
        }
    }

    let len = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;

    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    String::from_utf8(body).map(Some).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })
}

/// Write a DAP message with Content-Length framing.
pub fn dap_write_message<W: Write>(writer: &mut W, body: &str) -> io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(body.as_bytes())?;
    writer.flush()
}

/// Parse a DAP request from a JSON string (minimal parser).
pub fn parse_dap_request(json: &str) -> Option<DapRequest> {
    let seq = dap_extract_number(json, "seq")?;
    let command = dap_extract_string(json, "command")?;
    let arguments = dap_extract_object(json, "arguments").unwrap_or_default();
    Some(DapRequest { seq, command, arguments })
}

/// Format a DAP response.
pub fn dap_response(request_seq: i64, command: &str, body: &str) -> String {
    format!(
        "{{\"seq\":0,\"type\":\"response\",\"request_seq\":{},\"command\":\"{}\",\"success\":true,\"body\":{}}}",
        request_seq, command, body
    )
}

/// Format a DAP event.
pub fn dap_event(event: &str, body: &str) -> String {
    format!(
        "{{\"seq\":0,\"type\":\"event\",\"event\":\"{}\",\"body\":{}}}",
        event, body
    )
}

/// Handle a single DAP request and produce a response.
pub fn handle_dap_request(session: &mut DebugSession, req: &DapRequest) -> Option<String> {
    match req.command.as_str() {
        "initialize" => {
            session.emit(DapEvent::Initialized);
            Some(dap_response(req.seq, "initialize", "{\"supportsConfigurationDoneRequest\":true,\"supportsFunctionBreakpoints\":true,\"supportsConditionalBreakpoints\":true}"))
        }
        "configurationDone" => {
            Some(dap_response(req.seq, "configurationDone", "null"))
        }
        "launch" => {
            session.continue_execution();
            Some(dap_response(req.seq, "launch", "null"))
        }
        "disconnect" => {
            session.stop(0);
            Some(dap_response(req.seq, "disconnect", "null"))
        }
        "setBreakpoints" => {
            // Acknowledge — breakpoints set via future protocol messages
            Some(dap_response(req.seq, "setBreakpoints", "{\"breakpoints\":[]}"))
        }
        "threads" => {
            Some(dap_response(req.seq, "threads", "{\"threads\":[{\"id\":1,\"name\":\"main\"}]}"))
        }
        "stackTrace" => {
            let frames_json: Vec<String> = session.stack.iter().rev().map(|f| {
                format!(
                    "{{\"id\":{},\"name\":\"{}\",\"source\":{{\"path\":\"{}\"}},\"line\":{},\"column\":{}}}",
                    f.id, f.name, f.location.file, f.location.line, f.location.column
                )
            }).collect();
            Some(dap_response(req.seq, "stackTrace", &format!("{{\"stackFrames\":[{}],\"totalFrames\":{}}}", frames_json.join(","), session.stack.len())))
        }
        "continue" => {
            session.continue_execution();
            Some(dap_response(req.seq, "continue", "{\"allThreadsContinued\":true}"))
        }
        "stepIn" => {
            session.step_in();
            Some(dap_response(req.seq, "stepIn", "null"))
        }
        "stepOut" => {
            session.step_out();
            Some(dap_response(req.seq, "stepOut", "null"))
        }
        "next" => {
            session.step_over();
            Some(dap_response(req.seq, "next", "null"))
        }
        _ => {
            Some(format!(
                "{{\"seq\":0,\"type\":\"response\",\"request_seq\":{},\"command\":\"{}\",\"success\":false,\"message\":\"unsupported command\"}}",
                req.seq, req.command
            ))
        }
    }
}

/// Run the DAP server on stdin/stdout. Entry point for `vtc debug`.
pub fn run_dap_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut session = DebugSession::new();

    loop {
        let msg_text = match dap_read_message(&mut reader)? {
            Some(text) => text,
            None => break,
        };

        let req = match parse_dap_request(&msg_text) {
            Some(req) => req,
            None => continue,
        };

        let is_disconnect = req.command == "disconnect";

        if let Some(response) = handle_dap_request(&mut session, &req) {
            dap_write_message(&mut writer, &response)?;
        }

        if is_disconnect {
            break;
        }
    }

    Ok(())
}

// ─── Minimal JSON Helpers ───────────────────────────────────────────────

fn dap_extract_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if after.starts_with('"') {
        let start = 1;
        let end = after[start..].find('"')?;
        Some(after[start..start + end].to_string())
    } else {
        None
    }
}

fn dap_extract_number(json: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let end = after.find(|c: char| !c.is_ascii_digit() && c != '-').unwrap_or(after.len());
    after[..end].parse().ok()
}

fn dap_extract_object(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let brace = rest.find('{')?;
    let mut depth = 0;
    let start = brace;
    for (i, c) in rest[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dap_session_creation() {
        let session = DebugSession::new();
        assert_eq!(session.state, ExecutionState::Stopped);
        assert!(session.breakpoints.is_empty());
        assert!(session.stack.is_empty());
    }

    #[test]
    fn test_dap_add_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        assert_eq!(id, 1);
        assert_eq!(session.breakpoints.len(), 1);
        assert_eq!(session.breakpoints[0].location.line, 10);
    }

    #[test]
    fn test_dap_conditional_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_conditional_breakpoint("main.sl", 5, "x > 10");
        let bp = session.breakpoints.iter().find(|b| b.id == id).unwrap();
        assert_eq!(bp.condition.as_deref(), Some("x > 10"));
    }

    #[test]
    fn test_dap_remove_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        assert!(session.remove_breakpoint(id));
        assert!(session.breakpoints.is_empty());
    }

    #[test]
    fn test_dap_toggle_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        assert!(session.breakpoints[0].enabled);
        session.toggle_breakpoint(id);
        assert!(!session.breakpoints[0].enabled);
        session.toggle_breakpoint(id);
        assert!(session.breakpoints[0].enabled);
    }

    #[test]
    fn test_dap_should_stop() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 10);
        assert!(session.should_stop("main.sl", 10));
        assert!(!session.should_stop("main.sl", 11));
        assert!(!session.should_stop("other.sl", 10));
    }

    #[test]
    fn test_dap_hit_breakpoint() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 10);
        let id = session.hit_breakpoint("main.sl", 10);
        assert_eq!(id, Some(1));
        assert_eq!(session.state, ExecutionState::Paused);
        assert_eq!(session.breakpoints[0].hit_count, 1);
    }

    #[test]
    fn test_dap_stack_frames() {
        let mut session = DebugSession::new();
        session.push_frame("main", "main.sl", 1);
        session.push_frame("helper", "main.sl", 20);
        assert_eq!(session.call_depth(), 2);
        let frame = session.pop_frame().unwrap();
        assert_eq!(frame.name, "helper");
        assert_eq!(session.call_depth(), 1);
    }

    #[test]
    fn test_dap_locals() {
        let mut session = DebugSession::new();
        session.push_frame("main", "main.sl", 1);
        session.set_local("x", DebugValue::Int(42));
        let val = session.get_local("x").unwrap();
        assert!(matches!(val, DebugValue::Int(42)));
    }

    #[test]
    fn test_dap_continue() {
        let mut session = DebugSession::new();
        session.state = ExecutionState::Paused;
        session.continue_execution();
        assert_eq!(session.state, ExecutionState::Running);
    }

    #[test]
    fn test_dap_step_in() {
        let mut session = DebugSession::new();
        session.step_in();
        assert_eq!(session.state, ExecutionState::SteppingIn);
    }

    #[test]
    fn test_dap_step_over() {
        let mut session = DebugSession::new();
        session.step_over();
        assert_eq!(session.state, ExecutionState::SteppingOver);
    }

    #[test]
    fn test_dap_step_out() {
        let mut session = DebugSession::new();
        session.step_out();
        assert_eq!(session.state, ExecutionState::SteppingOut);
    }

    #[test]
    fn test_dap_pause() {
        let mut session = DebugSession::new();
        session.state = ExecutionState::Running;
        session.pause();
        assert_eq!(session.state, ExecutionState::Paused);
    }

    #[test]
    fn test_dap_stop() {
        let mut session = DebugSession::new();
        session.push_frame("main", "main.sl", 1);
        session.stop(0);
        assert_eq!(session.state, ExecutionState::Stopped);
        assert!(session.stack.is_empty());
    }

    #[test]
    fn test_dap_watches() {
        let mut session = DebugSession::new();
        session.add_watch("x + y");
        session.add_watch("arr.len()");
        assert_eq!(session.watches().len(), 2);
    }

    #[test]
    fn test_dap_debug_value_display() {
        assert_eq!(format!("{}", DebugValue::Int(42)), "42");
        assert_eq!(format!("{}", DebugValue::Bool(true)), "true");
        assert_eq!(format!("{}", DebugValue::Str("hi".into())), "\"hi\"");
        assert_eq!(format!("{}", DebugValue::Null), "null");
    }

    #[test]
    fn test_dap_debug_value_list_display() {
        let list = DebugValue::List(vec![
            DebugValue::Int(1),
            DebugValue::Int(2),
            DebugValue::Int(3),
        ]);
        assert_eq!(format!("{}", list), "[1, 2, 3]");
    }

    #[test]
    fn test_dap_debug_value_struct_display() {
        let s = DebugValue::Struct {
            name: "Point".into(),
            fields: vec![
                ("x".into(), DebugValue::Int(10)),
                ("y".into(), DebugValue::Int(20)),
            ],
        };
        assert_eq!(format!("{}", s), "Point { x: 10, y: 20 }");
    }

    #[test]
    fn test_dap_event_count() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 1);
        session.continue_execution();
        session.pause();
        assert_eq!(session.event_count(), 3); // bp verified + continued + stopped
    }

    #[test]
    fn test_dap_output_event() {
        let mut session = DebugSession::new();
        session.output("Hello world", OutputCategory::Stdout);
        assert_eq!(session.event_count(), 1);
    }

    #[test]
    fn test_dap_remove_nonexistent_bp() {
        let mut session = DebugSession::new();
        assert!(!session.remove_breakpoint(99));
    }

    #[test]
    fn test_dap_toggle_nonexistent_bp() {
        let mut session = DebugSession::new();
        assert!(!session.toggle_breakpoint(99));
    }

    #[test]
    fn test_dap_hit_no_breakpoint() {
        let mut session = DebugSession::new();
        assert!(session.hit_breakpoint("main.sl", 99).is_none());
    }

    #[test]
    fn test_dap_disabled_bp_no_stop() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        session.toggle_breakpoint(id); // disable
        assert!(!session.should_stop("main.sl", 10));
    }

    #[test]
    fn test_dap_multiple_breakpoints() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 5);
        session.add_breakpoint("main.sl", 10);
        session.add_breakpoint("utils.sl", 3);
        assert_eq!(session.breakpoints.len(), 3);
    }

    #[test]
    fn test_dap_locals_search_stack() {
        let mut session = DebugSession::new();
        session.push_frame("outer", "main.sl", 1);
        session.set_local("x", DebugValue::Int(10));
        session.push_frame("inner", "main.sl", 5);
        session.set_local("y", DebugValue::Int(20));
        // y found in top frame
        assert!(matches!(session.get_local("y"), Some(DebugValue::Int(20))));
        // x found by searching down
        assert!(matches!(session.get_local("x"), Some(DebugValue::Int(10))));
    }

    #[test]
    fn test_dap_float_display() {
        let val = DebugValue::Float(3.14);
        let s = format!("{}", val);
        assert!(s.starts_with("3.14"));
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation {
            file: "test.sl".into(),
            line: 42,
            column: 5,
        };
        assert_eq!(loc.file, "test.sl");
        assert_eq!(loc.line, 42);
    }

    // ── v104: DAP wire protocol tests ──────────────────────────────────

    #[test]
    fn test_dap_read_message() {
        let input = b"Content-Length: 13\r\n\r\n{\"test\":true}";
        let mut reader = std::io::BufReader::new(&input[..]);
        let msg = dap_read_message(&mut reader).unwrap();
        assert_eq!(msg, Some("{\"test\":true}".to_string()));
    }

    #[test]
    fn test_dap_read_message_eof() {
        let input = b"";
        let mut reader = std::io::BufReader::new(&input[..]);
        let msg = dap_read_message(&mut reader).unwrap();
        assert_eq!(msg, None);
    }

    #[test]
    fn test_dap_write_message() {
        let mut buf = Vec::new();
        dap_write_message(&mut buf, "{\"ok\":1}").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("Content-Length: 8\r\n\r\n"));
        assert!(output.ends_with("{\"ok\":1}"));
    }

    #[test]
    fn test_parse_dap_request() {
        let json = r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#;
        let req = parse_dap_request(json).unwrap();
        assert_eq!(req.seq, 1);
        assert_eq!(req.command, "initialize");
    }

    #[test]
    fn test_dap_response_format() {
        let resp = dap_response(1, "initialize", "{}");
        assert!(resp.contains("\"request_seq\":1"));
        assert!(resp.contains("\"command\":\"initialize\""));
        assert!(resp.contains("\"success\":true"));
    }

    #[test]
    fn test_dap_event_format() {
        let evt = dap_event("stopped", "{\"reason\":\"breakpoint\"}");
        assert!(evt.contains("\"event\":\"stopped\""));
        assert!(evt.contains("\"reason\":\"breakpoint\""));
    }

    #[test]
    fn test_handle_dap_initialize() {
        let mut session = DebugSession::new();
        let req = DapRequest { seq: 1, command: "initialize".to_string(), arguments: "{}".to_string() };
        let resp = handle_dap_request(&mut session, &req);
        assert!(resp.is_some());
        assert!(resp.unwrap().contains("supportsConfigurationDoneRequest"));
    }

    #[test]
    fn test_handle_dap_threads() {
        let mut session = DebugSession::new();
        let req = DapRequest { seq: 2, command: "threads".to_string(), arguments: "{}".to_string() };
        let resp = handle_dap_request(&mut session, &req).unwrap();
        assert!(resp.contains("\"name\":\"main\""));
    }

    #[test]
    fn test_handle_dap_stack_trace() {
        let mut session = DebugSession::new();
        session.push_frame("main", "test.sl", 1);
        session.push_frame("helper", "test.sl", 10);
        let req = DapRequest { seq: 3, command: "stackTrace".to_string(), arguments: "{}".to_string() };
        let resp = handle_dap_request(&mut session, &req).unwrap();
        assert!(resp.contains("\"name\":\"helper\""));
        assert!(resp.contains("\"totalFrames\":2"));
    }

    #[test]
    fn test_handle_dap_continue() {
        let mut session = DebugSession::new();
        let req = DapRequest { seq: 4, command: "continue".to_string(), arguments: "{}".to_string() };
        let resp = handle_dap_request(&mut session, &req).unwrap();
        assert!(resp.contains("allThreadsContinued"));
        assert_eq!(session.state, ExecutionState::Running);
    }

    #[test]
    fn test_handle_dap_step_operations() {
        let mut session = DebugSession::new();

        let req = DapRequest { seq: 5, command: "stepIn".to_string(), arguments: "{}".to_string() };
        handle_dap_request(&mut session, &req);
        assert_eq!(session.state, ExecutionState::SteppingIn);

        let req = DapRequest { seq: 6, command: "next".to_string(), arguments: "{}".to_string() };
        handle_dap_request(&mut session, &req);
        assert_eq!(session.state, ExecutionState::SteppingOver);

        let req = DapRequest { seq: 7, command: "stepOut".to_string(), arguments: "{}".to_string() };
        handle_dap_request(&mut session, &req);
        assert_eq!(session.state, ExecutionState::SteppingOut);
    }

    #[test]
    fn test_handle_dap_disconnect() {
        let mut session = DebugSession::new();
        let req = DapRequest { seq: 8, command: "disconnect".to_string(), arguments: "{}".to_string() };
        let resp = handle_dap_request(&mut session, &req).unwrap();
        assert!(resp.contains("\"success\":true"));
        assert_eq!(session.state, ExecutionState::Stopped);
    }

    #[test]
    fn test_handle_dap_unknown_command() {
        let mut session = DebugSession::new();
        let req = DapRequest { seq: 9, command: "unknown".to_string(), arguments: "{}".to_string() };
        let resp = handle_dap_request(&mut session, &req).unwrap();
        assert!(resp.contains("\"success\":false"));
        assert!(resp.contains("unsupported command"));
    }

    #[test]
    fn test_dap_json_helpers() {
        let json = r#"{"seq":42,"command":"launch","arguments":{"program":"test.sl"}}"#;
        assert_eq!(dap_extract_number(json, "seq"), Some(42));
        assert_eq!(dap_extract_string(json, "command"), Some("launch".to_string()));
        assert!(dap_extract_object(json, "arguments").is_some());
    }

    #[test]
    fn test_dap_full_lifecycle_via_messages() {
        let init = r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#;
        let config_done = r#"{"seq":2,"type":"request","command":"configurationDone","arguments":{}}"#;
        let launch = r#"{"seq":3,"type":"request","command":"launch","arguments":{}}"#;

        let mut input = Vec::new();
        for msg in &[init, config_done, launch] {
            let header = format!("Content-Length: {}\r\n\r\n", msg.len());
            input.extend_from_slice(header.as_bytes());
            input.extend_from_slice(msg.as_bytes());
        }

        let mut reader = std::io::BufReader::new(&input[..]);
        let mut writer = Vec::new();
        let mut session = DebugSession::new();

        for _ in 0..3 {
            if let Some(text) = dap_read_message(&mut reader).unwrap() {
                if let Some(req) = parse_dap_request(&text) {
                    if let Some(resp) = handle_dap_request(&mut session, &req) {
                        dap_write_message(&mut writer, &resp).unwrap();
                    }
                }
            }
        }

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("supportsConfigurationDoneRequest"));
        assert_eq!(session.state, ExecutionState::Running);
    }
}
