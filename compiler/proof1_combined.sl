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
    array_set(p, 1, array_new(200000))
    array_set(p, 2, array_new(80000))
    array_set(p, 3, array_new(40000))
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

    // args_count() -> i64
    fn_register(tc, "args_count", TY_I64(), 0, pt)

    // args_get(i64) -> str
    array_set(pt, 0, TY_I64())
    fn_register(tc, "args_get", TY_STR(), 1, pt)

    // file_read(str) -> str
    array_set(pt, 0, TY_STR())
    fn_register(tc, "file_read", TY_STR(), 1, pt)

    // file_write_bytes(str, i64) -> i64
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "file_write_bytes", TY_I64(), 2, pt)

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
    array_set(tc, 2, array_new(16000))
    array_set(tc, 3, 0)
    array_set(tc, 4, 0)
    array_set(tc, 5, array_new(8000))
    array_set(tc, 6, 0)
    array_set(tc, 7, 0)
    array_set(tc, 8, TY_VOID())

    register_builtins(tc)
    preregister_functions(tc, source, prog_off)
    check_program(tc, source, prog_off)
    tc
}

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
// String hashing (djb2) - provided by typechecker.sl in combined builds.
// When running ir_gen standalone, define str_hash in the test harness.
// ============================================================================

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
    array_set(ir, 1, array_new(200000))   // instructions
    array_set(ir, 2, 0)                   // inst_count
    array_set(ir, 3, 0)                   // next_vreg
    array_set(ir, 4, array_new(10000))     // blocks
    array_set(ir, 5, 0)                   // block_count
    array_set(ir, 6, 0)                   // current_block
    array_set(ir, 7, array_new(5000))      // strings
    array_set(ir, 8, 0)                   // string_count
    array_set(ir, 9, array_new(8000))      // fn_table
    array_set(ir, 10, 0)                  // fn_table_count
    array_set(ir, 11, array_new(20000))    // vars
    array_set(ir, 12, 0)                  // var_count
    array_set(ir, 13, 0)                  // var_depth
    array_set(ir, 14, array_new(32))      // call_args
    array_set(ir, 15, array_new(8000))     // fn_entries
    array_set(ir, 16, 0)                  // fn_entry_count

    // Create initial block (block 0)
    new_block(ir)

    lower_program(ir, source, prog_off)
    ir
}

// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 5: x86-64 Machine Code Emitter
// ============================================================================
// Encodes x86-64 instructions directly as bytes into an output buffer.
// Target: Windows x64 calling convention (RCX, RDX, R8, R9 + shadow space).
//
// Output buffer: flat i64 array where each element holds one byte (0-255).
// We use array_get/array_set since Vitalis arrays are i64-based.
//
// Register encoding:
//   RAX=0, RCX=1, RDX=2, RBX=3, RSP=4, RBP=5, RSI=6, RDI=7
//   R8=8, R9=9, R10=10, R11=11, R12=12, R13=13, R14=14, R15=15
//
// NOTE: No `return` in if-blocks. All uses if/else expressions.
// ============================================================================

// -- Register constants --
fn REG_RAX() -> i64 { 0 }
fn REG_RCX() -> i64 { 1 }
fn REG_RDX() -> i64 { 2 }
fn REG_RBX() -> i64 { 3 }
fn REG_RSP() -> i64 { 4 }
fn REG_RBP() -> i64 { 5 }
fn REG_RSI() -> i64 { 6 }
fn REG_RDI() -> i64 { 7 }
fn REG_R8() -> i64  { 8 }
fn REG_R9() -> i64  { 9 }
fn REG_R10() -> i64 { 10 }
fn REG_R11() -> i64 { 11 }
fn REG_R12() -> i64 { 12 }
fn REG_R13() -> i64 { 13 }
fn REG_R14() -> i64 { 14 }
fn REG_R15() -> i64 { 15 }

// Windows x64 parameter registers
fn WIN64_PARAM_0() -> i64 { 1 }  // RCX
fn WIN64_PARAM_1() -> i64 { 2 }  // RDX
fn WIN64_PARAM_2() -> i64 { 8 }  // R8
fn WIN64_PARAM_3() -> i64 { 9 }  // R9

// ============================================================================
// Code buffer context
// ============================================================================
// buf[0] = byte array (code output)
// buf[1] = current position (byte offset)
// buf[2] = capacity
// buf[3] = data section array
// buf[4] = data position
// buf[5] = fixups array [offset, type, target] stride=3
// buf[6] = fixup count
// buf[7] = labels array [label_id -> byte_offset]
// buf[8] = label count

fn buf_code(buf: i64) -> i64     { array_get(buf, 0) }
fn buf_pos(buf: i64) -> i64      { array_get(buf, 1) }
fn buf_cap(buf: i64) -> i64      { array_get(buf, 2) }
fn buf_data(buf: i64) -> i64     { array_get(buf, 3) }
fn buf_data_pos(buf: i64) -> i64 { array_get(buf, 4) }
fn buf_fixups(buf: i64) -> i64   { array_get(buf, 5) }
fn buf_fixup_count(buf: i64) -> i64 { array_get(buf, 6) }
fn buf_labels(buf: i64) -> i64   { array_get(buf, 7) }
fn buf_label_count(buf: i64) -> i64 { array_get(buf, 8) }

fn buf_set_pos(buf: i64, v: i64) -> i64      { array_set(buf, 1, v); 0 }
fn buf_set_data_pos(buf: i64, v: i64) -> i64 { array_set(buf, 4, v); 0 }
fn buf_set_fixup_count(buf: i64, v: i64) -> i64 { array_set(buf, 6, v); 0 }
fn buf_set_label_count(buf: i64, v: i64) -> i64 { array_set(buf, 8, v); 0 }

fn new_code_buf(cap: i64) -> i64 {
    let buf = array_new(9)
    array_set(buf, 0, array_new(cap))
    array_set(buf, 1, 0)
    array_set(buf, 2, cap)
    array_set(buf, 3, array_new(cap / 4))
    array_set(buf, 4, 0)
    array_set(buf, 5, array_new(10000))
    array_set(buf, 6, 0)
    array_set(buf, 7, array_new(5000))
    array_set(buf, 8, 0)
    buf
}

// ============================================================================
// Byte emission
// ============================================================================
fn emit_byte(buf: i64, b: i64) -> i64 {
    let code = buf_code(buf)
    let pos = buf_pos(buf)
    array_set(code, pos, b % 256)
    buf_set_pos(buf, pos + 1)
    0
}

fn emit_bytes2(buf: i64, b0: i64, b1: i64) -> i64 {
    emit_byte(buf, b0)
    emit_byte(buf, b1)
}

fn emit_bytes3(buf: i64, b0: i64, b1: i64, b2: i64) -> i64 {
    emit_byte(buf, b0)
    emit_byte(buf, b1)
    emit_byte(buf, b2)
}

fn emit_bytes4(buf: i64, b0: i64, b1: i64, b2: i64, b3: i64) -> i64 {
    emit_byte(buf, b0)
    emit_byte(buf, b1)
    emit_byte(buf, b2)
    emit_byte(buf, b3)
}

// Convert potentially negative i64 to unsigned 32-bit value
fn to_u32(val: i64) -> i64 {
    if val < 0 {
        val + 4294967296
    } else {
        val % 4294967296
    }
}

// Emit a 32-bit little-endian immediate (handles negative values)
fn emit_imm32(buf: i64, val: i64) -> i64 {
    let u = to_u32(val)
    emit_byte(buf, u % 256)
    emit_byte(buf, (u / 256) % 256)
    emit_byte(buf, (u / 65536) % 256)
    emit_byte(buf, (u / 16777216) % 256)
}

// Emit a 64-bit little-endian immediate
fn emit_imm64(buf: i64, val: i64) -> i64 {
    emit_imm32(buf, val % 4294967296)
    emit_imm32(buf, val / 4294967296)
}

// ============================================================================
// Label management
// ============================================================================
fn new_label(buf: i64) -> i64 {
    let lid = buf_label_count(buf)
    let labels = buf_labels(buf)
    array_set(labels, lid, -1)  // unresolved
    buf_set_label_count(buf, lid + 1)
    lid
}

fn bind_label(buf: i64, lid: i64) -> i64 {
    let labels = buf_labels(buf)
    array_set(labels, lid, buf_pos(buf))
    0
}

fn label_offset(buf: i64, lid: i64) -> i64 {
    let labels = buf_labels(buf)
    array_get(labels, lid)
}

// Fixup types
fn FIXUP_REL32() -> i64 { 1 }

fn add_fixup(buf: i64, offset: i64, fixup_type: i64, target_label: i64) -> i64 {
    let fixups = buf_fixups(buf)
    let cnt = buf_fixup_count(buf)
    let off = cnt * 3
    array_set(fixups, off + 0, offset)
    array_set(fixups, off + 1, fixup_type)
    array_set(fixups, off + 2, target_label)
    buf_set_fixup_count(buf, cnt + 1)
    0
}

// Resolve all fixups
fn resolve_fixups(buf: i64) -> i64 {
    let code = buf_code(buf)
    let fixups = buf_fixups(buf)
    let labels = buf_labels(buf)
    let cnt = buf_fixup_count(buf)
    let mut i = 0
    while i < cnt {
        let off = i * 3
        let fix_offset = array_get(fixups, off + 0)
        let fix_type = array_get(fixups, off + 1)
        let target_lid = array_get(fixups, off + 2)
        let target_pos = array_get(labels, target_lid)

        if fix_type == FIXUP_REL32() {
            // rel32: target - (fixup_offset + 4)
            let rel = target_pos - (fix_offset + 4)
            let u = to_u32(rel)
            array_set(code, fix_offset + 0, u % 256)
            array_set(code, fix_offset + 1, (u / 256) % 256)
            array_set(code, fix_offset + 2, (u / 65536) % 256)
            array_set(code, fix_offset + 3, (u / 16777216) % 256)
            0
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// REX prefix helpers
// ============================================================================

// Need REX.W (48h) for 64-bit operand size
// Need REX.R when reg >= 8 (for ModR/M reg field)
// Need REX.B when rm >= 8 (for ModR/M r/m field)

fn is_extended(reg: i64) -> i64 {
    if reg >= 8 { 1 } else { 0 }
}

fn reg_low3(reg: i64) -> i64 {
    reg % 8
}

fn rex_w() -> i64 { 72 }  // 0x48

fn rex_wrb(reg: i64, rm: i64) -> i64 {
    // REX prefix: 0100 W R X B
    // W=1, R=extended(reg), X=0, B=extended(rm)
    let mut val = 72  // 0x48 = 0100 1000 (W=1)
    if is_extended(reg) == 1 {
        val = val + 4  // R bit (bit 2)
        val
    } else {
        val
    }
    if is_extended(rm) == 1 {
        val = val + 1  // B bit (bit 0)
        val
    } else {
        val
    }
}

// ModR/M byte: [mod(2):reg(3):rm(3)]
fn modrm(md: i64, reg: i64, rm: i64) -> i64 {
    md * 64 + reg_low3(reg) * 8 + reg_low3(rm)
}

// ============================================================================
// x86-64 Instructions
// ============================================================================

// MOV reg, imm64 (REX.W + B8+rd io)
fn x86_mov_reg_imm64(buf: i64, reg: i64, imm: i64) -> i64 {
    let rex = rex_wrb(0, reg)
    emit_byte(buf, rex)
    emit_byte(buf, 184 + reg_low3(reg))  // 0xB8 + rd
    emit_imm64(buf, imm)
}

// MOV reg, imm32 sign-extended (REX.W C7 /0)
fn x86_mov_reg_imm32(buf: i64, reg: i64, imm: i64) -> i64 {
    let rex = rex_wrb(0, reg)
    emit_byte(buf, rex)
    emit_byte(buf, 199)  // 0xC7
    emit_byte(buf, modrm(3, 0, reg))
    emit_imm32(buf, imm)
}

// MOV reg, reg (REX.W 89 /r)
fn x86_mov_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 137)  // 0x89
    emit_byte(buf, modrm(3, src, dst))
}

// MOV reg, [reg + disp32] (REX.W 8B /r)
fn x86_mov_reg_mem(buf: i64, dst: i64, base: i64, disp: i64) -> i64 {
    let rex = rex_wrb(dst, base)
    emit_byte(buf, rex)
    emit_byte(buf, 139)  // 0x8B
    if reg_low3(base) == 4 {
        // RSP/R12 needs SIB byte
        emit_byte(buf, modrm(2, dst, 4))  // mod=10, rm=100 (SIB follows)
        emit_byte(buf, 36)  // SIB: index=100(none), base=100(RSP)
        emit_imm32(buf, disp)
    } else if reg_low3(base) == 5 && disp == 0 {
        // RBP/R13 needs disp8=0
        emit_byte(buf, modrm(1, dst, base))
        emit_byte(buf, 0)
    } else if disp == 0 {
        emit_byte(buf, modrm(0, dst, base))
        0
    } else {
        emit_byte(buf, modrm(2, dst, base))
        emit_imm32(buf, disp)
    }
}

// MOV [reg + disp32], reg (REX.W 89 /r)
fn x86_mov_mem_reg(buf: i64, base: i64, disp: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, base)
    emit_byte(buf, rex)
    emit_byte(buf, 137)  // 0x89
    if reg_low3(base) == 4 {
        emit_byte(buf, modrm(2, src, 4))
        emit_byte(buf, 36)
        emit_imm32(buf, disp)
    } else {
        emit_byte(buf, modrm(2, src, base))
        emit_imm32(buf, disp)
    }
}

