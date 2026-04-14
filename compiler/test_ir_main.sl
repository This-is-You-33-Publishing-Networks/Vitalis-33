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
