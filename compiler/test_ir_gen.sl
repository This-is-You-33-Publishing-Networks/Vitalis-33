// ====== COMBINED: Lexer + Parser + IR Gen + Tests ======

// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 1: Lexer (v3 return-free)
// ============================================================================
// Tokenizes .sl source code character-by-character using stdlib primitives.
// Produces a flat token array with source spans (no string storage):
//   [type, start, end, line, type, start, end, line, ...]
// Each token takes 4 array slots (stride 4):
//   [0] token type (integer constant)
//   [1] start position in source string
//   [2] end position in source string
//   [3] line number
//
// NOTE: Avoids `return` inside if-blocks due to Cranelift codegen issue.
//       All control flow uses if/else expressions instead.
// ============================================================================

// -- Token type constants --
fn TK_INT() -> i64    { 1 }
fn TK_FLOAT() -> i64  { 2 }
fn TK_STRING() -> i64 { 3 }
fn TK_IDENT() -> i64  { 4 }
fn TK_BOOL() -> i64   { 5 }

fn TK_FN() -> i64       { 10 }
fn TK_LET() -> i64      { 11 }
fn TK_IF() -> i64       { 12 }
fn TK_ELSE() -> i64     { 13 }
fn TK_WHILE() -> i64    { 14 }
fn TK_FOR() -> i64      { 15 }
fn TK_RETURN() -> i64   { 16 }
fn TK_STRUCT() -> i64   { 17 }
fn TK_ENUM() -> i64     { 18 }
fn TK_MATCH() -> i64    { 19 }
fn TK_IMPORT() -> i64   { 20 }
fn TK_CONST() -> i64    { 21 }
fn TK_TRAIT() -> i64    { 22 }
fn TK_IMPL() -> i64     { 23 }
fn TK_IN() -> i64       { 24 }
fn TK_MUT() -> i64      { 25 }
fn TK_LOOP() -> i64     { 26 }
fn TK_BREAK() -> i64    { 27 }
fn TK_CONTINUE() -> i64 { 28 }
fn TK_TYPE() -> i64     { 29 }
fn TK_AS() -> i64       { 30 }
fn TK_TRY() -> i64      { 31 }
fn TK_CATCH() -> i64    { 32 }
fn TK_THROW() -> i64    { 33 }
fn TK_ASYNC() -> i64    { 34 }
fn TK_AWAIT() -> i64    { 35 }
fn TK_SPAWN() -> i64    { 36 }
fn TK_EXTERN() -> i64   { 37 }
fn TK_PUB() -> i64      { 38 }
fn TK_SELF() -> i64     { 39 }
fn TK_SUPER() -> i64    { 40 }
fn TK_MOD() -> i64      { 41 }
fn TK_USE() -> i64      { 42 }

fn TK_PLUS() -> i64      { 50 }
fn TK_MINUS() -> i64     { 51 }
fn TK_STAR() -> i64      { 52 }
fn TK_SLASH() -> i64     { 53 }
fn TK_PERCENT() -> i64   { 54 }
fn TK_EQ() -> i64        { 55 }
fn TK_EQEQ() -> i64     { 56 }
fn TK_NEQ() -> i64       { 57 }
fn TK_LT() -> i64        { 58 }
fn TK_GT() -> i64        { 59 }
fn TK_LTE() -> i64       { 60 }
fn TK_GTE() -> i64       { 61 }
fn TK_AND() -> i64       { 62 }
fn TK_OR() -> i64        { 63 }
fn TK_NOT() -> i64       { 64 }
fn TK_ARROW() -> i64     { 65 }
fn TK_FAT_ARROW() -> i64 { 66 }
fn TK_PIPE() -> i64      { 67 }
fn TK_AMP() -> i64       { 68 }
fn TK_DOTDOT() -> i64    { 69 }
fn TK_QUESTION() -> i64  { 70 }
fn TK_PLUSEQ() -> i64    { 71 }
fn TK_MINUSEQ() -> i64   { 72 }
fn TK_STAREQ() -> i64    { 73 }
fn TK_SLASHEQ() -> i64   { 74 }

fn TK_LPAREN() -> i64     { 80 }
fn TK_RPAREN() -> i64     { 81 }
fn TK_LBRACE() -> i64     { 82 }
fn TK_RBRACE() -> i64     { 83 }
fn TK_LBRACKET() -> i64   { 84 }
fn TK_RBRACKET() -> i64   { 85 }
fn TK_COMMA() -> i64      { 86 }
fn TK_COLON() -> i64      { 87 }
fn TK_SEMICOLON() -> i64  { 88 }
fn TK_DOT() -> i64        { 89 }
fn TK_HASH() -> i64       { 90 }
fn TK_AT() -> i64         { 91 }
fn TK_COLONCOLON() -> i64 { 92 }

fn TK_EOF() -> i64   { 99 }
fn TK_ERROR() -> i64 { 100 }

// -- Character classification --
fn is_digit(code: i64) -> bool {
    code >= 48 && code <= 57
}

fn is_alpha(code: i64) -> bool {
    (code >= 65 && code <= 90) || (code >= 97 && code <= 122) || code == 95
}

fn is_alnum(code: i64) -> bool {
    is_digit(code) || is_alpha(code)
}

fn is_whitespace(code: i64) -> bool {
    code == 32 || code == 9 || code == 13 || code == 10
}

fn is_newline(code: i64) -> bool {
    code == 10
}

fn is_hex_digit(code: i64) -> bool {
    is_digit(code) || (code >= 65 && code <= 70) || (code >= 97 && code <= 102)
}

// -- Keyword lookup (flat if/else-if chain, no return) --
fn keyword_type(word: str) -> i64 {
    if str_eq(word, "fn") { 10 }
    else if str_eq(word, "let") { 11 }
    else if str_eq(word, "if") { 12 }
    else if str_eq(word, "else") { 13 }
    else if str_eq(word, "while") { 14 }
    else if str_eq(word, "for") { 15 }
    else if str_eq(word, "return") { 16 }
    else if str_eq(word, "struct") { 17 }
    else if str_eq(word, "enum") { 18 }
    else if str_eq(word, "match") { 19 }
    else if str_eq(word, "import") { 20 }
    else if str_eq(word, "const") { 21 }
    else if str_eq(word, "trait") { 22 }
    else if str_eq(word, "impl") { 23 }
    else if str_eq(word, "in") { 24 }
    else if str_eq(word, "mut") { 25 }
    else if str_eq(word, "loop") { 26 }
    else if str_eq(word, "break") { 27 }
    else if str_eq(word, "continue") { 28 }
    else if str_eq(word, "type") { 29 }
    else if str_eq(word, "as") { 30 }
    else if str_eq(word, "try") { 31 }
    else if str_eq(word, "catch") { 32 }
    else if str_eq(word, "throw") { 33 }
    else if str_eq(word, "async") { 34 }
    else if str_eq(word, "await") { 35 }
    else if str_eq(word, "spawn") { 36 }
    else if str_eq(word, "extern") { 37 }
    else if str_eq(word, "pub") { 38 }
    else if str_eq(word, "self") { 39 }
    else if str_eq(word, "super") { 40 }
    else if str_eq(word, "mod") { 41 }
    else if str_eq(word, "use") { 42 }
    else if str_eq(word, "true") { 5 }
    else if str_eq(word, "false") { 5 }
    else { 0 }
}

// -- Token array accessors (stride 4) --
fn tok_count(tokens: i64) -> i64 {
    array_len(tokens) / 4
}

fn tok_type(tokens: i64, idx: i64) -> i64 {
    array_get(tokens, idx * 4)
}

fn tok_start(tokens: i64, idx: i64) -> i64 {
    array_get(tokens, idx * 4 + 1)
}

fn tok_end(tokens: i64, idx: i64) -> i64 {
    array_get(tokens, idx * 4 + 2)
}

fn tok_line(tokens: i64, idx: i64) -> i64 {
    array_get(tokens, idx * 4 + 3)
}

fn tok_text(source: str, tokens: i64, idx: i64) -> str {
    let s = tok_start(tokens, idx)
    let e = tok_end(tokens, idx)
    if s < 0 { "" }
    else { str_substr(source, s, e - s) }
}

// -- Internal: grow-on-demand token buffer --
// Slot 0 = current length, slot 1 = capacity, data at slot 2+
fn tbuf_new() -> i64 {
    let cap = 1024
    let arr = array_new(cap + 2)
    array_set(arr, 0, 0)
    array_set(arr, 1, cap)
    arr
}

fn tbuf_push(tbuf: i64, val: i64) -> i64 {
    let len = array_get(tbuf, 0)
    let cap = array_get(tbuf, 1)
    if len >= cap {
        let nc = cap * 2
        let nb = array_new(nc + 2)
        array_set(nb, 0, len)
        array_set(nb, 1, nc)
        let mut ci = 0
        while ci < len {
            array_set(nb, ci + 2, array_get(tbuf, ci + 2))
            ci = ci + 1
        }
        array_set(nb, len + 2, val)
        array_set(nb, 0, len + 1)
        nb
    } else {
        array_set(tbuf, len + 2, val)
        array_set(tbuf, 0, len + 1)
        tbuf
    }
}

fn tbuf_emit(tbuf: i64, ttype: i64, start: i64, end: i64, ln: i64) -> i64 {
    let b = tbuf_push(tbuf, ttype)
    let b = tbuf_push(b, start)
    let b = tbuf_push(b, end)
    let b = tbuf_push(b, ln)
    b
}

fn tbuf_finalize(tbuf: i64) -> i64 {
    let len = array_get(tbuf, 0)
    let result = array_new(len)
    let mut fi = 0
    while fi < len {
        array_set(result, fi, array_get(tbuf, fi + 2))
        fi = fi + 1
    }
    result
}

// -- Peek helpers (no return, if/else) --
fn peek_char(src: str, p: i64) -> i64 {
    if p >= str_len(src) { -1 }
    else { char_to_int(str_char_at(src, p)) }
}

fn peek_char2(src: str, p: i64) -> i64 {
    if p + 1 >= str_len(src) { -1 }
    else { char_to_int(str_char_at(src, p + 1)) }
}

