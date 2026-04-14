// ============================================================================
// Phase 8 Tests - Runtime & OS Interface
// ============================================================================
// Tests runtime stub emission: entry point, print stubs, alloc, exit.
// Uses x86_emit buffer helpers.
// ============================================================================

fn test_rt_pass(name: str) -> i64 {
    print_str("[PASS] ")
    println_str(name)
    0
}

fn test_rt_fail(name: str, expected: i64, got: i64) -> i64 {
    print_str("[FAIL] ")
    print_str(name)
    print_str(" expected=")
    print_str(to_string_i64(expected))
    print_str(" got=")
    println_str(to_string_i64(got))
    0
}

fn rt_assert(name: str, expected: i64, got: i64) -> i64 {
    if expected == got {
        test_rt_pass(name)
    } else {
        test_rt_fail(name, expected, got)
    }
}

fn code_byte(buf: i64, idx: i64) -> i64 {
    array_get(buf_code(buf), idx)
}

// ============================================================================
// Test 1: Runtime creation
// ============================================================================
fn test_rt_create() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)  // IAT at RVA 0x2000

    // Should have labels allocated
    let entry_lbl = rt_entry_lbl(rt)
    rt_assert("create: entry label >= 0", 1, if entry_lbl >= 0 { 1 } else { 0 })

    // Data section should have heap_ptr, heap_end, newline
    let dp = buf_data_pos(buf)
    rt_assert("create: data_pos > 0", 1, if dp > 0 { 1 } else { 0 })
}

// ============================================================================
// Test 2: Entry point emission
// ============================================================================
fn test_entry_emit() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_entry_point(rt)

    let code_size = buf_pos(buf)
    rt_assert("entry: code emitted", 1, if code_size > 0 { 1 } else { 0 })

    // Entry label should be bound
    let entry_off = label_offset(buf, rt_entry_lbl(rt))
    rt_assert("entry: label bound", 0, entry_off)

    // First instruction should be SUB RSP, 40
    // REX.W=48, opcode=81, modrm=EC (sub rsp imm32)
    rt_assert("entry: REX.W", 72, code_byte(buf, 0))
    rt_assert("entry: SUB opcode", 129, code_byte(buf, 1))
}

// ============================================================================
// Test 3: All stubs emit without error
// ============================================================================
fn test_all_stubs() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_runtime(rt)

    let code_size = buf_pos(buf)
    rt_assert("stubs: total > 50", 1, if code_size > 50 { 1 } else { 0 })

    // All labels should be bound (offset >= 0)
    rt_assert("stubs: entry bound", 1, if label_offset(buf, rt_entry_lbl(rt)) >= 0 { 1 } else { 0 })
    rt_assert("stubs: exit bound", 1, if label_offset(buf, rt_exit_lbl(rt)) >= 0 { 1 } else { 0 })
    rt_assert("stubs: print_i64 bound", 1, if label_offset(buf, rt_print_i64_lbl(rt)) >= 0 { 1 } else { 0 })
    rt_assert("stubs: println bound", 1, if label_offset(buf, rt_println_lbl(rt)) >= 0 { 1 } else { 0 })
    rt_assert("stubs: print_str bound", 1, if label_offset(buf, rt_print_str_lbl(rt)) >= 0 { 1 } else { 0 })
    rt_assert("stubs: alloc bound", 1, if label_offset(buf, rt_alloc_lbl(rt)) >= 0 { 1 } else { 0 })
}

// ============================================================================
// Test 4: Exit stub has INT3 + RET
// ============================================================================
fn test_exit_stub() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_runtime(rt)

    let exit_off = label_offset(buf, rt_exit_lbl(rt))
    // INT3 = 0xCC = 204
    rt_assert("exit: INT3", 204, code_byte(buf, exit_off))
    // RET = 0xC3 = 195
    rt_assert("exit: RET", 195, code_byte(buf, exit_off + 1))
}

