//! Vitalis LSP Server — Language Server Protocol implementation.
//!
//! Provides IDE support features:
//! - **Diagnostics**: Real-time error/warning reporting
//! - **Hover**: Type information on hover
//! - **Go to Definition**: Jump to function/struct/variable definition
//! - **Completion**: Auto-complete for keywords, functions, types
//! - **Document Symbols**: Outline of all top-level items
//! - **Signature Help**: Parameter hints in function calls
//! - **JSON-RPC Wire Protocol**: Content-Length framed messages over stdin/stdout
//!
//! The LSP server reuses the Vitalis compiler pipeline (lex → parse → type-check)
//! to provide accurate, real-time feedback. It communicates via JSON-RPC over stdio.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead, Write};

// ─── LSP Position & Range ───────────────────────────────────────────────

/// A position in a text document (0-indexed line and character).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.character + 1)
    }
}

/// A range in a text document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn from_line(line: u32, start_char: u32, end_char: u32) -> Self {
        Self {
            start: Position::new(line, start_char),
            end: Position::new(line, end_char),
        }
    }

    pub fn contains(&self, pos: Position) -> bool {
        if pos.line < self.start.line || pos.line > self.end.line {
            return false;
        }
        if pos.line == self.start.line && pos.character < self.start.character {
            return false;
        }
        if pos.line == self.end.line && pos.character > self.end.character {
            return false;
        }
        true
    }
}

// ─── Diagnostics ────────────────────────────────────────────────────────

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// A diagnostic message (error, warning, etc.).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: String,
    pub code: Option<String>,
}

impl Diagnostic {
    pub fn error(range: Range, message: &str) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Error,
            message: message.to_string(),
            source: "vitalis".to_string(),
            code: None,
        }
    }

    pub fn warning(range: Range, message: &str) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Warning,
            message: message.to_string(),
            source: "vitalis".to_string(),
            code: None,
        }
    }

    pub fn hint(range: Range, message: &str) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Hint,
            message: message.to_string(),
            source: "vitalis".to_string(),
            code: None,
        }
    }
}

// ─── Completion ─────────────────────────────────────────────────────────

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword = 14,
    Function = 3,
    Variable = 6,
    Struct = 22,
    Enum = 13,
    Module = 9,
    Trait = 25,
    Type = 1,
    Snippet = 15,
}

/// A completion item suggested to the user.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub documentation: Option<String>,
}

impl CompletionItem {
    pub fn keyword(kw: &str) -> Self {
        Self {
            label: kw.to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("keyword".to_string()),
            insert_text: None,
            documentation: None,
        }
    }

    pub fn function(name: &str, sig: &str) -> Self {
        Self {
            label: name.to_string(),
            kind: CompletionKind::Function,
            detail: Some(sig.to_string()),
            insert_text: Some(format!("{}($0)", name)),
            documentation: None,
        }
    }

    pub fn type_item(name: &str) -> Self {
        Self {
            label: name.to_string(),
            kind: CompletionKind::Type,
            detail: Some("type".to_string()),
            insert_text: None,
            documentation: None,
        }
    }
}

/// Get all Vitalis keyword completions.
pub fn keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        "fn", "let", "mut", "if", "else", "match", "for", "in",
        "while", "loop", "break", "continue", "return", "struct",
        "enum", "impl", "trait", "type", "import", "extern", "pub",
        "self", "as", "try", "catch", "throw", "async", "await",
        "spawn", "module", "evolve", "pipeline", "parallel",
    ];
    keywords.iter().map(|kw| CompletionItem::keyword(kw)).collect()
}

/// Get type name completions.
pub fn type_completions() -> Vec<CompletionItem> {
    let types = ["i32", "i64", "f32", "f64", "bool", "str", "void"];
    types.iter().map(|t| CompletionItem::type_item(t)).collect()
}

// ─── Hover ──────────────────────────────────────────────────────────────

/// Hover information for a symbol.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range>,
}

impl HoverInfo {
    pub fn new(contents: &str) -> Self {
        Self {
            contents: contents.to_string(),
            range: None,
        }
    }

    pub fn with_range(mut self, range: Range) -> Self {
        self.range = Some(range);
        self
    }
}

// ─── Document Symbols ───────────────────────────────────────────────────

/// The kind of a document symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function = 12,
    Struct = 23,
    Enum = 10,
    Module = 2,
    Variable = 13,
    Constant = 14,
    Trait = 11,
    TypeAlias = 26,
}

/// A symbol in the document (for outline / breadcrumbs).
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub detail: Option<String>,
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    pub fn new(name: &str, kind: SymbolKind, range: Range) -> Self {
        Self {
            name: name.to_string(),
            kind,
            range,
            detail: None,
            children: Vec::new(),
        }
    }

    pub fn with_child(mut self, child: DocumentSymbol) -> Self {
        self.children.push(child);
        self
    }
}

// ─── Go to Definition ───────────────────────────────────────────────────

/// A location in a document (for go-to-definition, references).
#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn new(uri: &str, range: Range) -> Self {
        Self { uri: uri.to_string(), range }
    }
}

// ─── Signature Help ─────────────────────────────────────────────────────

/// Parameter information for signature help.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub label: String,
    pub documentation: Option<String>,
}

/// Signature help — shows function parameter hints.
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub active_parameter: u32,
}

impl SignatureHelp {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            documentation: None,
            parameters: Vec::new(),
            active_parameter: 0,
        }
    }

    pub fn add_param(&mut self, label: &str, doc: Option<&str>) {
        self.parameters.push(ParameterInfo {
            label: label.to_string(),
            documentation: doc.map(|d| d.to_string()),
        });
    }
}