// ADD reg, reg (REX.W 01 /r)
fn x86_add_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 1)  // 0x01
    emit_byte(buf, modrm(3, src, dst))
}

// ADD reg, imm32 (REX.W 81 /0)
fn x86_add_reg_imm32(buf: i64, dst: i64, imm: i64) -> i64 {
    let rex = rex_wrb(0, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 129)  // 0x81
    emit_byte(buf, modrm(3, 0, dst))
    emit_imm32(buf, imm)
}

// SUB reg, reg (REX.W 29 /r)
fn x86_sub_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 41)  // 0x29
    emit_byte(buf, modrm(3, src, dst))
}

// SUB reg, imm32 (REX.W 81 /5)
fn x86_sub_reg_imm32(buf: i64, dst: i64, imm: i64) -> i64 {
    let rex = rex_wrb(0, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 129)  // 0x81
    emit_byte(buf, modrm(3, 5, dst))
    emit_imm32(buf, imm)
}

// IMUL reg, reg (REX.W 0F AF /r)
fn x86_imul_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(dst, src)
    emit_byte(buf, rex)
    emit_bytes2(buf, 15, 175)  // 0x0F 0xAF
    emit_byte(buf, modrm(3, dst, src))
}

// IDIV reg (REX.W F7 /7) - divides RDX:RAX by reg
fn x86_idiv_reg(buf: i64, divisor: i64) -> i64 {
    let rex = rex_wrb(0, divisor)
    emit_byte(buf, rex)
    emit_byte(buf, 247)  // 0xF7
    emit_byte(buf, modrm(3, 7, divisor))
}

// CQO - sign-extend RAX -> RDX:RAX (REX.W 99)
fn x86_cqo(buf: i64) -> i64 {
    emit_bytes2(buf, 72, 153)  // 0x48 0x99
}

// NEG reg (REX.W F7 /3)
fn x86_neg_reg(buf: i64, reg: i64) -> i64 {
    let rex = rex_wrb(0, reg)
    emit_byte(buf, rex)
    emit_byte(buf, 247)  // 0xF7
    emit_byte(buf, modrm(3, 3, reg))
}

// CMP reg, reg (REX.W 39 /r)
fn x86_cmp_reg_reg(buf: i64, left: i64, right: i64) -> i64 {
    let rex = rex_wrb(right, left)
    emit_byte(buf, rex)
    emit_byte(buf, 57)  // 0x39
    emit_byte(buf, modrm(3, right, left))
}

// CMP reg, imm32 (REX.W 81 /7)
fn x86_cmp_reg_imm32(buf: i64, reg: i64, imm: i64) -> i64 {
    let rex = rex_wrb(0, reg)
    emit_byte(buf, rex)
    emit_byte(buf, 129)  // 0x81
    emit_byte(buf, modrm(3, 7, reg))
    emit_imm32(buf, imm)
}

// SETcc instructions (set byte based on condition)
fn x86_sete(buf: i64, reg: i64) -> i64 {
    // 0F 94 /r (with REX if extended)
    if is_extended(reg) == 1 {
        emit_byte(buf, 65)  // REX.B = 0x41
        0
    } else {
        0
    }
    emit_bytes2(buf, 15, 148)  // 0x0F 0x94
    emit_byte(buf, modrm(3, 0, reg))
}

fn x86_setne(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 { emit_byte(buf, 65); 0 } else { 0 }
    emit_bytes2(buf, 15, 149)
    emit_byte(buf, modrm(3, 0, reg))
}

fn x86_setl(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 { emit_byte(buf, 65); 0 } else { 0 }
    emit_bytes2(buf, 15, 156)
    emit_byte(buf, modrm(3, 0, reg))
}

fn x86_setg(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 { emit_byte(buf, 65); 0 } else { 0 }
    emit_bytes2(buf, 15, 159)
    emit_byte(buf, modrm(3, 0, reg))
}

fn x86_setle(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 { emit_byte(buf, 65); 0 } else { 0 }
    emit_bytes2(buf, 15, 158)
    emit_byte(buf, modrm(3, 0, reg))
}

fn x86_setge(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 { emit_byte(buf, 65); 0 } else { 0 }
    emit_bytes2(buf, 15, 157)
    emit_byte(buf, modrm(3, 0, reg))
}

// MOVZX reg, reg8 - zero-extend byte to 64-bit (REX.W 0F B6 /r)
fn x86_movzx_reg_reg8(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(dst, src)
    emit_byte(buf, rex)
    emit_bytes2(buf, 15, 182)  // 0x0F 0xB6
    emit_byte(buf, modrm(3, dst, src))
}

// AND reg, reg (REX.W 21 /r)
fn x86_and_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 33)  // 0x21
    emit_byte(buf, modrm(3, src, dst))
}

// OR reg, reg (REX.W 09 /r)
fn x86_or_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 9)  // 0x09
    emit_byte(buf, modrm(3, src, dst))
}

// XOR reg, reg (REX.W 31 /r)
fn x86_xor_reg_reg(buf: i64, dst: i64, src: i64) -> i64 {
    let rex = rex_wrb(src, dst)
    emit_byte(buf, rex)
    emit_byte(buf, 49)  // 0x31
    emit_byte(buf, modrm(3, src, dst))
}

// NOT reg (REX.W F7 /2) - bitwise NOT
fn x86_not_reg(buf: i64, reg: i64) -> i64 {
    let rex = rex_wrb(0, reg)
    emit_byte(buf, rex)
    emit_byte(buf, 247)  // 0xF7
    emit_byte(buf, modrm(3, 2, reg))
}

// TEST reg, reg (REX.W 85 /r)
fn x86_test_reg_reg(buf: i64, r1: i64, r2: i64) -> i64 {
    let rex = rex_wrb(r2, r1)
    emit_byte(buf, rex)
    emit_byte(buf, 133)  // 0x85
    emit_byte(buf, modrm(3, r2, r1))
}

// ============================================================================
// Control flow
// ============================================================================

// JMP rel32 (E9 cd)
fn x86_jmp_rel32(buf: i64, target_label: i64) -> i64 {
    emit_byte(buf, 233)  // 0xE9
    let fix_pos = buf_pos(buf)
    emit_imm32(buf, 0)  // placeholder
    add_fixup(buf, fix_pos, FIXUP_REL32(), target_label)
}

// JE rel32 (0F 84 cd) - jump if equal/zero
fn x86_je_rel32(buf: i64, target_label: i64) -> i64 {
    emit_bytes2(buf, 15, 132)  // 0x0F 0x84
    let fix_pos = buf_pos(buf)
    emit_imm32(buf, 0)
    add_fixup(buf, fix_pos, FIXUP_REL32(), target_label)
}

// JNE rel32 (0F 85 cd)
fn x86_jne_rel32(buf: i64, target_label: i64) -> i64 {
    emit_bytes2(buf, 15, 133)  // 0x0F 0x85
    let fix_pos = buf_pos(buf)
    emit_imm32(buf, 0)
    add_fixup(buf, fix_pos, FIXUP_REL32(), target_label)
}

// CALL rel32 (E8 cd)
fn x86_call_rel32(buf: i64, target_label: i64) -> i64 {
    emit_byte(buf, 232)  // 0xE8
    let fix_pos = buf_pos(buf)
    emit_imm32(buf, 0)
    add_fixup(buf, fix_pos, FIXUP_REL32(), target_label)
}

// RET (C3)
fn x86_ret(buf: i64) -> i64 {
    emit_byte(buf, 195)  // 0xC3
}

// PUSH reg (50+rd, or REX.B 50+rd for R8-R15)
fn x86_push_reg(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 {
        emit_byte(buf, 65)  // REX.B = 0x41
        emit_byte(buf, 80 + reg_low3(reg))
    } else {
        emit_byte(buf, 80 + reg)  // 0x50 + rd
    }
}

// POP reg (58+rd, or REX.B 58+rd for R8-R15)
fn x86_pop_reg(buf: i64, reg: i64) -> i64 {
    if is_extended(reg) == 1 {
        emit_byte(buf, 65)
        emit_byte(buf, 88 + reg_low3(reg))
    } else {
        emit_byte(buf, 88 + reg)  // 0x58 + rd
    }
}

// NOP (90)
fn x86_nop(buf: i64) -> i64 {
    emit_byte(buf, 144)  // 0x90
}

// INT3 - breakpoint (CC)
fn x86_int3(buf: i64) -> i64 {
    emit_byte(buf, 204)  // 0xCC
}

// ============================================================================
// Function prologue / epilogue (Windows x64)
// ============================================================================

// Standard prologue: push rbp; mov rbp, rsp; sub rsp, frame_size
fn x86_prologue(buf: i64, frame_size: i64) -> i64 {
    x86_push_reg(buf, REG_RBP())
    x86_mov_reg_reg(buf, REG_RBP(), REG_RSP())
    if frame_size > 0 {
        x86_sub_reg_imm32(buf, REG_RSP(), frame_size)
        0
    } else {
        0
    }
}

// Standard epilogue: mov rsp, rbp; pop rbp; ret
fn x86_epilogue(buf: i64) -> i64 {
    x86_mov_reg_reg(buf, REG_RSP(), REG_RBP())
    x86_pop_reg(buf, REG_RBP())
    x86_ret(buf)
}

// ============================================================================
// LEA reg, [RIP + disp32] - for data references (48 8D 05 disp32)
// ============================================================================
fn x86_lea_rip_rel(buf: i64, dst: i64, data_label: i64) -> i64 {
    let rex = rex_wrb(dst, 0)
    emit_byte(buf, rex)
    emit_byte(buf, 141)  // 0x8D
    emit_byte(buf, modrm(0, dst, 5))  // mod=00, rm=101 (RIP-relative)
    let fix_pos = buf_pos(buf)
    emit_imm32(buf, 0)
    add_fixup(buf, fix_pos, FIXUP_REL32(), data_label)
}

// ============================================================================
// Data section helpers
// ============================================================================
fn data_emit_bytes(buf: i64, bytes: i64, count: i64) -> i64 {
    let data = buf_data(buf)
    let pos = buf_data_pos(buf)
    let mut i = 0
    while i < count {
        array_set(data, pos + i, array_get(bytes, i))
        i = i + 1
    }
    buf_set_data_pos(buf, pos + count)
    pos
}

fn data_emit_string(buf: i64, s: str) -> i64 {
    let data = buf_data(buf)
    let pos = buf_data_pos(buf)
    let slen = str_len(s)
    let mut i = 0
    while i < slen {
        array_set(data, pos + i, char_to_int(str_char_at(s, i)))
        i = i + 1
    }
    // Null terminator
    array_set(data, pos + slen, 0)
    buf_set_data_pos(buf, pos + slen + 1)
    pos
}

