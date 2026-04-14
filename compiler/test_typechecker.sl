// ====== COMBINED: Lexer + Parser + TypeChecker + Tests ======

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


// ====== TYPE CHECKER ======
// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 3: Type Checker
// ============================================================================
// Walks the AST (from parser.sl), resolves variable names, checks types.
//
// Type representation: i64 constants
//   TY_I64=1, TY_F64=2, TY_BOOL=3, TY_STR=4, TY_VOID=5, TY_ERROR=99
//
// Symbol table: flat array, scope-based with push/pop
//   Each entry: [name_hash, type, scope_depth, mutable]  stride=4
//   Lookup scans backwards to find innermost binding.
//
// Function table: flat array
//   Each entry: [name_hash, return_type, param_count, p0_type, p1_type, ...]
//   Max 8 params per function, stride=11
//
// NOTE: No `return` in if-blocks. All control flow uses if/else expressions.
// ============================================================================

// -- Type constants --
fn TY_I64() -> i64   { 1 }
fn TY_F64() -> i64   { 2 }
fn TY_BOOL() -> i64  { 3 }
fn TY_STR() -> i64   { 4 }
fn TY_VOID() -> i64  { 5 }
fn TY_ERROR() -> i64 { 99 }

// -- AST node type & token constants are provided by lexer.sl/parser.sl
// -- in combined builds. See lexer.sl for TK_* and parser.sl for N_*

// ============================================================================
// String hashing (djb2) for symbol table keys
// ============================================================================
fn str_hash(s: str) -> i64 {
    let slen = str_len(s)
    let mut h = 5381
    let mut i = 0
    while i < slen {
        let c = char_to_int(str_char_at(s, i))
        h = h * 33 + c
        // Keep in positive 32-bit range
        h = h % 2147483647
        i = i + 1
    }
    h
}

// ============================================================================
// Type checker context (tc)
// ============================================================================
// tc[0] = parser context (p)
// tc[1] = source string (stored as i64 - token text lookup via parser)
// tc[2] = symbol table array
// tc[3] = sym_count
// tc[4] = scope_depth
// tc[5] = function table array
// tc[6] = fn_count
// tc[7] = error_count
// tc[8] = current_fn_return_type

fn SYM_STRIDE() -> i64 { 4 }
fn FN_STRIDE() -> i64  { 11 }

fn tc_p(tc: i64) -> i64          { array_get(tc, 0) }
fn tc_syms(tc: i64) -> i64       { array_get(tc, 2) }
fn tc_sym_count(tc: i64) -> i64  { array_get(tc, 3) }
fn tc_depth(tc: i64) -> i64      { array_get(tc, 4) }
fn tc_fns(tc: i64) -> i64        { array_get(tc, 5) }
fn tc_fn_count(tc: i64) -> i64   { array_get(tc, 6) }
fn tc_errors(tc: i64) -> i64     { array_get(tc, 7) }
fn tc_ret_type(tc: i64) -> i64   { array_get(tc, 8) }

fn tc_set_sym_count(tc: i64, v: i64) -> i64  { array_set(tc, 3, v); 0 }
fn tc_set_depth(tc: i64, v: i64) -> i64      { array_set(tc, 4, v); 0 }
fn tc_set_fn_count(tc: i64, v: i64) -> i64   { array_set(tc, 6, v); 0 }
fn tc_set_errors(tc: i64, v: i64) -> i64     { array_set(tc, 7, v); 0 }
fn tc_set_ret_type(tc: i64, v: i64) -> i64   { array_set(tc, 8, v); 0 }

// ============================================================================
// Type name resolution: "i64" -> TY_I64, etc.
// ============================================================================
fn resolve_type_name(name: str) -> i64 {
    if str_eq(name, "i64") { TY_I64() }
    else if str_eq(name, "f64") { TY_F64() }
    else if str_eq(name, "bool") { TY_BOOL() }
    else if str_eq(name, "str") { TY_STR() }
    else if str_eq(name, "void") { TY_VOID() }
    else { TY_ERROR() }
}