// ─── Symbol Index ───────────────────────────────────────────────────────

/// An indexed collection of symbols for fast lookup.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    functions: HashMap<String, Location>,
    structs: HashMap<String, Location>,
    enums: HashMap<String, Location>,
    traits: HashMap<String, Location>,
    variables: HashMap<String, Vec<Location>>,
    modules: HashMap<String, Location>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_function(&mut self, name: &str, location: Location) {
        self.functions.insert(name.to_string(), location);
    }

    pub fn add_struct(&mut self, name: &str, location: Location) {
        self.structs.insert(name.to_string(), location);
    }

    pub fn add_enum(&mut self, name: &str, location: Location) {
        self.enums.insert(name.to_string(), location);
    }

    pub fn add_trait(&mut self, name: &str, location: Location) {
        self.traits.insert(name.to_string(), location);
    }

    pub fn add_module(&mut self, name: &str, location: Location) {
        self.modules.insert(name.to_string(), location);
    }

    pub fn add_variable(&mut self, name: &str, location: Location) {
        self.variables.entry(name.to_string())
            .or_default()
            .push(location);
    }

    /// Find the definition location of a symbol by name.
    pub fn find_definition(&self, name: &str) -> Option<&Location> {
        self.functions.get(name)
            .or_else(|| self.structs.get(name))
            .or_else(|| self.enums.get(name))
            .or_else(|| self.traits.get(name))
            .or_else(|| self.modules.get(name))
    }

    /// Get all symbol names.
    pub fn all_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        names.extend(self.functions.keys().cloned());
        names.extend(self.structs.keys().cloned());
        names.extend(self.enums.keys().cloned());
        names.extend(self.traits.keys().cloned());
        names.extend(self.modules.keys().cloned());
        names.sort();
        names.dedup();
        names
    }

    pub fn function_count(&self) -> usize { self.functions.len() }
    pub fn struct_count(&self) -> usize { self.structs.len() }
    pub fn total_count(&self) -> usize {
        self.functions.len() + self.structs.len() + self.enums.len()
            + self.traits.len() + self.modules.len()
    }
}

// ─── LSP Server State ───────────────────────────────────────────────────

/// Server capabilities advertised during initialization.
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub hover: bool,
    pub completion: bool,
    pub go_to_definition: bool,
    pub document_symbols: bool,
    pub diagnostics: bool,
    pub signature_help: bool,
    pub references: bool,
    pub rename: bool,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            hover: true,
            completion: true,
            go_to_definition: true,
            document_symbols: true,
            diagnostics: true,
            signature_help: true,
            references: true,
            rename: true,
        }
    }
}