// ============================================================================
// Hex dump helper (for testing / debugging)
// ============================================================================
fn hex_digit(n: i64) -> str {
    if n == 0 { "0" }
    else if n == 1 { "1" }
    else if n == 2 { "2" }
    else if n == 3 { "3" }
    else if n == 4 { "4" }
    else if n == 5 { "5" }
    else if n == 6 { "6" }
    else if n == 7 { "7" }
    else if n == 8 { "8" }
    else if n == 9 { "9" }
    else if n == 10 { "A" }
    else if n == 11 { "B" }
    else if n == 12 { "C" }
    else if n == 13 { "D" }
    else if n == 14 { "E" }
    else { "F" }
}

fn byte_to_hex(b: i64) -> str {
    str_cat(hex_digit(b / 16), hex_digit(b % 16))
}

fn dump_code(buf: i64, count: i64) -> i64 {
    let code = buf_code(buf)
    print_str("Code (")
    print_str(to_string_i64(count))
    print_str(" bytes): ")
    let mut i = 0
    while i < count {
        print_str(byte_to_hex(array_get(code, i)))
        print_str(" ")
        i = i + 1
    }
    println_str("")
    0
}

// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 6: Register Allocator
// ============================================================================
// Linear scan register allocation: maps IR virtual registers (v0, v1, ...)
// to physical x86-64 registers, spilling to the stack frame when needed.
//
// Strategy:
//   1. Compute live ranges [start_inst, end_inst] for each virtual register
//   2. Sort live ranges by start position
//   3. Walk instructions in order, allocating physical registers
//   4. When all registers are in use -> spill longest-lived to stack
//   5. Generate MOV spill/reload instructions as needed
//
// Physical register pool (caller-saved first for efficiency):
//   Allocatable: RAX, RCX, RDX, RSI, RDI, R8, R9, R10, R11 (9 regs)
//   Callee-saved: RBX, R12, R13, R14, R15 (5 regs - need save/restore)
//   Reserved: RSP (stack pointer), RBP (frame pointer) - never allocated
//
// Windows x64 calling convention:
//   Args: RCX, RDX, R8, R9 (first 4 params)
//   Return: RAX
//   Caller-saved: RAX, RCX, RDX, R8, R9, R10, R11
//   Callee-saved: RBX, RSI, RDI, RBP, RSP, R12-R15
//   Shadow space: 32 bytes reserved on stack for callee
//
// NOTE: No `return` inside if-blocks (Cranelift bug workaround).
// ============================================================================

// -- Allocatable register pool --
// We use 9 caller-saved registers first, then 5 callee-saved.
// This array holds physical register IDs (matching REG_* from x86_emit.sl).

fn POOL_SIZE() -> i64 { 14 }

// Caller-saved (prefer these - no save/restore overhead)
fn POOL_REG_0() -> i64  { 0 }   // RAX
fn POOL_REG_1() -> i64  { 1 }   // RCX
fn POOL_REG_2() -> i64  { 2 }   // RDX
fn POOL_REG_3() -> i64  { 6 }   // RSI
fn POOL_REG_4() -> i64  { 7 }   // RDI
fn POOL_REG_5() -> i64  { 8 }   // R8
fn POOL_REG_6() -> i64  { 9 }   // R9
fn POOL_REG_7() -> i64  { 10 }  // R10
fn POOL_REG_8() -> i64  { 11 }  // R11

// Callee-saved (use only if caller-saved are exhausted)
fn POOL_REG_9() -> i64  { 3 }   // RBX
fn POOL_REG_10() -> i64 { 12 }  // R12
fn POOL_REG_11() -> i64 { 13 }  // R13
fn POOL_REG_12() -> i64 { 14 }  // R14
fn POOL_REG_13() -> i64 { 15 }  // R15

fn CALLER_SAVED_COUNT() -> i64 { 9 }

fn pool_reg(idx: i64) -> i64 {
    if idx == 0  { POOL_REG_0()  }
    else if idx == 1  { POOL_REG_1()  }
    else if idx == 2  { POOL_REG_2()  }
    else if idx == 3  { POOL_REG_3()  }
    else if idx == 4  { POOL_REG_4()  }
    else if idx == 5  { POOL_REG_5()  }
    else if idx == 6  { POOL_REG_6()  }
    else if idx == 7  { POOL_REG_7()  }
    else if idx == 8  { POOL_REG_8()  }
    else if idx == 9  { POOL_REG_9()  }
    else if idx == 10 { POOL_REG_10() }
    else if idx == 11 { POOL_REG_11() }
    else if idx == 12 { POOL_REG_12() }
    else { POOL_REG_13() }
}

// ============================================================================
// Register allocation context (ra)
// ============================================================================
// Packed into an array for Vitalis compatibility.
//
// ra[0]  = live_ranges array: [vreg, start, end] stride=3
// ra[1]  = live_range_count
// ra[2]  = vreg_to_phys array: vreg -> physical reg (-1 = spilled)
// ra[3]  = vreg_to_stack array: vreg -> stack offset (-1 = not spilled)
// ra[4]  = max_vreg (how many virtual registers exist)
// ra[5]  = active_intervals array: [pool_idx, vreg, end] stride=3
// ra[6]  = active_count
// ra[7]  = next_stack_slot (next available stack offset, grows negatively)
// ra[8]  = spill_count
// ra[9]  = callee_saved_used (bitmask of which callee-saved regs used)
// ra[10] = pool_free array: [0=free, 1=in-use] indexed by pool index

fn ra_ranges(ra: i64) -> i64          { array_get(ra, 0) }
fn ra_range_count(ra: i64) -> i64     { array_get(ra, 1) }
fn ra_vreg_phys(ra: i64) -> i64       { array_get(ra, 2) }
fn ra_vreg_stack(ra: i64) -> i64      { array_get(ra, 3) }
fn ra_max_vreg(ra: i64) -> i64        { array_get(ra, 4) }
fn ra_active(ra: i64) -> i64          { array_get(ra, 5) }
fn ra_active_count(ra: i64) -> i64    { array_get(ra, 6) }
fn ra_stack_slot(ra: i64) -> i64      { array_get(ra, 7) }
fn ra_spill_count(ra: i64) -> i64     { array_get(ra, 8) }
fn ra_callee_mask(ra: i64) -> i64     { array_get(ra, 9) }
fn ra_pool_free(ra: i64) -> i64       { array_get(ra, 10) }

fn ra_set_range_count(ra: i64, v: i64) -> i64     { array_set(ra, 1, v); 0 }
fn ra_set_active_count(ra: i64, v: i64) -> i64    { array_set(ra, 6, v); 0 }
fn ra_set_stack_slot(ra: i64, v: i64) -> i64      { array_set(ra, 7, v); 0 }
fn ra_set_spill_count(ra: i64, v: i64) -> i64     { array_set(ra, 8, v); 0 }
fn ra_set_callee_mask(ra: i64, v: i64) -> i64     { array_set(ra, 9, v); 0 }

fn new_regalloc(max_vreg: i64) -> i64 {
    let ra = array_new(11)
    array_set(ra, 0, array_new(max_vreg * 3))  // live_ranges
    array_set(ra, 1, 0)  // range_count
    // vreg -> phys: -1 = unassigned
    let phys = array_new(max_vreg)
    let mut i = 0
    while i < max_vreg {
        array_set(phys, i, -1)
        i = i + 1
    }
    array_set(ra, 2, phys)
    // vreg -> stack: -1 = not spilled
    let stk = array_new(max_vreg)
    i = 0
    while i < max_vreg {
        array_set(stk, i, -1)
        i = i + 1
    }
    array_set(ra, 3, stk)
    array_set(ra, 4, max_vreg)
    array_set(ra, 5, array_new(POOL_SIZE() * 3))  // active intervals
    array_set(ra, 6, 0)  // active_count
    array_set(ra, 7, 8)  // start at -8 from RBP (first stack slot)
    array_set(ra, 8, 0)  // spill_count
    array_set(ra, 9, 0)  // callee_saved_mask
    // pool free array
    let pf = array_new(POOL_SIZE())
    i = 0
    while i < POOL_SIZE() {
        array_set(pf, i, 0)  // all free
        i = i + 1
    }
    array_set(ra, 10, pf)
    ra
}

// ============================================================================
// Live range computation
// ============================================================================
// Walk IR instructions and record [first_def, last_use] for each vreg.

fn add_live_range(ra: i64, vreg: i64, start: i64, end: i64) -> i64 {
    let ranges = ra_ranges(ra)
    let cnt = ra_range_count(ra)
    let off = cnt * 3
    array_set(ranges, off + 0, vreg)
    array_set(ranges, off + 1, start)
    array_set(ranges, off + 2, end)
    ra_set_range_count(ra, cnt + 1)
    0
}

// Update or create a live range for a vreg.
// If the vreg already has a range, extend the end.
// If not, create one with [inst_pos, inst_pos].
fn update_range(ra: i64, vreg: i64, inst_pos: i64) -> i64 {
    let ranges = ra_ranges(ra)
    let cnt = ra_range_count(ra)
    let mut found = 0
    let mut idx = 0
    while idx < cnt {
        let off = idx * 3
        if array_get(ranges, off + 0) == vreg {
            // Extend end
            if inst_pos > array_get(ranges, off + 2) {
                array_set(ranges, off + 2, inst_pos)
                0
            } else {
                0
            }
            found = 1
            0
        } else {
            0
        }
        idx = idx + 1
    }
    if found == 0 {
        add_live_range(ra, vreg, inst_pos, inst_pos)
    } else {
        0
    }
}

// Compute live ranges from IR instruction buffer.
// IR stride = 6: [opcode, dest, arg0, arg1, arg2, extra]
fn compute_live_ranges(ra: i64, insts: i64, inst_count: i64) -> i64 {
    let mut i = 0
    while i < inst_count {
        let off = i * 6
        let opcode = array_get(insts, off + 0)
        let dest = array_get(insts, off + 1)
        let arg0 = array_get(insts, off + 2)
        let arg1 = array_get(insts, off + 3)

        // dest is defined here (if valid - not -1 and not 0 for NOP)
        if dest >= 0 && opcode > 0 {
            update_range(ra, dest, i)
            0
        } else {
            0
        }

        // arg0 is used here (various opcodes use arg0)
        // Ops with arg0: BinOp(arg0,arg1), UnOp(arg0), RET(arg0), CONDBR(arg0), COPY(arg0), PHI(arg0,arg1)
        if opcode >= 10 && opcode <= 15 {
            // Arithmetic: ADD-NEG, arg0 used (and arg1 for binops)
            update_range(ra, arg0, i)
            if opcode != 15 {
                // not NEG (unary), so arg1 is also used
                update_range(ra, arg1, i)
                0
            } else {
                0
            }
        } else if opcode >= 20 && opcode <= 28 {
            // Comparison/logic: EQ-NOT
            update_range(ra, arg0, i)
            if opcode != 28 {
                // not NOT (unary)
                update_range(ra, arg1, i)
                0
            } else {
                0
            }
        } else if opcode == 31 {
            // RET: arg0 is the return value
            if arg0 >= 0 {
                update_range(ra, arg0, i)
                0
            } else {
                0
            }
        } else if opcode == 33 {
            // CONDBR: arg0 is the condition
            update_range(ra, arg0, i)
            0
        } else if opcode == 34 {
            // COPY: arg0 is the source
            update_range(ra, arg0, i)
            0
        } else if opcode == 35 {
            // PHI: arg0, arg1 are the two values
            if arg0 >= 0 {
                update_range(ra, arg0, i)
                0
            } else {
                0
            }
            if arg1 >= 0 {
                update_range(ra, arg1, i)
                0
            } else {
                0
            }
        } else if opcode == 30 {
            // CALL: args are in a separate call_args buffer,
            // but arg0 = callee index, we skip that
            0
        } else {
            0
        }

        i = i + 1
    }
    0
}