fn type_name(t: i64) -> str {
    if t == TY_I64() { "i64" }
    else if t == TY_F64() { "f64" }
    else if t == TY_BOOL() { "bool" }
    else if t == TY_STR() { "str" }
    else if t == TY_VOID() { "void" }
    else { "error" }
}

// ============================================================================
// Node accessors - use parser's definitions when combined.
// Standalone versions provided here for reference:
//   node_type, node_d0..d4, list_item, p_tokens, tok_text
// In combined builds, these are provided by parser.sl
// ============================================================================
// (Removed to avoid duplicates in combined builds - parser.sl provides these)

// ============================================================================
// Symbol table operations
// ============================================================================

// Push a variable into the symbol table
fn sym_push(tc: i64, name: str, ty: i64, mutable: i64) -> i64 {
    let syms = tc_syms(tc)
    let cnt = tc_sym_count(tc)
    let off = cnt * SYM_STRIDE()
    let h = str_hash(name)
    array_set(syms, off + 0, h)
    array_set(syms, off + 1, ty)
    array_set(syms, off + 2, tc_depth(tc))
    array_set(syms, off + 3, mutable)
    tc_set_sym_count(tc, cnt + 1)
    0
}

// Lookup a variable by name, return its type (or TY_ERROR if not found)
fn sym_lookup(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let syms = tc_syms(tc)
    let cnt = tc_sym_count(tc)
    let mut i = cnt - 1
    let mut result = TY_ERROR()
    let mut found = false
    while i >= 0 && !found {
        let off = i * SYM_STRIDE()
        if array_get(syms, off) == h {
            result = array_get(syms, off + 1)
            found = true
            0
        } else {
            i = i - 1
        }
    }
    result
}

// Check if a variable is mutable
fn sym_is_mutable(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let syms = tc_syms(tc)
    let cnt = tc_sym_count(tc)
    let mut i = cnt - 1
    let mut result = 0
    let mut found = 0
    while i >= 0 && found == 0 {
        let off = i * SYM_STRIDE()
        if array_get(syms, off) == h {
            result = array_get(syms, off + 3)
            found = 1
            0
        } else {
            i = i - 1
        }
    }
    result
}

// Enter a new scope
fn scope_enter(tc: i64) -> i64 {
    tc_set_depth(tc, tc_depth(tc) + 1)
    tc_sym_count(tc)
}

// Leave a scope, pop symbols to restore point
fn scope_leave(tc: i64, restore_point: i64) -> i64 {
    tc_set_depth(tc, tc_depth(tc) - 1)
    tc_set_sym_count(tc, restore_point)
    0
}

// ============================================================================
// Function table operations
// ============================================================================

// Register a function: name, return_type, param_count, param_types...
fn fn_register(tc: i64, name: str, ret_type: i64, param_count: i64, param_types: i64) -> i64 {
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let off = cnt * FN_STRIDE()
    let h = str_hash(name)
    array_set(fns, off + 0, h)
    array_set(fns, off + 1, ret_type)
    array_set(fns, off + 2, param_count)
    // Copy param types from array
    let mut i = 0
    while i < param_count && i < 8 {
        array_set(fns, off + 3 + i, array_get(param_types, i))
        i = i + 1
    }
    tc_set_fn_count(tc, cnt + 1)
    0
}

// Lookup function return type by name
fn fn_lookup_ret(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let mut i = 0
    let mut result = TY_ERROR()
    let mut found = false
    while i < cnt && !found {
        let off = i * FN_STRIDE()
        if array_get(fns, off) == h {
            result = array_get(fns, off + 1)
            found = true
            0
        } else {
            i = i + 1
        }
    }
    result
}

// Lookup function param count
fn fn_lookup_param_count(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let mut i = 0
    let mut result = -1
    let mut found = false
    while i < cnt && !found {
        let off = i * FN_STRIDE()
        if array_get(fns, off) == h {
            result = array_get(fns, off + 2)
            found = true
            0
        } else {
            i = i + 1
        }
    }
    result
}

