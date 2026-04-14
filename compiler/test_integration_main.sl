// ============================================================================
// Phase 9 Tests - Integration Pipeline
// ============================================================================
// Tests the full compilation pipeline: source -> PE bytes.
// Verifies each stage works in sequence.
// ============================================================================

fn test_int_pass(name: str) -> i64 {
    print_str("[PASS] ")
    println_str(name)
    0
}

fn test_int_fail(name: str, expected: i64, got: i64) -> i64 {
    print_str("[FAIL] ")
    print_str(name)
    print_str(" expected=")
    print_str(to_string_i64(expected))
    print_str(" got=")
    println_str(to_string_i64(got))
    0
}

fn int_assert(name: str, expected: i64, got: i64) -> i64 {
    if expected == got {
        test_int_pass(name)
    } else {
        test_int_fail(name, expected, got)
    }
}

// ============================================================================
// Test 1: Lex stage
// ============================================================================
fn test_stage_lex() -> i64 {
    let src = "fn main() -> i64 { 42 }"
    let ctx = new_compile_ctx(src)
    stage_lex(ctx, src)

    int_assert("lex: no error", 0, ctx_error(ctx))
    int_assert("lex: tokens > 0", 1, if ctx_tok_count(ctx) > 0 { 1 } else { 0 })
}

// ============================================================================
// Test 2: Parse stage
// ============================================================================
fn test_stage_parse() -> i64 {
    let src = "fn main() -> i64 { 42 }"
    let ctx = new_compile_ctx(src)
    stage_lex(ctx, src)
    stage_parse(ctx, src)

    int_assert("parse: no error", 0, ctx_error(ctx))
    int_assert("parse: prog_off >= 0", 1, if ctx_prog_off(ctx) >= 0 { 1 } else { 0 })
}

// ============================================================================
// Test 3: Full compile of simple function
// ============================================================================
fn test_full_compile() -> i64 {
    let src = "fn main() -> i64 { 42 }"
    let ctx = compile(src)

    int_assert("full: no error", 0, ctx_error(ctx))
    int_assert("full: output > 0", 1, if compile_output_size(ctx) > 0 { 1 } else { 0 })

    // Output should be a valid PE (starts with MZ)
    let out = compile_pe_bytes(ctx)
    int_assert("full: MZ[0]=M", 77, array_get(out, 0))
    int_assert("full: MZ[1]=Z", 90, array_get(out, 1))

    // Should be aligned to 512
    int_assert("full: aligned", 0, compile_output_size(ctx) % 512)

    compile_summary(ctx)
}

// ============================================================================
// Test 4: Compile addition
// ============================================================================
fn test_compile_add() -> i64 {
    let src = "fn main() -> i64 { 1 + 2 }"
    let ctx = compile(src)

    int_assert("add: no error", 0, ctx_error(ctx))
    int_assert("add: has output", 1, if compile_output_size(ctx) > 0 { 1 } else { 0 })
}

// ============================================================================
// Test 5: Compile with let binding
// ============================================================================
fn test_compile_let() -> i64 {
    let src = "fn main() -> i64 { let x = 10; x }"
    let ctx = compile(src)

    int_assert("let: no error", 0, ctx_error(ctx))
    int_assert("let: has output", 1, if compile_output_size(ctx) > 0 { 1 } else { 0 })
}

// ============================================================================
// Test 6: Lex error detection
// ============================================================================
fn test_lex_error() -> i64 {
    // Empty source should still lex (may produce 0 tokens or just EOF)
    let src = ""
    let ctx = new_compile_ctx(src)
    stage_lex(ctx, src)
    // This might succeed or fail depending on lexer behavior
    // Just verify it doesn't crash
    test_int_pass("lex_error: no crash on empty")
}

// ============================================================================
// Test 7: Compile error stage name
// ============================================================================
fn test_error_names() -> i64 {
    int_assert("errname: none", 1, if str_eq(compile_error_stage(new_compile_ctx("")), "none") { 1 } else { 0 })
}

// ============================================================================
// Test 8: Multiple function compile
// ============================================================================
fn test_multi_fn() -> i64 {
    let src = "fn add(a: i64, b: i64) -> i64 { a + b } fn main() -> i64 { add(3, 4) }"
    let ctx = compile(src)

    int_assert("multi: no error", 0, ctx_error(ctx))
    int_assert("multi: has output", 1, if compile_output_size(ctx) > 0 { 1 } else { 0 })
    compile_summary(ctx)
}

// ============================================================================
// Test 9: PE structure validation
// ============================================================================
fn test_pe_structure() -> i64 {
    let src = "fn main() -> i64 { 0 }"
    let ctx = compile(src)

    if compile_ok(ctx) == 1 {
        let pe = ctx_pe(ctx)
        // PE signature at offset 64
        int_assert("pe: sig P", 80, pe_byte_at(pe, 64))
        int_assert("pe: sig E", 69, pe_byte_at(pe, 65))
        // Machine type at offset 68
        int_assert("pe: machine", 34404, pe_u16_at(pe, 68))
        // Optional header magic at offset 88
        int_assert("pe: magic", 523, pe_u16_at(pe, 88))
        0
    } else {
        test_int_fail("pe: compile failed", 0, ctx_error(ctx))
    }
}

// ============================================================================
// Test 10: Code section has instructions
// ============================================================================
fn test_code_content() -> i64 {
    let src = "fn main() -> i64 { 42 }"
    let ctx = compile(src)

    if compile_ok(ctx) == 1 {
        let buf = ctx_codebuf(ctx)
        let code_size = buf_pos(buf)
        int_assert("code: size > 0", 1, if code_size > 0 { 1 } else { 0 })
        // Should contain at least the runtime stubs + user code
        int_assert("code: size > 100", 1, if code_size > 100 { 1 } else { 0 })
        0
    } else {
        test_int_fail("code: compile failed", 0, ctx_error(ctx))
    }
}

// ============================================================================
// Main
// ============================================================================
fn main() -> i64 {
    println_str("=== Integration Pipeline Tests ===")

    test_stage_lex()
    test_stage_parse()
    test_full_compile()
    test_compile_add()
    test_compile_let()
    test_lex_error()
    test_error_names()
    test_multi_fn()
    test_pe_structure()
    test_code_content()

    println_str("=== All integration tests done ===")
    0
}