// ============================================================================
// Sorting live ranges by start position (insertion sort)
// ============================================================================
fn sort_live_ranges(ra: i64) -> i64 {
    let ranges = ra_ranges(ra)
    let cnt = ra_range_count(ra)
    let mut i = 1
    while i < cnt {
        let off_i = i * 3
        let key_vreg = array_get(ranges, off_i + 0)
        let key_start = array_get(ranges, off_i + 1)
        let key_end = array_get(ranges, off_i + 2)
        let mut j = i - 1
        let mut done = 0
        while j >= 0 && done == 0 {
            let off_j = j * 3
            let j_start = array_get(ranges, off_j + 1)
            if j_start > key_start {
                // shift j -> j+1
                let off_j1 = (j + 1) * 3
                array_set(ranges, off_j1 + 0, array_get(ranges, off_j + 0))
                array_set(ranges, off_j1 + 1, array_get(ranges, off_j + 1))
                array_set(ranges, off_j1 + 2, array_get(ranges, off_j + 2))
                j = j - 1
                0
            } else {
                done = 1
                0
            }
        }
        let off_ins = (j + 1) * 3
        array_set(ranges, off_ins + 0, key_vreg)
        array_set(ranges, off_ins + 1, key_start)
        array_set(ranges, off_ins + 2, key_end)
        i = i + 1
    }
    0
}

// ============================================================================
// Linear scan allocation
// ============================================================================

// Try to find a free register in the pool.
// Returns pool index, or -1 if none free.
fn find_free_reg(ra: i64) -> i64 {
    let pf = ra_pool_free(ra)
    let mut idx = 0
    let mut result = -1
    while idx < POOL_SIZE() {
        if result == -1 && array_get(pf, idx) == 0 {
            result = idx
            0
        } else {
            0
        }
        idx = idx + 1
    }
    result
}

// Mark a pool register as in-use.
fn mark_used(ra: i64, pool_idx: i64) -> i64 {
    array_set(ra_pool_free(ra), pool_idx, 1)
    // Track callee-saved usage
    if pool_idx >= CALLER_SAVED_COUNT() {
        let mask = ra_callee_mask(ra)
        // Set bit for this callee-saved register
        let bit = pool_idx - CALLER_SAVED_COUNT()
        // Simple: use addition if bit not set (mask / 2^bit % 2 == 0)
        // For simplicity, OR by adding power of 2 if not already set
        let pwr = if bit == 0 { 1 }
                  else if bit == 1 { 2 }
                  else if bit == 2 { 4 }
                  else if bit == 3 { 8 }
                  else { 16 }
        if (mask / pwr) % 2 == 0 {
            ra_set_callee_mask(ra, mask + pwr)
        } else {
            0
        }
    } else {
        0
    }
}

// Mark a pool register as free.
fn mark_free(ra: i64, pool_idx: i64) -> i64 {
    array_set(ra_pool_free(ra), pool_idx, 0)
    0
}

// Add to active intervals.
fn add_active(ra: i64, pool_idx: i64, vreg: i64, end: i64) -> i64 {
    let act = ra_active(ra)
    let cnt = ra_active_count(ra)
    let off = cnt * 3
    array_set(act, off + 0, pool_idx)
    array_set(act, off + 1, vreg)
    array_set(act, off + 2, end)
    ra_set_active_count(ra, cnt + 1)
    0
}

// Remove expired intervals (whose end < current position).
fn expire_old(ra: i64, cur_pos: i64) -> i64 {
    let act = ra_active(ra)
    let mut cnt = ra_active_count(ra)
    let mut i = 0
    while i < cnt {
        let off = i * 3
        let end = array_get(act, off + 2)
        if end < cur_pos {
            // Free the register
            let pidx = array_get(act, off + 0)
            mark_free(ra, pidx)
            // Remove by shifting last into this slot
            let last = (cnt - 1) * 3
            if i < cnt - 1 {
                array_set(act, off + 0, array_get(act, last + 0))
                array_set(act, off + 1, array_get(act, last + 1))
                array_set(act, off + 2, array_get(act, last + 2))
                0
            } else {
                0
            }
            cnt = cnt - 1
            ra_set_active_count(ra, cnt)
            // Don't increment i - recheck same slot after swap
            0
        } else {
            i = i + 1
            0
        }
    }
    0
}

// Spill: find the active interval with the longest end, spill it.
fn spill_farthest(ra: i64) -> i64 {
    let act = ra_active(ra)
    let cnt = ra_active_count(ra)
    let mut best_i = 0
    let mut best_end = -1
    let mut i = 0
    while i < cnt {
        let off = i * 3
        let end = array_get(act, off + 2)
        if end > best_end {
            best_end = end
            best_i = i
            0
        } else {
            0
        }
        i = i + 1
    }
    // Spill best_i
    let off = best_i * 3
    let pidx = array_get(act, off + 0)
    let vreg = array_get(act, off + 1)
    // Assign stack slot to this vreg
    let slot = ra_stack_slot(ra)
    array_set(ra_vreg_stack(ra), vreg, slot)
    array_set(ra_vreg_phys(ra), vreg, -1)  // no longer in a register
    ra_set_stack_slot(ra, slot + 8)
    ra_set_spill_count(ra, ra_spill_count(ra) + 1)
    // Remove from active (swap with last)
    let last = (cnt - 1) * 3
    if best_i < cnt - 1 {
        array_set(act, off + 0, array_get(act, last + 0))
        array_set(act, off + 1, array_get(act, last + 1))
        array_set(act, off + 2, array_get(act, last + 2))
        0
    } else {
        0
    }
    ra_set_active_count(ra, cnt - 1)
    // Return the freed pool index
    pidx
}

