// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 4: IR Generation
// ============================================================================
// Lowers typed AST (from parser.sl) to a flat SSA-form IR.
//
// IR layout: flat array of instructions, each stride=6:
//   [opcode, dest, arg0, arg1, arg2, extra]
//
// Virtual registers: v0, v1, v2, ... (incrementing counter)
// Basic blocks: b0, b1, b2, ... (each holds a range of instructions)
//
// Instructions (opcodes):
//   OP_ICONST=1   dest = immediate i64 value (in arg0, arg1 for 64-bit)
//   OP_SCONST=2   dest = string constant index
//   OP_FCONST=3   dest = float constant index
//   OP_ADD=10      dest = arg0 + arg1
//   OP_SUB=11      dest = arg0 - arg1
//   OP_MUL=12      dest = arg0 * arg1
//   OP_DIV=13      dest = arg0 / arg1
//   OP_MOD=14      dest = arg0 % arg1
//   OP_NEG=15      dest = -arg0
//   OP_EQ=20       dest = arg0 == arg1
//   OP_NEQ=21      dest = arg0 != arg1
//   OP_LT=22       dest = arg0 < arg1
//   OP_GT=23       dest = arg0 > arg1
//   OP_LTE=24      dest = arg0 <= arg1
//   OP_GTE=25      dest = arg0 >= arg1
//   OP_AND=26      dest = arg0 && arg1
//   OP_OR=27       dest = arg0 || arg1
//   OP_NOT=28      dest = !arg0
//   OP_CALL=30     dest = call func(arg0=fn_idx, arg1=args_start, arg2=arg_count)
//   OP_RET=31      return arg0 (dest unused)
//   OP_BR=32       unconditional branch to block arg0
//   OP_CONDBR=33   branch: if arg0 then block arg1 else block arg2
//   OP_COPY=34     dest = arg0 (register copy / move)
//   OP_PHI=35      dest = phi(arg0=block1_val, arg1=block2_val) -- simplified
//   OP_NOP=0       no operation
//
// NOTE: No `return` in if-blocks. All control flow uses if/else expressions.
// ============================================================================

// -- IR opcode constants --
fn OP_NOP() -> i64    { 0 }
fn OP_ICONST() -> i64 { 1 }
fn OP_SCONST() -> i64 { 2 }
fn OP_FCONST() -> i64 { 3 }
fn OP_BCONST() -> i64 { 4 }

fn OP_ADD() -> i64    { 10 }
fn OP_SUB() -> i64    { 11 }
fn OP_MUL() -> i64    { 12 }
fn OP_DIV() -> i64    { 13 }
fn OP_MOD() -> i64    { 14 }
fn OP_NEG() -> i64    { 15 }

fn OP_EQ() -> i64     { 20 }
fn OP_NEQ() -> i64    { 21 }
fn OP_LT() -> i64     { 22 }
fn OP_GT() -> i64     { 23 }
fn OP_LTE() -> i64    { 24 }
fn OP_GTE() -> i64    { 25 }
fn OP_AND() -> i64    { 26 }
fn OP_OR() -> i64     { 27 }
fn OP_NOT() -> i64    { 28 }

fn OP_CALL() -> i64   { 30 }
fn OP_RET() -> i64    { 31 }
fn OP_BR() -> i64     { 32 }
fn OP_CONDBR() -> i64 { 33 }
fn OP_COPY() -> i64   { 34 }
fn OP_PHI() -> i64    { 35 }

fn IR_STRIDE() -> i64 { 6 }

// ============================================================================
// String hashing (djb2) - provided by typechecker.sl in combined builds.
// When running ir_gen standalone, define str_hash in the test harness.
// ============================================================================

// ============================================================================
// IR context (ir)
// ============================================================================
// ir[0] = parser context (p)
// ir[1] = instructions array (flat, stride 6)
// ir[2] = inst_count
// ir[3] = next_vreg
// ir[4] = blocks array [start_inst, end_inst] stride=2
// ir[5] = block_count
// ir[6] = current_block
// ir[7] = string constants array (token indices for SCONST lookup)
// ir[8] = string_count
// ir[9] = function table array (for mapping fn names -> fn indices)
// ir[10] = fn_table_count
// ir[11] = variable map array [name_hash, vreg] stride=2
// ir[12] = var_count
// ir[13] = var scope depth
// ir[14] = call args temp buffer
// ir[15] = fn_ir_entries array [name_hash, start_block, end_block, param_count] stride=4
// ir[16] = fn_ir_count