// -- Main lexer --
fn lex(source: str) -> i64 {
    let slen = str_len(source)
    let mut pos = 0
    let mut ln = 1
    let mut buf = tbuf_new()

    while pos < slen {
        let ch = peek_char(source, pos)

        // Skip whitespace
        if is_whitespace(ch) {
            if is_newline(ch) { ln = ln + 1 }
            pos = pos + 1
            continue
        }

        // Skip line comments: //
        if ch == 47 && peek_char2(source, pos) == 47 {
            while pos < slen && peek_char(source, pos) != 10 {
                pos = pos + 1
            }
            continue
        }

        // Skip block comments: /* ... */ (with nesting)
        if ch == 47 && peek_char2(source, pos) == 42 {
            pos = pos + 2
            let mut depth = 1
            while pos < slen && depth > 0 {
                let c0 = peek_char(source, pos)
                if c0 == 47 && peek_char2(source, pos) == 42 {
                    depth = depth + 1
                    pos = pos + 2
                } else if c0 == 42 && peek_char2(source, pos) == 47 {
                    depth = depth - 1
                    pos = pos + 2
                } else {
                    if is_newline(c0) { ln = ln + 1 }
                    pos = pos + 1
                }
            }
            continue
        }

        // String literal: "..."
        if ch == 34 {
            let sstart = pos
            pos = pos + 1
            let mut esc = false
            while pos < slen {
                let sc = peek_char(source, pos)
                if esc {
                    esc = false
                    pos = pos + 1
                } else if sc == 92 {
                    esc = true
                    pos = pos + 1
                } else if sc == 34 {
                    pos = pos + 1
                    break
                } else {
                    if is_newline(sc) { ln = ln + 1 }
                    pos = pos + 1
                }
            }
            buf = tbuf_emit(buf, TK_STRING(), sstart + 1, pos - 1, ln)
            continue
        }

        // Numeric literal
        if is_digit(ch) {
            let nstart = pos
            let mut is_float = false
            if ch == 48 && peek_char2(source, pos) == 120 {
                pos = pos + 2
                while pos < slen && is_hex_digit(peek_char(source, pos)) {
                    pos = pos + 1
                }
            } else {
                while pos < slen && is_digit(peek_char(source, pos)) {
                    pos = pos + 1
                }
                if pos < slen && peek_char(source, pos) == 46 {
                    let nx = peek_char2(source, pos)
                    if nx >= 0 && is_digit(nx) {
                        is_float = true
                        pos = pos + 1
                        while pos < slen && is_digit(peek_char(source, pos)) {
                            pos = pos + 1
                        }
                    }
                }
            }
            if is_float {
                buf = tbuf_emit(buf, TK_FLOAT(), nstart, pos, ln)
            } else {
                buf = tbuf_emit(buf, TK_INT(), nstart, pos, ln)
            }
            continue
        }

        // Identifiers and keywords
        if is_alpha(ch) {
            let istart = pos
            while pos < slen && is_alnum(peek_char(source, pos)) {
                pos = pos + 1
            }
            let word = str_substr(source, istart, pos - istart)
            let kw = keyword_type(word)
            if kw > 0 {
                buf = tbuf_emit(buf, kw, istart, pos, ln)
            } else {
                buf = tbuf_emit(buf, TK_IDENT(), istart, pos, ln)
            }
            continue
        }

        // Two-character operators
        let ch2 = peek_char2(source, pos)
        if ch == 61 && ch2 == 61  { buf = tbuf_emit(buf, TK_EQEQ(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 33 && ch2 == 61  { buf = tbuf_emit(buf, TK_NEQ(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 60 && ch2 == 61  { buf = tbuf_emit(buf, TK_LTE(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 62 && ch2 == 61  { buf = tbuf_emit(buf, TK_GTE(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 38 && ch2 == 38  { buf = tbuf_emit(buf, TK_AND(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 124 && ch2 == 124 { buf = tbuf_emit(buf, TK_OR(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 45 && ch2 == 62  { buf = tbuf_emit(buf, TK_ARROW(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 61 && ch2 == 62  { buf = tbuf_emit(buf, TK_FAT_ARROW(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 124 && ch2 == 62 { buf = tbuf_emit(buf, TK_PIPE(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 46 && ch2 == 46  { buf = tbuf_emit(buf, TK_DOTDOT(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 58 && ch2 == 58  { buf = tbuf_emit(buf, TK_COLONCOLON(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 43 && ch2 == 61  { buf = tbuf_emit(buf, TK_PLUSEQ(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 45 && ch2 == 61  { buf = tbuf_emit(buf, TK_MINUSEQ(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 42 && ch2 == 61  { buf = tbuf_emit(buf, TK_STAREQ(), pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 47 && ch2 == 61  { buf = tbuf_emit(buf, TK_SLASHEQ(), pos, pos + 2, ln); pos = pos + 2; continue }

        // Single-character tokens
        if ch == 40  { buf = tbuf_emit(buf, TK_LPAREN(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 41  { buf = tbuf_emit(buf, TK_RPAREN(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 123 { buf = tbuf_emit(buf, TK_LBRACE(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 125 { buf = tbuf_emit(buf, TK_RBRACE(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 91  { buf = tbuf_emit(buf, TK_LBRACKET(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 93  { buf = tbuf_emit(buf, TK_RBRACKET(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 44  { buf = tbuf_emit(buf, TK_COMMA(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 58  { buf = tbuf_emit(buf, TK_COLON(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 59  { buf = tbuf_emit(buf, TK_SEMICOLON(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 46  { buf = tbuf_emit(buf, TK_DOT(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 43  { buf = tbuf_emit(buf, TK_PLUS(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 45  { buf = tbuf_emit(buf, TK_MINUS(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 42  { buf = tbuf_emit(buf, TK_STAR(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 47  { buf = tbuf_emit(buf, TK_SLASH(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 37  { buf = tbuf_emit(buf, TK_PERCENT(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 61  { buf = tbuf_emit(buf, TK_EQ(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 60  { buf = tbuf_emit(buf, TK_LT(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 62  { buf = tbuf_emit(buf, TK_GT(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 33  { buf = tbuf_emit(buf, TK_NOT(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 38  { buf = tbuf_emit(buf, TK_AMP(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 63  { buf = tbuf_emit(buf, TK_QUESTION(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 35  { buf = tbuf_emit(buf, TK_HASH(), pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 64  { buf = tbuf_emit(buf, TK_AT(), pos, pos + 1, ln); pos = pos + 1; continue }

        // Unknown character
        buf = tbuf_emit(buf, TK_ERROR(), pos, pos + 1, ln)
        pos = pos + 1
    }

    // EOF
    buf = tbuf_emit(buf, TK_EOF(), -1, -1, ln)
    tbuf_finalize(buf)
}

// -- Token type name (for debugging) --
fn tok_type_name(t: i64) -> str {
    if t == 1 { "INT" }
    else if t == 2 { "FLOAT" }
    else if t == 3 { "STRING" }
    else if t == 4 { "IDENT" }
    else if t == 5 { "BOOL" }
    else if t == 10 { "FN" }
    else if t == 11 { "LET" }
    else if t == 12 { "IF" }
    else if t == 13 { "ELSE" }
    else if t == 14 { "WHILE" }
    else if t == 15 { "FOR" }
    else if t == 16 { "RETURN" }
    else if t == 17 { "STRUCT" }
    else if t == 18 { "ENUM" }
    else if t == 19 { "MATCH" }
    else if t == 20 { "IMPORT" }
    else if t == 21 { "CONST" }
    else if t == 22 { "TRAIT" }
    else if t == 23 { "IMPL" }
    else if t == 24 { "IN" }
    else if t == 25 { "MUT" }
    else if t == 26 { "LOOP" }
    else if t == 27 { "BREAK" }
    else if t == 28 { "CONTINUE" }
    else if t == 29 { "TYPE" }
    else if t == 30 { "AS" }
    else if t == 31 { "TRY" }
    else if t == 32 { "CATCH" }
    else if t == 33 { "THROW" }
    else if t == 34 { "ASYNC" }
    else if t == 35 { "AWAIT" }
    else if t == 36 { "SPAWN" }
    else if t == 37 { "EXTERN" }
    else if t == 38 { "PUB" }
    else if t == 39 { "SELF" }
    else if t == 40 { "SUPER" }
    else if t == 41 { "MOD" }
    else if t == 42 { "USE" }
    else if t == 50 { "PLUS" }
    else if t == 51 { "MINUS" }
    else if t == 52 { "STAR" }
    else if t == 53 { "SLASH" }
    else if t == 54 { "PERCENT" }
    else if t == 55 { "EQ" }
    else if t == 56 { "EQEQ" }
    else if t == 57 { "NEQ" }
    else if t == 58 { "LT" }
    else if t == 59 { "GT" }
    else if t == 60 { "LTE" }
    else if t == 61 { "GTE" }
    else if t == 62 { "AND" }
    else if t == 63 { "OR" }
    else if t == 64 { "NOT" }
    else if t == 65 { "ARROW" }
    else if t == 66 { "FAT_ARROW" }
    else if t == 67 { "PIPE" }
    else if t == 68 { "AMP" }
    else if t == 69 { "DOTDOT" }
    else if t == 70 { "QUESTION" }
    else if t == 71 { "PLUSEQ" }
    else if t == 72 { "MINUSEQ" }
    else if t == 73 { "STAREQ" }
    else if t == 74 { "SLASHEQ" }
    else if t == 80 { "LPAREN" }
    else if t == 81 { "RPAREN" }
    else if t == 82 { "LBRACE" }
    else if t == 83 { "RBRACE" }
    else if t == 84 { "LBRACKET" }
    else if t == 85 { "RBRACKET" }
    else if t == 86 { "COMMA" }
    else if t == 87 { "COLON" }
    else if t == 88 { "SEMICOLON" }
    else if t == 89 { "DOT" }
    else if t == 90 { "HASH" }
    else if t == 91 { "AT" }
    else if t == 92 { "COLONCOLON" }
    else if t == 99 { "EOF" }
    else if t == 100 { "ERROR" }
    else { "UNKNOWN" }
}

// -- Test driver (split to keep functions small for Cranelift) --

// ====== PARSER ======
// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 2: Parser (recursive descent)
// ============================================================================
// Parses token buffer (from lexer.sl) into flat AST node arrays.
//
// AST storage: two flat arrays
//   nodes[]: stride 8, each node = [type, d0, d1, d2, d3, d4, d5, d6]
//   lists[]: variable-length child lists (stmt lists, arg lists, param lists)
//
// Parser context packed into a single i64 array (p):
//   p[0] = tokens buffer (from lexer)
//   p[1] = nodes buffer
//   p[2] = lists buffer
//   p[3] = list stack buffer (temp storage during list collection)
//   p[4] = state array [pos, nodes_used, lists_used, stack_sp]
//
// NOTE: All control flow uses if/else expressions, never bare `return`.
//       Functions kept small to avoid Cranelift code size issues.
// ============================================================================

// -- Node type constants --
fn N_INT() -> i64      { 1 }
fn N_FLOAT() -> i64    { 2 }
fn N_STR() -> i64      { 3 }
fn N_BOOL() -> i64     { 4 }
fn N_IDENT() -> i64    { 5 }
fn N_BINOP() -> i64    { 6 }
fn N_UNOP() -> i64     { 7 }
fn N_CALL() -> i64     { 8 }
fn N_INDEX() -> i64    { 9 }
fn N_FIELD() -> i64    { 10 }

fn N_LET() -> i64      { 20 }
fn N_ASSIGN() -> i64   { 21 }
fn N_IF() -> i64       { 22 }
fn N_WHILE() -> i64    { 23 }
fn N_FOR() -> i64      { 24 }
fn N_RETURN() -> i64   { 25 }
fn N_BREAK() -> i64    { 26 }
fn N_CONTINUE() -> i64 { 27 }
fn N_BLOCK() -> i64    { 28 }
fn N_EXPR_STMT() -> i64 { 29 }

fn N_FN() -> i64       { 30 }
fn N_PROGRAM() -> i64  { 31 }
fn N_EMPTY() -> i64    { 0 }

// Node stride
fn NODE_STRIDE() -> i64 { 8 }

// ============================================================================
// Token type constants (duplicated from lexer.sl for standalone use)
// ============================================================================

// ============================================================================
// Parser context (p) accessors
// ============================================================================
fn p_tokens(p: i64) -> i64 { array_get(p, 0) }
fn p_nodes(p: i64) -> i64  { array_get(p, 1) }
fn p_lists(p: i64) -> i64  { array_get(p, 2) }
fn p_stack(p: i64) -> i64  { array_get(p, 3) }
fn p_state(p: i64) -> i64  { array_get(p, 4) }

fn p_pos(p: i64) -> i64 { array_get(p_state(p), 0) }

fn p_set_pos(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 0, v)
    0
}

fn p_nodes_used(p: i64) -> i64 { array_get(p_state(p), 1) }

fn p_set_nodes_used(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 1, v)
    0
}

fn p_lists_used(p: i64) -> i64 { array_get(p_state(p), 2) }

fn p_set_lists_used(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 2, v)
    0
}

fn p_stack_sp(p: i64) -> i64 { array_get(p_state(p), 3) }

fn p_set_stack_sp(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 3, v)
    0
}

// ============================================================================
// Parser helpers
// ============================================================================

// Current token type
fn peek(p: i64) -> i64 {
    let tokens = p_tokens(p)
    let pos = p_pos(p)
    if pos >= tok_count(tokens) { TK_EOF() }
    else { tok_type(tokens, pos) }
}

// Advance by 1 token, return old position
fn advance(p: i64) -> i64 {
    let pos = p_pos(p)
    p_set_pos(p, pos + 1)
    pos
}

// Consume expected token type, advance; return token index
fn eat(p: i64, expected: i64) -> i64 {
    let t = peek(p)
    if t != expected {
        println_str("Parse error: unexpected token")
        println(t)
        println(expected)
        -1
    } else {
        advance(p)
    }
}

// Check if current token matches, but don't consume
fn check(p: i64, t: i64) -> bool {
    peek(p) == t
}

// Current token text
fn cur_text(p: i64, source: str) -> str {
    tok_text(source, p_tokens(p), p_pos(p))
}

// ============================================================================
// Node emission (stride 8)
// ============================================================================
fn emit_node(p: i64, ntype: i64, d0: i64, d1: i64, d2: i64, d3: i64, d4: i64, d5: i64) -> i64 {
    let nodes = p_nodes(p)
    let off = p_nodes_used(p)
    array_set(nodes, off + 0, ntype)
    array_set(nodes, off + 1, d0)
    array_set(nodes, off + 2, d1)
    array_set(nodes, off + 3, d2)
    array_set(nodes, off + 4, d3)
    array_set(nodes, off + 5, d4)
    array_set(nodes, off + 6, d5)
    array_set(nodes, off + 7, 0)
    p_set_nodes_used(p, off + 8)
    off
}

// Read a field from a node at offset
fn node_type(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off) }
fn node_d0(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 1) }
fn node_d1(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 2) }
fn node_d2(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 3) }
fn node_d3(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 4) }
fn node_d4(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 5) }

// Read from lists buffer
fn list_item(p: i64, idx: i64) -> i64 { array_get(p_lists(p), idx) }

// ============================================================================
// List stack operations
// ============================================================================

// Save current stack position as a mark
fn list_save(p: i64) -> i64 {
    p_stack_sp(p)
}

// Push a node index onto the list stack
fn list_push(p: i64, node_idx: i64) -> i64 {
    let sp = p_stack_sp(p)
    array_set(p_stack(p), sp, node_idx)
    p_set_stack_sp(p, sp + 1)
    0
}

// Flush stack items from mark..sp into lists buffer, return start index
fn list_flush(p: i64, mark: i64) -> i64 {
    let sp = p_stack_sp(p)
    let count = sp - mark
    let lstart = p_lists_used(p)
    let stack = p_stack(p)
    let lists = p_lists(p)
    let mut i = 0
    while i < count {
        array_set(lists, lstart + i, array_get(stack, mark + i))
        i = i + 1
    }
    p_set_lists_used(p, lstart + count)
    p_set_stack_sp(p, mark)
    lstart
}

// Count of items flushed = sp - mark (call before flush)
fn list_count(p: i64, mark: i64) -> i64 {
    p_stack_sp(p) - mark
}

// ============================================================================
// Operator precedence (for Pratt parsing)
// ============================================================================
fn prefix_bp(op: i64) -> i64 {
    if op == TK_MINUS() { 90 }
    else if op == TK_NOT() { 90 }
    else { -1 }
}

fn infix_bp_left(op: i64) -> i64 {
    if op == TK_OR()      { 10 }
    else if op == TK_AND()     { 20 }
    else if op == TK_EQEQ()   { 30 }
    else if op == TK_NEQ()     { 30 }
    else if op == TK_LT()     { 40 }
    else if op == TK_GT()     { 40 }
    else if op == TK_LTE()    { 40 }
    else if op == TK_GTE()    { 40 }
    else if op == TK_PLUS()   { 50 }
    else if op == TK_MINUS()  { 50 }
    else if op == TK_STAR()   { 60 }
    else if op == TK_SLASH()  { 60 }
    else if op == TK_PERCENT(){ 60 }
    else if op == TK_PIPE()   { 5 }
    else { -1 }
}

fn infix_bp_right(op: i64) -> i64 {
    if op == TK_OR()      { 11 }
    else if op == TK_AND()     { 21 }
    else if op == TK_EQEQ()   { 31 }
    else if op == TK_NEQ()     { 31 }
    else if op == TK_LT()     { 41 }
    else if op == TK_GT()     { 41 }
    else if op == TK_LTE()    { 41 }
    else if op == TK_GTE()    { 41 }
    else if op == TK_PLUS()   { 51 }
    else if op == TK_MINUS()  { 51 }
    else if op == TK_STAR()   { 61 }
    else if op == TK_SLASH()  { 61 }
    else if op == TK_PERCENT(){ 61 }
    else if op == TK_PIPE()   { 4 }
    else { -1 }
}

// ============================================================================
// Expression parsing (Pratt parser)
// ============================================================================

// Parse an integer literal from token text
fn parse_int_value(source: str, tokens: i64, idx: i64) -> i64 {
    let text = tok_text(source, tokens, idx)
    let tlen = str_len(text)
    // Check for hex: 0x or 0X
    if tlen > 2 && str_char_at(text, 0) == "0" {
        let c1 = str_char_at(text, 1)
        if str_eq(c1, "x") || str_eq(c1, "X") {
            parse_hex(text, 2, tlen)
        } else {
            parse_decimal(text, 0, tlen)
        }
    } else {
        parse_decimal(text, 0, tlen)
    }
}

fn parse_decimal(text: str, start: i64, end: i64) -> i64 {
    let mut val = 0
    let mut i = start
    while i < end {
        let d = char_to_int(str_char_at(text, i)) - 48
        val = val * 10 + d
        i = i + 1
    }
    val
}

fn parse_hex(text: str, start: i64, end: i64) -> i64 {
    let mut val = 0
    let mut i = start
    while i < end {
        let c = char_to_int(str_char_at(text, i))
        let d = if c >= 97 { c - 87 }
                else if c >= 65 { c - 55 }
                else { c - 48 }
        val = val * 16 + d
        i = i + 1
    }
    val
}

// Parse a primary (atom) expression
fn parse_primary(p: i64, source: str) -> i64 {
    let t = peek(p)
    if t == TK_INT() {
        let idx = advance(p)
        let val = parse_int_value(source, p_tokens(p), idx)
        emit_node(p, N_INT(), val, 0, 0, 0, 0, 0)
    } else if t == TK_FLOAT() {
        let idx = advance(p)
        emit_node(p, N_FLOAT(), idx, 0, 0, 0, 0, 0)
    } else if t == TK_STRING() {
        let idx = advance(p)
        emit_node(p, N_STR(), idx, 0, 0, 0, 0, 0)
    } else if t == TK_BOOL() {
        let idx = advance(p)
        let text = tok_text(source, p_tokens(p), idx)
        let val = if str_eq(text, "true") { 1 } else { 0 }
        emit_node(p, N_BOOL(), val, 0, 0, 0, 0, 0)
    } else if t == TK_IDENT() {
        parse_ident_or_call(p, source)
    } else if t == TK_LPAREN() {
        advance(p)
        let inner = parse_expr(p, source, 0)
        eat(p, TK_RPAREN())
        inner
    } else if t == TK_MINUS() || t == TK_NOT() {
        let op_idx = advance(p)
        let op_type = tok_type(p_tokens(p), op_idx)
        let operand = parse_expr(p, source, 90)
        emit_node(p, N_UNOP(), op_type, operand, 0, 0, 0, 0)
    } else {
        println_str("Parse error: unexpected token in expression")
        println(t)
        advance(p)
        emit_node(p, N_EMPTY(), 0, 0, 0, 0, 0, 0)
    }
}

// Parse identifier, possibly followed by function call (...)
fn parse_ident_or_call(p: i64, source: str) -> i64 {
    let idx = advance(p)
    if peek(p) == TK_LPAREN() {
        parse_call_args(p, source, idx)
    } else {
        emit_node(p, N_IDENT(), idx, 0, 0, 0, 0, 0)
    }
}

// Parse function call arguments: (expr, expr, ...)
fn parse_call_args(p: i64, source: str, callee_tok: i64) -> i64 {
    eat(p, TK_LPAREN())
    let mark = list_save(p)
    let mut count = 0
    if peek(p) != TK_RPAREN() {
        let a = parse_expr(p, source, 0)
        list_push(p, a)
        count = count + 1
        while peek(p) == TK_COMMA() {
            advance(p)
            let a2 = parse_expr(p, source, 0)
            list_push(p, a2)
            count = count + 1
        }
    } else {
        count = 0
    }
    eat(p, TK_RPAREN())
    let lstart = list_flush(p, mark)
    emit_node(p, N_CALL(), callee_tok, lstart, count, 0, 0, 0)
}

// Pratt expression parser with minimum binding power
fn parse_expr(p: i64, source: str, min_bp: i64) -> i64 {
    let mut left = parse_primary(p, source)
    left = parse_expr_loop(p, source, left, min_bp)
    left
}

// Iterative infix loop (separated to keep functions small)
fn parse_expr_loop(p: i64, source: str, lhs: i64, min_bp: i64) -> i64 {
    let mut left = lhs
    let mut done = false
    while !done {
        let op = peek(p)
        let lbp = infix_bp_left(op)
        if lbp < min_bp || lbp < 0 {
            done = true
        } else {
            advance(p)
            let rbp = infix_bp_right(op)
            let right = parse_expr(p, source, rbp)
            left = emit_node(p, N_BINOP(), op, left, right, 0, 0, 0)
            done = false
        }
    }
    left
}

// ============================================================================
// Statement parsing
// ============================================================================

fn parse_stmt(p: i64, source: str) -> i64 {
    let t = peek(p)
    if t == TK_LET() { parse_let(p, source) }
    else if t == TK_IF() { parse_if(p, source) }
    else if t == TK_WHILE() { parse_while(p, source) }
    else if t == TK_FOR() { parse_for(p, source) }
    else if t == TK_RETURN() { parse_return(p, source) }
    else if t == TK_BREAK() { advance(p); emit_node(p, N_BREAK(), 0, 0, 0, 0, 0, 0) }
    else if t == TK_CONTINUE() { advance(p); emit_node(p, N_CONTINUE(), 0, 0, 0, 0, 0, 0) }
    else { parse_expr_or_assign(p, source) }
}

// Parse let binding: let [mut] name [: type] = expr
fn parse_let(p: i64, source: str) -> i64 {
    eat(p, TK_LET())
    let mut mutable = 0
    if peek(p) == TK_MUT() {
        advance(p)
        mutable = 1
    } else {
        mutable = 0
    }
    let name_tok = eat(p, TK_IDENT())
    // Optional type annotation: : type
    let mut type_tok = -1
    if peek(p) == TK_COLON() {
        advance(p)
        type_tok = eat(p, TK_IDENT())
    } else {
        type_tok = -1
    }
    eat(p, TK_EQ())
    let value = parse_expr(p, source, 0)
    emit_node(p, N_LET(), name_tok, mutable, type_tok, value, 0, 0)
}

// Parse if/else if/else
fn parse_if(p: i64, source: str) -> i64 {
    eat(p, TK_IF())
    let cond = parse_expr(p, source, 0)
    let then_block = parse_block(p, source)
    let mut else_node = -1
    if peek(p) == TK_ELSE() {
        advance(p)
        if peek(p) == TK_IF() {
            else_node = parse_if(p, source)
        } else {
            else_node = parse_block(p, source)
        }
    } else {
        else_node = -1
    }
    emit_node(p, N_IF(), cond, then_block, else_node, 0, 0, 0)
}

// Parse while loop
fn parse_while(p: i64, source: str) -> i64 {
    eat(p, TK_WHILE())
    let cond = parse_expr(p, source, 0)
    let body = parse_block(p, source)
    emit_node(p, N_WHILE(), cond, body, 0, 0, 0, 0)
}

// Parse for loop: for ident in expr { ... }
fn parse_for(p: i64, source: str) -> i64 {
    eat(p, TK_FOR())
    let var_tok = eat(p, TK_IDENT())
    eat(p, TK_IN())
    let iter = parse_expr(p, source, 0)
    let body = parse_block(p, source)
    emit_node(p, N_FOR(), var_tok, iter, body, 0, 0, 0)
}

// Parse return statement
fn parse_return(p: i64, source: str) -> i64 {
    eat(p, TK_RETURN())
    let t = peek(p)
    if t == TK_RBRACE() || t == TK_EOF() {
        emit_node(p, N_RETURN(), -1, 0, 0, 0, 0, 0)
    } else {
        let val = parse_expr(p, source, 0)
        emit_node(p, N_RETURN(), val, 0, 0, 0, 0, 0)
    }
}

// Parse expression or assignment: expr  or  ident = expr
fn parse_expr_or_assign(p: i64, source: str) -> i64 {
    let lhs = parse_expr(p, source, 0)
    if peek(p) == TK_EQ() {
        advance(p)
        let rhs = parse_expr(p, source, 0)
        // lhs should be an ident node - extract the token index from it
        let target_tok = node_d0(p, lhs)
        emit_node(p, N_ASSIGN(), target_tok, rhs, 0, 0, 0, 0)
    } else {
        // Bare expression as statement
        emit_node(p, N_EXPR_STMT(), lhs, 0, 0, 0, 0, 0)
    }
}

// ============================================================================
// Block parsing: { stmt; stmt; ... }
// ============================================================================
fn parse_block(p: i64, source: str) -> i64 {
    eat(p, TK_LBRACE())
    let mark = list_save(p)
    let mut count = 0
    while peek(p) != TK_RBRACE() && peek(p) != TK_EOF() {
        // Skip semicolons
        if peek(p) == TK_SEMICOLON() {
            advance(p)
        } else {
            let stmt = parse_stmt(p, source)
            list_push(p, stmt)
            count = count + 1
        }
    }
    eat(p, TK_RBRACE())
    let lstart = list_flush(p, mark)
    emit_node(p, N_BLOCK(), lstart, count, 0, 0, 0, 0)
}

// ============================================================================
// Top-level parsing
// ============================================================================

// Parse function definition: fn name(params) [-> type] { body }
fn parse_function(p: i64, source: str) -> i64 {
    eat(p, TK_FN())
    let name_tok = eat(p, TK_IDENT())
    // Parse parameter list
    eat(p, TK_LPAREN())
    let param_mark = list_save(p)
    let mut param_count = 0
    if peek(p) != TK_RPAREN() {
        let pn = eat(p, TK_IDENT())
        eat(p, TK_COLON())
        let pt = eat(p, TK_IDENT())
        list_push(p, pn)
        list_push(p, pt)
        param_count = param_count + 1
        while peek(p) == TK_COMMA() {
            advance(p)
            let pn2 = eat(p, TK_IDENT())
            eat(p, TK_COLON())
            let pt2 = eat(p, TK_IDENT())
            list_push(p, pn2)
            list_push(p, pt2)
            param_count = param_count + 1
        }
    } else {
        param_count = 0
    }
    eat(p, TK_RPAREN())
    let params_start = list_flush(p, param_mark)
    // Optional return type: -> type
    let mut ret_tok = -1
    if peek(p) == TK_ARROW() {
        advance(p)
        ret_tok = eat(p, TK_IDENT())
    } else {
        ret_tok = -1
    }
    // Body
    let body = parse_block(p, source)
    emit_node(p, N_FN(), name_tok, params_start, param_count, ret_tok, body, 0)
}

// Parse a full program (list of top-level items)
fn parse_program(p: i64, source: str) -> i64 {
    let mark = list_save(p)
    let mut count = 0
    while peek(p) != TK_EOF() {
        let t = peek(p)
        if t == TK_FN() {
            let fn_node = parse_function(p, source)
            list_push(p, fn_node)
            count = count + 1
        } else {
            println_str("Parse error: expected top-level 'fn'")
            println(t)
            advance(p)
        }
    }
    let lstart = list_flush(p, mark)
    emit_node(p, N_PROGRAM(), lstart, count, 0, 0, 0, 0)
}

// ============================================================================
// Parser entry point
// ============================================================================
fn parser_new(tokens: i64) -> i64 {
    let p = array_new(5)
    array_set(p, 0, tokens)
    array_set(p, 1, array_new(16000))
    array_set(p, 2, array_new(8000))
    array_set(p, 3, array_new(4000))
    let state = array_new(4)
    array_set(state, 0, 0)
    array_set(state, 1, 0)
    array_set(state, 2, 0)
    array_set(state, 3, 0)
    array_set(p, 4, state)
    p
}

fn parse(tokens: i64, source: str) -> i64 {
    let p = parser_new(tokens)
    let prog = parse_program(p, source)
    p
}

// ============================================================================
// AST pretty-printer (for testing)
// ============================================================================
fn print_indent(depth: i64) -> i64 {
    let mut i = 0
    while i < depth {
        print_str("  ")
        i = i + 1
    }
    0
}

fn print_node(p: i64, source: str, off: i64, depth: i64) -> i64 {
    let ntype = node_type(p, off)
    if ntype == N_INT() { print_node_int(p, off, depth) }
    else if ntype == N_STR() { print_node_str(p, source, off, depth) }
    else if ntype == N_BOOL() { print_node_bool(p, off, depth) }
    else if ntype == N_IDENT() { print_node_ident(p, source, off, depth) }
    else if ntype == N_BINOP() { print_node_binop(p, source, off, depth) }
    else if ntype == N_UNOP() { print_node_unop(p, source, off, depth) }
    else if ntype == N_CALL() { print_node_call(p, source, off, depth) }
    else if ntype == N_LET() { print_node_let(p, source, off, depth) }
    else if ntype == N_ASSIGN() { print_node_assign(p, source, off, depth) }
    else if ntype == N_IF() { print_node_if(p, source, off, depth) }
    else if ntype == N_WHILE() { print_node_while(p, source, off, depth) }
    else if ntype == N_RETURN() { print_node_return(p, source, off, depth) }
    else if ntype == N_BLOCK() { print_node_block(p, source, off, depth) }
    else if ntype == N_FN() { print_node_fn(p, source, off, depth) }
    else if ntype == N_PROGRAM() { print_node_program(p, source, off, depth) }
    else if ntype == N_EXPR_STMT() { print_node(p, source, node_d0(p, off), depth) }
    else if ntype == N_BREAK() { print_indent(depth); println_str("BREAK") }
    else if ntype == N_CONTINUE() { print_indent(depth); println_str("CONTINUE") }
    else { print_indent(depth); print_str("UNKNOWN("); println(ntype) }
    0
}

fn print_node_int(p: i64, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    print_str("INT(")
    print(node_d0(p, off))
    println_str(")")
    0
}

fn print_node_str(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let tok_idx = node_d0(p, off)
    print_str("STR('")
    print_str(tok_text(source, p_tokens(p), tok_idx))
    println_str("')")
    0
}

fn print_node_bool(p: i64, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    if node_d0(p, off) == 1 { println_str("BOOL(true)") }
    else { println_str("BOOL(false)") }
    0
}

fn print_node_ident(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let tok_idx = node_d0(p, off)
    print_str("IDENT(")
    print_str(tok_text(source, p_tokens(p), tok_idx))
    println_str(")")
    0
}

fn print_node_binop(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let op = node_d0(p, off)
    print_str("BINOP(")
    print_op_name(op)
    println_str(")")
    print_node(p, source, node_d1(p, off), depth + 1)
    print_node(p, source, node_d2(p, off), depth + 1)
}

fn print_op_name(op: i64) -> i64 {
    if op == TK_PLUS() { print_str("+") }
    else if op == TK_MINUS() { print_str("-") }
    else if op == TK_STAR() { print_str("*") }
    else if op == TK_SLASH() { print_str("/") }
    else if op == TK_PERCENT() { print_str("%") }
    else if op == TK_EQEQ() { print_str("==") }
    else if op == TK_NEQ() { print_str("!=") }
    else if op == TK_LT() { print_str("<") }
    else if op == TK_GT() { print_str(">") }
    else if op == TK_LTE() { print_str("<=") }
    else if op == TK_GTE() { print_str(">=") }
    else if op == TK_AND() { print_str("&&") }
    else if op == TK_OR() { print_str("||") }
    else if op == TK_PIPE() { print_str("|>") }
    else { print_str("?op?") }
    0
}

fn print_node_unop(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let op = node_d0(p, off)
    if op == TK_MINUS() { println_str("UNOP(-)") }
    else if op == TK_NOT() { println_str("UNOP(!)") }
    else { println_str("UNOP(?)") }
    print_node(p, source, node_d1(p, off), depth + 1)
}

fn print_node_call(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let callee_tok = node_d0(p, off)
    print_str("CALL(")
    print_str(tok_text(source, p_tokens(p), callee_tok))
    println_str(")")
    let lstart = node_d1(p, off)
    let cnt = node_d2(p, off)
    let mut i = 0
    while i < cnt {
        let arg_node = list_item(p, lstart + i)
        print_node(p, source, arg_node, depth + 1)
        i = i + 1
    }
    0
}

fn print_node_let(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let name_tok = node_d0(p, off)
    let mutable = node_d1(p, off)
    if mutable == 1 { print_str("LET MUT ") }
    else { print_str("LET ") }
    println_str(tok_text(source, p_tokens(p), name_tok))
    let val_node = node_d3(p, off)
    print_node(p, source, val_node, depth + 1)
}

fn print_node_assign(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let target_tok = node_d0(p, off)
    print_str("ASSIGN ")
    println_str(tok_text(source, p_tokens(p), target_tok))
    print_node(p, source, node_d1(p, off), depth + 1)
}

fn print_node_if(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("IF")
    print_node(p, source, node_d0(p, off), depth + 1)
    print_indent(depth)
    println_str("THEN")
    print_node(p, source, node_d1(p, off), depth + 1)
    let else_node = node_d2(p, off)
    if else_node >= 0 {
        print_indent(depth)
        println_str("ELSE")
        print_node(p, source, else_node, depth + 1)
    } else {
        0
    }
}

fn print_node_while(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("WHILE")
    print_node(p, source, node_d0(p, off), depth + 1)
    print_node(p, source, node_d1(p, off), depth + 1)
}

fn print_node_return(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("RETURN")
    let val = node_d0(p, off)
    if val >= 0 {
        print_node(p, source, val, depth + 1)
    } else {
        0
    }
}

fn print_node_block(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("BLOCK")
    let lstart = node_d0(p, off)
    let cnt = node_d1(p, off)
    let mut i = 0
    while i < cnt {
        let stmt_node = list_item(p, lstart + i)
        print_node(p, source, stmt_node, depth + 1)
        i = i + 1
    }
    0
}

fn print_node_fn(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let name_tok = node_d0(p, off)
    print_str("FN ")
    print_str(tok_text(source, p_tokens(p), name_tok))
    let param_count = node_d2(p, off)
    print_str("(")
    let pstart = node_d1(p, off)
    let mut pi = 0
    while pi < param_count {
        if pi > 0 { print_str(", ") } else { 0 }
        let pn = list_item(p, pstart + pi * 2)
        let pt = list_item(p, pstart + pi * 2 + 1)
        print_str(tok_text(source, p_tokens(p), pn))
        print_str(": ")
        print_str(tok_text(source, p_tokens(p), pt))
        pi = pi + 1
    }
    print_str(")")
    let ret_tok = node_d3(p, off)
    if ret_tok >= 0 {
        print_str(" -> ")
        print_str(tok_text(source, p_tokens(p), ret_tok))
    } else {
        0
    }
    println_str("")
    let body_node = node_d4(p, off)
    print_node(p, source, body_node, depth + 1)
}

fn print_node_program(p: i64, source: str, off: i64, depth: i64) -> i64 {
    println_str("PROGRAM")
    let lstart = node_d0(p, off)
    let cnt = node_d1(p, off)
    let mut i = 0
    while i < cnt {
        let fn_node = list_item(p, lstart + i)
        print_node(p, source, fn_node, depth + 1)
        i = i + 1
    }
    0
}

// ============================================================================
// Tests
// ============================================================================


// ====== IR GENERATION ======
// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 4: IR Generation
// ============================================================================
// Lowers typed AST (from parser.sl) to a flat SSA-form IR.
//
// IR layout: flat array of instructions, each stride=6:
//   [opcode, dest, arg0, arg1, arg2, extra]
//
// Virtual registers: v0, v1, v2, ... (incrementing counter)
// Basic blocks: b0, b1, b2, ... (each holds a range of instructions)
//
// Instructions (opcodes):
//   OP_ICONST=1   dest = immediate i64 value (in arg0, arg1 for 64-bit)
//   OP_SCONST=2   dest = string constant index
//   OP_FCONST=3   dest = float constant index
//   OP_ADD=10      dest = arg0 + arg1
//   OP_SUB=11      dest = arg0 - arg1
//   OP_MUL=12      dest = arg0 * arg1
//   OP_DIV=13      dest = arg0 / arg1
//   OP_MOD=14      dest = arg0 % arg1
//   OP_NEG=15      dest = -arg0
//   OP_EQ=20       dest = arg0 == arg1
//   OP_NEQ=21      dest = arg0 != arg1
//   OP_LT=22       dest = arg0 < arg1
//   OP_GT=23       dest = arg0 > arg1
//   OP_LTE=24      dest = arg0 <= arg1
//   OP_GTE=25      dest = arg0 >= arg1
//   OP_AND=26      dest = arg0 && arg1
//   OP_OR=27       dest = arg0 || arg1
//   OP_NOT=28      dest = !arg0
//   OP_CALL=30     dest = call func(arg0=fn_idx, arg1=args_start, arg2=arg_count)
//   OP_RET=31      return arg0 (dest unused)
//   OP_BR=32       unconditional branch to block arg0
//   OP_CONDBR=33   branch: if arg0 then block arg1 else block arg2
//   OP_COPY=34     dest = arg0 (register copy / move)
//   OP_PHI=35      dest = phi(arg0=block1_val, arg1=block2_val) -- simplified
//   OP_NOP=0       no operation
//
// NOTE: No `return` in if-blocks. All control flow uses if/else expressions.
// ============================================================================

// -- IR opcode constants --
fn OP_NOP() -> i64    { 0 }
fn OP_ICONST() -> i64 { 1 }
fn OP_SCONST() -> i64 { 2 }
fn OP_FCONST() -> i64 { 3 }
fn OP_BCONST() -> i64 { 4 }

fn OP_ADD() -> i64    { 10 }
fn OP_SUB() -> i64    { 11 }
fn OP_MUL() -> i64    { 12 }
fn OP_DIV() -> i64    { 13 }
fn OP_MOD() -> i64    { 14 }
fn OP_NEG() -> i64    { 15 }

fn OP_EQ() -> i64     { 20 }
fn OP_NEQ() -> i64    { 21 }
fn OP_LT() -> i64     { 22 }
fn OP_GT() -> i64     { 23 }
fn OP_LTE() -> i64    { 24 }
fn OP_GTE() -> i64    { 25 }
fn OP_AND() -> i64    { 26 }
fn OP_OR() -> i64     { 27 }
fn OP_NOT() -> i64    { 28 }

fn OP_CALL() -> i64   { 30 }
fn OP_RET() -> i64    { 31 }
fn OP_BR() -> i64     { 32 }
fn OP_CONDBR() -> i64 { 33 }
fn OP_COPY() -> i64   { 34 }
fn OP_PHI() -> i64    { 35 }

fn IR_STRIDE() -> i64 { 6 }

// ============================================================================
// String hashing (djb2) - same as typechecker.sl
// Defined here for standalone use; in full pipeline, typechecker provides it.
// ============================================================================
fn str_hash(s: str) -> i64 {
    let slen = str_len(s)
    let mut h = 5381
    let mut i = 0
    while i < slen {
        let c = char_to_int(str_char_at(s, i))
        h = h * 33 + c
        h = h % 2147483647
        i = i + 1
    }
    h
}

// ============================================================================
// IR context (ir)
// ============================================================================
// ir[0] = parser context (p)
// ir[1] = instructions array (flat, stride 6)
// ir[2] = inst_count
// ir[3] = next_vreg
// ir[4] = blocks array [start_inst, end_inst] stride=2
// ir[5] = block_count
// ir[6] = current_block
// ir[7] = string constants array (token indices for SCONST lookup)
// ir[8] = string_count
// ir[9] = function table array (for mapping fn names -> fn indices)
// ir[10] = fn_table_count
// ir[11] = variable map array [name_hash, vreg] stride=2
// ir[12] = var_count
// ir[13] = var scope depth
// ir[14] = call args temp buffer
// ir[15] = fn_ir_entries array [name_hash, start_block, end_block, param_count] stride=4
// ir[16] = fn_ir_count

fn ir_p(ir: i64) -> i64          { array_get(ir, 0) }
fn ir_insts(ir: i64) -> i64      { array_get(ir, 1) }
fn ir_inst_count(ir: i64) -> i64  { array_get(ir, 2) }
fn ir_next_vreg(ir: i64) -> i64   { array_get(ir, 3) }
fn ir_blocks(ir: i64) -> i64      { array_get(ir, 4) }
fn ir_block_count(ir: i64) -> i64  { array_get(ir, 5) }
fn ir_cur_block(ir: i64) -> i64    { array_get(ir, 6) }
fn ir_strings(ir: i64) -> i64     { array_get(ir, 7) }
fn ir_string_count(ir: i64) -> i64 { array_get(ir, 8) }
fn ir_fn_table(ir: i64) -> i64    { array_get(ir, 9) }
fn ir_fn_table_count(ir: i64) -> i64 { array_get(ir, 10) }
fn ir_vars(ir: i64) -> i64        { array_get(ir, 11) }
fn ir_var_count(ir: i64) -> i64    { array_get(ir, 12) }
fn ir_var_depth(ir: i64) -> i64    { array_get(ir, 13) }
fn ir_call_args(ir: i64) -> i64    { array_get(ir, 14) }
fn ir_fn_entries(ir: i64) -> i64   { array_get(ir, 15) }
fn ir_fn_entry_count(ir: i64) -> i64 { array_get(ir, 16) }

fn ir_set_inst_count(ir: i64, v: i64) -> i64  { array_set(ir, 2, v); 0 }
fn ir_set_next_vreg(ir: i64, v: i64) -> i64   { array_set(ir, 3, v); 0 }
fn ir_set_block_count(ir: i64, v: i64) -> i64  { array_set(ir, 5, v); 0 }
fn ir_set_cur_block(ir: i64, v: i64) -> i64    { array_set(ir, 6, v); 0 }
fn ir_set_string_count(ir: i64, v: i64) -> i64 { array_set(ir, 8, v); 0 }
fn ir_set_fn_table_count(ir: i64, v: i64) -> i64 { array_set(ir, 10, v); 0 }
fn ir_set_var_count(ir: i64, v: i64) -> i64    { array_set(ir, 12, v); 0 }
fn ir_set_var_depth(ir: i64, v: i64) -> i64    { array_set(ir, 13, v); 0 }
fn ir_set_fn_entry_count(ir: i64, v: i64) -> i64 { array_set(ir, 16, v); 0 }

// ============================================================================
// Virtual register allocation
// ============================================================================
fn alloc_vreg(ir: i64) -> i64 {
    let v = ir_next_vreg(ir)
    ir_set_next_vreg(ir, v + 1)
    v
}

// ============================================================================
// Basic block management
// ============================================================================
fn new_block(ir: i64) -> i64 {
    let bid = ir_block_count(ir)
    let blocks = ir_blocks(ir)
    let off = bid * 2
    array_set(blocks, off + 0, ir_inst_count(ir))
    array_set(blocks, off + 1, ir_inst_count(ir))
    ir_set_block_count(ir, bid + 1)
    bid
}

fn seal_block(ir: i64, bid: i64) -> i64 {
    let blocks = ir_blocks(ir)
    let off = bid * 2
    array_set(blocks, off + 1, ir_inst_count(ir))
    0
}

fn switch_block(ir: i64, bid: i64) -> i64 {
    // Seal current block, switch to new one
    let cur = ir_cur_block(ir)
    seal_block(ir, cur)
    ir_set_cur_block(ir, bid)
    0
}

// ============================================================================
// Emit IR instructions
// ============================================================================
fn emit_ir(ir: i64, op: i64, dest: i64, a0: i64, a1: i64, a2: i64, extra: i64) -> i64 {
    let insts = ir_insts(ir)
    let idx = ir_inst_count(ir)
    let off = idx * IR_STRIDE()
    array_set(insts, off + 0, op)
    array_set(insts, off + 1, dest)
    array_set(insts, off + 2, a0)
    array_set(insts, off + 3, a1)
    array_set(insts, off + 4, a2)
    array_set(insts, off + 5, extra)
    ir_set_inst_count(ir, idx + 1)
    dest
}

fn emit_iconst(ir: i64, value: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_ICONST(), dest, value, 0, 0, 0)
}

fn emit_bconst(ir: i64, value: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_BCONST(), dest, value, 0, 0, 0)
}

fn emit_sconst(ir: i64, tok_idx: i64) -> i64 {
    // Store string constant index
    let strings = ir_strings(ir)
    let si = ir_string_count(ir)
    array_set(strings, si, tok_idx)
    ir_set_string_count(ir, si + 1)
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_SCONST(), dest, si, 0, 0, 0)
}

fn emit_binop(ir: i64, op: i64, left: i64, right: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, op, dest, left, right, 0, 0)
}

fn emit_unop(ir: i64, op: i64, operand: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, op, dest, operand, 0, 0, 0)
}

fn emit_copy(ir: i64, src: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_COPY(), dest, src, 0, 0, 0)
}

fn emit_call(ir: i64, fn_idx: i64, args_start: i64, arg_count: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_CALL(), dest, fn_idx, args_start, arg_count, 0)
}

fn emit_ret(ir: i64, val: i64) -> i64 {
    emit_ir(ir, OP_RET(), 0, val, 0, 0, 0)
}

fn emit_br(ir: i64, target_block: i64) -> i64 {
    emit_ir(ir, OP_BR(), 0, target_block, 0, 0, 0)
}

fn emit_condbr(ir: i64, cond: i64, then_block: i64, else_block: i64) -> i64 {
    emit_ir(ir, OP_CONDBR(), 0, cond, then_block, else_block, 0)
}

// ============================================================================
// Variable mapping (name_hash -> vreg)
// ============================================================================

fn var_set(ir: i64, name_hash: i64, vreg: i64) -> i64 {
    let vars = ir_vars(ir)
    let cnt = ir_var_count(ir)
    // Update existing entry if found (scan backwards for most recent)
    let mut i = cnt - 1
    let mut done = 0
    while i >= 0 && done == 0 {
        let off = i * 2
        if array_get(vars, off) == name_hash {
            array_set(vars, off + 1, vreg)
            done = 1
            0
        } else {
            i = i - 1
        }
    }
    // If not found, add new entry
    if done == 0 {
        let off = cnt * 2
        array_set(vars, off, name_hash)
        array_set(vars, off + 1, vreg)
        ir_set_var_count(ir, cnt + 1)
        0
    } else {
        0
    }
}

fn var_get(ir: i64, name_hash: i64) -> i64 {
    let vars = ir_vars(ir)
    let cnt = ir_var_count(ir)
    let mut i = cnt - 1
    let mut result = -1
    let mut done = 0
    while i >= 0 && done == 0 {
        let off = i * 2
        if array_get(vars, off) == name_hash {
            result = array_get(vars, off + 1)
            done = 1
            0
        } else {
            i = i - 1
        }
    }
    result
}

// Scope management for variables
fn var_scope_enter(ir: i64) -> i64 {
    ir_set_var_depth(ir, ir_var_depth(ir) + 1)
    ir_var_count(ir)
}

fn var_scope_leave(ir: i64, restore: i64) -> i64 {
    ir_set_var_depth(ir, ir_var_depth(ir) - 1)
    ir_set_var_count(ir, restore)
    0
}

// ============================================================================
// Function name -> index mapping
// ============================================================================
fn fn_register_ir(ir: i64, name_hash: i64) -> i64 {
    let ft = ir_fn_table(ir)
    let cnt = ir_fn_table_count(ir)
    // Check if already registered
    let mut i = 0
    let mut found = -1
    let mut done = 0
    while i < cnt && done == 0 {
        if array_get(ft, i) == name_hash {
            found = i
            done = 1
            0
        } else {
            i = i + 1
        }
    }
    if found >= 0 {
        found
    } else {
        array_set(ft, cnt, name_hash)
        ir_set_fn_table_count(ir, cnt + 1)
        cnt
    }
}

fn fn_lookup_ir(ir: i64, name_hash: i64) -> i64 {
    let ft = ir_fn_table(ir)
    let cnt = ir_fn_table_count(ir)
    let mut i = 0
    let mut result = -1
    let mut done = 0
    while i < cnt && done == 0 {
        if array_get(ft, i) == name_hash {
            result = i
            done = 1
            0
        } else {
            i = i + 1
        }
    }
    result
}

// Record which blocks belong to which function
fn fn_entry_add(ir: i64, name_hash: i64, start_block: i64, param_count: i64) -> i64 {
    let entries = ir_fn_entries(ir)
    let cnt = ir_fn_entry_count(ir)
    let off = cnt * 4
    array_set(entries, off + 0, name_hash)
    array_set(entries, off + 1, start_block)
    array_set(entries, off + 2, -1)  // end_block filled later
    array_set(entries, off + 3, param_count)
    ir_set_fn_entry_count(ir, cnt + 1)
    cnt
}

fn fn_entry_set_end(ir: i64, entry_idx: i64, end_block: i64) -> i64 {
    let entries = ir_fn_entries(ir)
    let off = entry_idx * 4
    array_set(entries, off + 2, end_block)
    0
}

// ============================================================================
// Token operator -> IR opcode mapping
// ============================================================================
fn tok_to_binop(tok_type: i64) -> i64 {
    if tok_type == TK_PLUS() { OP_ADD() }
    else if tok_type == TK_MINUS() { OP_SUB() }
    else if tok_type == TK_STAR() { OP_MUL() }
    else if tok_type == TK_SLASH() { OP_DIV() }
    else if tok_type == TK_PERCENT() { OP_MOD() }
    else if tok_type == TK_EQEQ() { OP_EQ() }
    else if tok_type == TK_NEQ() { OP_NEQ() }
    else if tok_type == TK_LT() { OP_LT() }
    else if tok_type == TK_GT() { OP_GT() }
    else if tok_type == TK_LTE() { OP_LTE() }
    else if tok_type == TK_GTE() { OP_GTE() }
    else if tok_type == TK_AND() { OP_AND() }
    else if tok_type == TK_OR() { OP_OR() }
    else { OP_NOP() }
}

// ============================================================================
// Lower expressions to IR
// ============================================================================

fn lower_expr(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let nt = node_type(p, off)
    if nt == N_INT() { lower_int(ir, p, off) }
    else if nt == N_BOOL() { lower_bool(ir, p, off) }
    else if nt == N_STR() { lower_str(ir, p, off) }
    else if nt == N_IDENT() { lower_ident(ir, source, off) }
    else if nt == N_BINOP() { lower_binop(ir, source, off) }
    else if nt == N_UNOP() { lower_unop(ir, source, off) }
    else if nt == N_CALL() { lower_call(ir, source, off) }
    else if nt == N_IF() { lower_if_expr(ir, source, off) }
    else {
        // Unknown expression type, return 0
        emit_iconst(ir, 0)
    }
}

fn lower_int(ir: i64, p: i64, off: i64) -> i64 {
    let value = node_d0(p, off)
    emit_iconst(ir, value)
}

fn lower_bool(ir: i64, p: i64, off: i64) -> i64 {
    let value = node_d0(p, off)
    emit_bconst(ir, value)
}

fn lower_str(ir: i64, p: i64, off: i64) -> i64 {
    let tok_idx = node_d0(p, off)
    emit_sconst(ir, tok_idx)
}

fn lower_ident(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let tok_idx = node_d0(p, off)
    let name = tok_text(source, p_tokens(p), tok_idx)
    let h = str_hash(name)
    let vreg = var_get(ir, h)
    if vreg >= 0 {
        vreg
    } else {
        // Should not happen after type checking, but emit 0
        emit_iconst(ir, 0)
    }
}

fn lower_binop(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let op_tok = node_d0(p, off)
    let left_off = node_d1(p, off)
    let right_off = node_d2(p, off)

    // Pipe operator: call RHS with LHS as argument
    if op_tok == TK_PIPE() {
        lower_pipe(ir, source, left_off, right_off)
    } else {
        let left_vreg = lower_expr(ir, source, left_off)
        let right_vreg = lower_expr(ir, source, right_off)
        let ir_op = tok_to_binop(op_tok)
        emit_binop(ir, ir_op, left_vreg, right_vreg)
    }
}

fn lower_pipe(ir: i64, source: str, lhs_off: i64, rhs_off: i64) -> i64 {
    // lhs |> rhs  ->  rhs(lhs)
    // RHS should be an identifier (function name)
    let p = ir_p(ir)
    let lhs_vreg = lower_expr(ir, source, lhs_off)

    // RHS should be N_IDENT for a simple pipe
    let rhs_nt = node_type(p, rhs_off)
    if rhs_nt == N_IDENT() {
        let fn_tok = node_d0(p, rhs_off)
        let fn_name = tok_text(source, p_tokens(p), fn_tok)
        let fn_hash = str_hash(fn_name)
        let fn_idx = fn_lookup_ir(ir, fn_hash)

        // Store arg
        let call_args = ir_call_args(ir)
        array_set(call_args, 0, lhs_vreg)
        emit_call(ir, fn_idx, 0, 1)
    } else {
        // Not a simple pipe, just return LHS
        lhs_vreg
    }
}

fn lower_unop(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let op_tok = node_d0(p, off)
    let operand_off = node_d1(p, off)
    let operand_vreg = lower_expr(ir, source, operand_off)

    if op_tok == TK_MINUS() {
        emit_unop(ir, OP_NEG(), operand_vreg)
    } else if op_tok == TK_NOT() {
        emit_unop(ir, OP_NOT(), operand_vreg)
    } else {
        operand_vreg
    }
}

fn lower_call(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let callee_tok = node_d0(p, off)
    let fn_name = tok_text(source, p_tokens(p), callee_tok)
    let fn_hash = str_hash(fn_name)
    let fn_idx = fn_register_ir(ir, fn_hash)
    let args_lstart = node_d1(p, off)
    let arg_count = node_d2(p, off)

    // Lower each argument and store in call args buffer
    let call_args = ir_call_args(ir)
    let mut i = 0
    while i < arg_count {
        let arg_off = list_item(p, args_lstart + i)
        let arg_vreg = lower_expr(ir, source, arg_off)
        array_set(call_args, i, arg_vreg)
        i = i + 1
    }

    emit_call(ir, fn_idx, 0, arg_count)
}

// ============================================================================
// Lower if expression (produces a value)
// ============================================================================
fn lower_if_expr(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    // Lower condition
    let cond_vreg = lower_expr(ir, source, cond_off)

    // Create blocks
    let then_block = new_block(ir)
    let merge_block = new_block(ir)

    if else_off >= 0 {
        let else_block = new_block(ir)
        emit_condbr(ir, cond_vreg, then_block, else_block)

        // Then block
        switch_block(ir, then_block)
        let then_val = lower_block_expr(ir, source, then_off)
        let then_result = emit_copy(ir, then_val)
        emit_br(ir, merge_block)

        // Else block
        switch_block(ir, else_block)
        let else_val = lower_block_or_if(ir, source, else_off)
        let else_result = emit_copy(ir, else_val)
        emit_br(ir, merge_block)

        // Merge block with phi
        switch_block(ir, merge_block)
        let result = alloc_vreg(ir)
        emit_ir(ir, OP_PHI(), result, then_result, else_result, 0, 0)
    } else {
        // No else: void result
        emit_condbr(ir, cond_vreg, then_block, merge_block)

        // Then block
        switch_block(ir, then_block)
        lower_block_stmts(ir, source, then_off)
        emit_br(ir, merge_block)

        // Merge block
        switch_block(ir, merge_block)
        emit_iconst(ir, 0)
    }
}

fn lower_block_or_if(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let nt = node_type(p, off)
    if nt == N_BLOCK() { lower_block_expr(ir, source, off) }
    else if nt == N_IF() { lower_if_expr(ir, source, off) }
    else { lower_expr(ir, source, off) }
}

// ============================================================================
// Lower statements
// ============================================================================

fn lower_stmt(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let nt = node_type(p, off)
    if nt == N_LET() { lower_let(ir, source, off) }
    else if nt == N_ASSIGN() { lower_assign(ir, source, off) }
    else if nt == N_IF() { lower_if_stmt(ir, source, off) }
    else if nt == N_WHILE() { lower_while(ir, source, off) }
    else if nt == N_FOR() { lower_for(ir, source, off) }
    else if nt == N_RETURN() { lower_return(ir, source, off) }
    else if nt == N_BLOCK() { lower_block_stmts(ir, source, off); 0 }
    else if nt == N_EXPR_STMT() { lower_expr(ir, source, node_d0(p, off)); 0 }
    else { lower_expr(ir, source, off); 0 }
}

fn lower_let(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let name_tok = node_d0(p, off)
    let val_off = node_d3(p, off)
    let tokens = p_tokens(p)
    let name = tok_text(source, tokens, name_tok)
    let h = str_hash(name)

    let val_vreg = lower_expr(ir, source, val_off)
    var_set(ir, h, val_vreg)
    0
}

fn lower_assign(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let target_tok = node_d0(p, off)
    let rhs_off = node_d1(p, off)
    let tokens = p_tokens(p)
    let name = tok_text(source, tokens, target_tok)
    let h = str_hash(name)

    let val_vreg = lower_expr(ir, source, rhs_off)
    var_set(ir, h, val_vreg)
    0
}

fn lower_if_stmt(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    let cond_vreg = lower_expr(ir, source, cond_off)

    let then_block = new_block(ir)
    let merge_block = new_block(ir)

    if else_off >= 0 {
        let else_block = new_block(ir)
        emit_condbr(ir, cond_vreg, then_block, else_block)

        switch_block(ir, then_block)
        lower_block_stmts(ir, source, then_off)
        emit_br(ir, merge_block)

        switch_block(ir, else_block)
        let nt = node_type(p, else_off)
        if nt == N_IF() {
            lower_if_stmt(ir, source, else_off)
            0
        } else {
            lower_block_stmts(ir, source, else_off)
            0
        }
        emit_br(ir, merge_block)

        switch_block(ir, merge_block)
        0
    } else {
        emit_condbr(ir, cond_vreg, then_block, merge_block)

        switch_block(ir, then_block)
        lower_block_stmts(ir, source, then_off)
        emit_br(ir, merge_block)

        switch_block(ir, merge_block)
        0
    }
}

fn lower_while(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let cond_off = node_d0(p, off)
    let body_off = node_d1(p, off)

    let cond_block = new_block(ir)
    let body_block = new_block(ir)
    let exit_block = new_block(ir)

    emit_br(ir, cond_block)

    switch_block(ir, cond_block)
    let cond_vreg = lower_expr(ir, source, cond_off)
    emit_condbr(ir, cond_vreg, body_block, exit_block)

    switch_block(ir, body_block)
    lower_block_stmts(ir, source, body_off)
    emit_br(ir, cond_block)

    switch_block(ir, exit_block)
    0
}

fn lower_for(ir: i64, source: str, off: i64) -> i64 {
    // For now, treat for as: let iter_var = 0; while iter_var < range { body; iter_var = iter_var + 1 }
    // This is a simplified version; real for/in range requires more work
    let p = ir_p(ir)
    let var_tok = node_d0(p, off)
    let iter_off = node_d1(p, off)
    let body_off = node_d2(p, off)
    let tokens = p_tokens(p)
    let name = tok_text(source, tokens, var_tok)
    let h = str_hash(name)

    // Lower the iterable (range end)
    let range_vreg = lower_expr(ir, source, iter_off)

    // Init loop var to 0
    let init_vreg = emit_iconst(ir, 0)
    var_set(ir, h, init_vreg)

    let cond_block = new_block(ir)
    let body_block = new_block(ir)
    let exit_block = new_block(ir)

    emit_br(ir, cond_block)

    // Condition: var < range
    switch_block(ir, cond_block)
    let cur_vreg = var_get(ir, h)
    let cond_vreg = emit_binop(ir, OP_LT(), cur_vreg, range_vreg)
    emit_condbr(ir, cond_vreg, body_block, exit_block)

    // Body
    switch_block(ir, body_block)
    lower_block_stmts(ir, source, body_off)
    // Increment
    let inc_cur = var_get(ir, h)
    let one = emit_iconst(ir, 1)
    let next_vreg = emit_binop(ir, OP_ADD(), inc_cur, one)
    var_set(ir, h, next_vreg)
    emit_br(ir, cond_block)

    switch_block(ir, exit_block)
    0
}

fn lower_return(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let val_off = node_d0(p, off)
    if val_off >= 0 {
        let val_vreg = lower_expr(ir, source, val_off)
        emit_ret(ir, val_vreg)
    } else {
        let zero = emit_iconst(ir, 0)
        emit_ret(ir, zero)
    }
}

// ============================================================================
// Block lowering
// ============================================================================

// Lower a block for its statements (no return value)
fn lower_block_stmts(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let save = var_scope_enter(ir)
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        lower_stmt(ir, source, stmt_off)
        i = i + 1
    }
    var_scope_leave(ir, save)
    0
}

// Lower a block and return the value of its last expression
fn lower_block_expr(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let save = var_scope_enter(ir)
    let mut last_vreg = emit_iconst(ir, 0)
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        if i == count - 1 {
            // Last item: treat as expression for value
            let nt = node_type(p, stmt_off)
            if nt == N_EXPR_STMT() {
                last_vreg = lower_expr(ir, source, node_d0(p, stmt_off))
                0
            } else if nt == N_LET() || nt == N_ASSIGN() || nt == N_WHILE() || nt == N_FOR() {
                lower_stmt(ir, source, stmt_off)
                last_vreg = emit_iconst(ir, 0)
                0
            } else {
                last_vreg = lower_expr(ir, source, stmt_off)
                0
            }
        } else {
            lower_stmt(ir, source, stmt_off)
            0
        }
        i = i + 1
    }
    var_scope_leave(ir, save)
    last_vreg
}

// ============================================================================
// Lower a function definition
// ============================================================================
fn lower_function(ir: i64, source: str, fn_off: i64) -> i64 {
    let p = ir_p(ir)
    let tokens = p_tokens(p)
    let name_tok = node_d0(p, fn_off)
    let params_start = node_d1(p, fn_off)
    let param_count = node_d2(p, fn_off)
    let body_off = node_d4(p, fn_off)

    let fn_name = tok_text(source, tokens, name_tok)
    let fn_hash = str_hash(fn_name)

    // Register function
    fn_register_ir(ir, fn_hash)

    // Create entry block
    let entry = new_block(ir)
    switch_block(ir, entry)

    // Record function entry
    let entry_idx = fn_entry_add(ir, fn_hash, entry, param_count)

    // Bind parameters as vregs
    let save = var_scope_enter(ir)
    let mut pi = 0
    while pi < param_count {
        let pn_tok = list_item(p, params_start + pi * 2)
        let pn = tok_text(source, tokens, pn_tok)
        let h = str_hash(pn)
        let param_vreg = alloc_vreg(ir)
        var_set(ir, h, param_vreg)
        pi = pi + 1
    }

    // Lower body
    let body_val = lower_block_expr(ir, source, body_off)
    emit_ret(ir, body_val)

    var_scope_leave(ir, save)

    // Seal the last block and record end
    seal_block(ir, ir_cur_block(ir))
    fn_entry_set_end(ir, entry_idx, ir_cur_block(ir))
    0
}

// ============================================================================
// Lower entire program
// ============================================================================
fn lower_program(ir: i64, source: str, prog_off: i64) -> i64 {
    let p = ir_p(ir)
    let lstart = node_d0(p, prog_off)
    let count = node_d1(p, prog_off)

    // Pre-register all function names
    let tokens = p_tokens(p)
    let mut i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            let name_tok = node_d0(p, fn_off)
            let fn_name = tok_text(source, tokens, name_tok)
            fn_register_ir(ir, str_hash(fn_name))
            0
        } else {
            0
        }
        i = i + 1
    }

    // Lower each function
    i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            lower_function(ir, source, fn_off)
            0
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// IR dump (for testing / debugging)
// ============================================================================
fn op_name(op: i64) -> str {
    if op == OP_NOP() { "NOP" }
    else if op == OP_ICONST() { "ICONST" }
    else if op == OP_SCONST() { "SCONST" }
    else if op == OP_FCONST() { "FCONST" }
    else if op == OP_BCONST() { "BCONST" }
    else if op == OP_ADD() { "ADD" }
    else if op == OP_SUB() { "SUB" }
    else if op == OP_MUL() { "MUL" }
    else if op == OP_DIV() { "DIV" }
    else if op == OP_MOD() { "MOD" }
    else if op == OP_NEG() { "NEG" }
    else if op == OP_EQ() { "EQ" }
    else if op == OP_NEQ() { "NEQ" }
    else if op == OP_LT() { "LT" }
    else if op == OP_GT() { "GT" }
    else if op == OP_LTE() { "LTE" }
    else if op == OP_GTE() { "GTE" }
    else if op == OP_AND() { "AND" }
    else if op == OP_OR() { "OR" }
    else if op == OP_NOT() { "NOT" }
    else if op == OP_CALL() { "CALL" }
    else if op == OP_RET() { "RET" }
    else if op == OP_BR() { "BR" }
    else if op == OP_CONDBR() { "CONDBR" }
    else if op == OP_COPY() { "COPY" }
    else if op == OP_PHI() { "PHI" }
    else { "???" }
}

fn dump_ir(ir: i64) -> i64 {
    let insts = ir_insts(ir)
    let count = ir_inst_count(ir)
    println_str("--- IR dump ---")
    let mut i = 0
    while i < count {
        let off = i * IR_STRIDE()
        let op = array_get(insts, off)
        let dest = array_get(insts, off + 1)
        let a0 = array_get(insts, off + 2)
        let a1 = array_get(insts, off + 3)
        let a2 = array_get(insts, off + 4)

        print_str("  ")
        print_str(to_string_i64(i))
        print_str(": ")

        if op == OP_ICONST() || op == OP_BCONST() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = ")
            print_str(op_name(op))
            print_str(" ")
            println_str(to_string_i64(a0))
        } else if op == OP_SCONST() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = SCONST #")
            println_str(to_string_i64(a0))
        } else if op == OP_ADD() || op == OP_SUB() || op == OP_MUL() || op == OP_DIV() || op == OP_MOD() || op == OP_EQ() || op == OP_NEQ() || op == OP_LT() || op == OP_GT() || op == OP_LTE() || op == OP_GTE() || op == OP_AND() || op == OP_OR() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = ")
            print_str(op_name(op))
            print_str(" v")
            print_str(to_string_i64(a0))
            print_str(", v")
            println_str(to_string_i64(a1))
        } else if op == OP_NEG() || op == OP_NOT() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = ")
            print_str(op_name(op))
            print_str(" v")
            println_str(to_string_i64(a0))
        } else if op == OP_COPY() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = COPY v")
            println_str(to_string_i64(a0))
        } else if op == OP_CALL() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = CALL fn#")
            print_str(to_string_i64(a0))
            print_str(" (")
            print_str(to_string_i64(a2))
            println_str(" args)")
        } else if op == OP_RET() {
            print_str("RET v")
            println_str(to_string_i64(a0))
        } else if op == OP_BR() {
            print_str("BR b")
            println_str(to_string_i64(a0))
        } else if op == OP_CONDBR() {
            print_str("CONDBR v")
            print_str(to_string_i64(a0))
            print_str(" ? b")
            print_str(to_string_i64(a1))
            print_str(" : b")
            println_str(to_string_i64(a2))
        } else if op == OP_PHI() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = PHI v")
            print_str(to_string_i64(a0))
            print_str(", v")
            println_str(to_string_i64(a1))
        } else {
            print_str(op_name(op))
            println_str("")
        }

        i = i + 1
    }
    println_str("--- end IR ---")
    0
}

// ============================================================================
// Entry point: create IR context and lower program
// ============================================================================
fn ir_generate(p: i64, source: str, prog_off: i64) -> i64 {
    let ir = array_new(17)
    array_set(ir, 0, p)
    array_set(ir, 1, array_new(20000))   // instructions
    array_set(ir, 2, 0)                   // inst_count
    array_set(ir, 3, 0)                   // next_vreg
    array_set(ir, 4, array_new(2000))     // blocks
    array_set(ir, 5, 0)                   // block_count
    array_set(ir, 6, 0)                   // current_block
    array_set(ir, 7, array_new(500))      // strings
    array_set(ir, 8, 0)                   // string_count
    array_set(ir, 9, array_new(500))      // fn_table
    array_set(ir, 10, 0)                  // fn_table_count
    array_set(ir, 11, array_new(2000))    // vars
    array_set(ir, 12, 0)                  // var_count
    array_set(ir, 13, 0)                  // var_depth
    array_set(ir, 14, array_new(32))      // call_args
    array_set(ir, 15, array_new(500))     // fn_entries
    array_set(ir, 16, 0)                  // fn_entry_count

    // Create initial block (block 0)
    new_block(ir)

    lower_program(ir, source, prog_off)
    ir
}

// ====== IR GEN TESTS ======
// ============================================================================
// IR Generation test harness
// ============================================================================

// ---- Helper: lex + parse + generate IR ----
fn ir_gen(source: str) -> i64 {
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let ir = ir_generate(p, source, prog)
    ir
}

// ---- Tests ----

fn test_ir_1() -> i64 {
    // Simple function returning constant
    let ir = ir_gen("fn main() -> i64 { 42 }")
    let count = ir_inst_count(ir)
    // Should have at least: ICONST 0 (default), ICONST 42, RET
    assert_true(count >= 2)
    // Find ICONST 42 in instructions
    let insts = ir_insts(ir)
    let mut found42 = 0
    let mut i = 0
    while i < count {
        let off = i * IR_STRIDE()
        let op = array_get(insts, off)
        let val = array_get(insts, off + 2)
        if op == OP_ICONST() && val == 42 {
            found42 = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found42, 1)
    println_str("IR Test 1 passed: constant return")
    0
}

fn test_ir_2() -> i64 {
    // Addition: 10 + 20
    let ir = ir_gen("fn f() -> i64 { 10 + 20 }")
    let count = ir_inst_count(ir)
    assert_true(count >= 3)
    // Find an ADD instruction
    let insts = ir_insts(ir)
    let mut found_add = 0
    let mut i = 0
    while i < count {
        let op = array_get(insts, i * IR_STRIDE())
        if op == OP_ADD() {
            found_add = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found_add, 1)
    println_str("IR Test 2 passed: addition")
    0
}

fn test_ir_3() -> i64 {
    // Let binding
    let ir = ir_gen("fn f() -> i64 { let x = 5; x }")
    let count = ir_inst_count(ir)
    assert_true(count >= 2)
    println_str("IR Test 3 passed: let binding")
    0
}

fn test_ir_4() -> i64 {
    // Function call
    let ir = ir_gen("fn add(a: i64, b: i64) -> i64 { a + b } fn main() -> i64 { add(1, 2) }")
    let count = ir_inst_count(ir)
    // Should have instructions for both functions
    assert_true(count >= 4)
    // Find a CALL instruction
    let insts = ir_insts(ir)
    let mut found_call = 0
    let mut i = 0
    while i < count {
        let op = array_get(insts, i * IR_STRIDE())
        if op == OP_CALL() {
            found_call = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found_call, 1)
    println_str("IR Test 4 passed: function call")
    0
}

fn test_ir_5() -> i64 {
    // If/else expression
    let ir = ir_gen("fn f(x: i64) -> i64 { if x > 0 { 1 } else { 0 } }")
    let count = ir_inst_count(ir)
    // Should have CONDBR
    let insts = ir_insts(ir)
    let mut found_condbr = 0
    let mut i = 0
    while i < count {
        let op = array_get(insts, i * IR_STRIDE())
        if op == OP_CONDBR() {
            found_condbr = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found_condbr, 1)
    println_str("IR Test 5 passed: if/else")
    0
}

fn test_ir_6() -> i64 {
    // While loop
    let ir = ir_gen("fn f() -> i64 { let mut x = 10; while x > 0 { x = x - 1 }; 0 }")
    let count = ir_inst_count(ir)
    // Should have BR + CONDBR for loop structure
    let insts = ir_insts(ir)
    let mut found_br = 0
    let mut found_condbr = 0
    let mut i = 0
    while i < count {
        let op = array_get(insts, i * IR_STRIDE())
        if op == OP_BR() {
            found_br = 1
            0
        } else if op == OP_CONDBR() {
            found_condbr = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found_br, 1)
    assert_eq(found_condbr, 1)
    println_str("IR Test 6 passed: while loop")
    0
}

fn test_ir_7() -> i64 {
    // Comparison operator produces bool
    let ir = ir_gen("fn f(a: i64, b: i64) -> i64 { if a == b { 1 } else { 0 } }")
    let count = ir_inst_count(ir)
    // Should have EQ instruction
    let insts = ir_insts(ir)
    let mut found_eq = 0
    let mut i = 0
    while i < count {
        let op = array_get(insts, i * IR_STRIDE())
        if op == OP_EQ() {
            found_eq = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found_eq, 1)
    println_str("IR Test 7 passed: comparison")
    0
}

fn test_ir_8() -> i64 {
    // IR dump smoke test
    let ir = ir_gen("fn add(a: i64, b: i64) -> i64 { a + b }")
    dump_ir(ir)
    println_str("IR Test 8 passed: IR dump")
    0
}

fn test_ir_9() -> i64 {
    // Unary negation
    let ir = ir_gen("fn f(x: i64) -> i64 { -x }")
    let count = ir_inst_count(ir)
    let insts = ir_insts(ir)
    let mut found_neg = 0
    let mut i = 0
    while i < count {
        let op = array_get(insts, i * IR_STRIDE())
        if op == OP_NEG() {
            found_neg = 1
            0
        } else {
            0
        }
        i = i + 1
    }
    assert_eq(found_neg, 1)
    println_str("IR Test 9 passed: unary neg")
    0
}

fn test_ir_10() -> i64 {
    // Multiple functions
    let ir = ir_gen("fn f() -> i64 { 1 } fn g() -> i64 { 2 } fn h() -> i64 { 3 }")
    let fn_count = ir_fn_entry_count(ir)
    assert_eq(fn_count, 3)
    println_str("IR Test 10 passed: multiple functions")
    0
}

fn main() -> i64 {
    test_ir_1()
    test_ir_2()
    test_ir_3()
    test_ir_4()
    test_ir_5()
    test_ir_6()
    test_ir_7()
    test_ir_8()
    test_ir_9()
    test_ir_10()
    println_str("All IR generation tests passed!")
    0
}
