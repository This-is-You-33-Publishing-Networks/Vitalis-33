// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 3: Type Checker
// ============================================================================
// Walks the AST (from parser.sl), resolves variable names, checks types.
//
// Type representation: i64 constants
//   TY_I64=1, TY_F64=2, TY_BOOL=3, TY_STR=4, TY_VOID=5, TY_ERROR=99
//
// Symbol table: flat array, scope-based with push/pop
//   Each entry: [name_hash, type, scope_depth, mutable]  stride=4
//   Lookup scans backwards to find innermost binding.
//
// Function table: flat array
//   Each entry: [name_hash, return_type, param_count, p0_type, p1_type, ...]
//   Max 8 params per function, stride=11
//
// NOTE: No `return` in if-blocks. All control flow uses if/else expressions.
// ============================================================================

// -- Type constants --
fn TY_I64() -> i64   { 1 }
fn TY_F64() -> i64   { 2 }
fn TY_BOOL() -> i64  { 3 }
fn TY_STR() -> i64   { 4 }
fn TY_VOID() -> i64  { 5 }
fn TY_ERROR() -> i64 { 99 }

// -- AST node type & token constants are provided by lexer.sl/parser.sl
// -- in combined builds. See lexer.sl for TK_* and parser.sl for N_*

// ============================================================================
// String hashing (djb2) for symbol table keys
// ============================================================================
fn str_hash(s: str) -> i64 {
    let slen = str_len(s)
    let mut h = 5381
    let mut i = 0
    while i < slen {
        let c = char_to_int(str_char_at(s, i))
        h = h * 33 + c
        // Keep in positive 32-bit range
        h = h % 2147483647
        i = i + 1
    }
    h
}

// ============================================================================
// Type checker context (tc)
// ============================================================================
// tc[0] = parser context (p)
// tc[1] = source string (stored as i64 - token text lookup via parser)
// tc[2] = symbol table array
// tc[3] = sym_count
// tc[4] = scope_depth
// tc[5] = function table array
// tc[6] = fn_count
// tc[7] = error_count
// tc[8] = current_fn_return_type

fn SYM_STRIDE() -> i64 { 4 }
fn FN_STRIDE() -> i64  { 11 }

fn tc_p(tc: i64) -> i64          { array_get(tc, 0) }
fn tc_syms(tc: i64) -> i64       { array_get(tc, 2) }
fn tc_sym_count(tc: i64) -> i64  { array_get(tc, 3) }
fn tc_depth(tc: i64) -> i64      { array_get(tc, 4) }
fn tc_fns(tc: i64) -> i64        { array_get(tc, 5) }
fn tc_fn_count(tc: i64) -> i64   { array_get(tc, 6) }
fn tc_errors(tc: i64) -> i64     { array_get(tc, 7) }
fn tc_ret_type(tc: i64) -> i64   { array_get(tc, 8) }

fn tc_set_sym_count(tc: i64, v: i64) -> i64  { array_set(tc, 3, v); 0 }
fn tc_set_depth(tc: i64, v: i64) -> i64      { array_set(tc, 4, v); 0 }
fn tc_set_fn_count(tc: i64, v: i64) -> i64   { array_set(tc, 6, v); 0 }
fn tc_set_errors(tc: i64, v: i64) -> i64     { array_set(tc, 7, v); 0 }
fn tc_set_ret_type(tc: i64, v: i64) -> i64   { array_set(tc, 8, v); 0 }

// ============================================================================
// Type name resolution: "i64" -> TY_I64, etc.
// ============================================================================
fn resolve_type_name(name: str) -> i64 {
    if str_eq(name, "i64") { TY_I64() }
    else if str_eq(name, "f64") { TY_F64() }
    else if str_eq(name, "bool") { TY_BOOL() }
    else if str_eq(name, "str") { TY_STR() }
    else if str_eq(name, "void") { TY_VOID() }
    else { TY_ERROR() }
}

fn type_name(t: i64) -> str {
    if t == TY_I64() { "i64" }
    else if t == TY_F64() { "f64" }
    else if t == TY_BOOL() { "bool" }
    else if t == TY_STR() { "str" }
    else if t == TY_VOID() { "void" }
    else { "error" }
}

