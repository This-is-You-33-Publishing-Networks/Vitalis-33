// ============================================================================
// Phase 6 Tests - Register Allocator
// ============================================================================

fn test_ra_pass(name: str) -> i64 {
    print_str("[PASS] ")
    println_str(name)
    0
}

fn test_ra_fail(name: str, expected: i64, got: i64) -> i64 {
    print_str("[FAIL] ")
    print_str(name)
    print_str(" expected=")
    print_str(to_string_i64(expected))
    print_str(" got=")
    println_str(to_string_i64(got))
    0
}

fn ra_assert(name: str, expected: i64, got: i64) -> i64 {
    if expected == got {
        test_ra_pass(name)
    } else {
        test_ra_fail(name, expected, got)
    }
}

// ============================================================================
// Test 1: Single vreg gets a register
// ============================================================================
fn test_single_vreg() -> i64 {
    let ra = new_regalloc(8)
    // v0 lives from inst 0 to inst 2
    add_live_range(ra, 0, 0, 2)
    allocate_registers(ra)

    let phys = get_phys_reg(ra, 0)
    // Should have gotten a register (not -1)
    ra_assert("single: v0 assigned", 1, if phys >= 0 { 1 } else { 0 })
    ra_assert("single: no spills", 0, ra_spill_count(ra))
}

// ============================================================================
// Test 2: Two non-overlapping ranges share a register
// ============================================================================
fn test_nonoverlapping() -> i64 {
    let ra = new_regalloc(8)
    // v0: [0, 2], v1: [3, 5]  - no overlap
    add_live_range(ra, 0, 0, 2)
    add_live_range(ra, 1, 3, 5)
    allocate_registers(ra)

    let p0 = get_phys_reg(ra, 0)
    let p1 = get_phys_reg(ra, 1)
    ra_assert("nonoverlap: v0 assigned", 1, if p0 >= 0 { 1 } else { 0 })
    ra_assert("nonoverlap: v1 assigned", 1, if p1 >= 0 { 1 } else { 0 })
    // They CAN share the same register (but don't have to)
    ra_assert("nonoverlap: no spills", 0, ra_spill_count(ra))
}

// ============================================================================
// Test 3: Two overlapping ranges get different registers
// ============================================================================
fn test_overlapping() -> i64 {
    let ra = new_regalloc(8)
    // v0: [0, 5], v1: [2, 7]  - overlap at [2,5]
    add_live_range(ra, 0, 0, 5)
    add_live_range(ra, 1, 2, 7)
    allocate_registers(ra)

    let p0 = get_phys_reg(ra, 0)
    let p1 = get_phys_reg(ra, 1)
    ra_assert("overlap: v0 assigned", 1, if p0 >= 0 { 1 } else { 0 })
    ra_assert("overlap: v1 assigned", 1, if p1 >= 0 { 1 } else { 0 })
    // They must be different
    ra_assert("overlap: diff regs", 1, if p0 != p1 { 1 } else { 0 })
    ra_assert("overlap: no spills", 0, ra_spill_count(ra))
}

// ============================================================================
// Test 4: Spilling when all registers exhausted
// ============================================================================
fn test_spill() -> i64 {
    // Create more live ranges than we have registers (14 pool)
    let ra = new_regalloc(20)
    // 15 overlapping ranges -> must spill at least 1
    let mut i = 0
    while i < 15 {
        add_live_range(ra, i, 0, 10)
        i = i + 1
    }
    allocate_registers(ra)

    ra_assert("spill: spill_count >= 1", 1, if ra_spill_count(ra) >= 1 { 1 } else { 0 })

    // At least one vreg should be spilled
    let mut spilled = 0
    i = 0
    while i < 15 {
        if is_spilled(ra, i) == 1 {
            spilled = spilled + 1
            0
        } else {
            0
        }
        i = i + 1
    }
    ra_assert("spill: at least 1 spilled", 1, if spilled >= 1 { 1 } else { 0 })
}

// ============================================================================
// Test 5: Frame size computation
// ============================================================================
fn test_frame_size() -> i64 {
    let ra = new_regalloc(4)
    // No spills -> frame = 32 (shadow space only), aligned to 16 = 32
    ra_assert("frame: no spill", 32, compute_frame_size(ra))

    // Set 1 spill
    ra_set_spill_count(ra, 1)
    // frame = 32 + 8 = 40, aligned to 16 = 48
    ra_assert("frame: 1 spill", 48, compute_frame_size(ra))

    // 2 spills
    ra_set_spill_count(ra, 2)
    // frame = 32 + 16 = 48 (already aligned)
    ra_assert("frame: 2 spills", 48, compute_frame_size(ra))
}