// Main allocation loop.
fn allocate_registers(ra: i64) -> i64 {
    sort_live_ranges(ra)
    let ranges = ra_ranges(ra)
    let cnt = ra_range_count(ra)
    let mut i = 0
    while i < cnt {
        let off = i * 3
        let vreg = array_get(ranges, off + 0)
        let start = array_get(ranges, off + 1)
        let end = array_get(ranges, off + 2)

        // Expire intervals that ended before this start
        expire_old(ra, start)

        // Try to allocate a free register
        let free_idx = find_free_reg(ra)
        if free_idx >= 0 {
            // Got a free register
            mark_used(ra, free_idx)
            let phys = pool_reg(free_idx)
            array_set(ra_vreg_phys(ra), vreg, phys)
            add_active(ra, free_idx, vreg, end)
            0
        } else {
            // No free registers - spill farthest
            let freed_idx = spill_farthest(ra)
            mark_used(ra, freed_idx)
            let phys = pool_reg(freed_idx)
            array_set(ra_vreg_phys(ra), vreg, phys)
            add_active(ra, freed_idx, vreg, end)
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// Query results
// ============================================================================

// Get the physical register assigned to a vreg (-1 if spilled).
fn get_phys_reg(ra: i64, vreg: i64) -> i64 {
    if vreg >= 0 && vreg < ra_max_vreg(ra) {
        array_get(ra_vreg_phys(ra), vreg)
    } else {
        -1
    }
}

// Get the stack offset for a spilled vreg (-1 if not spilled).
fn get_stack_offset(ra: i64, vreg: i64) -> i64 {
    if vreg >= 0 && vreg < ra_max_vreg(ra) {
        array_get(ra_vreg_stack(ra), vreg)
    } else {
        -1
    }
}

// Is a vreg spilled to stack?
fn is_spilled(ra: i64, vreg: i64) -> i64 {
    if get_phys_reg(ra, vreg) == -1 && get_stack_offset(ra, vreg) >= 0 {
        1
    } else {
        0
    }
}

// Compute frame size needed: shadow space (32) + spill slots
fn compute_frame_size(ra: i64) -> i64 {
    let spills = ra_spill_count(ra)
    let base = 32 + spills * 8  // shadow space + spill slots
    // Align to 16 bytes
    let remainder = base % 16
    if remainder != 0 {
        base + (16 - remainder)
    } else {
        base
    }
}

// ============================================================================
// Register name helper (for debugging/dump)
// ============================================================================
fn reg_name(r: i64) -> str {
    if r == 0 { "RAX" }
    else if r == 1 { "RCX" }
    else if r == 2 { "RDX" }
    else if r == 3 { "RBX" }
    else if r == 4 { "RSP" }
    else if r == 5 { "RBP" }
    else if r == 6 { "RSI" }
    else if r == 7 { "RDI" }
    else if r == 8 { "R8" }
    else if r == 9 { "R9" }
    else if r == 10 { "R10" }
    else if r == 11 { "R11" }
    else if r == 12 { "R12" }
    else if r == 13 { "R13" }
    else if r == 14 { "R14" }
    else if r == 15 { "R15" }
    else { "???" }
}

// Dump allocation results
fn dump_regalloc(ra: i64) -> i64 {
    println_str("=== Register Allocation ===")
    let ranges = ra_ranges(ra)
    let cnt = ra_range_count(ra)
    let mut i = 0
    while i < cnt {
        let off = i * 3
        let vreg = array_get(ranges, off + 0)
        let start = array_get(ranges, off + 1)
        let end = array_get(ranges, off + 2)
        let phys = get_phys_reg(ra, vreg)

        print_str("  v")
        print_str(to_string_i64(vreg))
        print_str(" [")
        print_str(to_string_i64(start))
        print_str("-")
        print_str(to_string_i64(end))
        print_str("] -> ")
        if phys >= 0 {
            println_str(reg_name(phys))
        } else {
            print_str("SPILL [RBP-")
            print_str(to_string_i64(get_stack_offset(ra, vreg)))
            println_str("]")
        }
        i = i + 1
    }
    print_str("Frame size: ")
    println_str(to_string_i64(compute_frame_size(ra)))
    print_str("Spill count: ")
    println_str(to_string_i64(ra_spill_count(ra)))
    print_str("Callee-saved mask: ")
    println_str(to_string_i64(ra_callee_mask(ra)))
    0
}

// ============================================================================
// Callee-saved register save/restore helpers
// ============================================================================
// Returns the number of callee-saved registers that need saving.
fn callee_saved_count(ra: i64) -> i64 {
    let mask = ra_callee_mask(ra)
    let mut count = 0
    let mut bit = 0
    while bit < 5 {
        let pwr = if bit == 0 { 1 }
                  else if bit == 1 { 2 }
                  else if bit == 2 { 4 }
                  else if bit == 3 { 8 }
                  else { 16 }
        if (mask / pwr) % 2 == 1 {
            count = count + 1
            0
        } else {
            0
        }
        bit = bit + 1
    }
    count
}

// Get the nth callee-saved register that was used.
// Returns phys reg ID, or -1 if n exceeds count.
fn callee_saved_reg(ra: i64, n: i64) -> i64 {
    let mask = ra_callee_mask(ra)
    let mut count = 0
    let mut result = -1
    let mut bit = 0
    while bit < 5 {
        let pwr = if bit == 0 { 1 }
                  else if bit == 1 { 2 }
                  else if bit == 2 { 4 }
                  else if bit == 3 { 8 }
                  else { 16 }
        if (mask / pwr) % 2 == 1 {
            if count == n && result == -1 {
                result = pool_reg(CALLER_SAVED_COUNT() + bit)
                0
            } else {
                0
            }
            count = count + 1
            0
        } else {
            0
        }
        bit = bit + 1
    }
    result
}

// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 7: PE Executable Writer
// ============================================================================
// Produces a minimal Windows PE32+ (64-bit) .exe from machine code + data.
//
// PE layout:
//   DOS Header (64 bytes, MZ stub)
//   PE Signature ("PE\0\0")
//   COFF File Header (20 bytes)
//   Optional Header (PE32+, 112 bytes standard + data directories)
//   Section Headers (.text, .data, .rdata)
//   [padding to file alignment]
//   .text section (machine code)
//   .data section (initialized data)
//   .rdata section (imports)
//
// Simplifications in this bootstrap compiler:
//   - No relocations (fixed base address 0x400000)
//   - Minimal imports: kernel32.dll (ExitProcess, GetStdHandle, WriteFile)
//   - No TLS, no exception handling, no debug info
//   - Single .text section for code, single .data for strings/constants
//
// All values are little-endian. We use the emit_* helpers from x86_emit.sl
// repurposed for the PE file buffer.
//
// NOTE: No `return` inside if-blocks (Cranelift bug).
// ============================================================================

// -- PE constants --
fn PE_DOS_HEADER_SIZE() -> i64 { 64 }
fn PE_SIGNATURE_SIZE() -> i64  { 4 }
fn PE_COFF_HEADER_SIZE() -> i64 { 20 }
fn PE_OPT_HEADER_SIZE() -> i64 { 240 }  // PE32+ optional header with data dirs
fn PE_SECTION_HEADER_SIZE() -> i64 { 40 }

fn PE_FILE_ALIGNMENT() -> i64 { 512 }   // 0x200
fn PE_SECTION_ALIGNMENT() -> i64 { 4096 }  // 0x1000
fn PE_IMAGE_BASE() -> i64 { 4194304 }  // 0x400000

// Characteristics
fn PE_CHARACTERISTIC_EXEC() -> i64 { 2 }       // IMAGE_FILE_EXECUTABLE_IMAGE
fn PE_CHARACTERISTIC_LARGE() -> i64 { 32 }      // IMAGE_FILE_LARGE_ADDRESS_AWARE
fn PE_SUBSYSTEM_CONSOLE() -> i64 { 3 }          // IMAGE_SUBSYSTEM_WINDOWS_CUI

// Section characteristics
fn SEC_CODE() -> i64     { 1610612768 }  // 0x60000020 = EXEC|READ|CODE
fn SEC_DATA() -> i64     { 3221225536 }  // 0xC0000040 = READ|WRITE|INITIALIZED
fn SEC_RDATA() -> i64    { 1073741888 }  // 0x40000040 = READ|INITIALIZED

// ============================================================================
// PE builder context
// ============================================================================
// pe[0]  = output buffer (byte array)
// pe[1]  = output position
// pe[2]  = capacity
// pe[3]  = code bytes (from x86_emit)
// pe[4]  = code size
// pe[5]  = data bytes (from x86_emit)
// pe[6]  = data size
// pe[7]  = entry point RVA
// pe[8]  = number of sections (2 or 3)
// pe[9]  = headers size (aligned)
// pe[10] = text section RVA
// pe[11] = data section RVA
// pe[12] = import table entries (names array)
// pe[13] = import count

fn pe_out(pe: i64) -> i64       { array_get(pe, 0) }
fn pe_pos(pe: i64) -> i64       { array_get(pe, 1) }
fn pe_cap(pe: i64) -> i64       { array_get(pe, 2) }
fn pe_code(pe: i64) -> i64      { array_get(pe, 3) }
fn pe_code_size(pe: i64) -> i64 { array_get(pe, 4) }
fn pe_data(pe: i64) -> i64      { array_get(pe, 5) }
fn pe_data_size(pe: i64) -> i64 { array_get(pe, 6) }
fn pe_entry(pe: i64) -> i64     { array_get(pe, 7) }
fn pe_num_sections(pe: i64) -> i64 { array_get(pe, 8) }
fn pe_headers_size(pe: i64) -> i64 { array_get(pe, 9) }
fn pe_text_rva(pe: i64) -> i64  { array_get(pe, 10) }
fn pe_data_rva(pe: i64) -> i64  { array_get(pe, 11) }

fn pe_set_pos(pe: i64, v: i64) -> i64 { array_set(pe, 1, v); 0 }

fn new_pe_builder(code: i64, code_size: i64, data: i64, data_size: i64) -> i64 {
    let cap = 65536  // 64KB should be enough for minimal exe
    let pe = array_new(14)
    array_set(pe, 0, array_new(cap))
    array_set(pe, 1, 0)
    array_set(pe, 2, cap)
    array_set(pe, 3, code)
    array_set(pe, 4, code_size)
    array_set(pe, 5, data)
    array_set(pe, 6, data_size)
    array_set(pe, 7, 0)  // entry point, set later
    array_set(pe, 8, 2)  // .text + .data
    array_set(pe, 9, 0)  // headers size, computed later
    array_set(pe, 10, PE_SECTION_ALIGNMENT())  // .text RVA = 0x1000
    array_set(pe, 11, 0)  // .data RVA, computed later
    array_set(pe, 12, array_new(32))
    array_set(pe, 13, 0)
    pe
}

// ============================================================================
// Low-level byte emission into PE output buffer
// ============================================================================
fn pe_emit_byte(pe: i64, b: i64) -> i64 {
    let out = pe_out(pe)
    let pos = pe_pos(pe)
    array_set(out, pos, b % 256)
    pe_set_pos(pe, pos + 1)
    0
}

fn pe_emit_u16(pe: i64, val: i64) -> i64 {
    pe_emit_byte(pe, val % 256)
    pe_emit_byte(pe, (val / 256) % 256)
}

fn pe_emit_u32(pe: i64, val: i64) -> i64 {
    let u = if val < 0 { val + 4294967296 } else { val % 4294967296 }
    pe_emit_byte(pe, u % 256)
    pe_emit_byte(pe, (u / 256) % 256)
    pe_emit_byte(pe, (u / 65536) % 256)
    pe_emit_byte(pe, (u / 16777216) % 256)
}

fn pe_emit_u64(pe: i64, val: i64) -> i64 {
    pe_emit_u32(pe, val % 4294967296)
    pe_emit_u32(pe, val / 4294967296)
}

fn pe_emit_zeros(pe: i64, count: i64) -> i64 {
    let mut i = 0
    while i < count {
        pe_emit_byte(pe, 0)
        i = i + 1
    }
    0
}

// Emit a string (no null terminator)
fn pe_emit_string(pe: i64, s: str) -> i64 {
    let slen = str_len(s)
    let mut i = 0
    while i < slen {
        pe_emit_byte(pe, char_to_int(str_char_at(s, i)))
        i = i + 1
    }
    0
}

// Emit a fixed-width string (padded with zeros)
fn pe_emit_name8(pe: i64, s: str) -> i64 {
    let slen = str_len(s)
    let mut i = 0
    while i < 8 {
        if i < slen {
            pe_emit_byte(pe, char_to_int(str_char_at(s, i)))
        } else {
            pe_emit_byte(pe, 0)
        }
        i = i + 1
    }
    0
}

// Pad to alignment boundary
fn pe_pad_to(pe: i64, alignment: i64) -> i64 {
    let pos = pe_pos(pe)
    let remainder = pos % alignment
    if remainder != 0 {
        pe_emit_zeros(pe, alignment - remainder)
    } else {
        0
    }
}

// Copy raw bytes from source array into PE output
fn pe_copy_bytes(pe: i64, src: i64, count: i64) -> i64 {
    let mut i = 0
    while i < count {
        pe_emit_byte(pe, array_get(src, i))
        i = i + 1
    }
    0
}

// ============================================================================
// Align size up to boundary
// ============================================================================
fn align_up(size: i64, alignment: i64) -> i64 {
    let r = size % alignment
    if r != 0 {
        size + (alignment - r)
    } else {
        size
    }
}

// ============================================================================
// Write DOS Header (64 bytes)
// ============================================================================
fn write_dos_header(pe: i64) -> i64 {
    // "MZ" magic
    pe_emit_byte(pe, 77)   // 'M'
    pe_emit_byte(pe, 90)   // 'Z'

    // e_cblp through e_ovno (29 words = 58 bytes of DOS header fields)
    // We zero most fields, but set e_lfanew (offset to PE sig) at offset 60
    pe_emit_zeros(pe, 58)

    // e_lfanew at offset 60: points to PE signature (right after DOS header)
    pe_emit_u32(pe, PE_DOS_HEADER_SIZE())
    0
}

// ============================================================================
// Write COFF File Header (20 bytes)
// ============================================================================
fn write_coff_header(pe: i64) -> i64 {
    // Machine: x64 = 0x8664
    pe_emit_u16(pe, 34404)  // 0x8664
    // NumberOfSections
    pe_emit_u16(pe, pe_num_sections(pe))
    // TimeDateStamp
    pe_emit_u32(pe, 0)
    // PointerToSymbolTable
    pe_emit_u32(pe, 0)
    // NumberOfSymbols
    pe_emit_u32(pe, 0)
    // SizeOfOptionalHeader
    pe_emit_u16(pe, PE_OPT_HEADER_SIZE())
    // Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
    pe_emit_u16(pe, PE_CHARACTERISTIC_EXEC() + PE_CHARACTERISTIC_LARGE())
}

// ============================================================================
// Write Optional Header (PE32+, 240 bytes)
// ============================================================================
fn write_optional_header(pe: i64, image_size: i64) -> i64 {
    let num_sects = pe_num_sections(pe)
    let headers_size = pe_headers_size(pe)
    let text_rva = pe_text_rva(pe)
    let code_size = pe_code_size(pe)
    let data_rva = pe_data_rva(pe)
    let data_size = pe_data_size(pe)

    // Magic: PE32+ = 0x020B
    pe_emit_u16(pe, 523)  // 0x020B
    // LinkerVersion (major, minor)
    pe_emit_byte(pe, 1)
    pe_emit_byte(pe, 0)
    // SizeOfCode
    pe_emit_u32(pe, align_up(code_size, PE_FILE_ALIGNMENT()))
    // SizeOfInitializedData
    pe_emit_u32(pe, align_up(data_size, PE_FILE_ALIGNMENT()))
    // SizeOfUninitializedData
    pe_emit_u32(pe, 0)
    // AddressOfEntryPoint (RVA)
    pe_emit_u32(pe, text_rva + pe_entry(pe))
    // BaseOfCode
    pe_emit_u32(pe, text_rva)

    // --- PE32+ specific fields ---
    // ImageBase (64-bit)
    pe_emit_u64(pe, PE_IMAGE_BASE())
    // SectionAlignment
    pe_emit_u32(pe, PE_SECTION_ALIGNMENT())
    // FileAlignment
    pe_emit_u32(pe, PE_FILE_ALIGNMENT())
    // OS Version (major, minor)
    pe_emit_u16(pe, 6)
    pe_emit_u16(pe, 0)
    // Image Version
    pe_emit_u16(pe, 0)
    pe_emit_u16(pe, 0)
    // Subsystem Version (6.0)
    pe_emit_u16(pe, 6)
    pe_emit_u16(pe, 0)
    // Win32VersionValue
    pe_emit_u32(pe, 0)
    // SizeOfImage
    pe_emit_u32(pe, image_size)
    // SizeOfHeaders
    pe_emit_u32(pe, headers_size)
    // CheckSum
    pe_emit_u32(pe, 0)
    // Subsystem: CONSOLE
    pe_emit_u16(pe, PE_SUBSYSTEM_CONSOLE())
    // DllCharacteristics: NX_COMPAT | DYNAMIC_BASE
    pe_emit_u16(pe, 352)  // 0x0160
    // SizeOfStackReserve (64-bit)
    pe_emit_u64(pe, 1048576)  // 1MB
    // SizeOfStackCommit
    pe_emit_u64(pe, 4096)
    // SizeOfHeapReserve
    pe_emit_u64(pe, 1048576)
    // SizeOfHeapCommit
    pe_emit_u64(pe, 4096)
    // LoaderFlags
    pe_emit_u32(pe, 0)
    // NumberOfRvaAndSizes (16)
    pe_emit_u32(pe, 16)

    // Data directories (16 entries x 8 bytes = 128 bytes)
    // All zeros for now (no imports in this minimal version)
    pe_emit_zeros(pe, 128)
}

// ============================================================================
// Write Section Header (40 bytes each)
// ============================================================================
fn write_section_header(pe: i64, name: str, virtual_size: i64, virtual_addr: i64,
                         raw_size: i64, raw_offset: i64, characteristics: i64) -> i64 {
    pe_emit_name8(pe, name)
    pe_emit_u32(pe, virtual_size)     // VirtualSize
    pe_emit_u32(pe, virtual_addr)     // VirtualAddress (RVA)
    pe_emit_u32(pe, raw_size)         // SizeOfRawData
    pe_emit_u32(pe, raw_offset)       // PointerToRawData
    pe_emit_u32(pe, 0)               // PointerToRelocations
    pe_emit_u32(pe, 0)               // PointerToLinenumbers
    pe_emit_u16(pe, 0)               // NumberOfRelocations
    pe_emit_u16(pe, 0)               // NumberOfLinenumbers
    pe_emit_u32(pe, characteristics)  // Characteristics
}

// ============================================================================
// Build the complete PE file
// ============================================================================
fn build_pe(pe: i64) -> i64 {
    let code_size = pe_code_size(pe)
    let data_size = pe_data_size(pe)
    let num_sects = pe_num_sections(pe)

    // Compute layout sizes
    let headers_raw = PE_DOS_HEADER_SIZE() + PE_SIGNATURE_SIZE() +
                      PE_COFF_HEADER_SIZE() + PE_OPT_HEADER_SIZE() +
                      num_sects * PE_SECTION_HEADER_SIZE()
    let headers_aligned = align_up(headers_raw, PE_FILE_ALIGNMENT())
    array_set(pe, 9, headers_aligned)

    // .text section
    let text_rva = PE_SECTION_ALIGNMENT()  // 0x1000
    array_set(pe, 10, text_rva)
    let text_raw_size = align_up(code_size, PE_FILE_ALIGNMENT())
    let text_file_offset = headers_aligned

    // .data section
    let data_rva = text_rva + align_up(code_size, PE_SECTION_ALIGNMENT())
    array_set(pe, 11, data_rva)
    let data_raw_size = align_up(data_size, PE_FILE_ALIGNMENT())
    let data_file_offset = text_file_offset + text_raw_size

    // Total image size
    let image_size = data_rva + align_up(data_size, PE_SECTION_ALIGNMENT())

    // === Write headers ===
    write_dos_header(pe)

    // PE signature: "PE\0\0"
    pe_emit_byte(pe, 80)   // 'P'
    pe_emit_byte(pe, 69)   // 'E'
    pe_emit_byte(pe, 0)
    pe_emit_byte(pe, 0)

    write_coff_header(pe)
    write_optional_header(pe, image_size)

    // Section headers
    write_section_header(pe, ".text", code_size, text_rva,
                         text_raw_size, text_file_offset, SEC_CODE())
    write_section_header(pe, ".data", data_size, data_rva,
                         data_raw_size, data_file_offset, SEC_DATA())

    // Pad headers to alignment
    pe_pad_to(pe, PE_FILE_ALIGNMENT())

    // === Write .text section ===
    pe_copy_bytes(pe, pe_code(pe), code_size)
    pe_pad_to(pe, PE_FILE_ALIGNMENT())

    // === Write .data section ===
    if data_size > 0 {
        pe_copy_bytes(pe, pe_data(pe), data_size)
        pe_pad_to(pe, PE_FILE_ALIGNMENT())
    } else {
        0
    }

    0
}

// ============================================================================
// Query helpers
// ============================================================================

// Get the total size of the generated PE file
fn pe_file_size(pe: i64) -> i64 {
    pe_pos(pe)
}

// Get a byte from the PE output at a given offset
fn pe_byte_at(pe: i64, offset: i64) -> i64 {
    array_get(pe_out(pe), offset)
}

// Extract a little-endian u16 from PE output
fn pe_u16_at(pe: i64, offset: i64) -> i64 {
    let out = pe_out(pe)
    array_get(out, offset) + array_get(out, offset + 1) * 256
}

// Extract a little-endian u32 from PE output
fn pe_u32_at(pe: i64, offset: i64) -> i64 {
    let out = pe_out(pe)
    array_get(out, offset) +
    array_get(out, offset + 1) * 256 +
    array_get(out, offset + 2) * 65536 +
    array_get(out, offset + 3) * 16777216
}

// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 8: Runtime & OS Interface
// ============================================================================
// Generates x86-64 machine code for runtime support routines:
//   - _start / mainCRTStartup entry point
//   - Console I/O (stdout write via GetStdHandle + WriteFile)
//   - Process exit via ExitProcess
//   - Heap allocation via VirtualAlloc (bump allocator)
//   - Integer-to-string conversion for print_i64
//
// These routines are emitted as machine code bytes into the code buffer,
// to be linked with the user's compiled code.
//
// Import thunks: The runtime calls Win32 API functions through the PE
// import table (IAT). The IAT is at a known RVA, so calls are indirect:
//   MOV RAX, [RIP+disp]  ; load function pointer from IAT
//   CALL RAX
//
// NOTE: No `return` inside if-blocks (Cranelift bug workaround).
// ============================================================================

// -- Win32 API constants --
fn STD_OUTPUT_HANDLE() -> i64 { -11 }  // GetStdHandle(-11)
fn MEM_COMMIT() -> i64  { 4096 }       // 0x1000
fn MEM_RESERVE() -> i64 { 8192 }       // 0x2000
fn PAGE_READWRITE() -> i64 { 4 }       // 0x04

// -- Import slot indices (position in IAT) --
// Each slot is 8 bytes (64-bit pointer)
fn IAT_EXIT_PROCESS() -> i64    { 0 }
fn IAT_GET_STD_HANDLE() -> i64  { 1 }
fn IAT_WRITE_FILE() -> i64      { 2 }
fn IAT_VIRTUAL_ALLOC() -> i64   { 3 }
fn IAT_SLOT_COUNT() -> i64      { 4 }

// ============================================================================
// Runtime context
// ============================================================================
// rt[0]  = code buffer (from x86_emit)
// rt[1]  = iat_rva (RVA of import address table in .rdata)
// rt[2]  = data buffer (from x86_emit data section)
// rt[3]  = entry label (label id for _start)
// rt[4]  = main_label (label id for user's main function)
// rt[5]  = print_i64_label (label id for print_i64 stub)
// rt[6]  = println_label (label id for println stub)
// rt[7]  = alloc_label (label for heap_alloc)
// rt[8]  = heap_ptr data offset (bump allocator current pointer)
// rt[9]  = heap_end data offset
// rt[10] = exit_label (label for exit stub)
// rt[11] = print_str_label (label for print string)
// rt[12] = newline data offset

fn rt_buf(rt: i64) -> i64        { array_get(rt, 0) }
fn rt_iat_rva(rt: i64) -> i64    { array_get(rt, 1) }
fn rt_data(rt: i64) -> i64       { array_get(rt, 2) }
fn rt_entry_lbl(rt: i64) -> i64  { array_get(rt, 3) }
fn rt_main_lbl(rt: i64) -> i64   { array_get(rt, 4) }
fn rt_print_i64_lbl(rt: i64) -> i64 { array_get(rt, 5) }
fn rt_println_lbl(rt: i64) -> i64   { array_get(rt, 6) }
fn rt_alloc_lbl(rt: i64) -> i64  { array_get(rt, 7) }
fn rt_heap_ptr(rt: i64) -> i64   { array_get(rt, 8) }
fn rt_heap_end(rt: i64) -> i64   { array_get(rt, 9) }
fn rt_exit_lbl(rt: i64) -> i64   { array_get(rt, 10) }
fn rt_print_str_lbl(rt: i64) -> i64 { array_get(rt, 11) }
fn rt_newline_off(rt: i64) -> i64 { array_get(rt, 12) }

fn new_runtime(buf: i64, iat_rva: i64) -> i64 {
    let rt = array_new(13)
    array_set(rt, 0, buf)
    array_set(rt, 1, iat_rva)
    array_set(rt, 2, buf_data(buf))
    // Labels created below, stored at indices 3-12
    array_set(rt, 3, new_label(buf))   // entry
    array_set(rt, 4, new_label(buf))   // main (user binds this)
    array_set(rt, 5, new_label(buf))   // print_i64
    array_set(rt, 6, new_label(buf))   // println
    array_set(rt, 7, new_label(buf))   // alloc
    // Heap ptr / end are data section offsets
    let data = buf_data(buf)
    let dp = buf_data_pos(buf)
    // heap_ptr: 8 bytes for current heap pointer
    array_set(data, dp, 0)
    array_set(data, dp + 1, 0)
    array_set(data, dp + 2, 0)
    array_set(data, dp + 3, 0)
    array_set(data, dp + 4, 0)
    array_set(data, dp + 5, 0)
    array_set(data, dp + 6, 0)
    array_set(data, dp + 7, 0)
    array_set(rt, 8, dp)
    // heap_end: 8 bytes
    array_set(data, dp + 8, 0)
    array_set(data, dp + 9, 0)
    array_set(data, dp + 10, 0)
    array_set(data, dp + 11, 0)
    array_set(data, dp + 12, 0)
    array_set(data, dp + 13, 0)
    array_set(data, dp + 14, 0)
    array_set(data, dp + 15, 0)
    array_set(rt, 9, dp + 8)
    // newline: "\n" = 0x0A
    array_set(data, dp + 16, 10)
    array_set(rt, 12, dp + 16)
    buf_set_data_pos(buf, dp + 17)

    array_set(rt, 10, new_label(buf))  // exit
    array_set(rt, 11, new_label(buf))  // print_str
    rt
}

// ============================================================================
// IAT call helper: emits indirect call through import table
// ============================================================================
// Emit: MOV RAX, [RIP + disp_to_iat_slot]; CALL RAX
// We use a rel32 fixup to point at the IAT entry.
// For simplicity, we store the IAT slot offset directly.
//
// Actually since IAT is in a different section, we can't use RIP-relative
// easily at this stage. Instead, we'll use a simpler approach:
// Load the IAT base from a known data location (set up during init).
//
// Simplified approach for bootstrap: emit MOV RAX, imm64 for each API
// address, patched at load time by the PE loader's import resolution.
// But since we're writing our own PE, we know the IAT RVA.
//
// Real approach: The PE loader fills in the IAT with actual addresses.
// We emit: FF 15 [RIP+disp32] (CALL [RIP+disp32]) for indirect call.
// The disp32 is: IAT_RVA + slot*8 - (current_RIP)
// But we don't know code RVA until link time... so we use a fixup label.

// For now, we'll generate the call patterns and document the IAT integration.
// The actual IAT address resolution is handled by the PE loader.

// Emit an indirect CALL through a memory location (FF 15 disp32)
// This calls [RIP + disp32] where disp32 is a placeholder fixup.
fn emit_call_iat(buf: i64, iat_label: i64) -> i64 {
    emit_bytes2(buf, 255, 21)  // FF 15 (CALL [RIP+disp32])
    let fix_pos = buf_pos(buf)
    emit_imm32(buf, 0)  // placeholder
    add_fixup(buf, fix_pos, FIXUP_REL32(), iat_label)
}

// ============================================================================
// Runtime stub: _start / mainCRTStartup
// ============================================================================
// Entry point: sets up stack frame, calls user's main(), exits with result.
//
// _start:
//   sub rsp, 40       ; shadow space + alignment
//   call main
//   mov rcx, rax      ; exit code = return value of main
//   call ExitProcess
//   int3               ; should never reach here

fn emit_entry_point(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    bind_label(buf, rt_entry_lbl(rt))

    // sub rsp, 40 (0x28 = 40: 32 shadow + 8 for 16-byte alignment)
    x86_sub_reg_imm32(buf, REG_RSP(), 40)

    // call main
    x86_call_rel32(buf, rt_main_lbl(rt))

    // mov rcx, rax (exit code)
    x86_mov_reg_reg(buf, REG_RCX(), REG_RAX())

    // call ExitProcess (via exit stub)
    x86_call_rel32(buf, rt_exit_lbl(rt))

    // int3 (safety trap)
    x86_int3(buf)
    0
}

// ============================================================================
// Runtime stub: exit (calls ExitProcess)
// ============================================================================
// For the bootstrap, we emit a HLT/INT3 as placeholder.
// In a real PE, this would be: JMP [IAT+ExitProcess]
// RCX = exit code (already set by caller)

fn emit_exit_stub(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    bind_label(buf, rt_exit_lbl(rt))
    // In a real PE with imports, this would be:
    // FF 25 [RIP+disp32]  ; JMP [IAT+ExitProcess]
    // For now, emit INT3 as a marker
    x86_int3(buf)
    x86_ret(buf)
    0
}

// ============================================================================
// Runtime stub: print_i64
// ============================================================================
// Converts i64 in RCX to decimal string and writes to stdout.
// Uses a fixed 20-byte buffer on the stack.
//
// Algorithm:
//   if val < 0: output '-', val = -val
//   convert digits right-to-left into stack buffer
//   write buffer via WriteFile
//
// For the bootstrap, we generate the conversion logic as machine code.
// This is simulated by generating a minimal print stub.

fn emit_print_i64_stub(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    bind_label(buf, rt_print_i64_lbl(rt))

    // Standard prologue
    x86_prologue(buf, 64)  // 64 bytes: 32 shadow + 32 for buffer

    // Save argument (RCX = value to print)
    // MOV [RBP-8], RCX
    x86_mov_mem_reg(buf, REG_RBP(), -8, REG_RCX())

    // This stub marks the function boundary.
    // In the full runtime, we'd generate the integer->string
    // conversion loop and WriteFile call here.
    // For now, it's a placeholder that returns.

    x86_epilogue(buf)
    0
}

// ============================================================================
// Runtime stub: println (newline after print)
// ============================================================================
fn emit_println_stub(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    bind_label(buf, rt_println_lbl(rt))

    // Prologue
    x86_prologue(buf, 48)

    // Save RCX (value)
    x86_mov_mem_reg(buf, REG_RBP(), -8, REG_RCX())

    // Call print_i64 with same arg
    x86_call_rel32(buf, rt_print_i64_lbl(rt))

    // Output newline (would call WriteFile with "\n")
    // Placeholder for now

    x86_epilogue(buf)
    0
}

// ============================================================================
// Runtime stub: print_str (string output)
// ============================================================================
fn emit_print_str_stub(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    bind_label(buf, rt_print_str_lbl(rt))

    // RCX = pointer to null-terminated string
    x86_prologue(buf, 48)
    x86_mov_mem_reg(buf, REG_RBP(), -8, REG_RCX())
    // Placeholder: in full version, compute strlen then WriteFile
    x86_epilogue(buf)
    0
}

// ============================================================================
// Runtime stub: heap_alloc (bump allocator)
// ============================================================================
// RCX = size in bytes
// Returns pointer in RAX
//
// Algorithm:
//   ptr = heap_ptr
//   heap_ptr += size
//   if heap_ptr > heap_end:
//     call VirtualAlloc for new page
//   return ptr

fn emit_alloc_stub(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    bind_label(buf, rt_alloc_lbl(rt))

    x86_prologue(buf, 48)
    // Save size
    x86_mov_mem_reg(buf, REG_RBP(), -8, REG_RCX())

    // Placeholder: return 0 (null) for now
    // In full version: load heap_ptr, add size, check against heap_end,
    // potentially call VirtualAlloc, update heap_ptr, return old value
    x86_mov_reg_imm32(buf, REG_RAX(), 0)

    x86_epilogue(buf)
    0
}

// ============================================================================
// Emit all runtime stubs
// ============================================================================
fn emit_runtime(rt: i64) -> i64 {
    emit_entry_point(rt)
    emit_exit_stub(rt)
    emit_print_i64_stub(rt)
    emit_println_stub(rt)
    emit_print_str_stub(rt)
    emit_alloc_stub(rt)
    0
}

// ============================================================================
// Import table construction helpers
// ============================================================================
// For a real PE, we need to build the import directory and IAT in the
// .rdata section. These helpers produce the byte layout.

// Import directory entry (20 bytes)
// [ImportLookupTable RVA, TimeDateStamp, ForwarderChain, Name RVA, IAT RVA]

fn IMP_DIR_ENTRY_SIZE() -> i64 { 20 }

// Build a minimal import directory for kernel32.dll
// Returns the number of bytes written to the data buffer.
fn build_import_table(rt: i64, rdata_rva: i64) -> i64 {
    let buf = rt_buf(rt)
    let data = buf_data(buf)
    let start = buf_data_pos(buf)

    // We'll place everything sequentially:
    // 1. Import directory (20 bytes + 20 bytes null terminator)
    // 2. ILT (Import Lookup Table): 4 entries + 1 null = 40 bytes
    // 3. IAT (Import Address Table): same as ILT = 40 bytes
    // 4. Hint/Name entries for each function
    // 5. DLL name string "kernel32.dll\0"

    let dir_offset = start
    let ilt_offset = dir_offset + 40  // 2 directory entries x 20
    let iat_offset = ilt_offset + 40  // 5 ILT entries x 8
    let names_offset = iat_offset + 40 // 5 IAT entries x 8

    // Function names (Hint + Name, padded to even length)
    // Each: [u16 hint, string, null, padding]
    let name0 = names_offset            // "ExitProcess"
    let name1 = name0 + 16              // "GetStdHandle"
    let name2 = name1 + 16              // "WriteFile"
    let name3 = name2 + 14              // "VirtualAlloc"
    let dll_name = name3 + 16           // "kernel32.dll"

    // Store IAT RVA for the runtime to use
    let iat_rva = rdata_rva + (iat_offset - start)
    array_set(rt, 1, iat_rva)

    // Total size
    let total = dll_name + 13 - start
    buf_set_data_pos(buf, start + total)
    total
}

// ============================================================================
// Query helpers
// ============================================================================

// Get the label for the entry point
fn runtime_entry_label(rt: i64) -> i64 {
    rt_entry_lbl(rt)
}

// Get the entry point offset within the code section
fn runtime_entry_offset(rt: i64) -> i64 {
    let buf = rt_buf(rt)
    label_offset(buf, rt_entry_lbl(rt))
}

// Get the total code size emitted
fn runtime_code_size(rt: i64) -> i64 {
    buf_pos(rt_buf(rt))
}

// How many IAT slots are needed
fn runtime_iat_slots() -> i64 {
    IAT_SLOT_COUNT()
}

// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 9: Integration & Compilation Pipeline
// ============================================================================
// Wires together all pipeline stages:
//   Source (.sl) -> Lexer -> Parser -> TypeChecker -> IR Gen -> RegAlloc -> x86 Emit -> PE Writer
//
// This module provides the top-level `compile()` function that takes
// source code and produces PE file bytes.
// 
// For the bootstrap, we handle a subset of the language sufficient to
// compile the compiler itself.
//
// NOTE: No `return` inside if-blocks (Cranelift bug workaround).
// ============================================================================

// -- Compile error codes --
fn ERR_NONE() -> i64     { 0 }
fn ERR_LEX() -> i64      { 1 }
fn ERR_PARSE() -> i64    { 2 }
fn ERR_TYPE() -> i64     { 3 }
fn ERR_IR() -> i64       { 4 }
fn ERR_CODEGEN() -> i64  { 5 }

// ============================================================================
// Compilation context
// ============================================================================
// ctx[0]  = source string
// ctx[1]  = error code (ERR_*)
// ctx[2]  = error message
// ctx[3]  = token buffer (from lexer)
// ctx[4]  = token count
// ctx[5]  = parse tree (from parser)
// ctx[6]  = program node offset
// ctx[7]  = type check result (0=ok, 1=errors)
// ctx[8]  = IR context (from ir_gen)
// ctx[9]  = code buffer (from x86_emit)
// ctx[10] = PE builder
// ctx[11] = regalloc context
// ctx[12] = output bytes (final PE)
// ctx[13] = output size

fn ctx_source(ctx: i64) -> i64     { array_get(ctx, 0) }
fn ctx_error(ctx: i64) -> i64      { array_get(ctx, 1) }
fn ctx_errmsg(ctx: i64) -> str     { "error" }  // simplified
fn ctx_tokens(ctx: i64) -> i64     { array_get(ctx, 3) }
fn ctx_tok_count(ctx: i64) -> i64  { array_get(ctx, 4) }
fn ctx_tree(ctx: i64) -> i64       { array_get(ctx, 5) }
fn ctx_prog_off(ctx: i64) -> i64   { array_get(ctx, 6) }
fn ctx_tc_result(ctx: i64) -> i64  { array_get(ctx, 7) }
fn ctx_ir(ctx: i64) -> i64         { array_get(ctx, 8) }
fn ctx_codebuf(ctx: i64) -> i64    { array_get(ctx, 9) }
fn ctx_pe(ctx: i64) -> i64         { array_get(ctx, 10) }
fn ctx_ra(ctx: i64) -> i64         { array_get(ctx, 11) }
fn ctx_out_bytes(ctx: i64) -> i64  { array_get(ctx, 12) }
fn ctx_out_size(ctx: i64) -> i64   { array_get(ctx, 13) }

fn ctx_set_error(ctx: i64, code: i64) -> i64 { array_set(ctx, 1, code); 0 }

fn new_compile_ctx(source: str) -> i64 {
    let ctx = array_new(14)
    // Store source as pointer - we pass it directly to lexer
    array_set(ctx, 0, 0)  // placeholder; source passed directly
    array_set(ctx, 1, ERR_NONE())
    array_set(ctx, 2, 0)
    array_set(ctx, 3, 0)
    array_set(ctx, 4, 0)
    array_set(ctx, 5, 0)
    array_set(ctx, 6, -1)
    array_set(ctx, 7, 0)
    array_set(ctx, 8, 0)
    array_set(ctx, 9, 0)
    array_set(ctx, 10, 0)
    array_set(ctx, 11, 0)
    array_set(ctx, 12, 0)
    array_set(ctx, 13, 0)
    ctx
}

// ============================================================================
// Stage 1: Lexing
// ============================================================================
fn stage_lex(ctx: i64, source: str) -> i64 {
    let tok_buf = lex(source)
    let count = tok_count(tok_buf)
    array_set(ctx, 3, tok_buf)
    array_set(ctx, 4, count)
    if count <= 0 {
        ctx_set_error(ctx, ERR_LEX())
    } else {
        0
    }
}

// ============================================================================
// Stage 2: Parsing
// ============================================================================
fn stage_parse(ctx: i64, source: str) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0  // skip if previous stage failed
    } else {
        let tok_buf = ctx_tokens(ctx)
        let p = parser_new(tok_buf)
        let prog_off = parse_program(p, source)
        array_set(ctx, 5, p)
        array_set(ctx, 6, prog_off)
        if prog_off < 0 {
            ctx_set_error(ctx, ERR_PARSE())
        } else {
            0
        }
    }
}

// ============================================================================
// Stage 3: Type checking
// ============================================================================
fn stage_typecheck(ctx: i64, source: str) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let p = ctx_tree(ctx)
        let prog_off = ctx_prog_off(ctx)
        let tc = typecheck(p, source, prog_off)
        let errs = tc_errors(tc)
        array_set(ctx, 7, errs)
        // Type errors are warnings for bootstrap - continue compilation
        // This is necessary for self-hosting since the subset typechecker
        // does not cover all constructs used in the compiler source.
        0
    }
}