// Lookup function param type at index
fn fn_lookup_param_type(tc: i64, name: str, pidx: i64) -> i64 {
    let h = str_hash(name)
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let mut i = 0
    let mut result = TY_ERROR()
    let mut found = false
    while i < cnt && !found {
        let off = i * FN_STRIDE()
        if array_get(fns, off) == h {
            result = array_get(fns, off + 3 + pidx)
            found = true
            0
        } else {
            i = i + 1
        }
    }
    result
}

// ============================================================================
// Error reporting
// ============================================================================
fn tc_error(tc: i64, msg: str) -> i64 {
    print_str("Type error: ")
    println_str(msg)
    tc_set_errors(tc, tc_errors(tc) + 1)
    0
}

fn tc_error2(tc: i64, msg: str, detail: str) -> i64 {
    print_str("Type error: ")
    print_str(msg)
    print_str(" '")
    print_str(detail)
    println_str("'")
    tc_set_errors(tc, tc_errors(tc) + 1)
    0
}

// ============================================================================
// Register built-in functions (stdlib)
// ============================================================================
fn register_builtins(tc: i64) -> i64 {
    let pt = array_new(8)

    // print(i64) -> void
    array_set(pt, 0, TY_I64())
    fn_register(tc, "print", TY_VOID(), 1, pt)

    // println(i64) -> void
    fn_register(tc, "println", TY_VOID(), 1, pt)

    // print_str(str) -> void
    array_set(pt, 0, TY_STR())
    fn_register(tc, "print_str", TY_VOID(), 1, pt)

    // println_str(str) -> void
    fn_register(tc, "println_str", TY_VOID(), 1, pt)

    // str_len(str) -> i64
    fn_register(tc, "str_len", TY_I64(), 1, pt)

    // str_eq(str, str) -> bool
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_STR())
    fn_register(tc, "str_eq", TY_BOOL(), 2, pt)

    // str_cat(str, str) -> str
    fn_register(tc, "str_cat", TY_STR(), 2, pt)

    // str_char_at(str, i64) -> str
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "str_char_at", TY_STR(), 2, pt)

    // str_substr(str, i64, i64) -> str
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_I64())
    array_set(pt, 2, TY_I64())
    fn_register(tc, "str_substr", TY_STR(), 3, pt)

    // str_contains(str, str) -> bool
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_STR())
    fn_register(tc, "str_contains", TY_BOOL(), 2, pt)

    // str_index_of(str, str) -> i64
    fn_register(tc, "str_index_of", TY_I64(), 2, pt)

    // char_to_int(str) -> i64
    array_set(pt, 0, TY_STR())
    fn_register(tc, "char_to_int", TY_I64(), 1, pt)

    // int_to_char(i64) -> str
    array_set(pt, 0, TY_I64())
    fn_register(tc, "int_to_char", TY_STR(), 1, pt)

    // array_new(i64) -> i64
    fn_register(tc, "array_new", TY_I64(), 1, pt)

    // array_len(i64) -> i64
    fn_register(tc, "array_len", TY_I64(), 1, pt)

    // array_get(i64, i64) -> i64
    array_set(pt, 0, TY_I64())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "array_get", TY_I64(), 2, pt)

    // array_set(i64, i64, i64) -> void
    array_set(pt, 2, TY_I64())
    fn_register(tc, "array_set", TY_VOID(), 3, pt)

    // assert_eq(i64, i64) -> void
    array_set(pt, 0, TY_I64())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "assert_eq", TY_VOID(), 2, pt)

    // assert_true(bool) -> void
    array_set(pt, 0, TY_BOOL())
    fn_register(tc, "assert_true", TY_VOID(), 1, pt)

    // exit(i64) -> void
    array_set(pt, 0, TY_I64())
    fn_register(tc, "exit", TY_VOID(), 1, pt)

    // to_string_i64(i64) -> str
    fn_register(tc, "to_string_i64", TY_STR(), 1, pt)

    // str_starts_with(str, str) -> bool
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_STR())
    fn_register(tc, "str_starts_with", TY_BOOL(), 2, pt)

    // str_split_count(str, str) -> i64
    fn_register(tc, "str_split_count", TY_I64(), 2, pt)

    // str_split_get(str, str, i64) -> str
    array_set(pt, 2, TY_I64())
    fn_register(tc, "str_split_get", TY_STR(), 3, pt)

    // abs(i64) -> i64
    array_set(pt, 0, TY_I64())
    fn_register(tc, "abs", TY_I64(), 1, pt)

    // min(i64, i64) -> i64
    array_set(pt, 1, TY_I64())
    fn_register(tc, "min", TY_I64(), 2, pt)

    // max(i64, i64) -> i64
    fn_register(tc, "max", TY_I64(), 2, pt)

    0
}