// ============================================================================
// Test 5: Print stubs have prologue/epilogue
// ============================================================================
fn test_print_stubs() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_runtime(rt)

    // print_i64 should start with PUSH RBP (0x55)
    let pi_off = label_offset(buf, rt_print_i64_lbl(rt))
    rt_assert("print_i64: push rbp", 85, code_byte(buf, pi_off))

    // println should also start with PUSH RBP
    let pn_off = label_offset(buf, rt_println_lbl(rt))
    rt_assert("println: push rbp", 85, code_byte(buf, pn_off))

    // print_str should start with PUSH RBP
    let ps_off = label_offset(buf, rt_print_str_lbl(rt))
    rt_assert("print_str: push rbp", 85, code_byte(buf, ps_off))
}

// ============================================================================
// Test 6: Alloc stub returns 0 (placeholder)
// ============================================================================
fn test_alloc_stub() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_runtime(rt)

    let al_off = label_offset(buf, rt_alloc_lbl(rt))
    // Should start with prologue
    rt_assert("alloc: push rbp", 85, code_byte(buf, al_off))
}

// ============================================================================
// Test 7: Labels are at unique positions
// ============================================================================
fn test_label_positions() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_runtime(rt)

    let off_entry = label_offset(buf, rt_entry_lbl(rt))
    let off_exit = label_offset(buf, rt_exit_lbl(rt))
    let off_pi64 = label_offset(buf, rt_print_i64_lbl(rt))
    let off_pln = label_offset(buf, rt_println_lbl(rt))
    let off_pstr = label_offset(buf, rt_print_str_lbl(rt))
    let off_alloc = label_offset(buf, rt_alloc_lbl(rt))

    // All should be different
    rt_assert("labels: entry != exit", 1, if off_entry != off_exit { 1 } else { 0 })
    rt_assert("labels: exit != pi64", 1, if off_exit != off_pi64 { 1 } else { 0 })
    rt_assert("labels: pi64 != pln", 1, if off_pi64 != off_pln { 1 } else { 0 })
    rt_assert("labels: pln != pstr", 1, if off_pln != off_pstr { 1 } else { 0 })
    rt_assert("labels: pstr != alloc", 1, if off_pstr != off_alloc { 1 } else { 0 })
}

// ============================================================================
// Test 8: Runtime query helpers
// ============================================================================
fn test_rt_queries() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    emit_runtime(rt)

    let entry_off = runtime_entry_offset(rt)
    rt_assert("query: entry_off = 0", 0, entry_off)

    let code_sz = runtime_code_size(rt)
    rt_assert("query: code_size > 0", 1, if code_sz > 0 { 1 } else { 0 })

    rt_assert("query: iat_slots = 4", 4, runtime_iat_slots())
}

// ============================================================================
// Test 9: Hex dump of runtime (visual)
// ============================================================================
fn test_rt_dump() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)
    emit_runtime(rt)

    let total = buf_pos(buf)
    print_str("Runtime total: ")
    print_str(to_string_i64(total))
    println_str(" bytes")
    // Dump first 32 bytes
    let show = if total > 32 { 32 } else { total }
    dump_code(buf, show)
    test_rt_pass("dump: visual check")
}

// ============================================================================
// Test 10: Data section has heap slots + newline
// ============================================================================
fn test_rt_data() -> i64 {
    let buf = new_code_buf(4096)
    let rt = new_runtime(buf, 8192)

    let heap_ptr_off = rt_heap_ptr(rt)
    let heap_end_off = rt_heap_end(rt)
    let newline_off = rt_newline_off(rt)

    rt_assert("data: heap_ptr offset valid", 1, if heap_ptr_off >= 0 { 1 } else { 0 })
    rt_assert("data: heap_end > heap_ptr", 1, if heap_end_off > heap_ptr_off { 1 } else { 0 })
    rt_assert("data: newline > heap_end", 1, if newline_off > heap_end_off { 1 } else { 0 })

    // Newline byte should be 10 (0x0A)
    let data = buf_data(buf)
    rt_assert("data: newline=10", 10, array_get(data, newline_off))
}

// ============================================================================
// Main
// ============================================================================
fn main() -> i64 {
    println_str("=== Runtime Tests ===")

    test_rt_create()
    test_entry_emit()
    test_all_stubs()
    test_exit_stub()
    test_print_stubs()
    test_alloc_stub()
    test_label_positions()
    test_rt_queries()
    test_rt_dump()
    test_rt_data()

    println_str("=== All runtime tests done ===")
    0
}