/// The LSP server state.
#[derive(Debug)]
pub struct LspServer {
    pub capabilities: ServerCapabilities,
    pub documents: HashMap<String, String>,
    pub index: SymbolIndex,
    pub initialized: bool,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            capabilities: ServerCapabilities::default(),
            documents: HashMap::new(),
            index: SymbolIndex::new(),
            initialized: false,
        }
    }

    /// Initialize the server.
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Open a document.
    pub fn open_document(&mut self, uri: &str, text: &str) {
        self.documents.insert(uri.to_string(), text.to_string());
        self.reindex_document(uri, text);
    }

    /// Update a document.
    pub fn update_document(&mut self, uri: &str, text: &str) {
        self.documents.insert(uri.to_string(), text.to_string());
        self.reindex_document(uri, text);
    }

    /// Close a document.
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get diagnostics for a document by running the compiler pipeline.
    pub fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return vec![],
        };

        let mut diagnostics = Vec::new();

        // Run lexer + parser
        let (_program, errors) = crate::parser::parse(source);
        for err in &errors {
            diagnostics.push(Diagnostic::error(
                Range::from_line(0, 0, 1),
                &err.message,
            ));
        }

        diagnostics
    }

    /// Get hover information at a position.
    pub fn hover(&self, uri: &str, pos: Position) -> Option<HoverInfo> {
        let source = self.documents.get(uri)?;
        let (program, _errors) = crate::parser::parse(source);

        // Find the word at the cursor position
        let lines: Vec<&str> = source.lines().collect();
        let line = lines.get(pos.line as usize)?;
        let col = pos.character as usize;
        if col >= line.len() { return None; }

        // Extract the identifier at cursor
        let bytes = line.as_bytes();
        let mut start = col;
        while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            start -= 1;
        }
        let mut end = col;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        if start == end { return None; }
        let word = &line[start..end];

        // Look up in AST top-level items
        for item in &program.items {
            match item {
                crate::ast::TopLevel::Function(f) if f.name == word => {
                    let params: Vec<String> = f.params.iter()
                        .map(|p| format!("{}: {}", p.name, p.ty))
                        .collect();
                    let ret = match &f.return_type {
                        Some(t) => format!("{}", t),
                        None => "void".to_string(),
                    };
                    let sig = format!("fn {}({}) -> {}", f.name, params.join(", "), ret);
                    return Some(HoverInfo::new(&sig).with_range(
                        Range::from_line(pos.line, start as u32, end as u32),
                    ));
                }
                crate::ast::TopLevel::Struct(s) if s.name == word => {
                    let fields: Vec<String> = s.fields.iter()
                        .map(|f| format!("  {}: {}", f.name, f.ty))
                        .collect();
                    let info = format!("struct {} {{\n{}\n}}", s.name, fields.join(",\n"));
                    return Some(HoverInfo::new(&info).with_range(
                        Range::from_line(pos.line, start as u32, end as u32),
                    ));
                }
                crate::ast::TopLevel::Enum(e) if e.name == word => {
                    let variants: Vec<String> = e.variants.iter()
                        .map(|v| format!("  {}", v.name))
                        .collect();
                    let info = format!("enum {} {{\n{}\n}}", e.name, variants.join(",\n"));
                    return Some(HoverInfo::new(&info).with_range(
                        Range::from_line(pos.line, start as u32, end as u32),
                    ));
                }
                _ => {}
            }
        }

        // Check if it's a keyword
        let keywords = ["fn", "let", "mut", "if", "else", "match", "for", "while",
                        "loop", "return", "struct", "enum", "impl", "trait", "async", "await"];
        if keywords.contains(&word) {
            return Some(HoverInfo::new(&format!("(keyword) {}", word)));
        }

        None
    }

    /// Get completions at a position.
    pub fn completions(&self, _uri: &str, _pos: Position) -> Vec<CompletionItem> {
        let mut items = keyword_completions();
        items.extend(type_completions());

        // Add known functions/structs
        for name in self.index.all_names() {
            items.push(CompletionItem {
                label: name.clone(),
                kind: CompletionKind::Function,
                detail: None,
                insert_text: None,
                documentation: None,
            });
        }

        items
    }

    /// Find the definition of a symbol.
    pub fn goto_definition(&self, _uri: &str, name: &str) -> Option<Location> {
        self.index.find_definition(name).cloned()
    }

    /// Get document symbols (outline).
    pub fn document_symbols(&self, uri: &str) -> Vec<DocumentSymbol> {
        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return vec![],
        };

        let mut symbols = Vec::new();
        let (program, _errors) = crate::parser::parse(source);

        for item in &program.items {
            match item {
                crate::ast::TopLevel::Function(f) => {
                    symbols.push(DocumentSymbol::new(
                        &f.name,
                        SymbolKind::Function,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Struct(s) => {
                    symbols.push(DocumentSymbol::new(
                        &s.name,
                        SymbolKind::Struct,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Enum(e) => {
                    symbols.push(DocumentSymbol::new(
                        &e.name,
                        SymbolKind::Enum,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Trait(t) => {
                    symbols.push(DocumentSymbol::new(
                        &t.name,
                        SymbolKind::Trait,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Module(m) => {
                    symbols.push(DocumentSymbol::new(
                        &m.name,
                        SymbolKind::Module,
                        Range::default(),
                    ));
                }
                _ => {}
            }
        }

        symbols
    }

    /// Find all references to a symbol across all open documents.
    pub fn find_references(&self, _uri: &str, name: &str) -> Vec<Location> {
        let mut refs = Vec::new();

        for (doc_uri, source) in &self.documents {
            let lines: Vec<&str> = source.lines().collect();
            for (line_idx, line) in lines.iter().enumerate() {
                let mut search_start = 0;
                while let Some(col) = line[search_start..].find(name) {
                    let abs_col = search_start + col;
                    // Verify it's a whole word match (not a substring)
                    let before_ok = abs_col == 0
                        || !line.as_bytes()[abs_col - 1].is_ascii_alphanumeric()
                            && line.as_bytes()[abs_col - 1] != b'_';
                    let after_pos = abs_col + name.len();
                    let after_ok = after_pos >= line.len()
                        || !line.as_bytes()[after_pos].is_ascii_alphanumeric()
                            && line.as_bytes()[after_pos] != b'_';

                    if before_ok && after_ok {
                        refs.push(Location::new(
                            doc_uri,
                            Range::from_line(
                                line_idx as u32,
                                abs_col as u32,
                                (abs_col + name.len()) as u32,
                            ),
                        ));
                    }
                    search_start = abs_col + name.len().max(1);
                }
            }
        }

        refs
    }

    /// Rename a symbol across all open documents.
    /// Returns a map of uri → list of (range, new_text) edits.
    pub fn rename_symbol(
        &self,
        _uri: &str,
        name: &str,
        new_name: &str,
    ) -> Vec<(String, Vec<(Range, String)>)> {
        if name == new_name || name.is_empty() || new_name.is_empty() {
            return Vec::new();
        }

        // Check for conflicts: new_name must not already be a top-level symbol
        if self.index.find_definition(new_name).is_some() {
            return Vec::new(); // Conflict — symbol already exists
        }

        let mut edits_by_uri: HashMap<String, Vec<(Range, String)>> = HashMap::new();

        for (doc_uri, source) in &self.documents {
            let lines: Vec<&str> = source.lines().collect();
            for (line_idx, line) in lines.iter().enumerate() {
                let mut search_start = 0;
                while let Some(col) = line[search_start..].find(name) {
                    let abs_col = search_start + col;
                    let before_ok = abs_col == 0
                        || !line.as_bytes()[abs_col - 1].is_ascii_alphanumeric()
                            && line.as_bytes()[abs_col - 1] != b'_';
                    let after_pos = abs_col + name.len();
                    let after_ok = after_pos >= line.len()
                        || !line.as_bytes()[after_pos].is_ascii_alphanumeric()
                            && line.as_bytes()[after_pos] != b'_';

                    if before_ok && after_ok {
                        edits_by_uri.entry(doc_uri.clone())
                            .or_default()
                            .push((
                                Range::from_line(
                                    line_idx as u32,
                                    abs_col as u32,
                                    (abs_col + name.len()) as u32,
                                ),
                                new_name.to_string(),
                            ));
                    }
                    search_start = abs_col + name.len().max(1);
                }
            }
        }

        edits_by_uri.into_iter().collect()
    }

    /// Re-index a document after changes.
    fn reindex_document(&mut self, uri: &str, source: &str) {
        let (program, _errors) = crate::parser::parse(source);

        for item in &program.items {
            match item {
                crate::ast::TopLevel::Function(f) => {
                    self.index.add_function(
                        &f.name,
                        Location::new(uri, Range::default()),
                    );
                }
                crate::ast::TopLevel::Struct(s) => {
                    self.index.add_struct(
                        &s.name,
                        Location::new(uri, Range::default()),
                    );
                }
                crate::ast::TopLevel::Enum(e) => {
                    self.index.add_enum(
                        &e.name,
                        Location::new(uri, Range::default()),
                    );
                }
                crate::ast::TopLevel::Trait(t) => {
                    self.index.add_trait(
                        &t.name,
                        Location::new(uri, Range::default()),
                    );
                }
                _ => {}
            }
        }
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  JSON-RPC Wire Protocol (LSP Transport Layer)
// ═══════════════════════════════════════════════════════════════════════

/// A parsed JSON-RPC message from the LSP client.
#[derive(Debug, Clone)]
pub struct RpcMessage {
    pub id: Option<i64>,
    pub method: String,
    pub params: String,
}

/// Read a single LSP message from a buffered reader.
/// Format: `Content-Length: <len>\r\n\r\n<json body>`
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    let mut header_line = String::new();

    // Read headers until empty line
    loop {
        header_line.clear();
        let n = reader.read_line(&mut header_line)?;
        if n == 0 { return Ok(None); } // EOF

        let trimmed = header_line.trim();
        if trimmed.is_empty() { break; } // End of headers

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

/// Write an LSP message to a writer with Content-Length framing.
pub fn write_message<W: Write>(writer: &mut W, body: &str) -> io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(body.as_bytes())?;
    writer.flush()
}

/// Parse a JSON-RPC method name from a raw JSON body (minimal parser — no serde dependency).
pub fn parse_rpc_message(json: &str) -> Option<RpcMessage> {
    let method = extract_json_string(json, "method")?;
    let id = extract_json_number(json, "id");
    let params = extract_json_object(json, "params").unwrap_or_default();
    Some(RpcMessage { id, method, params })
}

/// Format a JSON-RPC response.
pub fn rpc_response(id: i64, result: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}", id, result)
}

/// Format a JSON-RPC notification (no id).
pub fn rpc_notification(method: &str, params: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{}\",\"params\":{}}}", method, params)
}

/// Format a JSON-RPC error response.
pub fn rpc_error(id: i64, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{},\"message\":\"{}\"}}}}",
        id, code, message.replace('"', "\\\"")
    )
}

/// Handle a single LSP request and produce a response.
pub fn handle_message(server: &mut LspServer, msg: &RpcMessage) -> Option<String> {
    match msg.method.as_str() {
        "initialize" => {
            server.initialize();
            let caps = r#"{"capabilities":{"textDocumentSync":1,"hoverProvider":true,"completionProvider":{"triggerCharacters":[".",":"]},"definitionProvider":true,"documentSymbolProvider":true}}"#;
            msg.id.map(|id| rpc_response(id, caps))
        }
        "initialized" => None, // Notification, no response
        "shutdown" => msg.id.map(|id| rpc_response(id, "null")),
        "exit" => None,
        "textDocument/didOpen" => {
            if let (Some(uri), Some(text)) = (
                extract_json_string(&msg.params, "uri"),
                extract_json_string(&msg.params, "text"),
            ) {
                server.open_document(&uri, &text);
            }
            None
        }
        "textDocument/didChange" => {
            if let Some(uri) = extract_nested_string(&msg.params, "textDocument", "uri") {
                // Full document sync (syncKind=1)
                if let Some(text) = extract_content_change_text(&msg.params) {
                    server.update_document(&uri, &text);
                }
            }
            None
        }
        "textDocument/didClose" => {
            if let Some(uri) = extract_nested_string(&msg.params, "textDocument", "uri") {
                server.close_document(&uri);
            }
            None
        }
        "textDocument/hover" => {
            let uri = extract_nested_string(&msg.params, "textDocument", "uri")
                .unwrap_or_default();
            let pos = extract_position(&msg.params).unwrap_or_default();
            let hover = server.hover(&uri, pos);
            msg.id.map(|id| match hover {
                Some(h) => rpc_response(id, &format!(
                    "{{\"contents\":{{\"kind\":\"plaintext\",\"value\":\"{}\"}}}}",
                    h.contents.replace('"', "\\\"").replace('\n', "\\n")
                )),
                None => rpc_response(id, "null"),
            })
        }
        "textDocument/completion" => {
            let uri = extract_nested_string(&msg.params, "textDocument", "uri")
                .unwrap_or_default();
            let pos = extract_position(&msg.params).unwrap_or_default();
            let items = server.completions(&uri, pos);
            let items_json: Vec<String> = items.iter().map(|item| {
                format!("{{\"label\":\"{}\",\"kind\":{}}}", item.label, item.kind as i32)
            }).collect();
            msg.id.map(|id| rpc_response(id, &format!("[{}]", items_json.join(","))))
        }
        "textDocument/definition" => {
            // Extract word at position from document
            let uri = extract_nested_string(&msg.params, "textDocument", "uri")
                .unwrap_or_default();
            let pos = extract_position(&msg.params).unwrap_or_default();
            let word = server.documents.get(&uri).and_then(|src| {
                let lines: Vec<&str> = src.lines().collect();
                let line = lines.get(pos.line as usize)?;
                let bytes = line.as_bytes();
                let col = pos.character as usize;
                if col >= bytes.len() { return None; }
                let mut s = col;
                while s > 0 && (bytes[s-1].is_ascii_alphanumeric() || bytes[s-1] == b'_') { s -= 1; }
                let mut e = col;
                while e < bytes.len() && (bytes[e].is_ascii_alphanumeric() || bytes[e] == b'_') { e += 1; }
                if s < e { Some(line[s..e].to_string()) } else { None }
            });
            msg.id.map(|id| match word.and_then(|w| server.goto_definition(&uri, &w)) {
                Some(loc) => rpc_response(id, &format!(
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\"character\":{}}}}}}}",
                    loc.uri, loc.range.start.line, loc.range.start.character,
                    loc.range.end.line, loc.range.end.character
                )),
                None => rpc_response(id, "null"),
            })
        }
        _ => {
            // Unknown method — return error for requests, ignore notifications
            msg.id.map(|id| rpc_error(id, -32601, &format!("method not found: {}", msg.method)))
        }
    }
}

// ─── Minimal JSON Helpers (no serde dependency) ─────────────────────────

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let after_key = &json[idx + pattern.len()..];
    // Skip `:` and whitespace
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('"') { return None; }
    let content = &trimmed[1..];
    // Find closing quote (handle escaped quotes)
    let mut end = 0;
    let bytes = content.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end - 1] != b'\\') { break; }
        end += 1;
    }
    if end >= bytes.len() { return None; }
    Some(content[..end].to_string())
}

fn extract_json_number(json: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let after_key = &json[idx + pattern.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let trimmed = after_colon.trim_start();
    let end = trimmed.find(|c: char| !c.is_ascii_digit() && c != '-').unwrap_or(trimmed.len());
    trimmed[..end].parse().ok()
}

fn extract_json_object(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let after_key = &json[idx + pattern.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?;
    let trimmed = after_colon.trim_start();
    if !trimmed.starts_with('{') { return None; }
    let mut depth = 0;
    let mut end = 0;
    for (i, ch) in trimmed.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => { depth -= 1; if depth == 0 { end = i + 1; break; } }
            _ => {}
        }
    }
    if end == 0 { None } else { Some(trimmed[..end].to_string()) }
}

fn extract_nested_string(json: &str, outer_key: &str, inner_key: &str) -> Option<String> {
    let obj = extract_json_object(json, outer_key)?;
    extract_json_string(&obj, inner_key)
}

fn extract_position(json: &str) -> Option<Position> {
    let pos_obj = extract_json_object(json, "position")?;
    let line = extract_json_number(&pos_obj, "line")? as u32;
    let character = extract_json_number(&pos_obj, "character")? as u32;
    Some(Position::new(line, character))
}

fn extract_content_change_text(json: &str) -> Option<String> {
    // contentChanges is an array — extract the first "text" value
    let idx = json.find("\"contentChanges\"")?;
    let rest = &json[idx..];
    extract_json_string(rest, "text")
}

// ─── Stdio Transport ────────────────────────────────────────────────────

/// Run the LSP server on stdin/stdout. This is the main entry point for `vtc lsp`.
///
/// Loops reading JSON-RPC messages from stdin, dispatching to `handle_message`,
/// and writing responses back to stdout. Exits on `exit` notification or EOF.
pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut server = LspServer::new();

    loop {
        let msg_text = match read_message(&mut reader)? {
            Some(text) => text,
            None => break, // EOF
        };

        let rpc = match parse_rpc_message(&msg_text) {
            Some(rpc) => rpc,
            None => continue,
        };

        let is_exit = rpc.method == "exit";

        if let Some(response) = handle_message(&mut server, &rpc) {
            write_message(&mut writer, &response)?;
        }

        // Publish diagnostics after document changes
        if rpc.method == "textDocument/didOpen" || rpc.method == "textDocument/didChange" {
            if let Some(uri) = extract_nested_string(&msg_text, "textDocument", "uri")
                .or_else(|| extract_json_string(&msg_text, "uri"))
            {
                let diags = server.get_diagnostics(&uri);
                let diag_json: Vec<String> = diags.iter().map(|d| {
                    format!(
                        "{{\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\"character\":{}}}}},\"severity\":{},\"source\":\"vitalis\",\"message\":\"{}\"}}",
                        d.range.start.line, d.range.start.character,
                        d.range.end.line, d.range.end.character,
                        d.severity as i32,
                        d.message.replace('"', "\\\"")
                    )
                }).collect();
                let notification = rpc_notification(
                    "textDocument/publishDiagnostics",
                    &format!("{{\"uri\":\"{}\",\"diagnostics\":[{}]}}", uri, diag_json.join(",")),
                );
                write_message(&mut writer, &notification)?;
            }
        }

        if is_exit {
            break;
        }
    }

    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position() {
        let p = Position::new(0, 5);
        assert_eq!(format!("{}", p), "1:6");
    }

    #[test]
    fn test_range_contains() {
        let r = Range::from_line(5, 10, 20);
        assert!(r.contains(Position::new(5, 15)));
        assert!(!r.contains(Position::new(5, 25)));
        assert!(!r.contains(Position::new(6, 0)));
    }

    #[test]
    fn test_diagnostic_error() {
        let d = Diagnostic::error(Range::default(), "undefined variable");
        assert_eq!(d.severity, DiagnosticSeverity::Error);
        assert_eq!(d.source, "vitalis");
    }

    #[test]
    fn test_diagnostic_warning() {
        let d = Diagnostic::warning(Range::default(), "unused variable");
        assert_eq!(d.severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_completion_keyword() {
        let c = CompletionItem::keyword("fn");
        assert_eq!(c.kind, CompletionKind::Keyword);
        assert_eq!(c.label, "fn");
    }

    #[test]
    fn test_completion_function() {
        let c = CompletionItem::function("print", "fn print(s: str)");
        assert_eq!(c.kind, CompletionKind::Function);
        assert_eq!(c.insert_text, Some("print($0)".to_string()));
    }

    #[test]
    fn test_keyword_completions() {
        let completions = keyword_completions();
        assert!(completions.len() > 20);
        assert!(completions.iter().any(|c| c.label == "fn"));
        assert!(completions.iter().any(|c| c.label == "async"));
        assert!(completions.iter().any(|c| c.label == "await"));
    }

    #[test]
    fn test_type_completions() {
        let completions = type_completions();
        assert!(completions.iter().any(|c| c.label == "i64"));
        assert!(completions.iter().any(|c| c.label == "str"));
    }

    #[test]
    fn test_hover_info() {
        let h = HoverInfo::new("fn main() -> i64")
            .with_range(Range::from_line(0, 0, 10));
        assert!(h.range.is_some());
        assert!(h.contents.contains("main"));
    }

    #[test]
    fn test_document_symbol() {
        let sym = DocumentSymbol::new("main", SymbolKind::Function, Range::default())
            .with_child(DocumentSymbol::new("x", SymbolKind::Variable, Range::default()));
        assert_eq!(sym.children.len(), 1);
    }

    #[test]
    fn test_symbol_index() {
        let mut idx = SymbolIndex::new();
        idx.add_function("main", Location::new("file.sl", Range::default()));
        idx.add_struct("Point", Location::new("file.sl", Range::default()));

        assert!(idx.find_definition("main").is_some());
        assert!(idx.find_definition("Point").is_some());
        assert!(idx.find_definition("unknown").is_none());
        assert_eq!(idx.function_count(), 1);
        assert_eq!(idx.struct_count(), 1);
        assert_eq!(idx.total_count(), 2);
    }

    #[test]
    fn test_symbol_index_all_names() {
        let mut idx = SymbolIndex::new();
        idx.add_function("foo", Location::new("f.sl", Range::default()));
        idx.add_struct("Bar", Location::new("f.sl", Range::default()));
        let names = idx.all_names();
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"Bar".to_string()));
    }

    #[test]
    fn test_signature_help() {
        let mut sh = SignatureHelp::new("fn add(a: i64, b: i64) -> i64");
        sh.add_param("a: i64", Some("First number"));
        sh.add_param("b: i64", Some("Second number"));
        assert_eq!(sh.parameters.len(), 2);
    }

    #[test]
    fn test_lsp_server_init() {
        let mut server = LspServer::new();
        assert!(!server.initialized);
        server.initialize();
        assert!(server.initialized);
    }

    #[test]
    fn test_lsp_open_document() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        assert!(server.documents.contains_key("test.sl"));
    }

    #[test]
    fn test_lsp_diagnostics() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        let diags = server.get_diagnostics("test.sl");
        assert!(diags.is_empty()); // Valid code → no errors
    }

    #[test]
    fn test_lsp_diagnostics_error() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("bad.sl", "fn {{{ broken");
        let diags = server.get_diagnostics("bad.sl");
        assert!(!diags.is_empty()); // Invalid code → errors
    }

    #[test]
    fn test_lsp_completions() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        let completions = server.completions("test.sl", Position::new(0, 0));
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_lsp_document_symbols() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }\nfn add(a: i64, b: i64) -> i64 { a + b }");
        let symbols = server.document_symbols("test.sl");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "main");
        assert_eq!(symbols[1].name, "add");
    }

    #[test]
    fn test_lsp_goto_definition() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        let loc = server.goto_definition("test.sl", "main");
        assert!(loc.is_some());
    }

    #[test]
    fn test_lsp_close_document() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        server.close_document("test.sl");
        assert!(!server.documents.contains_key("test.sl"));
    }

    #[test]
    fn test_server_capabilities() {
        let caps = ServerCapabilities::default();
        assert!(caps.hover);
        assert!(caps.completion);
        assert!(caps.go_to_definition);
        assert!(caps.diagnostics);
        assert!(caps.rename); // v370: Now implemented
    }

    #[test]
    fn test_location() {
        let loc = Location::new("file.sl", Range::from_line(5, 0, 10));
        assert_eq!(loc.uri, "file.sl");
    }

    #[test]
    fn test_lsp_reindex_struct() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "struct Point { x: i64 }\nfn main() -> i64 { 0 }");
        assert!(server.index.find_definition("Point").is_some());
    }

    // ── v370: References & Rename Tests ────────────────────────────────

    #[test]
    fn test_find_references_single_doc() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn add(a: i64, b: i64) -> i64 { a + b }\nfn main() -> i64 { add(1, 2) }");
        let refs = server.find_references("test.sl", "add");
        assert!(refs.len() >= 2); // Definition + call site
    }

    #[test]
    fn test_find_references_multi_doc() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("a.sl", "fn helper() -> i64 { 1 }");
        server.open_document("b.sl", "fn main() -> i64 { helper() }");
        let refs = server.find_references("a.sl", "helper");
        assert!(refs.len() >= 2);
    }

    #[test]
    fn test_find_references_no_partial_match() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn foo_bar() -> i64 { 0 }\nfn foo() -> i64 { 1 }");
        let refs = server.find_references("test.sl", "foo");
        // Should find "foo" but not "foo" inside "foo_bar"
        assert!(refs.iter().all(|r| {
            let line = "fn foo_bar() -> i64 { 0 }\nfn foo() -> i64 { 1 }".lines()
                .nth(r.range.start.line as usize).unwrap_or("");
            let start = r.range.start.character as usize;
            let end = r.range.end.character as usize;
            &line[start..end] == "foo"
        }));
    }

    #[test]
    fn test_rename_symbol() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn add(a: i64, b: i64) -> i64 { a + b }\nfn main() -> i64 { add(1, 2) }");
        let edits = server.rename_symbol("test.sl", "add", "sum");
        assert!(!edits.is_empty());
        let total_edits: usize = edits.iter().map(|(_, e)| e.len()).sum();
        assert!(total_edits >= 2); // Definition + call site
    }

    #[test]
    fn test_rename_conflict_blocked() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn add() -> i64 { 0 }\nfn sub() -> i64 { 0 }");
        // Try to rename add to sub — should be blocked (conflict)
        let edits = server.rename_symbol("test.sl", "add", "sub");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_rename_empty_name_blocked() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn foo() -> i64 { 0 }");
        let edits = server.rename_symbol("test.sl", "foo", "");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_capabilities_now_enabled() {
        let caps = ServerCapabilities::default();
        assert!(caps.references);
        assert!(caps.rename);
    }

    // ── v64: JSON-RPC Wire Protocol Tests ──────────────────────────────

    #[test]
    fn test_read_message() {
        let input = b"Content-Length: 13\r\n\r\n{\"test\":true}";
        let mut reader = std::io::BufReader::new(&input[..]);
        let msg = read_message(&mut reader).unwrap();
        assert_eq!(msg, Some("{\"test\":true}".to_string()));
    }

    #[test]
    fn test_read_message_eof() {
        let input = b"";
        let mut reader = std::io::BufReader::new(&input[..]);
        let msg = read_message(&mut reader).unwrap();
        assert_eq!(msg, None);
    }

    #[test]
    fn test_write_message() {
        let mut buf = Vec::new();
        write_message(&mut buf, "{\"ok\":1}").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("Content-Length: 8\r\n\r\n"));
        assert!(output.ends_with("{\"ok\":1}"));
    }

    #[test]
    fn test_parse_rpc_message() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let msg = parse_rpc_message(json).unwrap();
        assert_eq!(msg.method, "initialize");
        assert_eq!(msg.id, Some(1));
    }

    #[test]
    fn test_parse_rpc_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
        let msg = parse_rpc_message(json).unwrap();
        assert_eq!(msg.method, "initialized");
        assert_eq!(msg.id, None);
    }

    #[test]
    fn test_rpc_response_format() {
        let resp = rpc_response(42, "null");
        assert!(resp.contains("\"id\":42"));
        assert!(resp.contains("\"result\":null"));
        assert!(resp.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn test_rpc_error_format() {
        let err = rpc_error(1, -32601, "method not found");
        assert!(err.contains("\"error\""));
        assert!(err.contains("-32601"));
        assert!(err.contains("method not found"));
    }

    #[test]
    fn test_handle_initialize() {
        let mut server = LspServer::new();
        let msg = RpcMessage { id: Some(1), method: "initialize".to_string(), params: "{}".to_string() };
        let resp = handle_message(&mut server, &msg);
        assert!(resp.is_some());
        assert!(resp.unwrap().contains("hoverProvider"));
        assert!(server.initialized);
    }

    #[test]
    fn test_handle_shutdown() {
        let mut server = LspServer::new();
        let msg = RpcMessage { id: Some(99), method: "shutdown".to_string(), params: "{}".to_string() };
        let resp = handle_message(&mut server, &msg);
        assert!(resp.is_some());
        assert!(resp.unwrap().contains("\"result\":null"));
    }

    #[test]
    fn test_handle_unknown_method() {
        let mut server = LspServer::new();
        let msg = RpcMessage { id: Some(5), method: "unknown/foo".to_string(), params: "{}".to_string() };
        let resp = handle_message(&mut server, &msg);
        assert!(resp.unwrap().contains("method not found"));
    }

    #[test]
    fn test_hover_on_function() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "fn add(a: i64, b: i64) -> i64 { a }");
        let hover = server.hover("test.sl", Position::new(0, 4));
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("fn add"));
        assert!(info.contents.contains("a: i64"));
    }

    #[test]
    fn test_hover_on_struct() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "struct Point { x: i64, y: i64 }\nfn main() -> i64 { 0 }");
        let hover = server.hover("test.sl", Position::new(0, 9));
        assert!(hover.is_some());
        assert!(hover.unwrap().contents.contains("struct Point"));
    }

    #[test]
    fn test_hover_on_keyword() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "fn main() -> i64 { return 1 }");
        let hover = server.hover("test.sl", Position::new(0, 21));
        assert!(hover.is_some());
        assert!(hover.unwrap().contents.contains("keyword"));
    }

    #[test]
    fn test_extract_json_string() {
        let json = r#"{"name":"hello","value":42}"#;
        assert_eq!(extract_json_string(json, "name"), Some("hello".to_string()));
    }

    #[test]
    fn test_extract_json_number() {
        let json = r#"{"id":42,"method":"test"}"#;
        assert_eq!(extract_json_number(json, "id"), Some(42));
    }

    #[test]
    fn test_extract_position() {
        let json = r#"{"textDocument":{"uri":"f.sl"},"position":{"line":5,"character":10}}"#;
        let pos = extract_position(json).unwrap();
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
    }

    // ── v101: LSP stdio transport tests ──────────────────────────────────

    #[test]
    fn test_run_stdio_eof() {
        // Empty stdin → immediate EOF → exits cleanly
        let input: &[u8] = b"";
        let mut reader = std::io::BufReader::new(input);
        let server = LspServer::new();

        // Simulate the loop: reading returns None on empty
        let msg = read_message(&mut reader).unwrap();
        assert_eq!(msg, None);
        drop(server);
    }

    #[test]
    fn test_run_stdio_initialize_shutdown_exit() {
        // Full lifecycle: initialize → shutdown → exit
        let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let shutdown_req = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#;
        let exit_notif = r#"{"jsonrpc":"2.0","method":"exit","params":{}}"#;

        let mut input = Vec::new();
        for msg in &[init_req, shutdown_req, exit_notif] {
            let header = format!("Content-Length: {}\r\n\r\n", msg.len());
            input.extend_from_slice(header.as_bytes());
            input.extend_from_slice(msg.as_bytes());
        }

        let mut reader = std::io::BufReader::new(&input[..]);
        let mut writer = Vec::new();
        let mut server = LspServer::new();

        // Process all three messages
        for _ in 0..3 {
            if let Some(text) = read_message(&mut reader).unwrap() {
                if let Some(rpc) = parse_rpc_message(&text) {
                    if let Some(resp) = handle_message(&mut server, &rpc) {
                        write_message(&mut writer, &resp).unwrap();
                    }
                }
            }
        }

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("hoverProvider")); // initialize response
        assert!(output.contains("\"result\":null")); // shutdown response
        assert!(server.initialized);
    }

    #[test]
    fn test_run_stdio_diagnostics_published() {
        let did_open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"test.sl","text":"fn 123bad name"}}"#;

        let mut input = Vec::new();
        let header = format!("Content-Length: {}\r\n\r\n", did_open.len());
        input.extend_from_slice(header.as_bytes());
        input.extend_from_slice(did_open.as_bytes());

        let mut reader = std::io::BufReader::new(&input[..]);
        let mut writer = Vec::new();
        let mut server = LspServer::new();

        if let Some(text) = read_message(&mut reader).unwrap() {
            if let Some(rpc) = parse_rpc_message(&text) {
                let _ = handle_message(&mut server, &rpc);
                // Publish diagnostics
                if let Some(uri) = extract_json_string(&text, "uri") {
                    let diags = server.get_diagnostics(&uri);
                    assert!(!diags.is_empty(), "broken code should produce diagnostics");
                    let notification = rpc_notification(
                        "textDocument/publishDiagnostics",
                        &format!("{{\"uri\":\"{}\",\"diagnostics\":[]}}", uri),
                    );
                    write_message(&mut writer, &notification).unwrap();
                }
            }
        }

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("publishDiagnostics"));
    }

    #[test]
    fn test_rpc_notification_format() {
        let notif = rpc_notification("textDocument/publishDiagnostics", "{\"uri\":\"test.sl\",\"diagnostics\":[]}");
        assert!(notif.contains("\"method\":\"textDocument/publishDiagnostics\""));
        assert!(notif.contains("\"diagnostics\":[]"));
        assert!(!notif.contains("\"id\"")); // notifications have no id
    }

    #[test]
    fn test_run_stdio_handles_notification_no_response() {
        let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
        let mut input = Vec::new();
        let header = format!("Content-Length: {}\r\n\r\n", initialized.len());
        input.extend_from_slice(header.as_bytes());
        input.extend_from_slice(initialized.as_bytes());

        let mut reader = std::io::BufReader::new(&input[..]);
        let mut writer = Vec::new();
        let mut server = LspServer::new();

        if let Some(text) = read_message(&mut reader).unwrap() {
            if let Some(rpc) = parse_rpc_message(&text) {
                let response = handle_message(&mut server, &rpc);
                assert!(response.is_none(), "notifications should not produce responses");
                if let Some(resp) = response {
                    write_message(&mut writer, &resp).unwrap();
                }
            }
        }

        assert!(writer.is_empty(), "no output should be written for notifications");
    }

    #[test]
    fn test_run_stdio_multiple_hover_requests() {
        let open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"test.sl","text":"fn add(a: i64, b: i64) -> i64 { a + b }"}}"#;
        let hover = r#"{"jsonrpc":"2.0","id":5,"method":"textDocument/hover","params":{"textDocument":{"uri":"test.sl"},"position":{"line":0,"character":4}}}"#;

        let mut input = Vec::new();
        for msg in &[open, hover] {
            let header = format!("Content-Length: {}\r\n\r\n", msg.len());
            input.extend_from_slice(header.as_bytes());
            input.extend_from_slice(msg.as_bytes());
        }

        let mut reader = std::io::BufReader::new(&input[..]);
        let mut writer = Vec::new();
        let mut server = LspServer::new();

        for _ in 0..2 {
            if let Some(text) = read_message(&mut reader).unwrap() {
                if let Some(rpc) = parse_rpc_message(&text) {
                    if let Some(resp) = handle_message(&mut server, &rpc) {
                        write_message(&mut writer, &resp).unwrap();
                    }
                }
            }
        }

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("fn add")); // hover contents
    }
}