// ============================================================================
// Pre-registration pass: scan all fn definitions to register signatures
// ============================================================================
fn preregister_functions(tc: i64, source: str, prog_off: i64) -> i64 {
    let p = tc_p(tc)
    let lstart = node_d0(p, prog_off)
    let count = node_d1(p, prog_off)
    let tokens = p_tokens(p)
    let pt = array_new(8)
    let mut i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            let name_tok = node_d0(p, fn_off)
            let name = tok_text(source, tokens, name_tok)
            let param_count = node_d2(p, fn_off)
            let params_start = node_d1(p, fn_off)
            let ret_tok = node_d3(p, fn_off)

            // Resolve return type
            let mut ret_type = TY_VOID()
            if ret_tok >= 0 {
                ret_type = resolve_type_name(tok_text(source, tokens, ret_tok))
            } else {
                ret_type = TY_VOID()
            }

            // Resolve param types
            let mut pi = 0
            while pi < param_count && pi < 8 {
                let pt_tok = list_item(p, params_start + pi * 2 + 1)
                let pt_name = tok_text(source, tokens, pt_tok)
                array_set(pt, pi, resolve_type_name(pt_name))
                pi = pi + 1
            }
            fn_register(tc, name, ret_type, param_count, pt)
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// Expression type checking
// ============================================================================

fn check_expr(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let nt = node_type(p, off)
    if nt == N_INT() { TY_I64() }
    else if nt == N_FLOAT() { TY_F64() }
    else if nt == N_STR() { TY_STR() }
    else if nt == N_BOOL() { TY_BOOL() }
    else if nt == N_IDENT() { check_ident(tc, source, off) }
    else if nt == N_BINOP() { check_binop(tc, source, off) }
    else if nt == N_UNOP() { check_unop(tc, source, off) }
    else if nt == N_CALL() { check_call(tc, source, off) }
    else if nt == N_IF() { check_if_expr(tc, source, off) }
    else { TY_ERROR() }
}

fn check_ident(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let tok_idx = node_d0(p, off)
    let name = tok_text(source, p_tokens(p), tok_idx)
    let ty = sym_lookup(tc, name)
    if ty == TY_ERROR() {
        tc_error2(tc, "undefined variable", name)
        TY_ERROR()
    } else {
        ty
    }
}

fn check_binop(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let op = node_d0(p, off)
    let left_off = node_d1(p, off)
    let right_off = node_d2(p, off)
    let lt = check_expr(tc, source, left_off)
    let rt = check_expr(tc, source, right_off)

    // Pipe operator: special - result type is the return type of the RHS function
    if op == TK_PIPE() {
        rt
    }
    // Comparison operators: operands must be numeric, result is bool
    else if op == TK_EQEQ() || op == TK_NEQ() || op == TK_LT() || op == TK_GT() || op == TK_LTE() || op == TK_GTE() {
        if lt != rt && lt != TY_ERROR() && rt != TY_ERROR() {
            tc_error(tc, "comparison operands must have same type")
            TY_ERROR()
        } else {
            TY_BOOL()
        }
    }
    // Logical operators: operands must be bool
    else if op == TK_AND() || op == TK_OR() {
        if lt != TY_BOOL() && lt != TY_ERROR() {
            tc_error(tc, "&& / || requires bool operands")
            TY_ERROR()
        } else if rt != TY_BOOL() && rt != TY_ERROR() {
            tc_error(tc, "&& / || requires bool operands")
            TY_ERROR()
        } else {
            TY_BOOL()
        }
    }
    // Arithmetic operators: operands must be numeric
    else if op == TK_PLUS() || op == TK_MINUS() || op == TK_STAR() || op == TK_SLASH() || op == TK_PERCENT() {
        if lt == TY_STR() && op == TK_PLUS() {
            // String concatenation via +
            TY_STR()
        } else if (lt == TY_I64() || lt == TY_F64()) && (rt == TY_I64() || rt == TY_F64()) {
            // If either is f64, result is f64
            if lt == TY_F64() || rt == TY_F64() { TY_F64() }
            else { TY_I64() }
        } else if lt == TY_ERROR() || rt == TY_ERROR() {
            TY_ERROR()
        } else {
            tc_error(tc, "arithmetic requires numeric operands")
            TY_ERROR()
        }
    } else {
        TY_ERROR()
    }
}

fn check_unop(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let op = node_d0(p, off)
    let operand_off = node_d1(p, off)
    let ot = check_expr(tc, source, operand_off)

    if op == TK_MINUS() {
        if ot == TY_I64() || ot == TY_F64() { ot }
        else if ot == TY_ERROR() { TY_ERROR() }
        else { tc_error(tc, "unary - requires numeric operand"); TY_ERROR() }
    } else if op == TK_NOT() {
        if ot == TY_BOOL() { TY_BOOL() }
        else if ot == TY_ERROR() { TY_ERROR() }
        else { tc_error(tc, "! requires bool operand"); TY_ERROR() }
    } else {
        TY_ERROR()
    }
}

fn check_call(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let callee_tok = node_d0(p, off)
    let name = tok_text(source, p_tokens(p), callee_tok)
    let lstart = node_d1(p, off)
    let arg_count = node_d2(p, off)

    let ret = fn_lookup_ret(tc, name)
    if ret == TY_ERROR() {
        tc_error2(tc, "undefined function", name)
        TY_ERROR()
    } else {
        // Check argument count
        let expected_params = fn_lookup_param_count(tc, name)
        if expected_params >= 0 && arg_count != expected_params {
            tc_error2(tc, "wrong number of arguments for", name)
            TY_ERROR()
        } else {
            // Check argument types
            check_call_args(tc, source, name, lstart, arg_count)
            ret
        }
    }
}

fn check_call_args(tc: i64, source: str, name: str, lstart: i64, arg_count: i64) -> i64 {
    let p = tc_p(tc)
    let mut i = 0
    while i < arg_count {
        let arg_off = list_item(p, lstart + i)
        let arg_ty = check_expr(tc, source, arg_off)
        let expected = fn_lookup_param_type(tc, name, i)
        if arg_ty != expected && arg_ty != TY_ERROR() && expected != TY_ERROR() {
            // Allow i64 where f64 expected (implicit promotion for now)
            if expected == TY_F64() && arg_ty == TY_I64() {
                0
            } else {
                tc_error2(tc, "argument type mismatch in call to", name)
                0
            }
        } else {
            0
        }
        i = i + 1
    }
    0
}

// If-expression type: both branches must agree
fn check_if_expr(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    let ct = check_expr(tc, source, cond_off)
    if ct != TY_BOOL() && ct != TY_ERROR() {
        tc_error(tc, "if condition must be bool")
        0
    } else {
        0
    }

    let tt = check_block_type(tc, source, then_off)

    if else_off >= 0 {
        let et = check_block_or_if_type(tc, source, else_off)
        // Both branches should yield same type
        if tt != et && tt != TY_ERROR() && et != TY_ERROR() && tt != TY_VOID() && et != TY_VOID() {
            tc_error(tc, "if/else branches have different types")
            TY_ERROR()
        } else {
            tt
        }
    } else {
        TY_VOID()
    }
}

fn check_block_or_if_type(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let nt = node_type(p, off)
    if nt == N_BLOCK() { check_block_type(tc, source, off) }
    else if nt == N_IF() { check_if_expr(tc, source, off) }
    else { check_expr(tc, source, off) }
}

// ============================================================================
// Statement type checking
// ============================================================================

fn check_stmt(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let nt = node_type(p, off)
    if nt == N_LET() { check_let(tc, source, off) }
    else if nt == N_ASSIGN() { check_assign(tc, source, off) }
    else if nt == N_IF() { check_if_stmt(tc, source, off) }
    else if nt == N_WHILE() { check_while(tc, source, off) }
    else if nt == N_FOR() { check_for(tc, source, off) }
    else if nt == N_RETURN() { check_return(tc, source, off) }
    else if nt == N_BLOCK() { check_block(tc, source, off); TY_VOID() }
    else if nt == N_EXPR_STMT() { check_expr(tc, source, node_d0(p, off)); TY_VOID() }
    else if nt == N_BREAK() || nt == N_CONTINUE() { TY_VOID() }
    else { check_expr(tc, source, off) }
}

// Let: infer type from RHS, push to symbol table
fn check_let(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let name_tok = node_d0(p, off)
    let mutable = node_d1(p, off)
    let type_tok = node_d2(p, off)
    let val_off = node_d3(p, off)
    let tokens = p_tokens(p)

    let name = tok_text(source, tokens, name_tok)

    // Check RHS type
    let rhs_type = check_expr(tc, source, val_off)

    // If explicit type annotation, check it matches
    let mut final_type = rhs_type
    if type_tok >= 0 {
        let ann_type = resolve_type_name(tok_text(source, tokens, type_tok))
        if ann_type != rhs_type && rhs_type != TY_ERROR() && ann_type != TY_ERROR() {
            tc_error2(tc, "type annotation mismatch for", name)
            final_type = ann_type
        } else {
            final_type = ann_type
        }
    } else {
        final_type = rhs_type
    }

    sym_push(tc, name, final_type, mutable)
    TY_VOID()
}

// Assignment: check target is mutable and types match
fn check_assign(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let target_tok = node_d0(p, off)
    let rhs_off = node_d1(p, off)
    let tokens = p_tokens(p)

    let name = tok_text(source, tokens, target_tok)
    let var_type = sym_lookup(tc, name)

    if var_type == TY_ERROR() {
        tc_error2(tc, "undefined variable in assignment", name)
        TY_ERROR()
    } else {
        if sym_is_mutable(tc, name) == 0 {
            tc_error2(tc, "cannot assign to immutable variable", name)
            0
        } else {
            0
        }
        let rhs_type = check_expr(tc, source, rhs_off)
        if rhs_type != var_type && rhs_type != TY_ERROR() && var_type != TY_ERROR() {
            tc_error2(tc, "assignment type mismatch for", name)
            TY_ERROR()
        } else {
            TY_VOID()
        }
    }
}

// If statement (no value needed)
fn check_if_stmt(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    let ct = check_expr(tc, source, cond_off)
    if ct != TY_BOOL() && ct != TY_ERROR() {
        tc_error(tc, "if condition must be bool")
        0
    } else {
        0
    }

    check_block(tc, source, then_off)

    if else_off >= 0 {
        let nt = node_type(p, else_off)
        if nt == N_IF() { check_if_stmt(tc, source, else_off) }
        else { check_block(tc, source, else_off) }
    } else {
        0
    }
    TY_VOID()
}

// While loop
fn check_while(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let cond_off = node_d0(p, off)
    let body_off = node_d1(p, off)

    let ct = check_expr(tc, source, cond_off)
    if ct != TY_BOOL() && ct != TY_ERROR() {
        tc_error(tc, "while condition must be bool")
        0
    } else {
        0
    }

    check_block(tc, source, body_off)
    TY_VOID()
}

// For loop
fn check_for(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let var_tok = node_d0(p, off)
    let iter_off = node_d1(p, off)
    let body_off = node_d2(p, off)
    let tokens = p_tokens(p)

    let name = tok_text(source, tokens, var_tok)
    let iter_type = check_expr(tc, source, iter_off)

    let save = scope_enter(tc)
    // For now, loop variable is i64
    sym_push(tc, name, TY_I64(), 0)
    check_block(tc, source, body_off)
    scope_leave(tc, save)
    TY_VOID()
}

// Return statement
fn check_return(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let val_off = node_d0(p, off)
    let expected = tc_ret_type(tc)

    if val_off >= 0 {
        let rt = check_expr(tc, source, val_off)
        if rt != expected && rt != TY_ERROR() && expected != TY_ERROR() {
            tc_error(tc, "return type mismatch")
            0
        } else {
            0
        }
    } else {
        if expected != TY_VOID() && expected != TY_ERROR() {
            tc_error(tc, "missing return value")
            0
        } else {
            0
        }
    }
    TY_VOID()
}

// ============================================================================
// Block checking
// ============================================================================

// Check a block, return void (used for statement blocks)
fn check_block(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let save = scope_enter(tc)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        check_stmt(tc, source, stmt_off)
        i = i + 1
    }
    scope_leave(tc, save)
    0
}