fn ir_p(ir: i64) -> i64          { array_get(ir, 0) }
fn ir_insts(ir: i64) -> i64      { array_get(ir, 1) }
fn ir_inst_count(ir: i64) -> i64  { array_get(ir, 2) }
fn ir_next_vreg(ir: i64) -> i64   { array_get(ir, 3) }
fn ir_blocks(ir: i64) -> i64      { array_get(ir, 4) }
fn ir_block_count(ir: i64) -> i64  { array_get(ir, 5) }
fn ir_cur_block(ir: i64) -> i64    { array_get(ir, 6) }
fn ir_strings(ir: i64) -> i64     { array_get(ir, 7) }
fn ir_string_count(ir: i64) -> i64 { array_get(ir, 8) }
fn ir_fn_table(ir: i64) -> i64    { array_get(ir, 9) }
fn ir_fn_table_count(ir: i64) -> i64 { array_get(ir, 10) }
fn ir_vars(ir: i64) -> i64        { array_get(ir, 11) }
fn ir_var_count(ir: i64) -> i64    { array_get(ir, 12) }
fn ir_var_depth(ir: i64) -> i64    { array_get(ir, 13) }
fn ir_call_args(ir: i64) -> i64    { array_get(ir, 14) }
fn ir_fn_entries(ir: i64) -> i64   { array_get(ir, 15) }
fn ir_fn_entry_count(ir: i64) -> i64 { array_get(ir, 16) }

fn ir_set_inst_count(ir: i64, v: i64) -> i64  { array_set(ir, 2, v); 0 }
fn ir_set_next_vreg(ir: i64, v: i64) -> i64   { array_set(ir, 3, v); 0 }
fn ir_set_block_count(ir: i64, v: i64) -> i64  { array_set(ir, 5, v); 0 }
fn ir_set_cur_block(ir: i64, v: i64) -> i64    { array_set(ir, 6, v); 0 }
fn ir_set_string_count(ir: i64, v: i64) -> i64 { array_set(ir, 8, v); 0 }
fn ir_set_fn_table_count(ir: i64, v: i64) -> i64 { array_set(ir, 10, v); 0 }
fn ir_set_var_count(ir: i64, v: i64) -> i64    { array_set(ir, 12, v); 0 }
fn ir_set_var_depth(ir: i64, v: i64) -> i64    { array_set(ir, 13, v); 0 }
fn ir_set_fn_entry_count(ir: i64, v: i64) -> i64 { array_set(ir, 16, v); 0 }

// ============================================================================
// Virtual register allocation
// ============================================================================
fn alloc_vreg(ir: i64) -> i64 {
    let v = ir_next_vreg(ir)
    ir_set_next_vreg(ir, v + 1)
    v
}

// ============================================================================
// Basic block management
// ============================================================================
fn new_block(ir: i64) -> i64 {
    let bid = ir_block_count(ir)
    let blocks = ir_blocks(ir)
    let off = bid * 2
    array_set(blocks, off + 0, ir_inst_count(ir))
    array_set(blocks, off + 1, ir_inst_count(ir))
    ir_set_block_count(ir, bid + 1)
    bid
}

fn seal_block(ir: i64, bid: i64) -> i64 {
    let blocks = ir_blocks(ir)
    let off = bid * 2
    array_set(blocks, off + 1, ir_inst_count(ir))
    0
}

fn switch_block(ir: i64, bid: i64) -> i64 {
    // Seal current block, switch to new one
    let cur = ir_cur_block(ir)
    seal_block(ir, cur)
    ir_set_cur_block(ir, bid)
    0
}

// ============================================================================
// Emit IR instructions
// ============================================================================
fn emit_ir(ir: i64, op: i64, dest: i64, a0: i64, a1: i64, a2: i64, extra: i64) -> i64 {
    let insts = ir_insts(ir)
    let idx = ir_inst_count(ir)
    let off = idx * IR_STRIDE()
    array_set(insts, off + 0, op)
    array_set(insts, off + 1, dest)
    array_set(insts, off + 2, a0)
    array_set(insts, off + 3, a1)
    array_set(insts, off + 4, a2)
    array_set(insts, off + 5, extra)
    ir_set_inst_count(ir, idx + 1)
    dest
}

