// ============================================================================
// Phase 5 Tests - x86-64 Machine Code Emitter
// ============================================================================
// Each test creates a code buffer, emits instructions, and verifies
// the output byte sequence.
// ============================================================================

fn test_x86_pass(name: str) -> i64 {
    print_str("[PASS] ")
    println_str(name)
    0
}

fn test_x86_fail(name: str, expected: i64, got: i64) -> i64 {
    print_str("[FAIL] ")
    print_str(name)
    print_str(" expected=")
    print_str(to_string_i64(expected))
    print_str(" got=")
    println_str(to_string_i64(got))
    0
}

fn x86_assert(name: str, expected: i64, got: i64) -> i64 {
    if expected == got {
        test_x86_pass(name)
    } else {
        test_x86_fail(name, expected, got)
    }
}

fn code_byte(buf: i64, idx: i64) -> i64 {
    array_get(buf_code(buf), idx)
}

// ============================================================================
// Test 1: emit_byte and buffer position
// ============================================================================
fn test_emit_byte() -> i64 {
    let buf = new_code_buf(256)
    emit_byte(buf, 144)  // NOP = 0x90
    emit_byte(buf, 195)  // RET = 0xC3

    x86_assert("emit_byte: pos", 2, buf_pos(buf))
    x86_assert("emit_byte: byte0=0x90", 144, code_byte(buf, 0))
    x86_assert("emit_byte: byte1=0xC3", 195, code_byte(buf, 1))
}

// ============================================================================
// Test 2: emit_imm32 (little-endian)
// ============================================================================
fn test_emit_imm32() -> i64 {
    let buf = new_code_buf(256)
    // 0x12345678 = 305419896
    emit_imm32(buf, 305419896)

    x86_assert("imm32: pos", 4, buf_pos(buf))
    x86_assert("imm32: byte0=0x78", 120, code_byte(buf, 0))
    x86_assert("imm32: byte1=0x56", 86, code_byte(buf, 1))
    x86_assert("imm32: byte2=0x34", 52, code_byte(buf, 2))
    x86_assert("imm32: byte3=0x12", 18, code_byte(buf, 3))
}

// ============================================================================
// Test 3: MOV RAX, imm64 (REX.W B8+ io)
// ============================================================================
fn test_mov_reg_imm64() -> i64 {
    let buf = new_code_buf(256)
    // MOV RAX, 42 -> 48 B8 2A 00 00 00 00 00 00 00
    x86_mov_reg_imm64(buf, REG_RAX(), 42)

    x86_assert("mov_imm64: pos", 10, buf_pos(buf))
    x86_assert("mov_imm64: REX.W", 72, code_byte(buf, 0))
    x86_assert("mov_imm64: B8", 184, code_byte(buf, 1))
    x86_assert("mov_imm64: imm[0]=42", 42, code_byte(buf, 2))
    x86_assert("mov_imm64: imm[1]=0", 0, code_byte(buf, 3))
}

// ============================================================================
// Test 4: MOV reg, reg
// ============================================================================
fn test_mov_reg_reg() -> i64 {
    let buf = new_code_buf(256)
    // MOV RAX, RCX -> 48 89 C8 (89 /r, src=RCX(1), dst=RAX(0))
    x86_mov_reg_reg(buf, REG_RAX(), REG_RCX())

    x86_assert("mov_rr: pos", 3, buf_pos(buf))
    x86_assert("mov_rr: REX.W", 72, code_byte(buf, 0))
    x86_assert("mov_rr: opcode=0x89", 137, code_byte(buf, 1))
    // ModR/M = mod=11(3), reg=RCX(001), rm=RAX(000) = 11_001_000 = 0xC8 = 200
    x86_assert("mov_rr: modrm=0xC8", 200, code_byte(buf, 2))
}

// ============================================================================
// Test 5: ADD, SUB reg, reg
// ============================================================================
fn test_add_sub() -> i64 {
    let buf = new_code_buf(256)
    // ADD RAX, RBX -> 48 01 D8 (01 /r, src=RBX(3), dst=RAX(0))
    x86_add_reg_reg(buf, REG_RAX(), REG_RBX())

    x86_assert("add_rr: REX.W", 72, code_byte(buf, 0))
    x86_assert("add_rr: opcode=0x01", 1, code_byte(buf, 1))
    // ModR/M = 11_011_000 = 0xD8 = 216
    x86_assert("add_rr: modrm=0xD8", 216, code_byte(buf, 2))

    let pos_before = buf_pos(buf)
    // SUB RCX, RDX -> 48 29 D1 (29 /r, src=RDX(2), dst=RCX(1))
    x86_sub_reg_reg(buf, REG_RCX(), REG_RDX())

    x86_assert("sub_rr: opcode=0x29", 41, code_byte(buf, pos_before + 1))
    // ModR/M = 11_010_001 = 0xD1 = 209
    x86_assert("sub_rr: modrm=0xD1", 209, code_byte(buf, pos_before + 2))
}