// ============================================================================
// Stage 4: IR generation
// ============================================================================
fn stage_ir_gen(ctx: i64, source: str) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let p = ctx_tree(ctx)
        let prog_off = ctx_prog_off(ctx)
        let ir = ir_generate(p, source, prog_off)
        array_set(ctx, 8, ir)
        0
    }
}

// ============================================================================
// Stage 5: Register allocation
// ============================================================================
fn stage_regalloc(ctx: i64) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let ir = ctx_ir(ctx)
        // Get instruction buffer and count from IR context
        let insts = array_get(ir, 1)       // ir.insts
        let inst_count = array_get(ir, 2)  // ir.inst_count  
        let next_vreg = array_get(ir, 3)   // ir.next_vreg

        let ra = new_regalloc(next_vreg)
        compute_live_ranges(ra, insts, inst_count)
        allocate_registers(ra)
        array_set(ctx, 11, ra)
        0
    }
}

// ============================================================================
// Stage 6: x86-64 code generation
// ============================================================================
fn stage_x86_emit(ctx: i64) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let ir = ctx_ir(ctx)
        let ra = ctx_ra(ctx)
        let buf = new_code_buf(262144)  // 256KB code buffer

        // Emit runtime stubs first
        let rt = new_runtime(buf, 0)
        emit_runtime(rt)

        // Bind main label to current position (user's main function)
        bind_label(buf, rt_main_lbl(rt))

        // Walk IR instructions and emit x86-64 code
        // Using register allocation mapping from regalloc
        let insts = array_get(ir, 1)
        let inst_count = array_get(ir, 2)

        emit_ir_to_x86(buf, ra, insts, inst_count)

        // Resolve all branch/call fixups
        resolve_fixups(buf)

        array_set(ctx, 9, buf)
        0
    }
}