fn emit_iconst(ir: i64, value: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_ICONST(), dest, value, 0, 0, 0)
}

fn emit_bconst(ir: i64, value: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_BCONST(), dest, value, 0, 0, 0)
}

fn emit_sconst(ir: i64, tok_idx: i64) -> i64 {
    // Store string constant index
    let strings = ir_strings(ir)
    let si = ir_string_count(ir)
    array_set(strings, si, tok_idx)
    ir_set_string_count(ir, si + 1)
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_SCONST(), dest, si, 0, 0, 0)
}

fn emit_binop(ir: i64, op: i64, left: i64, right: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, op, dest, left, right, 0, 0)
}

fn emit_unop(ir: i64, op: i64, operand: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, op, dest, operand, 0, 0, 0)
}

fn emit_copy(ir: i64, src: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_COPY(), dest, src, 0, 0, 0)
}

fn emit_call(ir: i64, fn_idx: i64, args_start: i64, arg_count: i64) -> i64 {
    let dest = alloc_vreg(ir)
    emit_ir(ir, OP_CALL(), dest, fn_idx, args_start, arg_count, 0)
}

fn emit_ret(ir: i64, val: i64) -> i64 {
    emit_ir(ir, OP_RET(), 0, val, 0, 0, 0)
}

fn emit_br(ir: i64, target_block: i64) -> i64 {
    emit_ir(ir, OP_BR(), 0, target_block, 0, 0, 0)
}

fn emit_condbr(ir: i64, cond: i64, then_block: i64, else_block: i64) -> i64 {
    emit_ir(ir, OP_CONDBR(), 0, cond, then_block, else_block, 0)
}

// ============================================================================
// Variable mapping (name_hash -> vreg)
// ============================================================================

fn var_set(ir: i64, name_hash: i64, vreg: i64) -> i64 {
    let vars = ir_vars(ir)
    let cnt = ir_var_count(ir)
    // Update existing entry if found (scan backwards for most recent)
    let mut i = cnt - 1
    let mut done = 0
    while i >= 0 && done == 0 {
        let off = i * 2
        if array_get(vars, off) == name_hash {
            array_set(vars, off + 1, vreg)
            done = 1
            0
        } else {
            i = i - 1
        }
    }
    // If not found, add new entry
    if done == 0 {
        let off = cnt * 2
        array_set(vars, off, name_hash)
        array_set(vars, off + 1, vreg)
        ir_set_var_count(ir, cnt + 1)
        0
    } else {
        0
    }
}

fn var_get(ir: i64, name_hash: i64) -> i64 {
    let vars = ir_vars(ir)
    let cnt = ir_var_count(ir)
    let mut i = cnt - 1
    let mut result = -1
    let mut done = 0
    while i >= 0 && done == 0 {
        let off = i * 2
        if array_get(vars, off) == name_hash {
            result = array_get(vars, off + 1)
            done = 1
            0
        } else {
            i = i - 1
        }
    }
    result
}

// Scope management for variables
fn var_scope_enter(ir: i64) -> i64 {
    ir_set_var_depth(ir, ir_var_depth(ir) + 1)
    ir_var_count(ir)
}

fn var_scope_leave(ir: i64, restore: i64) -> i64 {
    ir_set_var_depth(ir, ir_var_depth(ir) - 1)
    ir_set_var_count(ir, restore)
    0
}

// ============================================================================
// Function name -> index mapping
// ============================================================================
fn fn_register_ir(ir: i64, name_hash: i64) -> i64 {
    let ft = ir_fn_table(ir)
    let cnt = ir_fn_table_count(ir)
    // Check if already registered
    let mut i = 0
    let mut found = -1
    let mut done = 0
    while i < cnt && done == 0 {
        if array_get(ft, i) == name_hash {
            found = i
            done = 1
            0
        } else {
            i = i + 1
        }
    }
    if found >= 0 {
        found
    } else {
        array_set(ft, cnt, name_hash)
        ir_set_fn_table_count(ir, cnt + 1)
        cnt
    }
}

fn fn_lookup_ir(ir: i64, name_hash: i64) -> i64 {
    let ft = ir_fn_table(ir)
    let cnt = ir_fn_table_count(ir)
    let mut i = 0
    let mut result = -1
    let mut done = 0
    while i < cnt && done == 0 {
        if array_get(ft, i) == name_hash {
            result = i
            done = 1
            0
        } else {
            i = i + 1
        }
    }
    result
}

