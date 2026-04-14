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
