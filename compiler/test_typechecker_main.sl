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