// Record which blocks belong to which function
fn fn_entry_add(ir: i64, name_hash: i64, start_block: i64, param_count: i64) -> i64 {
    let entries = ir_fn_entries(ir)
    let cnt = ir_fn_entry_count(ir)
    let off = cnt * 4
    array_set(entries, off + 0, name_hash)
    array_set(entries, off + 1, start_block)
    array_set(entries, off + 2, -1)  // end_block filled later
    array_set(entries, off + 3, param_count)
    ir_set_fn_entry_count(ir, cnt + 1)
    cnt
}

fn fn_entry_set_end(ir: i64, entry_idx: i64, end_block: i64) -> i64 {
    let entries = ir_fn_entries(ir)
    let off = entry_idx * 4
    array_set(entries, off + 2, end_block)
    0
}

// ============================================================================
// Token operator -> IR opcode mapping
// ============================================================================
fn tok_to_binop(tok_type: i64) -> i64 {
    if tok_type == TK_PLUS() { OP_ADD() }
    else if tok_type == TK_MINUS() { OP_SUB() }
    else if tok_type == TK_STAR() { OP_MUL() }
    else if tok_type == TK_SLASH() { OP_DIV() }
    else if tok_type == TK_PERCENT() { OP_MOD() }
    else if tok_type == TK_EQEQ() { OP_EQ() }
    else if tok_type == TK_NEQ() { OP_NEQ() }
    else if tok_type == TK_LT() { OP_LT() }
    else if tok_type == TK_GT() { OP_GT() }
    else if tok_type == TK_LTE() { OP_LTE() }
    else if tok_type == TK_GTE() { OP_GTE() }
    else if tok_type == TK_AND() { OP_AND() }
    else if tok_type == TK_OR() { OP_OR() }
    else { OP_NOP() }
}

// ============================================================================
// Lower expressions to IR
// ============================================================================

fn lower_expr(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let nt = node_type(p, off)
    if nt == N_INT() { lower_int(ir, p, off) }
    else if nt == N_BOOL() { lower_bool(ir, p, off) }
    else if nt == N_STR() { lower_str(ir, p, off) }
    else if nt == N_IDENT() { lower_ident(ir, source, off) }
    else if nt == N_BINOP() { lower_binop(ir, source, off) }
    else if nt == N_UNOP() { lower_unop(ir, source, off) }
    else if nt == N_CALL() { lower_call(ir, source, off) }
    else if nt == N_IF() { lower_if_expr(ir, source, off) }
    else {
        // Unknown expression type, return 0
        emit_iconst(ir, 0)
    }
}

fn lower_int(ir: i64, p: i64, off: i64) -> i64 {
    let value = node_d0(p, off)
    emit_iconst(ir, value)
}

fn lower_bool(ir: i64, p: i64, off: i64) -> i64 {
    let value = node_d0(p, off)
    emit_bconst(ir, value)
}

fn lower_str(ir: i64, p: i64, off: i64) -> i64 {
    let tok_idx = node_d0(p, off)
    emit_sconst(ir, tok_idx)
}

fn lower_ident(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let tok_idx = node_d0(p, off)
    let name = tok_text(source, p_tokens(p), tok_idx)
    let h = str_hash(name)
    let vreg = var_get(ir, h)
    if vreg >= 0 {
        vreg
    } else {
        // Should not happen after type checking, but emit 0
        emit_iconst(ir, 0)
    }
}

fn lower_binop(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let op_tok = node_d0(p, off)
    let left_off = node_d1(p, off)
    let right_off = node_d2(p, off)

    // Pipe operator: call RHS with LHS as argument
    if op_tok == TK_PIPE() {
        lower_pipe(ir, source, left_off, right_off)
    } else {
        let left_vreg = lower_expr(ir, source, left_off)
        let right_vreg = lower_expr(ir, source, right_off)
        let ir_op = tok_to_binop(op_tok)
        emit_binop(ir, ir_op, left_vreg, right_vreg)
    }
}

