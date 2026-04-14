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
fn test_1() -> i64 {
    let src1 = "let x = 42"
    let t1 = lex(src1)
    assert_eq(tok_count(t1), 5)
    assert_eq(tok_type(t1, 0), TK_LET())
    assert_eq(tok_type(t1, 1), TK_IDENT())
    assert_eq(tok_type(t1, 2), TK_EQ())
    assert_eq(tok_type(t1, 3), TK_INT())
    assert_eq(tok_type(t1, 4), TK_EOF())
    assert_true(str_eq(tok_text(src1, t1, 1), "x"))
    assert_true(str_eq(tok_text(src1, t1, 3), "42"))
    println_str("Test 1 passed: simple let binding")
    0
}

fn test_2() -> i64 {
    let src2 = "fn add(a: i64, b: i64) -> i64 { a + b }"
    let t2 = lex(src2)
    assert_eq(tok_type(t2, 0), TK_FN())
    assert_eq(tok_type(t2, 1), TK_IDENT())
    assert_eq(tok_type(t2, 2), TK_LPAREN())
    assert_eq(tok_type(t2, 3), TK_IDENT())
    assert_eq(tok_type(t2, 4), TK_COLON())
    assert_eq(tok_type(t2, 5), TK_IDENT())
    assert_true(str_eq(tok_text(src2, t2, 1), "add"))
    println_str("Test 2 passed: function definition")
    0
}

fn test_3() -> i64 {
    let src3 = "x == 5 && y != 3 || z <= 10"
    let t3 = lex(src3)
    assert_eq(tok_type(t3, 1), TK_EQEQ())
    assert_eq(tok_type(t3, 3), TK_AND())
    assert_eq(tok_type(t3, 5), TK_NEQ())
    assert_eq(tok_type(t3, 7), TK_OR())
    assert_eq(tok_type(t3, 9), TK_LTE())
    println_str("Test 3 passed: operators")
    0
}

fn test_4() -> i64 {
    // Build: let s = "hello world" using int_to_char(34) for quote chars
    let q = int_to_char(34)
    let src4 = str_cat(str_cat(str_cat("let s = ", q), "hello world"), q)
    let t4 = lex(src4)
    assert_eq(tok_type(t4, 3), TK_STRING())
    assert_true(str_eq(tok_text(src4, t4, 3), "hello world"))
    println_str("Test 4 passed: string literal")
    0
}

fn test_5() -> i64 {
    let src5 = "fn f() -> i64 { x |> g }"
    let t5 = lex(src5)
    // fn f ( ) -> i64 { x |> g } EOF
    // 0  1 2 3  4  5  6 7  8 9 10 11
    assert_eq(tok_type(t5, 4), TK_ARROW())
    assert_eq(tok_type(t5, 8), TK_PIPE())
    assert_true(str_eq(tok_text(src5, t5, 8), "|>"))
    println_str("Test 5 passed: arrow and pipe")
    0
}

fn test_6() -> i64 {
    // Build: "let x = 1 // comment\nlet y = 2" with actual newline
    let nl = int_to_char(10)
    let src6 = str_cat(str_cat("let x = 1 // comment", nl), "let y = 2")
    let t6 = lex(src6)
    assert_eq(tok_count(t6), 9)
    println_str("Test 6 passed: line comments")
    0
}

fn test_7() -> i64 {
    let src7 = "let /* skip */ x = 1"
    let t7 = lex(src7)
    assert_eq(tok_count(t7), 5)
    println_str("Test 7 passed: block comments")
    0
}

fn test_8() -> i64 {
    let src8 = "3.14"
    let t8 = lex(src8)
    assert_eq(tok_type(t8, 0), TK_FLOAT())
    assert_true(str_eq(tok_text(src8, t8, 0), "3.14"))
    println_str("Test 8 passed: float literal")
    0
}

fn test_9() -> i64 {
    let src9 = "0xFF"
    let t9 = lex(src9)
    assert_eq(tok_type(t9, 0), TK_INT())
    assert_true(str_eq(tok_text(src9, t9, 0), "0xFF"))
    println_str("Test 9 passed: hex literal")
    0
}

fn test_10() -> i64 {
    let src10 = "if else while for match struct enum trait impl"
    let t10 = lex(src10)
    assert_eq(tok_type(t10, 0), TK_IF())
    assert_eq(tok_type(t10, 1), TK_ELSE())
    assert_eq(tok_type(t10, 2), TK_WHILE())
    assert_eq(tok_type(t10, 3), TK_FOR())
    assert_eq(tok_type(t10, 4), TK_MATCH())
    assert_eq(tok_type(t10, 5), TK_STRUCT())
    assert_eq(tok_type(t10, 6), TK_ENUM())
    assert_eq(tok_type(t10, 7), TK_TRAIT())
    assert_eq(tok_type(t10, 8), TK_IMPL())
    println_str("Test 10 passed: keywords")
    0
}

fn test_11() -> i64 {
    println_str("--- Token dump: fn main() -> i64 { 42 } ---")
    let src11 = "fn main() -> i64 { 42 }"
    let t11 = lex(src11)
    let mut di = 0
    while di < tok_count(t11) {
        let tname = tok_type_name(tok_type(t11, di))
        let ttext = tok_text(src11, t11, di)
        print_str(tname)
        print_str(" '")
        print_str(ttext)
        print_str("' L")
        println(tok_line(t11, di))
        di = di + 1
    }
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
    println_str("All lexer tests passed!")
    0
}