// ============================================================================
// Node accessors - use parser's definitions when combined.
// Standalone versions provided here for reference:
//   node_type, node_d0..d4, list_item, p_tokens, tok_text
// In combined builds, these are provided by parser.sl
// ============================================================================
// (Removed to avoid duplicates in combined builds - parser.sl provides these)

// ============================================================================
// Symbol table operations
// ============================================================================

// Push a variable into the symbol table
fn sym_push(tc: i64, name: str, ty: i64, mutable: i64) -> i64 {
    let syms = tc_syms(tc)
    let cnt = tc_sym_count(tc)
    let off = cnt * SYM_STRIDE()
    let h = str_hash(name)
    array_set(syms, off + 0, h)
    array_set(syms, off + 1, ty)
    array_set(syms, off + 2, tc_depth(tc))
    array_set(syms, off + 3, mutable)
    tc_set_sym_count(tc, cnt + 1)
    0
}

// Lookup a variable by name, return its type (or TY_ERROR if not found)
fn sym_lookup(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let syms = tc_syms(tc)
    let cnt = tc_sym_count(tc)
    let mut i = cnt - 1
    let mut result = TY_ERROR()
    let mut found = false
    while i >= 0 && !found {
        let off = i * SYM_STRIDE()
        if array_get(syms, off) == h {
            result = array_get(syms, off + 1)
            found = true
            0
        } else {
            i = i - 1
        }
    }
    result
}

// Check if a variable is mutable
fn sym_is_mutable(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let syms = tc_syms(tc)
    let cnt = tc_sym_count(tc)
    let mut i = cnt - 1
    let mut result = 0
    let mut found = 0
    while i >= 0 && found == 0 {
        let off = i * SYM_STRIDE()
        if array_get(syms, off) == h {
            result = array_get(syms, off + 3)
            found = 1
            0
        } else {
            i = i - 1
        }
    }
    result
}

// Enter a new scope
fn scope_enter(tc: i64) -> i64 {
    tc_set_depth(tc, tc_depth(tc) + 1)
    tc_sym_count(tc)
}

// Leave a scope, pop symbols to restore point
fn scope_leave(tc: i64, restore_point: i64) -> i64 {
    tc_set_depth(tc, tc_depth(tc) - 1)
    tc_set_sym_count(tc, restore_point)
    0
}

// ============================================================================
// Function table operations
// ============================================================================

// Register a function: name, return_type, param_count, param_types...
fn fn_register(tc: i64, name: str, ret_type: i64, param_count: i64, param_types: i64) -> i64 {
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let off = cnt * FN_STRIDE()
    let h = str_hash(name)
    array_set(fns, off + 0, h)
    array_set(fns, off + 1, ret_type)
    array_set(fns, off + 2, param_count)
    // Copy param types from array
    let mut i = 0
    while i < param_count && i < 8 {
        array_set(fns, off + 3 + i, array_get(param_types, i))
        i = i + 1
    }
    tc_set_fn_count(tc, cnt + 1)
    0
}

// Lookup function return type by name
fn fn_lookup_ret(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let mut i = 0
    let mut result = TY_ERROR()
    let mut found = false
    while i < cnt && !found {
        let off = i * FN_STRIDE()
        if array_get(fns, off) == h {
            result = array_get(fns, off + 1)
            found = true
            0
        } else {
            i = i + 1
        }
    }
    result
}

// Lookup function param count
fn fn_lookup_param_count(tc: i64, name: str) -> i64 {
    let h = str_hash(name)
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let mut i = 0
    let mut result = -1
    let mut found = false
    while i < cnt && !found {
        let off = i * FN_STRIDE()
        if array_get(fns, off) == h {
            result = array_get(fns, off + 2)
            found = true
            0
        } else {
            i = i + 1
        }
    }
    result
}

// Lookup function param type at index
fn fn_lookup_param_type(tc: i64, name: str, pidx: i64) -> i64 {
    let h = str_hash(name)
    let fns = tc_fns(tc)
    let cnt = tc_fn_count(tc)
    let mut i = 0
    let mut result = TY_ERROR()
    let mut found = false
    while i < cnt && !found {
        let off = i * FN_STRIDE()
        if array_get(fns, off) == h {
            result = array_get(fns, off + 3 + pidx)
            found = true
            0
        } else {
            i = i + 1
        }
    }
    result
}