fn lower_pipe(ir: i64, source: str, lhs_off: i64, rhs_off: i64) -> i64 {
    // lhs |> rhs  ->  rhs(lhs)
    // RHS should be an identifier (function name)
    let p = ir_p(ir)
    let lhs_vreg = lower_expr(ir, source, lhs_off)

    // RHS should be N_IDENT for a simple pipe
    let rhs_nt = node_type(p, rhs_off)
    if rhs_nt == N_IDENT() {
        let fn_tok = node_d0(p, rhs_off)
        let fn_name = tok_text(source, p_tokens(p), fn_tok)
        let fn_hash = str_hash(fn_name)
        let fn_idx = fn_lookup_ir(ir, fn_hash)

        // Store arg
        let call_args = ir_call_args(ir)
        array_set(call_args, 0, lhs_vreg)
        emit_call(ir, fn_idx, 0, 1)
    } else {
        // Not a simple pipe, just return LHS
        lhs_vreg
    }
}

fn lower_unop(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let op_tok = node_d0(p, off)
    let operand_off = node_d1(p, off)
    let operand_vreg = lower_expr(ir, source, operand_off)

    if op_tok == TK_MINUS() {
        emit_unop(ir, OP_NEG(), operand_vreg)
    } else if op_tok == TK_NOT() {
        emit_unop(ir, OP_NOT(), operand_vreg)
    } else {
        operand_vreg
    }
}

fn lower_call(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let callee_tok = node_d0(p, off)
    let fn_name = tok_text(source, p_tokens(p), callee_tok)
    let fn_hash = str_hash(fn_name)
    let fn_idx = fn_register_ir(ir, fn_hash)
    let args_lstart = node_d1(p, off)
    let arg_count = node_d2(p, off)

    // Lower each argument and store in call args buffer
    let call_args = ir_call_args(ir)
    let mut i = 0
    while i < arg_count {
        let arg_off = list_item(p, args_lstart + i)
        let arg_vreg = lower_expr(ir, source, arg_off)
        array_set(call_args, i, arg_vreg)
        i = i + 1
    }

    emit_call(ir, fn_idx, 0, arg_count)
}

// ============================================================================
// Lower if expression (produces a value)
// ============================================================================
fn lower_if_expr(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    // Lower condition
    let cond_vreg = lower_expr(ir, source, cond_off)

    // Create blocks
    let then_block = new_block(ir)
    let merge_block = new_block(ir)

    if else_off >= 0 {
        let else_block = new_block(ir)
        emit_condbr(ir, cond_vreg, then_block, else_block)

        // Then block
        switch_block(ir, then_block)
        let then_val = lower_block_expr(ir, source, then_off)
        let then_result = emit_copy(ir, then_val)
        emit_br(ir, merge_block)

        // Else block
        switch_block(ir, else_block)
        let else_val = lower_block_or_if(ir, source, else_off)
        let else_result = emit_copy(ir, else_val)
        emit_br(ir, merge_block)

        // Merge block with phi
        switch_block(ir, merge_block)
        let result = alloc_vreg(ir)
        emit_ir(ir, OP_PHI(), result, then_result, else_result, 0, 0)
    } else {
        // No else: void result
        emit_condbr(ir, cond_vreg, then_block, merge_block)

        // Then block
        switch_block(ir, then_block)
        lower_block_stmts(ir, source, then_off)
        emit_br(ir, merge_block)

        // Merge block
        switch_block(ir, merge_block)
        emit_iconst(ir, 0)
    }
}

fn lower_block_or_if(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let nt = node_type(p, off)
    if nt == N_BLOCK() { lower_block_expr(ir, source, off) }
    else if nt == N_IF() { lower_if_expr(ir, source, off) }
    else { lower_expr(ir, source, off) }
}

// ============================================================================
// Lower statements
// ============================================================================

fn lower_stmt(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let nt = node_type(p, off)
    if nt == N_LET() { lower_let(ir, source, off) }
    else if nt == N_ASSIGN() { lower_assign(ir, source, off) }
    else if nt == N_IF() { lower_if_stmt(ir, source, off) }
    else if nt == N_WHILE() { lower_while(ir, source, off) }
    else if nt == N_FOR() { lower_for(ir, source, off) }
    else if nt == N_RETURN() { lower_return(ir, source, off) }
    else if nt == N_BLOCK() { lower_block_stmts(ir, source, off); 0 }
    else if nt == N_EXPR_STMT() { lower_expr(ir, source, node_d0(p, off)); 0 }
    else { lower_expr(ir, source, off); 0 }
}

