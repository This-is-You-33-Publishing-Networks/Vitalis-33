fn is_digit(code: i64) -> bool { code >= 48 && code <= 57 }
fn is_alpha(code: i64) -> bool { (code >= 65 && code <= 90) || (code >= 97 && code <= 122) || code == 95 }
fn is_alnum(code: i64) -> bool { is_digit(code) || is_alpha(code) }
fn is_whitespace(code: i64) -> bool { code == 32 || code == 9 || code == 13 || code == 10 }
fn is_newline(code: i64) -> bool { code == 10 }
fn is_hex_digit(code: i64) -> bool { is_digit(code) || (code >= 65 && code <= 70) || (code >= 97 && code <= 102) }

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
    else if str_eq(word, "true") { 5 }
    else if str_eq(word, "false") { 5 }
    else { 0 }
}

fn tok_count(tokens: i64) -> i64 { array_len(tokens) / 4 }
fn tok_type(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4) }
fn tok_start(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4 + 1) }
fn tok_end(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4 + 2) }
fn tok_line(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4 + 3) }

fn tok_text(source: str, tokens: i64, idx: i64) -> str {
    let s = tok_start(tokens, idx)
    let e = tok_end(tokens, idx)
    if s < 0 { "" }
    else { str_substr(source, s, e - s) }
}

fn peek_char(src: str, p: i64) -> i64 {
    if p >= str_len(src) { -1 }
    else { char_to_int(str_char_at(src, p)) }
}

fn peek_char2(src: str, p: i64) -> i64 {
    if p + 1 >= str_len(src) { -1 }
    else { char_to_int(str_char_at(src, p + 1)) }
}

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

fn lex(source: str) -> i64 {
    let slen = str_len(source)
    let mut pos = 0
    let mut ln = 1
    let mut buf = tbuf_new()

    while pos < slen {
        let ch = peek_char(source, pos)

        if is_whitespace(ch) {
            if is_newline(ch) { ln = ln + 1 }
            pos = pos + 1
            continue
        }

        // Line comment
        if ch == 47 && peek_char2(source, pos) == 47 {
            while pos < slen && peek_char(source, pos) != 10 {
                pos = pos + 1
            }
            continue
        }

        // Numeric literal
        if is_digit(ch) {
            let nstart = pos
            while pos < slen && is_digit(peek_char(source, pos)) {
                pos = pos + 1
            }
            buf = tbuf_emit(buf, 1, nstart, pos, ln)
            continue
        }

        // Identifier/keyword
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
                buf = tbuf_emit(buf, 4, istart, pos, ln)
            }
            continue
        }

        // Two-char ops
        let ch2 = peek_char2(source, pos)
        if ch == 61 && ch2 == 61  { buf = tbuf_emit(buf, 56, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 33 && ch2 == 61  { buf = tbuf_emit(buf, 57, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 60 && ch2 == 61  { buf = tbuf_emit(buf, 60, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 62 && ch2 == 61  { buf = tbuf_emit(buf, 61, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 38 && ch2 == 38  { buf = tbuf_emit(buf, 62, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 124 && ch2 == 124 { buf = tbuf_emit(buf, 63, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 45 && ch2 == 62  { buf = tbuf_emit(buf, 65, pos, pos + 2, ln); pos = pos + 2; continue }
        if ch == 124 && ch2 == 62 { buf = tbuf_emit(buf, 67, pos, pos + 2, ln); pos = pos + 2; continue }

        // Single-char ops
        if ch == 40  { buf = tbuf_emit(buf, 80, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 41  { buf = tbuf_emit(buf, 81, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 123 { buf = tbuf_emit(buf, 82, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 125 { buf = tbuf_emit(buf, 83, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 44  { buf = tbuf_emit(buf, 86, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 58  { buf = tbuf_emit(buf, 87, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 61  { buf = tbuf_emit(buf, 55, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 43  { buf = tbuf_emit(buf, 50, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 45  { buf = tbuf_emit(buf, 51, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 42  { buf = tbuf_emit(buf, 52, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 60  { buf = tbuf_emit(buf, 58, pos, pos + 1, ln); pos = pos + 1; continue }
        if ch == 62  { buf = tbuf_emit(buf, 59, pos, pos + 1, ln); pos = pos + 1; continue }

        // Unknown
        buf = tbuf_emit(buf, 100, pos, pos + 1, ln)
        pos = pos + 1
    }

    buf = tbuf_emit(buf, 99, -1, -1, ln)
    tbuf_finalize(buf)
}

fn main() -> i64 {
    let src1 = "let x = 42"
    let t1 = lex(src1)
    assert_eq(tok_count(t1), 5)
    assert_eq(tok_type(t1, 0), 11)
    assert_eq(tok_type(t1, 1), 4)
    assert_eq(tok_type(t1, 2), 55)
    assert_eq(tok_type(t1, 3), 1)
    assert_eq(tok_type(t1, 4), 99)
    assert_true(str_eq(tok_text(src1, t1, 1), "x"))
    assert_true(str_eq(tok_text(src1, t1, 3), "42"))
    println_str("Test 1 passed")

    let src2 = "fn add(a: i64) -> i64 { a + 1 }"
    let t2 = lex(src2)
    assert_eq(tok_type(t2, 0), 10)
    assert_eq(tok_type(t2, 1), 4)
    assert_true(str_eq(tok_text(src2, t2, 1), "add"))
    println_str("Test 2 passed")

    let src3 = "x == 5 && y != 3"
    let t3 = lex(src3)
    assert_eq(tok_type(t3, 1), 56)
    assert_eq(tok_type(t3, 3), 62)
    assert_eq(tok_type(t3, 5), 57)
    println_str("Test 3 passed")

    println_str("Core lexer tests passed!")
    0
}