// Check a block and return the type of its last expression (for if-expr)
fn check_block_type(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let save = scope_enter(tc)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let mut last_type = TY_VOID()
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        last_type = check_stmt(tc, source, stmt_off)
        i = i + 1
    }
    scope_leave(tc, save)
    last_type
}

// ============================================================================
// Function checking
// ============================================================================
fn check_function(tc: i64, source: str, fn_off: i64) -> i64 {
    let p = tc_p(tc)
    let tokens = p_tokens(p)
    let name_tok = node_d0(p, fn_off)
    let params_start = node_d1(p, fn_off)
    let param_count = node_d2(p, fn_off)
    let ret_tok = node_d3(p, fn_off)
    let body_off = node_d4(p, fn_off)

    let name = tok_text(source, tokens, name_tok)

    // Set current return type for return-statement checking
    let mut ret_type = TY_VOID()
    if ret_tok >= 0 {
        ret_type = resolve_type_name(tok_text(source, tokens, ret_tok))
    } else {
        ret_type = TY_VOID()
    }
    tc_set_ret_type(tc, ret_type)

    // Enter function scope and bind parameters
    let save = scope_enter(tc)
    let mut pi = 0
    while pi < param_count {
        let pn_tok = list_item(p, params_start + pi * 2)
        let pt_tok = list_item(p, params_start + pi * 2 + 1)
        let pn = tok_text(source, tokens, pn_tok)
        let pt = resolve_type_name(tok_text(source, tokens, pt_tok))
        sym_push(tc, pn, pt, 0)
        pi = pi + 1
    }

    // Type-check the body
    check_block(tc, source, body_off)
    scope_leave(tc, save)
    0
}