fn lower_let(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let name_tok = node_d0(p, off)
    let val_off = node_d3(p, off)
    let tokens = p_tokens(p)
    let name = tok_text(source, tokens, name_tok)
    let h = str_hash(name)

    let val_vreg = lower_expr(ir, source, val_off)
    var_set(ir, h, val_vreg)
    0
}

fn lower_assign(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let target_tok = node_d0(p, off)
    let rhs_off = node_d1(p, off)
    let tokens = p_tokens(p)
    let name = tok_text(source, tokens, target_tok)
    let h = str_hash(name)

    let val_vreg = lower_expr(ir, source, rhs_off)
    var_set(ir, h, val_vreg)
    0
}

fn lower_if_stmt(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    let cond_vreg = lower_expr(ir, source, cond_off)

    let then_block = new_block(ir)
    let merge_block = new_block(ir)

    if else_off >= 0 {
        let else_block = new_block(ir)
        emit_condbr(ir, cond_vreg, then_block, else_block)

        switch_block(ir, then_block)
        lower_block_stmts(ir, source, then_off)
        emit_br(ir, merge_block)

        switch_block(ir, else_block)
        let nt = node_type(p, else_off)
        if nt == N_IF() {
            lower_if_stmt(ir, source, else_off)
            0
        } else {
            lower_block_stmts(ir, source, else_off)
            0
        }
        emit_br(ir, merge_block)

        switch_block(ir, merge_block)
        0
    } else {
        emit_condbr(ir, cond_vreg, then_block, merge_block)

        switch_block(ir, then_block)
        lower_block_stmts(ir, source, then_off)
        emit_br(ir, merge_block)

        switch_block(ir, merge_block)
        0
    }
}

fn lower_while(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let cond_off = node_d0(p, off)
    let body_off = node_d1(p, off)

    let cond_block = new_block(ir)
    let body_block = new_block(ir)
    let exit_block = new_block(ir)

    emit_br(ir, cond_block)

    switch_block(ir, cond_block)
    let cond_vreg = lower_expr(ir, source, cond_off)
    emit_condbr(ir, cond_vreg, body_block, exit_block)

    switch_block(ir, body_block)
    lower_block_stmts(ir, source, body_off)
    emit_br(ir, cond_block)

    switch_block(ir, exit_block)
    0
}

fn lower_for(ir: i64, source: str, off: i64) -> i64 {
    // For now, treat for as: let iter_var = 0; while iter_var < range { body; iter_var = iter_var + 1 }
    // This is a simplified version; real for/in range requires more work
    let p = ir_p(ir)
    let var_tok = node_d0(p, off)
    let iter_off = node_d1(p, off)
    let body_off = node_d2(p, off)
    let tokens = p_tokens(p)
    let name = tok_text(source, tokens, var_tok)
    let h = str_hash(name)

    // Lower the iterable (range end)
    let range_vreg = lower_expr(ir, source, iter_off)

    // Init loop var to 0
    let init_vreg = emit_iconst(ir, 0)
    var_set(ir, h, init_vreg)

    let cond_block = new_block(ir)
    let body_block = new_block(ir)
    let exit_block = new_block(ir)

    emit_br(ir, cond_block)

    // Condition: var < range
    switch_block(ir, cond_block)
    let cur_vreg = var_get(ir, h)
    let cond_vreg = emit_binop(ir, OP_LT(), cur_vreg, range_vreg)
    emit_condbr(ir, cond_vreg, body_block, exit_block)

    // Body
    switch_block(ir, body_block)
    lower_block_stmts(ir, source, body_off)
    // Increment
    let inc_cur = var_get(ir, h)
    let one = emit_iconst(ir, 1)
    let next_vreg = emit_binop(ir, OP_ADD(), inc_cur, one)
    var_set(ir, h, next_vreg)
    emit_br(ir, cond_block)

    switch_block(ir, exit_block)
    0
}

fn lower_return(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let val_off = node_d0(p, off)
    if val_off >= 0 {
        let val_vreg = lower_expr(ir, source, val_off)
        emit_ret(ir, val_vreg)
    } else {
        let zero = emit_iconst(ir, 0)
        emit_ret(ir, zero)
    }
}

// ============================================================================
// Block lowering
// ============================================================================

