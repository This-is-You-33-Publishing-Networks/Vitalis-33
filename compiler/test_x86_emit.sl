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
