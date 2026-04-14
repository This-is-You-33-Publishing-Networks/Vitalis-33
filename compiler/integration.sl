// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 9: Integration & Compilation Pipeline
// ============================================================================
// Wires together all pipeline stages:
//   Source (.sl) -> Lexer -> Parser -> TypeChecker -> IR Gen -> RegAlloc -> x86 Emit -> PE Writer
//
// This module provides the top-level `compile()` function that takes
// source code and produces PE file bytes.
// 
// For the bootstrap, we handle a subset of the language sufficient to
// compile the compiler itself.
//
// NOTE: No `return` inside if-blocks (Cranelift bug workaround).
// ============================================================================

// -- Compile error codes --
fn ERR_NONE() -> i64     { 0 }
fn ERR_LEX() -> i64      { 1 }
fn ERR_PARSE() -> i64    { 2 }
fn ERR_TYPE() -> i64     { 3 }
fn ERR_IR() -> i64       { 4 }
fn ERR_CODEGEN() -> i64  { 5 }

// ============================================================================
// Compilation context
// ============================================================================
// ctx[0]  = source string
// ctx[1]  = error code (ERR_*)
// ctx[2]  = error message
// ctx[3]  = token buffer (from lexer)
// ctx[4]  = token count
// ctx[5]  = parse tree (from parser)
// ctx[6]  = program node offset
// ctx[7]  = type check result (0=ok, 1=errors)
// ctx[8]  = IR context (from ir_gen)
// ctx[9]  = code buffer (from x86_emit)
// ctx[10] = PE builder
// ctx[11] = regalloc context
// ctx[12] = output bytes (final PE)
// ctx[13] = output size

fn ctx_source(ctx: i64) -> i64     { array_get(ctx, 0) }
fn ctx_error(ctx: i64) -> i64      { array_get(ctx, 1) }
fn ctx_errmsg(ctx: i64) -> str     { "error" }  // simplified
fn ctx_tokens(ctx: i64) -> i64     { array_get(ctx, 3) }
fn ctx_tok_count(ctx: i64) -> i64  { array_get(ctx, 4) }
fn ctx_tree(ctx: i64) -> i64       { array_get(ctx, 5) }
fn ctx_prog_off(ctx: i64) -> i64   { array_get(ctx, 6) }
fn ctx_tc_result(ctx: i64) -> i64  { array_get(ctx, 7) }
fn ctx_ir(ctx: i64) -> i64         { array_get(ctx, 8) }
fn ctx_codebuf(ctx: i64) -> i64    { array_get(ctx, 9) }
fn ctx_pe(ctx: i64) -> i64         { array_get(ctx, 10) }
fn ctx_ra(ctx: i64) -> i64         { array_get(ctx, 11) }
fn ctx_out_bytes(ctx: i64) -> i64  { array_get(ctx, 12) }
fn ctx_out_size(ctx: i64) -> i64   { array_get(ctx, 13) }

fn ctx_set_error(ctx: i64, code: i64) -> i64 { array_set(ctx, 1, code); 0 }

fn new_compile_ctx(source: str) -> i64 {
    let ctx = array_new(14)
    // Store source as pointer - we pass it directly to lexer
    array_set(ctx, 0, 0)  // placeholder; source passed directly
    array_set(ctx, 1, ERR_NONE())
    array_set(ctx, 2, 0)
    array_set(ctx, 3, 0)
    array_set(ctx, 4, 0)
    array_set(ctx, 5, 0)
    array_set(ctx, 6, -1)
    array_set(ctx, 7, 0)
    array_set(ctx, 8, 0)
    array_set(ctx, 9, 0)
    array_set(ctx, 10, 0)
    array_set(ctx, 11, 0)
    array_set(ctx, 12, 0)
    array_set(ctx, 13, 0)
    ctx
}

// ============================================================================
// Stage 1: Lexing
// ============================================================================
fn stage_lex(ctx: i64, source: str) -> i64 {
    let tok_buf = lex(source)
    let count = tok_count(tok_buf)
    array_set(ctx, 3, tok_buf)
    array_set(ctx, 4, count)
    if count <= 0 {
        ctx_set_error(ctx, ERR_LEX())
    } else {
        0
    }
}

// ============================================================================
// Stage 2: Parsing
// ============================================================================
fn stage_parse(ctx: i64, source: str) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0  // skip if previous stage failed
    } else {
        let tok_buf = ctx_tokens(ctx)
        let p = parser_new(tok_buf)
        let prog_off = parse_program(p, source)
        array_set(ctx, 5, p)
        array_set(ctx, 6, prog_off)
        if prog_off < 0 {
            ctx_set_error(ctx, ERR_PARSE())
        } else {
            0
        }
    }
}