// ============================================================================
// Test 6: PUSH/POP
// ============================================================================
fn test_push_pop() -> i64 {
    let buf = new_code_buf(256)
    // PUSH RBP -> 55 (0x50 + 5)
    x86_push_reg(buf, REG_RBP())
    x86_assert("push_rbp: byte=0x55", 85, code_byte(buf, 0))

    // POP RBP -> 5D (0x58 + 5)
    x86_pop_reg(buf, REG_RBP())
    x86_assert("pop_rbp: byte=0x5D", 93, code_byte(buf, 1))

    // PUSH R8 -> 41 50 (REX.B + 50)
    let p2 = buf_pos(buf)
    x86_push_reg(buf, REG_R8())
    x86_assert("push_r8: REX.B=0x41", 65, code_byte(buf, p2))
    x86_assert("push_r8: 50", 80, code_byte(buf, p2 + 1))
}

// ============================================================================
// Test 7: Prologue + Epilogue
// ============================================================================
fn test_prologue_epilogue() -> i64 {
    let buf = new_code_buf(256)
    x86_prologue(buf, 32)

    // push rbp = 55
    x86_assert("prologue: push rbp", 85, code_byte(buf, 0))
    // mov rbp, rsp = 48 89 E5  (89 /r, src=RSP(4), dst=RBP(5))
    x86_assert("prologue: REX.W", 72, code_byte(buf, 1))
    x86_assert("prologue: mov opcode", 137, code_byte(buf, 2))
    // ModR/M = 11_100_101 = 0xE5 = 229
    x86_assert("prologue: mov modrm", 229, code_byte(buf, 3))
    // sub rsp, 32 = 48 81 EC 20 00 00 00

    let epi_start = buf_pos(buf)
    x86_epilogue(buf)

    // mov rsp, rbp = 48 89 EC -> wait: mov rsp, rbp = 48 89 ... modrm(3, RBP, RSP)=11_101_100=0xEC=236
    x86_assert("epilogue: mov opcode", 137, code_byte(buf, epi_start + 1))
    // ret = C3 is the last byte
    let final_pos = buf_pos(buf) - 1
    x86_assert("epilogue: ret", 195, code_byte(buf, final_pos))
}

// ============================================================================
// Test 8: Jump + Label fixup
// ============================================================================
fn test_jump_fixup() -> i64 {
    let buf = new_code_buf(256)
    let lbl = new_label(buf)

    // jmp to unresolved label
    x86_jmp_rel32(buf, lbl)
    // pos is now 5 (E9 + 4 bytes imm32)
    x86_assert("jmp: pos=5", 5, buf_pos(buf))
    x86_assert("jmp: opcode=0xE9", 233, code_byte(buf, 0))

    // emit some nops
    x86_nop(buf)
    x86_nop(buf)

    // bind label here (pos=7)
    bind_label(buf, lbl)

    // resolve
    resolve_fixups(buf)

    // rel32 should be: target(7) - (fixup_offset(1) + 4) = 7 - 5 = 2
    x86_assert("jmp: rel32=2", 2, code_byte(buf, 1))
    x86_assert("jmp: rel32 b1=0", 0, code_byte(buf, 2))
}

// ============================================================================
// Test 9: CMP + SETcc + MOVZX
// ============================================================================
fn test_cmp_setcc() -> i64 {
    let buf = new_code_buf(256)
    // CMP RAX, RCX
    x86_cmp_reg_reg(buf, REG_RAX(), REG_RCX())
    x86_assert("cmp: opcode=0x39", 57, code_byte(buf, 1))

    let p1 = buf_pos(buf)
    // SETE AL
    x86_sete(buf, REG_RAX())
    x86_assert("sete: 0F", 15, code_byte(buf, p1))
    x86_assert("sete: 94", 148, code_byte(buf, p1 + 1))

    let p2 = buf_pos(buf)
    // MOVZX RAX, AL
    x86_movzx_reg_reg8(buf, REG_RAX(), REG_RAX())
    x86_assert("movzx: REX.W", 72, code_byte(buf, p2))
    x86_assert("movzx: 0F", 15, code_byte(buf, p2 + 1))
    x86_assert("movzx: B6", 182, code_byte(buf, p2 + 2))
}

// ============================================================================
// Test 10: Extended registers (R8-R15)
// ============================================================================
fn test_extended_regs() -> i64 {
    let buf = new_code_buf(256)

    // MOV R8, R9 -> REX=4D, 89, modrm(3,R9,R8) = modrm(3,1,0) = 0xC8
    x86_mov_reg_reg(buf, REG_R8(), REG_R9())
    // REX = 0x48 + R(R9 ext=1->+4) + B(R8 ext=1->+1) = 0x48+4+1 = 0x4D = 77
    x86_assert("ext: REX=0x4D", 77, code_byte(buf, 0))
    x86_assert("ext: opcode=0x89", 137, code_byte(buf, 1))
    // modrm = 11_001_000 = 0xC8 = 200 (reg=R9 low3=1, rm=R8 low3=0)
    x86_assert("ext: modrm=0xC8", 200, code_byte(buf, 2))

    let p2 = buf_pos(buf)
    // ADD R10, R11 -> REX=4D, 01, modrm(3,3,2) = 11_011_010 = 0xDA = 218
    x86_add_reg_reg(buf, REG_R10(), REG_R11())
    x86_assert("ext_add: REX=0x4D", 77, code_byte(buf, p2))
    x86_assert("ext_add: modrm=0xDA", 218, code_byte(buf, p2 + 2))
}

