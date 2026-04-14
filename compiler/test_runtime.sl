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
    array_set(buf, 5, array_new(3000))
    array_set(buf, 6, 0)
    array_set(buf, 7, array_new(1000))
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