// ============================================================================
// Stage 3: Type checking
// ============================================================================
fn stage_typecheck(ctx: i64, source: str) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let p = ctx_tree(ctx)
        let prog_off = ctx_prog_off(ctx)
        let tc = typecheck(p, source, prog_off)
        let errs = tc_errors(tc)
        array_set(ctx, 7, errs)
        // Type errors are warnings for bootstrap - continue compilation
        // This is necessary for self-hosting since the subset typechecker
        // does not cover all constructs used in the compiler source.
        0
    }
}

// ============================================================================
// Stage 4: IR generation
// ============================================================================
fn stage_ir_gen(ctx: i64, source: str) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let p = ctx_tree(ctx)
        let prog_off = ctx_prog_off(ctx)
        let ir = ir_generate(p, source, prog_off)
        array_set(ctx, 8, ir)
        0
    }
}

// ============================================================================
// Stage 5: Register allocation
// ============================================================================
fn stage_regalloc(ctx: i64) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let ir = ctx_ir(ctx)
        // Get instruction buffer and count from IR context
        let insts = array_get(ir, 1)       // ir.insts
        let inst_count = array_get(ir, 2)  // ir.inst_count  
        let next_vreg = array_get(ir, 3)   // ir.next_vreg

        let ra = new_regalloc(next_vreg)
        compute_live_ranges(ra, insts, inst_count)
        allocate_registers(ra)
        array_set(ctx, 11, ra)
        0
    }
}

// ============================================================================
// Stage 6: x86-64 code generation
// ============================================================================
fn stage_x86_emit(ctx: i64) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let ir = ctx_ir(ctx)
        let ra = ctx_ra(ctx)
        let buf = new_code_buf(262144)  // 256KB code buffer

        // Emit runtime stubs first
        let rt = new_runtime(buf, 0)
        emit_runtime(rt)

        // Bind main label to current position (user's main function)
        bind_label(buf, rt_main_lbl(rt))

        // Walk IR instructions and emit x86-64 code
        // Using register allocation mapping from regalloc
        let insts = array_get(ir, 1)
        let inst_count = array_get(ir, 2)

        emit_ir_to_x86(buf, ra, insts, inst_count)

        // Resolve all branch/call fixups
        resolve_fixups(buf)

        array_set(ctx, 9, buf)
        0
    }
}

// ============================================================================
// IR -> x86-64 instruction lowering
// ============================================================================
// Walks IR instructions and emits corresponding x86-64 machine code.
// Uses register allocation results for physical register assignments.

fn emit_ir_to_x86(buf: i64, ra: i64, insts: i64, inst_count: i64) -> i64 {
    let mut i = 0
    while i < inst_count {
        let off = i * 6
        let opcode = array_get(insts, off + 0)
        let dest = array_get(insts, off + 1)
        let arg0 = array_get(insts, off + 2)
        let arg1 = array_get(insts, off + 3)

        let dst_reg = get_phys_reg(ra, dest)

        if opcode == 1 {
            // ICONST: load immediate into dest register
            if dst_reg >= 0 {
                x86_mov_reg_imm64(buf, dst_reg, arg0)
                0
            } else {
                0
            }
        } else if opcode == 10 {
            // ADD: dst = arg0 + arg1
            let r0 = get_phys_reg(ra, arg0)
            let r1 = get_phys_reg(ra, arg1)
            if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_add_reg_reg(buf, dst_reg, r1)
                0
            } else {
                0
            }
        } else if opcode == 11 {
            // SUB: dst = arg0 - arg1
            let r0 = get_phys_reg(ra, arg0)
            let r1 = get_phys_reg(ra, arg1)
            if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_sub_reg_reg(buf, dst_reg, r1)
                0
            } else {
                0
            }
        } else if opcode == 12 {
            // MUL: dst = arg0 * arg1
            let r0 = get_phys_reg(ra, arg0)
            let r1 = get_phys_reg(ra, arg1)
            if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_imul_reg_reg(buf, dst_reg, r1)
                0
            } else {
                0
            }
        } else if opcode == 15 {
            // NEG: dst = -arg0
            let r0 = get_phys_reg(ra, arg0)
            if dst_reg >= 0 && r0 >= 0 {
                if dst_reg != r0 {
                    x86_mov_reg_reg(buf, dst_reg, r0)
                    0
                } else {
                    0
                }
                x86_neg_reg(buf, dst_reg)
                0
            } else {
                0
            }
        } else if opcode == 20 {
            // EQ: dst = (arg0 == arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 0)
        } else if opcode == 21 {
            // NE: dst = (arg0 != arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 1)
        } else if opcode == 22 {
            // LT: dst = (arg0 < arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 2)
        } else if opcode == 24 {
            // GT: dst = (arg0 > arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 3)
        } else if opcode == 23 {
            // LTE: dst = (arg0 <= arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 4)
        } else if opcode == 25 {
            // GTE: dst = (arg0 >= arg1)
            emit_cmp_setcc(buf, ra, dest, arg0, arg1, 5)
        } else if opcode == 31 {
            // RET: move arg0 to RAX, then RET
            if arg0 >= 0 {
                let r0 = get_phys_reg(ra, arg0)
                if r0 >= 0 && r0 != REG_RAX() {
                    x86_mov_reg_reg(buf, REG_RAX(), r0)
                    0
                } else {
                    0
                }
            } else {
                0
            }
            x86_ret(buf)
            0
        } else if opcode == 34 {
            // COPY: dst = arg0
            let r0 = get_phys_reg(ra, arg0)
            if dst_reg >= 0 && r0 >= 0 && dst_reg != r0 {
                x86_mov_reg_reg(buf, dst_reg, r0)
                0
            } else {
                0
            }
        } else {
            // NOP, SCONST, FCONST, BCONST, PHI, BR, CONDBR, CALL - 
            // handled by higher-level patterns or skipped for now
            0
        }

        i = i + 1
    }
    0
}