// ============================================================================
// Error reporting
// ============================================================================
fn tc_error(tc: i64, msg: str) -> i64 {
    print_str("Type error: ")
    println_str(msg)
    tc_set_errors(tc, tc_errors(tc) + 1)
    0
}

fn tc_error2(tc: i64, msg: str, detail: str) -> i64 {
    print_str("Type error: ")
    print_str(msg)
    print_str(" '")
    print_str(detail)
    println_str("'")
    tc_set_errors(tc, tc_errors(tc) + 1)
    0
}

// ============================================================================
// Register built-in functions (stdlib)
// ============================================================================
fn register_builtins(tc: i64) -> i64 {
    let pt = array_new(8)

    // print(i64) -> void
    array_set(pt, 0, TY_I64())
    fn_register(tc, "print", TY_VOID(), 1, pt)

    // println(i64) -> void
    fn_register(tc, "println", TY_VOID(), 1, pt)

    // print_str(str) -> void
    array_set(pt, 0, TY_STR())
    fn_register(tc, "print_str", TY_VOID(), 1, pt)

    // println_str(str) -> void
    fn_register(tc, "println_str", TY_VOID(), 1, pt)

    // str_len(str) -> i64
    fn_register(tc, "str_len", TY_I64(), 1, pt)

    // str_eq(str, str) -> bool
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_STR())
    fn_register(tc, "str_eq", TY_BOOL(), 2, pt)

    // str_cat(str, str) -> str
    fn_register(tc, "str_cat", TY_STR(), 2, pt)

    // str_char_at(str, i64) -> str
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "str_char_at", TY_STR(), 2, pt)

    // str_substr(str, i64, i64) -> str
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_I64())
    array_set(pt, 2, TY_I64())
    fn_register(tc, "str_substr", TY_STR(), 3, pt)

    // str_contains(str, str) -> bool
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_STR())
    fn_register(tc, "str_contains", TY_BOOL(), 2, pt)

    // str_index_of(str, str) -> i64
    fn_register(tc, "str_index_of", TY_I64(), 2, pt)

    // char_to_int(str) -> i64
    array_set(pt, 0, TY_STR())
    fn_register(tc, "char_to_int", TY_I64(), 1, pt)

    // int_to_char(i64) -> str
    array_set(pt, 0, TY_I64())
    fn_register(tc, "int_to_char", TY_STR(), 1, pt)

    // array_new(i64) -> i64
    fn_register(tc, "array_new", TY_I64(), 1, pt)

    // array_len(i64) -> i64
    fn_register(tc, "array_len", TY_I64(), 1, pt)

    // array_get(i64, i64) -> i64
    array_set(pt, 0, TY_I64())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "array_get", TY_I64(), 2, pt)

    // array_set(i64, i64, i64) -> void
    array_set(pt, 2, TY_I64())
    fn_register(tc, "array_set", TY_VOID(), 3, pt)

    // assert_eq(i64, i64) -> void
    array_set(pt, 0, TY_I64())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "assert_eq", TY_VOID(), 2, pt)

    // assert_true(bool) -> void
    array_set(pt, 0, TY_BOOL())
    fn_register(tc, "assert_true", TY_VOID(), 1, pt)

    // exit(i64) -> void
    array_set(pt, 0, TY_I64())
    fn_register(tc, "exit", TY_VOID(), 1, pt)

    // to_string_i64(i64) -> str
    fn_register(tc, "to_string_i64", TY_STR(), 1, pt)

    // str_starts_with(str, str) -> bool
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_STR())
    fn_register(tc, "str_starts_with", TY_BOOL(), 2, pt)

    // str_split_count(str, str) -> i64
    fn_register(tc, "str_split_count", TY_I64(), 2, pt)

    // str_split_get(str, str, i64) -> str
    array_set(pt, 2, TY_I64())
    fn_register(tc, "str_split_get", TY_STR(), 3, pt)

    // abs(i64) -> i64
    array_set(pt, 0, TY_I64())
    fn_register(tc, "abs", TY_I64(), 1, pt)

    // min(i64, i64) -> i64
    array_set(pt, 1, TY_I64())
    fn_register(tc, "min", TY_I64(), 2, pt)

    // max(i64, i64) -> i64
    fn_register(tc, "max", TY_I64(), 2, pt)

    // args_count() -> i64
    fn_register(tc, "args_count", TY_I64(), 0, pt)

    // args_get(i64) -> str
    array_set(pt, 0, TY_I64())
    fn_register(tc, "args_get", TY_STR(), 1, pt)

    // file_read(str) -> str
    array_set(pt, 0, TY_STR())
    fn_register(tc, "file_read", TY_STR(), 1, pt)

    // file_write_bytes(str, i64) -> i64
    array_set(pt, 0, TY_STR())
    array_set(pt, 1, TY_I64())
    fn_register(tc, "file_write_bytes", TY_I64(), 2, pt)

    0
}