// ============================================================================
// Test 11: IMUL
// ============================================================================
fn test_imul() -> i64 {
    let buf = new_code_buf(256)
    // IMUL RAX, RCX -> 48 0F AF C1
    x86_imul_reg_reg(buf, REG_RAX(), REG_RCX())
    x86_assert("imul: REX.W", 72, code_byte(buf, 0))
    x86_assert("imul: 0F", 15, code_byte(buf, 1))
    x86_assert("imul: AF", 175, code_byte(buf, 2))
    // modrm(3, RAX(0), RCX(1)) = 11_000_001 = 0xC1 = 193
    x86_assert("imul: modrm=0xC1", 193, code_byte(buf, 3))
}

// ============================================================================
// Test 12: CALL rel32
// ============================================================================
fn test_call_rel32() -> i64 {
    let buf = new_code_buf(256)
    let lbl = new_label(buf)

    // some nops before target
    x86_nop(buf)
    x86_nop(buf)

    // bind label at pos 2
    bind_label(buf, lbl)

    // emit some code
    x86_nop(buf)
    x86_nop(buf)
    x86_nop(buf)

    // CALL back to label at pos 2 (current pos = 5)
    x86_call_rel32(buf, lbl)
    // pos is now 10 (5 + E8 + 4 bytes)

    resolve_fixups(buf)

    x86_assert("call: opcode=0xE8", 232, code_byte(buf, 5))
    // rel32 = target(2) - (fixup_offset(6) + 4) = 2 - 10 = -8
    // -8 as u8 low byte = 248 (0xF8)
    x86_assert("call: rel32 low=0xF8", 248, code_byte(buf, 6))
    // Next bytes of rel32: FF FF FF (sign extension of negative)
    x86_assert("call: rel32 b1=0xFF", 255, code_byte(buf, 7))
}

// ============================================================================
// Test 13: Hex dump (visual / smoke test)
// ============================================================================
fn test_hex_dump() -> i64 {
    let buf = new_code_buf(256)
    x86_prologue(buf, 48)
    x86_mov_reg_imm64(buf, REG_RAX(), 100)
    x86_epilogue(buf)

    let total = buf_pos(buf)
    dump_code(buf, total)
    x86_assert("hex_dump: total > 0", 1, if total > 0 { 1 } else { 0 })
}

// ============================================================================
// Test 14: MOV reg, [RBP + disp]
// ============================================================================
fn test_mov_mem() -> i64 {
    let buf = new_code_buf(256)
    // MOV RAX, [RBP - 8] -> REX.W 8B modrm(2, RAX, RBP) disp32(-8)
    x86_mov_reg_mem(buf, REG_RAX(), REG_RBP(), -8)

    x86_assert("mov_mem: REX.W", 72, code_byte(buf, 0))
    x86_assert("mov_mem: opcode=0x8B", 139, code_byte(buf, 1))

    let p2 = buf_pos(buf)
    // MOV [RBP + 16], RCX
    x86_mov_mem_reg(buf, REG_RBP(), 16, REG_RCX())
    x86_assert("mov_mem_reg: opcode=0x89", 137, code_byte(buf, p2 + 1))
}

// ============================================================================
// Test 15: Data section string emission
// ============================================================================
fn test_data_string() -> i64 {
    let buf = new_code_buf(1024)
    let pos = data_emit_string(buf, "Hi")

    x86_assert("data_str: pos=0", 0, pos)
    let data = buf_data(buf)
    // 'H' = 72, 'i' = 105, null = 0
    x86_assert("data_str: H", 72, array_get(data, 0))
    x86_assert("data_str: i", 105, array_get(data, 1))
    x86_assert("data_str: null", 0, array_get(data, 2))
    x86_assert("data_str: data_pos=3", 3, buf_data_pos(buf))
}

// ============================================================================
// Main
// ============================================================================
fn main() -> i64 {
    println_str("=== x86-64 Emitter Tests ===")

    test_emit_byte()
    test_emit_imm32()
    test_mov_reg_imm64()
    test_mov_reg_reg()
    test_add_sub()
    test_push_pop()
    test_prologue_epilogue()
    test_jump_fixup()
    test_cmp_setcc()
    test_extended_regs()
    test_imul()
    test_call_rel32()
    test_hex_dump()
    test_mov_mem()
    test_data_string()

    println_str("=== All x86-64 tests done ===")
    0
}