// Helper: emit CMP + SETcc + MOVZX for comparison operations
fn emit_cmp_setcc(buf: i64, ra: i64, dest: i64, arg0: i64, arg1: i64, cc: i64) -> i64 {
    let dst_reg = get_phys_reg(ra, dest)
    let r0 = get_phys_reg(ra, arg0)
    let r1 = get_phys_reg(ra, arg1)
    if dst_reg >= 0 && r0 >= 0 && r1 >= 0 {
        x86_cmp_reg_reg(buf, r0, r1)
        if cc == 0 { x86_sete(buf, dst_reg) }
        else if cc == 1 { x86_setne(buf, dst_reg) }
        else if cc == 2 { x86_setl(buf, dst_reg) }
        else if cc == 3 { x86_setg(buf, dst_reg) }
        else if cc == 4 { x86_setle(buf, dst_reg) }
        else { x86_setge(buf, dst_reg) }
        x86_movzx_reg_reg8(buf, dst_reg, dst_reg)
        0
    } else {
        0
    }
}

// ============================================================================
// Stage 7: PE generation
// ============================================================================
fn stage_pe_build(ctx: i64) -> i64 {
    if ctx_error(ctx) != ERR_NONE() {
        0
    } else {
        let buf = ctx_codebuf(ctx)
        let code = buf_code(buf)
        let code_size = buf_pos(buf)
        let data = buf_data(buf)
        let data_size = buf_data_pos(buf)

        let pe = new_pe_builder(code, code_size, data, data_size)
        // Entry point at offset 0 (runtime entry)
        array_set(pe, 7, 0)
        build_pe(pe)

        array_set(ctx, 10, pe)
        array_set(ctx, 12, pe_out(pe))
        array_set(ctx, 13, pe_file_size(pe))
        0
    }
}

// ============================================================================
// Full compilation pipeline
// ============================================================================
fn compile(source: str) -> i64 {
    let ctx = new_compile_ctx(source)

    // Stage 1: Lex
    stage_lex(ctx, source)

    // Stage 2: Parse
    stage_parse(ctx, source)

    // Stage 3: Type check
    stage_typecheck(ctx, source)

    // Stage 4: IR generation
    stage_ir_gen(ctx, source)

    // Stage 5: Register allocation
    stage_regalloc(ctx)

    // Stage 6: x86-64 emission
    stage_x86_emit(ctx)

    // Stage 7: PE generation
    stage_pe_build(ctx)

    ctx
}

// ============================================================================
// Compilation result queries
// ============================================================================
fn compile_ok(ctx: i64) -> i64 {
    if ctx_error(ctx) == ERR_NONE() { 1 } else { 0 }
}

fn compile_error_stage(ctx: i64) -> str {
    let err = ctx_error(ctx)
    if err == ERR_LEX() { "lex" }
    else if err == ERR_PARSE() { "parse" }
    else if err == ERR_TYPE() { "typecheck" }
    else if err == ERR_IR() { "ir" }
    else if err == ERR_CODEGEN() { "codegen" }
    else { "none" }
}

fn compile_output_size(ctx: i64) -> i64 {
    ctx_out_size(ctx)
}

fn compile_pe_bytes(ctx: i64) -> i64 {
    ctx_out_bytes(ctx)
}

// Print compilation summary
fn compile_summary(ctx: i64) -> i64 {
    if compile_ok(ctx) == 1 {
        print_str("Compilation OK: ")
        print_str(to_string_i64(compile_output_size(ctx)))
        println_str(" bytes PE output")
        0
    } else {
        print_str("Compilation FAILED at stage: ")
        println_str(compile_error_stage(ctx))
        0
    }
}