// ============================================================================
// Pre-registration pass: scan all fn definitions to register signatures
// ============================================================================
fn preregister_functions(tc: i64, source: str, prog_off: i64) -> i64 {
    let p = tc_p(tc)
    let lstart = node_d0(p, prog_off)
    let count = node_d1(p, prog_off)
    let tokens = p_tokens(p)
    let pt = array_new(8)
    let mut i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            let name_tok = node_d0(p, fn_off)
            let name = tok_text(source, tokens, name_tok)
            let param_count = node_d2(p, fn_off)
            let params_start = node_d1(p, fn_off)
            let ret_tok = node_d3(p, fn_off)

            // Resolve return type
            let mut ret_type = TY_VOID()
            if ret_tok >= 0 {
                ret_type = resolve_type_name(tok_text(source, tokens, ret_tok))
            } else {
                ret_type = TY_VOID()
            }

            // Resolve param types
            let mut pi = 0
            while pi < param_count && pi < 8 {
                let pt_tok = list_item(p, params_start + pi * 2 + 1)
                let pt_name = tok_text(source, tokens, pt_tok)
                array_set(pt, pi, resolve_type_name(pt_name))
                pi = pi + 1
            }
            fn_register(tc, name, ret_type, param_count, pt)
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// Expression type checking
// ============================================================================

fn check_expr(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let nt = node_type(p, off)
    if nt == N_INT() { TY_I64() }
    else if nt == N_FLOAT() { TY_F64() }
    else if nt == N_STR() { TY_STR() }
    else if nt == N_BOOL() { TY_BOOL() }
    else if nt == N_IDENT() { check_ident(tc, source, off) }
    else if nt == N_BINOP() { check_binop(tc, source, off) }
    else if nt == N_UNOP() { check_unop(tc, source, off) }
    else if nt == N_CALL() { check_call(tc, source, off) }
    else if nt == N_IF() { check_if_expr(tc, source, off) }
    else { TY_ERROR() }
}

fn check_ident(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let tok_idx = node_d0(p, off)
    let name = tok_text(source, p_tokens(p), tok_idx)
    let ty = sym_lookup(tc, name)
    if ty == TY_ERROR() {
        tc_error2(tc, "undefined variable", name)
        TY_ERROR()
    } else {
        ty
    }
}

fn check_binop(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let op = node_d0(p, off)
    let left_off = node_d1(p, off)
    let right_off = node_d2(p, off)
    let lt = check_expr(tc, source, left_off)
    let rt = check_expr(tc, source, right_off)

    // Pipe operator: special - result type is the return type of the RHS function
    if op == TK_PIPE() {
        rt
    }
    // Comparison operators: operands must be numeric, result is bool
    else if op == TK_EQEQ() || op == TK_NEQ() || op == TK_LT() || op == TK_GT() || op == TK_LTE() || op == TK_GTE() {
        if lt != rt && lt != TY_ERROR() && rt != TY_ERROR() {
            tc_error(tc, "comparison operands must have same type")
            TY_ERROR()
        } else {
            TY_BOOL()
        }
    }
    // Logical operators: operands must be bool
    else if op == TK_AND() || op == TK_OR() {
        if lt != TY_BOOL() && lt != TY_ERROR() {
            tc_error(tc, "&& / || requires bool operands")
            TY_ERROR()
        } else if rt != TY_BOOL() && rt != TY_ERROR() {
            tc_error(tc, "&& / || requires bool operands")
            TY_ERROR()
        } else {
            TY_BOOL()
        }
    }
    // Arithmetic operators: operands must be numeric
    else if op == TK_PLUS() || op == TK_MINUS() || op == TK_STAR() || op == TK_SLASH() || op == TK_PERCENT() {
        if lt == TY_STR() && op == TK_PLUS() {
            // String concatenation via +
            TY_STR()
        } else if (lt == TY_I64() || lt == TY_F64()) && (rt == TY_I64() || rt == TY_F64()) {
            // If either is f64, result is f64
            if lt == TY_F64() || rt == TY_F64() { TY_F64() }
            else { TY_I64() }
        } else if lt == TY_ERROR() || rt == TY_ERROR() {
            TY_ERROR()
        } else {
            tc_error(tc, "arithmetic requires numeric operands")
            TY_ERROR()
        }
    } else {
        TY_ERROR()
    }
}

fn check_unop(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let op = node_d0(p, off)
    let operand_off = node_d1(p, off)
    let ot = check_expr(tc, source, operand_off)

    if op == TK_MINUS() {
        if ot == TY_I64() || ot == TY_F64() { ot }
        else if ot == TY_ERROR() { TY_ERROR() }
        else { tc_error(tc, "unary - requires numeric operand"); TY_ERROR() }
    } else if op == TK_NOT() {
        if ot == TY_BOOL() { TY_BOOL() }
        else if ot == TY_ERROR() { TY_ERROR() }
        else { tc_error(tc, "! requires bool operand"); TY_ERROR() }
    } else {
        TY_ERROR()
    }
}

fn check_call(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let callee_tok = node_d0(p, off)
    let name = tok_text(source, p_tokens(p), callee_tok)
    let lstart = node_d1(p, off)
    let arg_count = node_d2(p, off)

    let ret = fn_lookup_ret(tc, name)
    if ret == TY_ERROR() {
        tc_error2(tc, "undefined function", name)
        TY_ERROR()
    } else {
        // Check argument count
        let expected_params = fn_lookup_param_count(tc, name)
        if expected_params >= 0 && arg_count != expected_params {
            tc_error2(tc, "wrong number of arguments for", name)
            TY_ERROR()
        } else {
            // Check argument types
            check_call_args(tc, source, name, lstart, arg_count)
            ret
        }
    }
}

fn check_call_args(tc: i64, source: str, name: str, lstart: i64, arg_count: i64) -> i64 {
    let p = tc_p(tc)
    let mut i = 0
    while i < arg_count {
        let arg_off = list_item(p, lstart + i)
        let arg_ty = check_expr(tc, source, arg_off)
        let expected = fn_lookup_param_type(tc, name, i)
        if arg_ty != expected && arg_ty != TY_ERROR() && expected != TY_ERROR() {
            // Allow i64 where f64 expected (implicit promotion for now)
            if expected == TY_F64() && arg_ty == TY_I64() {
                0
            } else {
                tc_error2(tc, "argument type mismatch in call to", name)
                0
            }
        } else {
            0
        }
        i = i + 1
    }
    0
}

// If-expression type: both branches must agree
fn check_if_expr(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    let ct = check_expr(tc, source, cond_off)
    if ct != TY_BOOL() && ct != TY_ERROR() {
        tc_error(tc, "if condition must be bool")
        0
    } else {
        0
    }

    let tt = check_block_type(tc, source, then_off)

    if else_off >= 0 {
        let et = check_block_or_if_type(tc, source, else_off)
        // Both branches should yield same type
        if tt != et && tt != TY_ERROR() && et != TY_ERROR() && tt != TY_VOID() && et != TY_VOID() {
            tc_error(tc, "if/else branches have different types")
            TY_ERROR()
        } else {
            tt
        }
    } else {
        TY_VOID()
    }
}

fn check_block_or_if_type(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let nt = node_type(p, off)
    if nt == N_BLOCK() { check_block_type(tc, source, off) }
    else if nt == N_IF() { check_if_expr(tc, source, off) }
    else { check_expr(tc, source, off) }
}

// ============================================================================
// Statement type checking
// ============================================================================

fn check_stmt(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let nt = node_type(p, off)
    if nt == N_LET() { check_let(tc, source, off) }
    else if nt == N_ASSIGN() { check_assign(tc, source, off) }
    else if nt == N_IF() { check_if_stmt(tc, source, off) }
    else if nt == N_WHILE() { check_while(tc, source, off) }
    else if nt == N_FOR() { check_for(tc, source, off) }
    else if nt == N_RETURN() { check_return(tc, source, off) }
    else if nt == N_BLOCK() { check_block(tc, source, off); TY_VOID() }
    else if nt == N_EXPR_STMT() { check_expr(tc, source, node_d0(p, off)); TY_VOID() }
    else if nt == N_BREAK() || nt == N_CONTINUE() { TY_VOID() }
    else { check_expr(tc, source, off) }
}

// Let: infer type from RHS, push to symbol table
fn check_let(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let name_tok = node_d0(p, off)
    let mutable = node_d1(p, off)
    let type_tok = node_d2(p, off)
    let val_off = node_d3(p, off)
    let tokens = p_tokens(p)

    let name = tok_text(source, tokens, name_tok)

    // Check RHS type
    let rhs_type = check_expr(tc, source, val_off)

    // If explicit type annotation, check it matches
    let mut final_type = rhs_type
    if type_tok >= 0 {
        let ann_type = resolve_type_name(tok_text(source, tokens, type_tok))
        if ann_type != rhs_type && rhs_type != TY_ERROR() && ann_type != TY_ERROR() {
            tc_error2(tc, "type annotation mismatch for", name)
            final_type = ann_type
        } else {
            final_type = ann_type
        }
    } else {
        final_type = rhs_type
    }

    sym_push(tc, name, final_type, mutable)
    TY_VOID()
}

// Assignment: check target is mutable and types match
fn check_assign(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let target_tok = node_d0(p, off)
    let rhs_off = node_d1(p, off)
    let tokens = p_tokens(p)

    let name = tok_text(source, tokens, target_tok)
    let var_type = sym_lookup(tc, name)

    if var_type == TY_ERROR() {
        tc_error2(tc, "undefined variable in assignment", name)
        TY_ERROR()
    } else {
        if sym_is_mutable(tc, name) == 0 {
            tc_error2(tc, "cannot assign to immutable variable", name)
            0
        } else {
            0
        }
        let rhs_type = check_expr(tc, source, rhs_off)
        if rhs_type != var_type && rhs_type != TY_ERROR() && var_type != TY_ERROR() {
            tc_error2(tc, "assignment type mismatch for", name)
            TY_ERROR()
        } else {
            TY_VOID()
        }
    }
}

// If statement (no value needed)
fn check_if_stmt(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let cond_off = node_d0(p, off)
    let then_off = node_d1(p, off)
    let else_off = node_d2(p, off)

    let ct = check_expr(tc, source, cond_off)
    if ct != TY_BOOL() && ct != TY_ERROR() {
        tc_error(tc, "if condition must be bool")
        0
    } else {
        0
    }

    check_block(tc, source, then_off)

    if else_off >= 0 {
        let nt = node_type(p, else_off)
        if nt == N_IF() { check_if_stmt(tc, source, else_off) }
        else { check_block(tc, source, else_off) }
    } else {
        0
    }
    TY_VOID()
}

// While loop
fn check_while(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let cond_off = node_d0(p, off)
    let body_off = node_d1(p, off)

    let ct = check_expr(tc, source, cond_off)
    if ct != TY_BOOL() && ct != TY_ERROR() {
        tc_error(tc, "while condition must be bool")
        0
    } else {
        0
    }

    check_block(tc, source, body_off)
    TY_VOID()
}

// For loop
fn check_for(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let var_tok = node_d0(p, off)
    let iter_off = node_d1(p, off)
    let body_off = node_d2(p, off)
    let tokens = p_tokens(p)

    let name = tok_text(source, tokens, var_tok)
    let iter_type = check_expr(tc, source, iter_off)

    let save = scope_enter(tc)
    // For now, loop variable is i64
    sym_push(tc, name, TY_I64(), 0)
    check_block(tc, source, body_off)
    scope_leave(tc, save)
    TY_VOID()
}

// Return statement
fn check_return(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let val_off = node_d0(p, off)
    let expected = tc_ret_type(tc)

    if val_off >= 0 {
        let rt = check_expr(tc, source, val_off)
        if rt != expected && rt != TY_ERROR() && expected != TY_ERROR() {
            tc_error(tc, "return type mismatch")
            0
        } else {
            0
        }
    } else {
        if expected != TY_VOID() && expected != TY_ERROR() {
            tc_error(tc, "missing return value")
            0
        } else {
            0
        }
    }
    TY_VOID()
}

// ============================================================================
// Block checking
// ============================================================================

// Check a block, return void (used for statement blocks)
fn check_block(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let save = scope_enter(tc)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        check_stmt(tc, source, stmt_off)
        i = i + 1
    }
    scope_leave(tc, save)
    0
}