// Lower a block for its statements (no return value)
fn lower_block_stmts(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let save = var_scope_enter(ir)
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        lower_stmt(ir, source, stmt_off)
        i = i + 1
    }
    var_scope_leave(ir, save)
    0
}

// Lower a block and return the value of its last expression
fn lower_block_expr(ir: i64, source: str, off: i64) -> i64 {
    let p = ir_p(ir)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let save = var_scope_enter(ir)
    let mut last_vreg = emit_iconst(ir, 0)
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        if i == count - 1 {
            // Last item: treat as expression for value
            let nt = node_type(p, stmt_off)
            if nt == N_EXPR_STMT() {
                last_vreg = lower_expr(ir, source, node_d0(p, stmt_off))
                0
            } else if nt == N_LET() || nt == N_ASSIGN() || nt == N_WHILE() || nt == N_FOR() {
                lower_stmt(ir, source, stmt_off)
                last_vreg = emit_iconst(ir, 0)
                0
            } else {
                last_vreg = lower_expr(ir, source, stmt_off)
                0
            }
        } else {
            lower_stmt(ir, source, stmt_off)
            0
        }
        i = i + 1
    }
    var_scope_leave(ir, save)
    last_vreg
}

// ============================================================================
// Lower a function definition
// ============================================================================
fn lower_function(ir: i64, source: str, fn_off: i64) -> i64 {
    let p = ir_p(ir)
    let tokens = p_tokens(p)
    let name_tok = node_d0(p, fn_off)
    let params_start = node_d1(p, fn_off)
    let param_count = node_d2(p, fn_off)
    let body_off = node_d4(p, fn_off)

    let fn_name = tok_text(source, tokens, name_tok)
    let fn_hash = str_hash(fn_name)

    // Register function
    fn_register_ir(ir, fn_hash)

    // Create entry block
    let entry = new_block(ir)
    switch_block(ir, entry)

    // Record function entry
    let entry_idx = fn_entry_add(ir, fn_hash, entry, param_count)

    // Bind parameters as vregs
    let save = var_scope_enter(ir)
    let mut pi = 0
    while pi < param_count {
        let pn_tok = list_item(p, params_start + pi * 2)
        let pn = tok_text(source, tokens, pn_tok)
        let h = str_hash(pn)
        let param_vreg = alloc_vreg(ir)
        var_set(ir, h, param_vreg)
        pi = pi + 1
    }

    // Lower body
    let body_val = lower_block_expr(ir, source, body_off)
    emit_ret(ir, body_val)

    var_scope_leave(ir, save)

    // Seal the last block and record end
    seal_block(ir, ir_cur_block(ir))
    fn_entry_set_end(ir, entry_idx, ir_cur_block(ir))
    0
}

// ============================================================================
// Lower entire program
// ============================================================================
fn lower_program(ir: i64, source: str, prog_off: i64) -> i64 {
    let p = ir_p(ir)
    let lstart = node_d0(p, prog_off)
    let count = node_d1(p, prog_off)

    // Pre-register all function names
    let tokens = p_tokens(p)
    let mut i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            let name_tok = node_d0(p, fn_off)
            let fn_name = tok_text(source, tokens, name_tok)
            fn_register_ir(ir, str_hash(fn_name))
            0
        } else {
            0
        }
        i = i + 1
    }

    // Lower each function
    i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            lower_function(ir, source, fn_off)
            0
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// IR dump (for testing / debugging)
// ============================================================================
fn op_name(op: i64) -> str {
    if op == OP_NOP() { "NOP" }
    else if op == OP_ICONST() { "ICONST" }
    else if op == OP_SCONST() { "SCONST" }
    else if op == OP_FCONST() { "FCONST" }
    else if op == OP_BCONST() { "BCONST" }
    else if op == OP_ADD() { "ADD" }
    else if op == OP_SUB() { "SUB" }
    else if op == OP_MUL() { "MUL" }
    else if op == OP_DIV() { "DIV" }
    else if op == OP_MOD() { "MOD" }
    else if op == OP_NEG() { "NEG" }
    else if op == OP_EQ() { "EQ" }
    else if op == OP_NEQ() { "NEQ" }
    else if op == OP_LT() { "LT" }
    else if op == OP_GT() { "GT" }
    else if op == OP_LTE() { "LTE" }
    else if op == OP_GTE() { "GTE" }
    else if op == OP_AND() { "AND" }
    else if op == OP_OR() { "OR" }
    else if op == OP_NOT() { "NOT" }
    else if op == OP_CALL() { "CALL" }
    else if op == OP_RET() { "RET" }
    else if op == OP_BR() { "BR" }
    else if op == OP_CONDBR() { "CONDBR" }
    else if op == OP_COPY() { "COPY" }
    else if op == OP_PHI() { "PHI" }
    else { "???" }
}