// ============================================================================
// Test 6: Sort live ranges by start
// ============================================================================
fn test_sort_ranges() -> i64 {
    let ra = new_regalloc(8)
    // Add in non-sorted order
    add_live_range(ra, 2, 5, 8)
    add_live_range(ra, 0, 0, 3)
    add_live_range(ra, 1, 2, 6)

    sort_live_ranges(ra)

    let ranges = ra_ranges(ra)
    // First should be v0 (start=0)
    ra_assert("sort: first vreg", 0, array_get(ranges, 0))
    ra_assert("sort: first start", 0, array_get(ranges, 1))
    // Second should be v1 (start=2)
    ra_assert("sort: second vreg", 1, array_get(ranges, 3))
    ra_assert("sort: second start", 2, array_get(ranges, 4))
    // Third should be v2 (start=5)
    ra_assert("sort: third vreg", 2, array_get(ranges, 6))
    ra_assert("sort: third start", 5, array_get(ranges, 7))
}

// ============================================================================
// Test 7: Callee-saved tracking
// ============================================================================
fn test_callee_saved() -> i64 {
    // Allocate enough overlapping ranges to need callee-saved regs
    let ra = new_regalloc(16)
    // 10 overlapping ranges -> needs all 9 caller-saved + 1 callee-saved
    let mut i = 0
    while i < 10 {
        add_live_range(ra, i, 0, 10)
        i = i + 1
    }
    allocate_registers(ra)

    // Should have used at least 1 callee-saved register
    let cs_count = callee_saved_count(ra)
    ra_assert("callee: count >= 1", 1, if cs_count >= 1 { 1 } else { 0 })
    ra_assert("callee: no spills", 0, ra_spill_count(ra))
}

// ============================================================================
// Test 8: Expire old intervals frees registers
// ============================================================================
fn test_expire() -> i64 {
    let ra = new_regalloc(8)
    // v0: [0, 2], v1: [0, 2] - 2 overlapping, take 2 registers
    add_live_range(ra, 0, 0, 2)
    add_live_range(ra, 1, 0, 2)
    allocate_registers(ra)

    let p0 = get_phys_reg(ra, 0)
    let p1 = get_phys_reg(ra, 1)
    ra_assert("expire: both assigned", 1, if p0 >= 0 && p1 >= 0 { 1 } else { 0 })

    // Now create another allocator for [0,2]+[5,8]
    let ra2 = new_regalloc(8)
    add_live_range(ra2, 0, 0, 2)
    add_live_range(ra2, 1, 5, 8)
    allocate_registers(ra2)
    // Both should be assigned and could reuse the same register
    ra_assert("expire: v0 assigned", 1, if get_phys_reg(ra2, 0) >= 0 { 1 } else { 0 })
    ra_assert("expire: v1 assigned", 1, if get_phys_reg(ra2, 1) >= 0 { 1 } else { 0 })
    ra_assert("expire: no spills", 0, ra_spill_count(ra2))
}

// ============================================================================
// Test 9: Dump output (visual test)
// ============================================================================
fn test_dump() -> i64 {
    let ra = new_regalloc(8)
    add_live_range(ra, 0, 0, 5)
    add_live_range(ra, 1, 1, 3)
    add_live_range(ra, 2, 4, 7)
    allocate_registers(ra)
    dump_regalloc(ra)
    test_ra_pass("dump: visual check")
}

// ============================================================================
// Test 10: reg_name helper
// ============================================================================
fn test_reg_name() -> i64 {
    ra_assert("regname: RAX", 1, if str_eq(reg_name(0), "RAX") { 1 } else { 0 })
    ra_assert("regname: R15", 1, if str_eq(reg_name(15), "R15") { 1 } else { 0 })
    ra_assert("regname: ???", 1, if str_eq(reg_name(99), "???") { 1 } else { 0 })
}

// ============================================================================
// Main
// ============================================================================
fn main() -> i64 {
    println_str("=== Register Allocator Tests ===")

    test_single_vreg()
    test_nonoverlapping()
    test_overlapping()
    test_spill()
    test_frame_size()
    test_sort_ranges()
    test_callee_saved()
    test_expire()
    test_dump()
    test_reg_name()

    println_str("=== All regalloc tests done ===")
    0
}