// Check a block and return the type of its last expression (for if-expr)
fn check_block_type(tc: i64, source: str, off: i64) -> i64 {
    let p = tc_p(tc)
    let save = scope_enter(tc)
    let lstart = node_d0(p, off)
    let count = node_d1(p, off)
    let mut last_type = TY_VOID()
    let mut i = 0
    while i < count {
        let stmt_off = list_item(p, lstart + i)
        last_type = check_stmt(tc, source, stmt_off)
        i = i + 1
    }
    scope_leave(tc, save)
    last_type
}

// ============================================================================
// Function checking
// ============================================================================
fn check_function(tc: i64, source: str, fn_off: i64) -> i64 {
    let p = tc_p(tc)
    let tokens = p_tokens(p)
    let name_tok = node_d0(p, fn_off)
    let params_start = node_d1(p, fn_off)
    let param_count = node_d2(p, fn_off)
    let ret_tok = node_d3(p, fn_off)
    let body_off = node_d4(p, fn_off)

    let name = tok_text(source, tokens, name_tok)

    // Set current return type for return-statement checking
    let mut ret_type = TY_VOID()
    if ret_tok >= 0 {
        ret_type = resolve_type_name(tok_text(source, tokens, ret_tok))
    } else {
        ret_type = TY_VOID()
    }
    tc_set_ret_type(tc, ret_type)

    // Enter function scope and bind parameters
    let save = scope_enter(tc)
    let mut pi = 0
    while pi < param_count {
        let pn_tok = list_item(p, params_start + pi * 2)
        let pt_tok = list_item(p, params_start + pi * 2 + 1)
        let pn = tok_text(source, tokens, pn_tok)
        let pt = resolve_type_name(tok_text(source, tokens, pt_tok))
        sym_push(tc, pn, pt, 0)
        pi = pi + 1
    }

    // Type-check the body
    check_block(tc, source, body_off)
    scope_leave(tc, save)
    0
}

// ============================================================================
// Top-level: check whole program
// ============================================================================
fn check_program(tc: i64, source: str, prog_off: i64) -> i64 {
    let p = tc_p(tc)
    let lstart = node_d0(p, prog_off)
    let count = node_d1(p, prog_off)
    let mut i = 0
    while i < count {
        let fn_off = list_item(p, lstart + i)
        if node_type(p, fn_off) == N_FN() {
            check_function(tc, source, fn_off)
        } else {
            0
        }
        i = i + 1
    }
    0
}

// ============================================================================
// Entry point
// ============================================================================
fn typecheck(p: i64, source: str, prog_off: i64) -> i64 {
    let tc = array_new(9)
    array_set(tc, 0, p)
    array_set(tc, 1, 0)
    array_set(tc, 2, array_new(16000))
    array_set(tc, 3, 0)
    array_set(tc, 4, 0)
    array_set(tc, 5, array_new(8000))
    array_set(tc, 6, 0)
    array_set(tc, 7, 0)
    array_set(tc, 8, TY_VOID())

    register_builtins(tc)
    preregister_functions(tc, source, prog_off)
    check_program(tc, source, prog_off)
    tc
}