fn dump_ir(ir: i64) -> i64 {
    let insts = ir_insts(ir)
    let count = ir_inst_count(ir)
    println_str("--- IR dump ---")
    let mut i = 0
    while i < count {
        let off = i * IR_STRIDE()
        let op = array_get(insts, off)
        let dest = array_get(insts, off + 1)
        let a0 = array_get(insts, off + 2)
        let a1 = array_get(insts, off + 3)
        let a2 = array_get(insts, off + 4)

        print_str("  ")
        print_str(to_string_i64(i))
        print_str(": ")

        if op == OP_ICONST() || op == OP_BCONST() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = ")
            print_str(op_name(op))
            print_str(" ")
            println_str(to_string_i64(a0))
        } else if op == OP_SCONST() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = SCONST #")
            println_str(to_string_i64(a0))
        } else if op == OP_ADD() || op == OP_SUB() || op == OP_MUL() || op == OP_DIV() || op == OP_MOD() || op == OP_EQ() || op == OP_NEQ() || op == OP_LT() || op == OP_GT() || op == OP_LTE() || op == OP_GTE() || op == OP_AND() || op == OP_OR() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = ")
            print_str(op_name(op))
            print_str(" v")
            print_str(to_string_i64(a0))
            print_str(", v")
            println_str(to_string_i64(a1))
        } else if op == OP_NEG() || op == OP_NOT() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = ")
            print_str(op_name(op))
            print_str(" v")
            println_str(to_string_i64(a0))
        } else if op == OP_COPY() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = COPY v")
            println_str(to_string_i64(a0))
        } else if op == OP_CALL() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = CALL fn#")
            print_str(to_string_i64(a0))
            print_str(" (")
            print_str(to_string_i64(a2))
            println_str(" args)")
        } else if op == OP_RET() {
            print_str("RET v")
            println_str(to_string_i64(a0))
        } else if op == OP_BR() {
            print_str("BR b")
            println_str(to_string_i64(a0))
        } else if op == OP_CONDBR() {
            print_str("CONDBR v")
            print_str(to_string_i64(a0))
            print_str(" ? b")
            print_str(to_string_i64(a1))
            print_str(" : b")
            println_str(to_string_i64(a2))
        } else if op == OP_PHI() {
            print_str("v")
            print_str(to_string_i64(dest))
            print_str(" = PHI v")
            print_str(to_string_i64(a0))
            print_str(", v")
            println_str(to_string_i64(a1))
        } else {
            print_str(op_name(op))
            println_str("")
        }

        i = i + 1
    }
    println_str("--- end IR ---")
    0
}

// ============================================================================
// Entry point: create IR context and lower program
// ============================================================================
fn ir_generate(p: i64, source: str, prog_off: i64) -> i64 {
    let ir = array_new(17)
    array_set(ir, 0, p)
    array_set(ir, 1, array_new(200000))   // instructions
    array_set(ir, 2, 0)                   // inst_count
    array_set(ir, 3, 0)                   // next_vreg
    array_set(ir, 4, array_new(10000))     // blocks
    array_set(ir, 5, 0)                   // block_count
    array_set(ir, 6, 0)                   // current_block
    array_set(ir, 7, array_new(5000))      // strings
    array_set(ir, 8, 0)                   // string_count
    array_set(ir, 9, array_new(8000))      // fn_table
    array_set(ir, 10, 0)                  // fn_table_count
    array_set(ir, 11, array_new(20000))    // vars
    array_set(ir, 12, 0)                  // var_count
    array_set(ir, 13, 0)                  // var_depth
    array_set(ir, 14, array_new(32))      // call_args
    array_set(ir, 15, array_new(8000))     // fn_entries
    array_set(ir, 16, 0)                  // fn_entry_count

    // Create initial block (block 0)
    new_block(ir)

    lower_program(ir, source, prog_off)
    ir
}