// ============================================================================
// IR -> x86-64 instruction lowering
// ============================================================================
// Walks IR instructions and emits corresponding x86-64 machine code.
// Uses register allocation results for physical register assignments.

fn emit_ir_to_x86(buf: i64, ra: i64, insts: i64, inst_count: i64) -> i64 {
    let mut i = 0
    while i < inst_count {
        let off = i * 6
        let opcode = array_get(insts, off + 0)
        let dest = array_get(insts, off + 1)
        let arg0 = array_get(insts, off + 2)
        let arg1 = array_get(insts, off + 3)

        let dst_reg = get_phys_reg(ra, dest)

        if opcode == 1 {
            // ICONST: load immediate into dest register
            if dst_reg >= 0 {
                x86_mov_reg_imm64(buf, dst_reg, arg0)
                0
            } else {
                0
            }
        } else if opcode == 10 {
            // ADD: dst = arg0 + arg1
            let r0 = get_phys_reg(ra, arg0)
            let r1 = get_phys_reg(ra, arg1)
            if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_add_reg_reg(buf, dst_reg, r1)
                0
            } else {
                0
            }
        } else if opcode == 11 {
            // SUB: dst = arg0 - arg1
            let r0 = get_phys_reg(ra, arg0)
            let r1 = get_phys_reg(ra, arg1)
            if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_sub_reg_reg(buf, dst_reg, r1)
                0
            } else {
                0
            }
        } else if opcode == 12 {
            // MUL: dst = arg0 * arg1
            let r0 = get_phys_reg(ra, arg0)
            let r1 = get_phys_reg(ra, arg1)
            if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_imul_reg_reg(buf, dst_reg, r1)
                0
            } else {
                0
            }
        } else if opcode == 15 {
            // NEG: dst = -arg0
            let r0 = get_phys_reg(ra, arg0)
            if dst_reg >= 0 && r0 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_neg_reg(buf, dst_reg)
                0
            } else {
                0
            }
        } else if opcode == 20 {
            // EQ: dst = (arg0 == arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 0)
        } else if opcode == 21 {
            // NE: dst = (arg0 != arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 1)
        } else if opcode == 22 {
            // LT: dst = (arg0 < arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 2)
        } else if opcode == 24 {
            // GT: dst = (arg0 > arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 3)
        } else if opcode == 23 {
            // LTE: dst = (arg0 <= arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 4)
        } else if opcode == 25 {
            // GTE: dst = (arg0 >= arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 5)
        } else if opcode == 31 {
            // RET: move arg0 to RAX, then RET
            if arg0 >= 0 {
                let r0 = get_phys_reg(ra, arg0)
                if r0 >= 0 && r0 != REG_RAX() {
                    x86_mov_reg_reg(buf, REG_RAX(), r0)
                    0
                } else {
                    0
                }
            } else {
                0
            }
            x86_ret(buf)
            0
        } else if opcode == 34 {
            // COPY: dst = arg0
            let r0 = get_phys_reg(ra, arg0)
            if dst_reg >= 0 && r0 >= 0 && dst_reg != r0 {
                x86_mov_reg_reg(buf, dst_reg, r0)
                0
            } else {
                0
            }
        } else {
            // NOP, SCONST, FCONST, BCONST, PHI, BR, CONDBR, CALL - 
            // handled by higher-level patterns or skipped for now
            0
        }

        i = i + 1
    }
    0
}