// ============================================================================
// Top-level: check whole program
// ============================================================================
fn check_program(tc: i64, source: str, prog_off: i64) -> i64 {
    let p = tc_p(tc)
    let lstart = node_d0(p, prog_off)
    let count = node_d1(p, prog_off)
    let mut i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            check_function(tc, source, fn_off)
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// Entry point
// ============================================================================
fn typecheck(p: i64, source: str, prog_off: i64) -> i64 {
    let tc = array_new(9)
    array_set(tc, 0, p)
    array_set(tc, 1, 0)
    array_set(tc, 2, array_new(4000))
    array_set(tc, 3, 0)
    array_set(tc, 4, 0)
    array_set(tc, 5, array_new(2000))
    array_set(tc, 6, 0)
    array_set(tc, 7, 0)
    array_set(tc, 8, TY_VOID())

    register_builtins(tc)
    preregister_functions(tc, source, prog_off)
    check_program(tc, source, prog_off)
    tc
}

// ====== TYPE CHECKER TESTS ======
// ============================================================================
// Type Checker test harness - combines lexer + parser + typechecker
// ============================================================================

// ---- Helper: lex + parse + typecheck a source string ----
fn tc_check(source: str) -> i64 {
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let tc = typecheck(p, source, prog)
    tc
}

fn tc_check_errors(source: str) -> i64 {
    let tc = tc_check(source)
    tc_errors(tc)
}

// ---- Tests ----

fn test_1() -> i64 {
    // Well-typed: simple function
    let errs = tc_check_errors("fn main() -> i64 { 42 }")
    assert_eq(errs, 0)
    println_str("TC Test 1 passed: simple fn no errors")
    0
}

fn test_2() -> i64 {
    // Well-typed: let binding + arithmetic
    let errs = tc_check_errors("fn f() -> i64 { let x = 10; let y = 20; x + y }")
    assert_eq(errs, 0)
    println_str("TC Test 2 passed: let + arithmetic")
    0
}

fn test_3() -> i64 {
    // Well-typed: function with params
    let errs = tc_check_errors("fn add(a: i64, b: i64) -> i64 { a + b }")
    assert_eq(errs, 0)
    println_str("TC Test 3 passed: params")
    0
}

fn test_4() -> i64 {
    // Well-typed: if/else with bool condition
    let errs = tc_check_errors("fn f(x: i64) -> i64 { if x > 0 { 1 } else { 0 } }")
    assert_eq(errs, 0)
    println_str("TC Test 4 passed: if/else")
    0
}

fn test_5() -> i64 {
    // Well-typed: while loop with mutable var
    let errs = tc_check_errors("fn f() -> i64 { let mut x = 10; while x > 0 { x = x - 1 }; 0 }")
    assert_eq(errs, 0)
    println_str("TC Test 5 passed: while + mut")
    0
}

fn test_6() -> i64 {
    // Error: undefined variable
    let errs = tc_check_errors("fn f() -> i64 { y + 1 }")
    assert_true(errs > 0)
    println_str("TC Test 6 passed: undefined var detected")
    0
}

fn test_7() -> i64 {
    // Error: assign to immutable
    let errs = tc_check_errors("fn f() -> i64 { let x = 10; x = 20; 0 }")
    assert_true(errs > 0)
    println_str("TC Test 7 passed: immutable assign detected")
    0
}

fn test_8() -> i64 {
    // Well-typed: function call
    let errs = tc_check_errors("fn add(a: i64, b: i64) -> i64 { a + b } fn main() -> i64 { add(1, 2) }")
    assert_eq(errs, 0)
    println_str("TC Test 8 passed: function call")
    0
}

fn test_9() -> i64 {
    // Well-typed: calling builtin println
    let errs = tc_check_errors("fn main() -> i64 { println(42); 0 }")
    assert_eq(errs, 0)
    println_str("TC Test 9 passed: builtin call")
    0
}

fn test_10() -> i64 {
    // Error: wrong number of args
    let errs = tc_check_errors("fn add(a: i64, b: i64) -> i64 { a + b } fn main() -> i64 { add(1) }")
    assert_true(errs > 0)
    println_str("TC Test 10 passed: wrong arg count detected")
    0
}

fn test_11() -> i64 {
    // Well-typed: nested scopes
    let errs = tc_check_errors("fn f() -> i64 { let x = 1; if x > 0 { let y = 2; y } else { 0 } }")
    assert_eq(errs, 0)
    println_str("TC Test 11 passed: nested scopes")
    0
}

fn test_12() -> i64 {
    // Well-typed: comparison returns bool
    let errs = tc_check_errors("fn f(a: i64, b: i64) -> bool { a == b }")
    assert_eq(errs, 0)
    println_str("TC Test 12 passed: comparison -> bool")
    0
}

fn main() -> i64 {
    test_1()
    test_2()
    test_3()
    test_4()
    test_5()
    test_6()
    test_7()
    test_8()
    test_9()
    test_10()
    test_11()
    test_12()
    println_str("All type checker tests passed!")
    0
}