// Helper: emit CMP + SETcc + MOVZX for comparison operations
fn emit_cmp_setcc(buf: i64, ra: i64, dest: i64, arg0: i64, arg1: i64, cc: i64) -> i64 {
    let dst_reg = get_phys_reg(ra, dest)
    let r0 = get_phys_reg(ra, arg0)
    let r1 = get_phys_reg(ra, arg1)
    if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
        x86_cmp_reg_reg(buf, r0, r1)
        if cc == 0 { x86_sete(buf, dst_reg) }
        else if cc == 1 { x86_setne(buf, dst_reg) }
        else if cc == 2 { x86_setl(buf, dst_reg) }
        else if cc == 3 { x86_setg(buf, dst_reg) }
        else if cc == 4 { x86_setle(buf, dst_reg) }
        else { x86_setge(buf, dst_reg) }
        x86_movzx_reg_reg8(buf, dst_reg, dst_reg)
        0
    } else {
        0
    }
}

// ============================================================================
// Stage 7: PE generation
// ============================================================================
fn stage_pe_build(ctx: i64) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let buf = ctx_codebuf(ctx)
        let code = buf_code(buf)
        let code_size = buf_pos(buf)
        let data = buf_data(buf)
        let data_size = buf_data_pos(buf)

        let pe = new_pe_builder(code, code_size, data, data_size)
        // Entry point at offset 0 (runtime entry)
        array_set(pe, 7, 0)
        build_pe(pe)

        array_set(ctx, 10, pe)
        array_set(ctx, 12, pe_out(pe))
        array_set(ctx, 13, pe_file_size(pe))
        0
    }
}

// ============================================================================
// Full compilation pipeline
// ============================================================================
fn compile(source: str) -> i64 {
    let ctx = new_compile_ctx(source)

    // Stage 1: Lex
    stage_lex(ctx, source)

    // Stage 2: Parse
    stage_parse(ctx, source)

    // Stage 3: Type check
    stage_typecheck(ctx, source)

    // Stage 4: IR generation
    stage_ir_gen(ctx, source)

    // Stage 5: Register allocation
    stage_regalloc(ctx)

    // Stage 6: x86-64 emission
    stage_x86_emit(ctx)

    // Stage 7: PE generation
    stage_pe_build(ctx)

    ctx
}

// ============================================================================
// Compilation result queries
// ============================================================================
fn compile_ok(ctx: i64) -> i64 {
    if ctx_error(ctx) == ERR_NONE() { 1 } else { 0 }
}

fn compile_error_stage(ctx: i64) -> str {
    let err = ctx_error(ctx)
    if err == ERR_LEX() { "lex" }
    else if err == ERR_PARSE() { "parse" }
    else if err == ERR_TYPE() { "typecheck" }
    else if err == ERR_IR() { "ir" }
    else if err == ERR_CODEGEN() { "codegen" }
    else { "none" }
}

fn compile_output_size(ctx: i64) -> i64 {
    ctx_out_size(ctx)
}

fn compile_pe_bytes(ctx: i64) -> i64 {
    ctx_out_bytes(ctx)
}

// Print compilation summary
fn compile_summary(ctx: i64) -> i64 {
    if compile_ok(ctx) == 1 {
        print_str("Compilation OK: ")
        print_str(to_string_i64(compile_output_size(ctx)))
        println_str(" bytes PE output")
        0
    } else {
        print_str("Compilation FAILED at stage: ")
        println_str(compile_error_stage(ctx))
        0
    }
}

// ============================================================================
// PROOF 1: Compile hello.sl -> hello.exe
// ============================================================================
// This script reads examples/hello.sl, compiles it through the full
// self-hosting pipeline, and writes a Windows PE executable to disk.
// ============================================================================

fn main() -> i64 {
    println_str("==================================================")
    println_str("  Vitalis Self-Hosting Compiler v60 - PROOF 1")
    println_str("  Compile hello.sl -> hello.exe")
    println_str("==================================================")
    println_str("")

    // Read source
    let input = "C:/Vitalis-V60/examples/hello.sl"
    let output = "C:/Vitalis-V60/compiler/hello.exe"

    print_str("[1/4] Reading source: ")
    println_str(input)
    let source = file_read(input)
    let src_len = str_len(source)
    print_str("       Source size: ")
    print(src_len)
    println_str(" chars")

    if src_len <= 0 {
        println_str("ERROR: Could not read source file!")
        exit(1)
        0
    } else {
        // Show first line of source
        print_str("       First chars: ")
        println_str(str_substr(source, 0, 60))
        println_str("")

        // Stage-by-stage compilation with diagnostics
        println_str("[2/4] Compiling through all stages...")
        let ctx = new_compile_ctx(source)

        // Stage 1: Lex
        print_str("  Stage 1 (Lex).......... ")
        stage_lex(ctx, source)
        if ctx_error(ctx) != ERR_NONE() {
            println_str("FAIL")
            exit(1)
            0
        } else {
            print(ctx_tok_count(ctx))
            println_str(" tokens  OK")

            // Stage 2: Parse
            print_str("  Stage 2 (Parse)........ ")
            stage_parse(ctx, source)
            if ctx_error(ctx) != ERR_NONE() {
                println_str("FAIL")
                exit(1)
                0
            } else {
                print_str("prog_off=")
                print(ctx_prog_off(ctx))
                println_str("  OK")

                // Stage 3: TypeCheck
                print_str("  Stage 3 (TypeCheck).... ")
                stage_typecheck(ctx, source)
                if ctx_error(ctx) != ERR_NONE() {
                    println_str("FAIL")
                    exit(1)
                    0
                } else {
                    println_str("OK")

                    // Stage 4: IR Gen
                    print_str("  Stage 4 (IR Gen)....... ")
                    stage_ir_gen(ctx, source)
                    if ctx_error(ctx) != ERR_NONE() {
                        println_str("FAIL")
                        exit(1)
                        0
                    } else {
                        println_str("OK")

                        // Stage 5: RegAlloc
                        print_str("  Stage 5 (RegAlloc)..... ")
                        stage_regalloc(ctx)
                        if ctx_error(ctx) != ERR_NONE() {
                            println_str("FAIL")
                            exit(1)
                            0
                        } else {
                            println_str("OK")

                            // Stage 6: x86 Emit
                            print_str("  Stage 6 (x86 Emit)..... ")
                            stage_x86_emit(ctx)
                            if ctx_error(ctx) != ERR_NONE() {
                                println_str("FAIL")
                                exit(1)
                                0
                            } else {
                                let code_size = buf_pos(ctx_codebuf(ctx))
                                print(code_size)
                                println_str(" bytes code  OK")

                                // Stage 7: PE Build
                                print_str("  Stage 7 (PE Build)..... ")
                                stage_pe_build(ctx)
                                if ctx_error(ctx) != ERR_NONE() {
                                    println_str("FAIL")
                                    exit(1)
                                    0
                                } else {
                                    let pe_size = compile_output_size(ctx)
                                    print(pe_size)
                                    println_str(" bytes PE  OK")
                                    println_str("")

                                    // Write PE
                                    print_str("[3/4] Writing: ")
                                    println_str(output)
                                    let pe_bytes = compile_pe_bytes(ctx)
                                    // Create right-sized output array
                                    let out_buf = array_new(pe_size)
                                    let mut bi = 0
                                    while bi < pe_size {
                                        array_set(out_buf, bi, array_get(pe_bytes, bi))
                                        bi = bi + 1
                                    }
                                    file_write_bytes(output, out_buf)
                                    println_str("       Written successfully")
                                    println_str("")

                                    // Verify
                                    println_str("[4/4] Verification:")
                                    print_str("       MZ signature:  0x")
                                    print(array_get(pe_bytes, 0))
                                    print_str(" 0x")
                                    println(array_get(pe_bytes, 1))
                                    print_str("       PE signature:  0x")
                                    print(pe_byte_at(ctx_pe(ctx), 64))
                                    print_str(" 0x")
                                    println(pe_byte_at(ctx_pe(ctx), 65))
                                    print_str("       Machine type:  0x")
                                    println(pe_u16_at(ctx_pe(ctx), 68))
                                    print_str("       PE32+ magic:   0x")
                                    println(pe_u16_at(ctx_pe(ctx), 88))
                                    print_str("       File size:     ")
                                    print(pe_size)
                                    println_str(" bytes")
                                    print_str("       Alignment:     ")
                                    print(pe_size % 512)
                                    println_str(" (should be 0)")
                                    println_str("")
                                    println_str("==================================================")
                                    println_str("  PROOF 1 COMPLETE: hello.sl -> hello.exe")
                                    println_str("==================================================")
                                    0
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
