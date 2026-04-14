// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 2: Parser (recursive descent)
// ============================================================================
// Parses token buffer (from lexer.sl) into flat AST node arrays.
//
// AST storage: two flat arrays
//   nodes[]: stride 8, each node = [type, d0, d1, d2, d3, d4, d5, d6]
//   lists[]: variable-length child lists (stmt lists, arg lists, param lists)
//
// Parser context packed into a single i64 array (p):
//   p[0] = tokens buffer (from lexer)
//   p[1] = nodes buffer
//   p[2] = lists buffer
//   p[3] = list stack buffer (temp storage during list collection)
//   p[4] = state array [pos, nodes_used, lists_used, stack_sp]
//
// NOTE: All control flow uses if/else expressions, never bare `return`.
//       Functions kept small to avoid Cranelift code size issues.
// ============================================================================

// -- Node type constants --
fn N_INT() -> i64      { 1 }
fn N_FLOAT() -> i64    { 2 }
fn N_STR() -> i64      { 3 }
fn N_BOOL() -> i64     { 4 }
fn N_IDENT() -> i64    { 5 }
fn N_BINOP() -> i64    { 6 }
fn N_UNOP() -> i64     { 7 }
fn N_CALL() -> i64     { 8 }
fn N_INDEX() -> i64    { 9 }
fn N_FIELD() -> i64    { 10 }

fn N_LET() -> i64      { 20 }
fn N_ASSIGN() -> i64   { 21 }
fn N_IF() -> i64       { 22 }
fn N_WHILE() -> i64    { 23 }
fn N_FOR() -> i64      { 24 }
fn N_RETURN() -> i64   { 25 }
fn N_BREAK() -> i64    { 26 }
fn N_CONTINUE() -> i64 { 27 }
fn N_BLOCK() -> i64    { 28 }
fn N_EXPR_STMT() -> i64 { 29 }

fn N_FN() -> i64       { 30 }
fn N_PROGRAM() -> i64  { 31 }
fn N_EMPTY() -> i64    { 0 }

// Node stride
fn NODE_STRIDE() -> i64 { 8 }

// ============================================================================
// Token type constants (duplicated from lexer.sl for standalone use)
// ============================================================================
fn TK_INT() -> i64    { 1 }
fn TK_FLOAT() -> i64  { 2 }
fn TK_STRING() -> i64 { 3 }
fn TK_IDENT() -> i64  { 4 }
fn TK_BOOL() -> i64   { 5 }

fn TK_FN() -> i64       { 10 }
fn TK_LET() -> i64      { 11 }
fn TK_IF() -> i64       { 12 }
fn TK_ELSE() -> i64     { 13 }
fn TK_WHILE() -> i64    { 14 }
fn TK_FOR() -> i64      { 15 }
fn TK_RETURN() -> i64   { 16 }
fn TK_STRUCT() -> i64   { 17 }
fn TK_ENUM() -> i64     { 18 }
fn TK_MATCH() -> i64    { 19 }
fn TK_IMPORT() -> i64   { 20 }
fn TK_CONST() -> i64    { 21 }
fn TK_TRAIT() -> i64    { 22 }
fn TK_IMPL() -> i64     { 23 }
fn TK_IN() -> i64       { 24 }
fn TK_MUT() -> i64      { 25 }
fn TK_LOOP() -> i64     { 26 }
fn TK_BREAK() -> i64    { 27 }
fn TK_CONTINUE() -> i64 { 28 }

fn TK_PLUS() -> i64      { 50 }
fn TK_MINUS() -> i64     { 51 }
fn TK_STAR() -> i64      { 52 }
fn TK_SLASH() -> i64     { 53 }
fn TK_PERCENT() -> i64   { 54 }
fn TK_EQ() -> i64        { 55 }
fn TK_EQEQ() -> i64     { 56 }
fn TK_NEQ() -> i64       { 57 }
fn TK_LT() -> i64        { 58 }
fn TK_GT() -> i64        { 59 }
fn TK_LTE() -> i64       { 60 }
fn TK_GTE() -> i64       { 61 }
fn TK_AND() -> i64       { 62 }
fn TK_OR() -> i64        { 63 }
fn TK_NOT() -> i64       { 64 }
fn TK_ARROW() -> i64     { 65 }
fn TK_FAT_ARROW() -> i64 { 66 }
fn TK_PIPE() -> i64      { 67 }

fn TK_LPAREN() -> i64     { 80 }
fn TK_RPAREN() -> i64     { 81 }
fn TK_LBRACE() -> i64     { 82 }
fn TK_RBRACE() -> i64     { 83 }
fn TK_LBRACKET() -> i64   { 84 }
fn TK_RBRACKET() -> i64   { 85 }
fn TK_COMMA() -> i64      { 86 }
fn TK_COLON() -> i64      { 87 }
fn TK_SEMICOLON() -> i64  { 88 }
fn TK_DOT() -> i64        { 89 }
fn TK_COLONCOLON() -> i64 { 92 }

fn TK_EOF() -> i64   { 99 }

// ============================================================================
// Token buffer accessors (from lexer, stride 4)
// ============================================================================
fn tok_count(tokens: i64) -> i64 { array_len(tokens) / 4 }
fn tok_type(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4) }
fn tok_start(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4 + 1) }
fn tok_end(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4 + 2) }
fn tok_line(tokens: i64, idx: i64) -> i64 { array_get(tokens, idx * 4 + 3) }

fn tok_text(source: str, tokens: i64, idx: i64) -> str {
    let s = tok_start(tokens, idx)
    let e = tok_end(tokens, idx)
    if s < 0 { "" }
    else { str_substr(source, s, e - s) }
}

// ============================================================================
// Parser context (p) accessors
// ============================================================================
fn p_tokens(p: i64) -> i64 { array_get(p, 0) }
fn p_nodes(p: i64) -> i64  { array_get(p, 1) }
fn p_lists(p: i64) -> i64  { array_get(p, 2) }
fn p_stack(p: i64) -> i64  { array_get(p, 3) }
fn p_state(p: i64) -> i64  { array_get(p, 4) }

fn p_pos(p: i64) -> i64 { array_get(p_state(p), 0) }

fn p_set_pos(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 0, v)
    0
}

fn p_nodes_used(p: i64) -> i64 { array_get(p_state(p), 1) }

fn p_set_nodes_used(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 1, v)
    0
}

fn p_lists_used(p: i64) -> i64 { array_get(p_state(p), 2) }

fn p_set_lists_used(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 2, v)
    0
}

fn p_stack_sp(p: i64) -> i64 { array_get(p_state(p), 3) }

fn p_set_stack_sp(p: i64, v: i64) -> i64 {
    array_set(p_state(p), 3, v)
    0
}

// ============================================================================
// Parser helpers
// ============================================================================

// Current token type
fn peek(p: i64) -> i64 {
    let tokens = p_tokens(p)
    let pos = p_pos(p)
    if pos >= tok_count(tokens) { TK_EOF() }
    else { tok_type(tokens, pos) }
}

// Advance by 1 token, return old position
fn advance(p: i64) -> i64 {
    let pos = p_pos(p)
    p_set_pos(p, pos + 1)
    pos
}

// Consume expected token type, advance; return token index
fn eat(p: i64, expected: i64) -> i64 {
    let t = peek(p)
    if t != expected {
        println_str("Parse error: unexpected token")
        println(t)
        println(expected)
        -1
    } else {
        advance(p)
    }
}

// Check if current token matches, but don't consume
fn check(p: i64, t: i64) -> bool {
    peek(p) == t
}

// Current token text
fn cur_text(p: i64, source: str) -> str {
    tok_text(source, p_tokens(p), p_pos(p))
}

// ============================================================================
// Node emission (stride 8)
// ============================================================================
fn emit_node(p: i64, ntype: i64, d0: i64, d1: i64, d2: i64, d3: i64, d4: i64, d5: i64) -> i64 {
    let nodes = p_nodes(p)
    let off = p_nodes_used(p)
    array_set(nodes, off + 0, ntype)
    array_set(nodes, off + 1, d0)
    array_set(nodes, off + 2, d1)
    array_set(nodes, off + 3, d2)
    array_set(nodes, off + 4, d3)
    array_set(nodes, off + 5, d4)
    array_set(nodes, off + 6, d5)
    array_set(nodes, off + 7, 0)
    p_set_nodes_used(p, off + 8)
    off
}

// Read a field from a node at offset
fn node_type(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off) }
fn node_d0(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 1) }
fn node_d1(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 2) }
fn node_d2(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 3) }
fn node_d3(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 4) }
fn node_d4(p: i64, off: i64) -> i64 { array_get(p_nodes(p), off + 5) }

// Read from lists buffer
fn list_item(p: i64, idx: i64) -> i64 { array_get(p_lists(p), idx) }

// ============================================================================
// List stack operations
// ============================================================================

// Save current stack position as a mark
fn list_save(p: i64) -> i64 {
    p_stack_sp(p)
}

// Push a node index onto the list stack
fn list_push(p: i64, node_idx: i64) -> i64 {
    let sp = p_stack_sp(p)
    array_set(p_stack(p), sp, node_idx)
    p_set_stack_sp(p, sp + 1)
    0
}

// Flush stack items from mark..sp into lists buffer, return start index
fn list_flush(p: i64, mark: i64) -> i64 {
    let sp = p_stack_sp(p)
    let count = sp - mark
    let lstart = p_lists_used(p)
    let stack = p_stack(p)
    let lists = p_lists(p)
    let mut i = 0
    while i < count {
        array_set(lists, lstart + i, array_get(stack, mark + i))
        i = i + 1
    }
    p_set_lists_used(p, lstart + count)
    p_set_stack_sp(p, mark)
    lstart
}

// Count of items flushed = sp - mark (call before flush)
fn list_count(p: i64, mark: i64) -> i64 {
    p_stack_sp(p) - mark
}

// ============================================================================
// Operator precedence (for Pratt parsing)
// ============================================================================
fn prefix_bp(op: i64) -> i64 {
    if op == TK_MINUS() { 90 }
    else if op == TK_NOT() { 90 }
    else { -1 }
}

fn infix_bp_left(op: i64) -> i64 {
    if op == TK_OR()      { 10 }
    else if op == TK_AND()     { 20 }
    else if op == TK_EQEQ()   { 30 }
    else if op == TK_NEQ()     { 30 }
    else if op == TK_LT()     { 40 }
    else if op == TK_GT()     { 40 }
    else if op == TK_LTE()    { 40 }
    else if op == TK_GTE()    { 40 }
    else if op == TK_PLUS()   { 50 }
    else if op == TK_MINUS()  { 50 }
    else if op == TK_STAR()   { 60 }
    else if op == TK_SLASH()  { 60 }
    else if op == TK_PERCENT(){ 60 }
    else if op == TK_PIPE()   { 5 }
    else { -1 }
}

fn infix_bp_right(op: i64) -> i64 {
    if op == TK_OR()      { 11 }
    else if op == TK_AND()     { 21 }
    else if op == TK_EQEQ()   { 31 }
    else if op == TK_NEQ()     { 31 }
    else if op == TK_LT()     { 41 }
    else if op == TK_GT()     { 41 }
    else if op == TK_LTE()    { 41 }
    else if op == TK_GTE()    { 41 }
    else if op == TK_PLUS()   { 51 }
    else if op == TK_MINUS()  { 51 }
    else if op == TK_STAR()   { 61 }
    else if op == TK_SLASH()  { 61 }
    else if op == TK_PERCENT(){ 61 }
    else if op == TK_PIPE()   { 4 }
    else { -1 }
}

// ============================================================================
// Expression parsing (Pratt parser)
// ============================================================================

// Parse an integer literal from token text
fn parse_int_value(source: str, tokens: i64, idx: i64) -> i64 {
    let text = tok_text(source, tokens, idx)
    let tlen = str_len(text)
    // Check for hex: 0x or 0X
    if tlen > 2 && str_char_at(text, 0) == "0" {
        let c1 = str_char_at(text, 1)
        if str_eq(c1, "x") || str_eq(c1, "X") {
            parse_hex(text, 2, tlen)
        } else {
            parse_decimal(text, 0, tlen)
        }
    } else {
        parse_decimal(text, 0, tlen)
    }
}

fn parse_decimal(text: str, start: i64, end: i64) -> i64 {
    let mut val = 0
    let mut i = start
    while i < end {
        let d = char_to_int(str_char_at(text, i)) - 48
        val = val * 10 + d
        i = i + 1
    }
    val
}

fn parse_hex(text: str, start: i64, end: i64) -> i64 {
    let mut val = 0
    let mut i = start
    while i < end {
        let c = char_to_int(str_char_at(text, i))
        let d = if c >= 97 { c - 87 }
                else if c >= 65 { c - 55 }
                else { c - 48 }
        val = val * 16 + d
        i = i + 1
    }
    val
}

// Parse a primary (atom) expression
fn parse_primary(p: i64, source: str) -> i64 {
    let t = peek(p)
    if t == TK_INT() {
        let idx = advance(p)
        let val = parse_int_value(source, p_tokens(p), idx)
        emit_node(p, N_INT(), val, 0, 0, 0, 0, 0)
    } else if t == TK_FLOAT() {
        let idx = advance(p)
        emit_node(p, N_FLOAT(), idx, 0, 0, 0, 0, 0)
    } else if t == TK_STRING() {
        let idx = advance(p)
        emit_node(p, N_STR(), idx, 0, 0, 0, 0, 0)
    } else if t == TK_BOOL() {
        let idx = advance(p)
        let text = tok_text(source, p_tokens(p), idx)
        let val = if str_eq(text, "true") { 1 } else { 0 }
        emit_node(p, N_BOOL(), val, 0, 0, 0, 0, 0)
    } else if t == TK_IDENT() {
        parse_ident_or_call(p, source)
    } else if t == TK_LPAREN() {
        advance(p)
        let inner = parse_expr(p, source, 0)
        eat(p, TK_RPAREN())
        inner
    } else if t == TK_MINUS() || t == TK_NOT() {
        let op_idx = advance(p)
        let op_type = tok_type(p_tokens(p), op_idx)
        let operand = parse_expr(p, source, 90)
        emit_node(p, N_UNOP(), op_type, operand, 0, 0, 0, 0)
    } else {
        println_str("Parse error: unexpected token in expression")
        println(t)
        advance(p)
        emit_node(p, N_EMPTY(), 0, 0, 0, 0, 0, 0)
    }
}

// Parse identifier, possibly followed by function call (...)
fn parse_ident_or_call(p: i64, source: str) -> i64 {
    let idx = advance(p)
    if peek(p) == TK_LPAREN() {
        parse_call_args(p, source, idx)
    } else {
        emit_node(p, N_IDENT(), idx, 0, 0, 0, 0, 0)
    }
}

// Parse function call arguments: (expr, expr, ...)
fn parse_call_args(p: i64, source: str, callee_tok: i64) -> i64 {
    eat(p, TK_LPAREN())
    let mark = list_save(p)
    let mut count = 0
    if peek(p) != TK_RPAREN() {
        let a = parse_expr(p, source, 0)
        list_push(p, a)
        count = count + 1
        while peek(p) == TK_COMMA() {
            advance(p)
            let a2 = parse_expr(p, source, 0)
            list_push(p, a2)
            count = count + 1
        }
    } else {
        count = 0
    }
    eat(p, TK_RPAREN())
    let lstart = list_flush(p, mark)
    emit_node(p, N_CALL(), callee_tok, lstart, count, 0, 0, 0)
}

// Pratt expression parser with minimum binding power
fn parse_expr(p: i64, source: str, min_bp: i64) -> i64 {
    let mut left = parse_primary(p, source)
    left = parse_expr_loop(p, source, left, min_bp)
    left
}

// Iterative infix loop (separated to keep functions small)
fn parse_expr_loop(p: i64, source: str, lhs: i64, min_bp: i64) -> i64 {
    let mut left = lhs
    let mut done = false
    while !done {
        let op = peek(p)
        let lbp = infix_bp_left(op)
        if lbp < min_bp || lbp < 0 {
            done = true
        } else {
            advance(p)
            let rbp = infix_bp_right(op)
            let right = parse_expr(p, source, rbp)
            left = emit_node(p, N_BINOP(), op, left, right, 0, 0, 0)
            done = false
        }
    }
    left
}

// ============================================================================
// Statement parsing
// ============================================================================

fn parse_stmt(p: i64, source: str) -> i64 {
    let t = peek(p)
    if t == TK_LET() { parse_let(p, source) }
    else if t == TK_IF() { parse_if(p, source) }
    else if t == TK_WHILE() { parse_while(p, source) }
    else if t == TK_FOR() { parse_for(p, source) }
    else if t == TK_RETURN() { parse_return(p, source) }
    else if t == TK_BREAK() { advance(p); emit_node(p, N_BREAK(), 0, 0, 0, 0, 0, 0) }
    else if t == TK_CONTINUE() { advance(p); emit_node(p, N_CONTINUE(), 0, 0, 0, 0, 0, 0) }
    else { parse_expr_or_assign(p, source) }
}

// Parse let binding: let [mut] name [: type] = expr
fn parse_let(p: i64, source: str) -> i64 {
    eat(p, TK_LET())
    let mut mutable = 0
    if peek(p) == TK_MUT() {
        advance(p)
        mutable = 1
    } else {
        mutable = 0
    }
    let name_tok = eat(p, TK_IDENT())
    // Optional type annotation: : type
    let mut type_tok = -1
    if peek(p) == TK_COLON() {
        advance(p)
        type_tok = eat(p, TK_IDENT())
    } else {
        type_tok = -1
    }
    eat(p, TK_EQ())
    let value = parse_expr(p, source, 0)
    emit_node(p, N_LET(), name_tok, mutable, type_tok, value, 0, 0)
}

// Parse if/else if/else
fn parse_if(p: i64, source: str) -> i64 {
    eat(p, TK_IF())
    let cond = parse_expr(p, source, 0)
    let then_block = parse_block(p, source)
    let mut else_node = -1
    if peek(p) == TK_ELSE() {
        advance(p)
        if peek(p) == TK_IF() {
            else_node = parse_if(p, source)
        } else {
            else_node = parse_block(p, source)
        }
    } else {
        else_node = -1
    }
    emit_node(p, N_IF(), cond, then_block, else_node, 0, 0, 0)
}

// Parse while loop
fn parse_while(p: i64, source: str) -> i64 {
    eat(p, TK_WHILE())
    let cond = parse_expr(p, source, 0)
    let body = parse_block(p, source)
    emit_node(p, N_WHILE(), cond, body, 0, 0, 0, 0)
}

// Parse for loop: for ident in expr { ... }
fn parse_for(p: i64, source: str) -> i64 {
    eat(p, TK_FOR())
    let var_tok = eat(p, TK_IDENT())
    eat(p, TK_IN())
    let iter = parse_expr(p, source, 0)
    let body = parse_block(p, source)
    emit_node(p, N_FOR(), var_tok, iter, body, 0, 0, 0)
}

// Parse return statement
fn parse_return(p: i64, source: str) -> i64 {
    eat(p, TK_RETURN())
    let t = peek(p)
    if t == TK_RBRACE() || t == TK_EOF() {
        emit_node(p, N_RETURN(), -1, 0, 0, 0, 0, 0)
    } else {
        let val = parse_expr(p, source, 0)
        emit_node(p, N_RETURN(), val, 0, 0, 0, 0, 0)
    }
}

// Parse expression or assignment: expr  or  ident = expr
fn parse_expr_or_assign(p: i64, source: str) -> i64 {
    let lhs = parse_expr(p, source, 0)
    if peek(p) == TK_EQ() {
        advance(p)
        let rhs = parse_expr(p, source, 0)
        // lhs should be an ident node - extract the token index from it
        let target_tok = node_d0(p, lhs)
        emit_node(p, N_ASSIGN(), target_tok, rhs, 0, 0, 0, 0)
    } else {
        // Bare expression as statement
        emit_node(p, N_EXPR_STMT(), lhs, 0, 0, 0, 0, 0)
    }
}

// ============================================================================
// Block parsing: { stmt; stmt; ... }
// ============================================================================
fn parse_block(p: i64, source: str) -> i64 {
    eat(p, TK_LBRACE())
    let mark = list_save(p)
    let mut count = 0
    while peek(p) != TK_RBRACE() && peek(p) != TK_EOF() {
        // Skip semicolons
        if peek(p) == TK_SEMICOLON() {
            advance(p)
        } else {
            let stmt = parse_stmt(p, source)
            list_push(p, stmt)
            count = count + 1
        }
    }
    eat(p, TK_RBRACE())
    let lstart = list_flush(p, mark)
    emit_node(p, N_BLOCK(), lstart, count, 0, 0, 0, 0)
}

// ============================================================================
// Top-level parsing
// ============================================================================

// Parse function definition: fn name(params) [-> type] { body }
fn parse_function(p: i64, source: str) -> i64 {
    eat(p, TK_FN())
    let name_tok = eat(p, TK_IDENT())
    // Parse parameter list
    eat(p, TK_LPAREN())
    let param_mark = list_save(p)
    let mut param_count = 0
    if peek(p) != TK_RPAREN() {
        let pn = eat(p, TK_IDENT())
        eat(p, TK_COLON())
        let pt = eat(p, TK_IDENT())
        list_push(p, pn)
        list_push(p, pt)
        param_count = param_count + 1
        while peek(p) == TK_COMMA() {
            advance(p)
            let pn2 = eat(p, TK_IDENT())
            eat(p, TK_COLON())
            let pt2 = eat(p, TK_IDENT())
            list_push(p, pn2)
            list_push(p, pt2)
            param_count = param_count + 1
        }
    } else {
        param_count = 0
    }
    eat(p, TK_RPAREN())
    let params_start = list_flush(p, param_mark)
    // Optional return type: -> type
    let mut ret_tok = -1
    if peek(p) == TK_ARROW() {
        advance(p)
        ret_tok = eat(p, TK_IDENT())
    } else {
        ret_tok = -1
    }
    // Body
    let body = parse_block(p, source)
    emit_node(p, N_FN(), name_tok, params_start, param_count, ret_tok, body, 0)
}

// Parse a full program (list of top-level items)
fn parse_program(p: i64, source: str) -> i64 {
    let mark = list_save(p)
    let mut count = 0
    while peek(p) != TK_EOF() {
        let t = peek(p)
        if t == TK_FN() {
            let fn_node = parse_function(p, source)
            list_push(p, fn_node)
            count = count + 1
        } else {
            println_str("Parse error: expected top-level 'fn'")
            println(t)
            advance(p)
        }
    }
    let lstart = list_flush(p, mark)
    emit_node(p, N_PROGRAM(), lstart, count, 0, 0, 0, 0)
}

// ============================================================================
// Parser entry point
// ============================================================================
fn parser_new(tokens: i64) -> i64 {
    let p = array_new(5)
    array_set(p, 0, tokens)
    array_set(p, 1, array_new(200000))
    array_set(p, 2, array_new(80000))
    array_set(p, 3, array_new(40000))
    let state = array_new(4)
    array_set(state, 0, 0)
    array_set(state, 1, 0)
    array_set(state, 2, 0)
    array_set(state, 3, 0)
    array_set(p, 4, state)
    p
}

fn parse(tokens: i64, source: str) -> i64 {
    let p = parser_new(tokens)
    let prog = parse_program(p, source)
    p
}

// ============================================================================
// AST pretty-printer (for testing)
// ============================================================================
fn print_indent(depth: i64) -> i64 {
    let mut i = 0
    while i < depth {
        print_str("  ")
        i = i + 1
    }
    0
}

fn print_node(p: i64, source: str, off: i64, depth: i64) -> i64 {
    let ntype = node_type(p, off)
    if ntype == N_INT() { print_node_int(p, off, depth) }
    else if ntype == N_STR() { print_node_str(p, source, off, depth) }
    else if ntype == N_BOOL() { print_node_bool(p, off, depth) }
    else if ntype == N_IDENT() { print_node_ident(p, source, off, depth) }
    else if ntype == N_BINOP() { print_node_binop(p, source, off, depth) }
    else if ntype == N_UNOP() { print_node_unop(p, source, off, depth) }
    else if ntype == N_CALL() { print_node_call(p, source, off, depth) }
    else if ntype == N_LET() { print_node_let(p, source, off, depth) }
    else if ntype == N_ASSIGN() { print_node_assign(p, source, off, depth) }
    else if ntype == N_IF() { print_node_if(p, source, off, depth) }
    else if ntype == N_WHILE() { print_node_while(p, source, off, depth) }
    else if ntype == N_RETURN() { print_node_return(p, source, off, depth) }
    else if ntype == N_BLOCK() { print_node_block(p, source, off, depth) }
    else if ntype == N_FN() { print_node_fn(p, source, off, depth) }
    else if ntype == N_PROGRAM() { print_node_program(p, source, off, depth) }
    else if ntype == N_EXPR_STMT() { print_node(p, source, node_d0(p, off), depth) }
    else if ntype == N_BREAK() { print_indent(depth); println_str("BREAK") }
    else if ntype == N_CONTINUE() { print_indent(depth); println_str("CONTINUE") }
    else { print_indent(depth); print_str("UNKNOWN("); println(ntype) }
    0
}

fn print_node_int(p: i64, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    print_str("INT(")
    print(node_d0(p, off))
    println_str(")")
    0
}

fn print_node_str(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let tok_idx = node_d0(p, off)
    print_str("STR('")
    print_str(tok_text(source, p_tokens(p), tok_idx))
    println_str("')")
    0
}

fn print_node_bool(p: i64, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    if node_d0(p, off) == 1 { println_str("BOOL(true)") }
    else { println_str("BOOL(false)") }
    0
}

fn print_node_ident(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let tok_idx = node_d0(p, off)
    print_str("IDENT(")
    print_str(tok_text(source, p_tokens(p), tok_idx))
    println_str(")")
    0
}

fn print_node_binop(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let op = node_d0(p, off)
    print_str("BINOP(")
    print_op_name(op)
    println_str(")")
    print_node(p, source, node_d1(p, off), depth + 1)
    print_node(p, source, node_d2(p, off), depth + 1)
}

fn print_op_name(op: i64) -> i64 {
    if op == TK_PLUS() { print_str("+") }
    else if op == TK_MINUS() { print_str("-") }
    else if op == TK_STAR() { print_str("*") }
    else if op == TK_SLASH() { print_str("/") }
    else if op == TK_PERCENT() { print_str("%") }
    else if op == TK_EQEQ() { print_str("==") }
    else if op == TK_NEQ() { print_str("!=") }
    else if op == TK_LT() { print_str("<") }
    else if op == TK_GT() { print_str(">") }
    else if op == TK_LTE() { print_str("<=") }
    else if op == TK_GTE() { print_str(">=") }
    else if op == TK_AND() { print_str("&&") }
    else if op == TK_OR() { print_str("||") }
    else if op == TK_PIPE() { print_str("|>") }
    else { print_str("?op?") }
    0
}

fn print_node_unop(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let op = node_d0(p, off)
    if op == TK_MINUS() { println_str("UNOP(-)") }
    else if op == TK_NOT() { println_str("UNOP(!)") }
    else { println_str("UNOP(?)") }
    print_node(p, source, node_d1(p, off), depth + 1)
}

fn print_node_call(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let callee_tok = node_d0(p, off)
    print_str("CALL(")
    print_str(tok_text(source, p_tokens(p), callee_tok))
    println_str(")")
    let lstart = node_d1(p, off)
    let cnt = node_d2(p, off)
    let mut i = 0
    while i < cnt {
        let arg_node = list_item(p, lstart + i)
        print_node(p, source, arg_node, depth + 1)
        i = i + 1
    }
    0
}

fn print_node_let(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let name_tok = node_d0(p, off)
    let mutable = node_d1(p, off)
    if mutable == 1 { print_str("LET MUT ") }
    else { print_str("LET ") }
    println_str(tok_text(source, p_tokens(p), name_tok))
    let val_node = node_d3(p, off)
    print_node(p, source, val_node, depth + 1)
}

fn print_node_assign(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let target_tok = node_d0(p, off)
    print_str("ASSIGN ")
    println_str(tok_text(source, p_tokens(p), target_tok))
    print_node(p, source, node_d1(p, off), depth + 1)
}

fn print_node_if(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("IF")
    print_node(p, source, node_d0(p, off), depth + 1)
    print_indent(depth)
    println_str("THEN")
    print_node(p, source, node_d1(p, off), depth + 1)
    let else_node = node_d2(p, off)
    if else_node >= 0 {
        print_indent(depth)
        println_str("ELSE")
        print_node(p, source, else_node, depth + 1)
    } else {
        0
    }
}

fn print_node_while(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("WHILE")
    print_node(p, source, node_d0(p, off), depth + 1)
    print_node(p, source, node_d1(p, off), depth + 1)
}

fn print_node_return(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("RETURN")
    let val = node_d0(p, off)
    if val >= 0 {
        print_node(p, source, val, depth + 1)
    } else {
        0
    }
}

fn print_node_block(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    println_str("BLOCK")
    let lstart = node_d0(p, off)
    let cnt = node_d1(p, off)
    let mut i = 0
    while i < cnt {
        let stmt_node = list_item(p, lstart + i)
        print_node(p, source, stmt_node, depth + 1)
        i = i + 1
    }
    0
}

fn print_node_fn(p: i64, source: str, off: i64, depth: i64) -> i64 {
    print_indent(depth)
    let name_tok = node_d0(p, off)
    print_str("FN ")
    print_str(tok_text(source, p_tokens(p), name_tok))
    let param_count = node_d2(p, off)
    print_str("(")
    let pstart = node_d1(p, off)
    let mut pi = 0
    while pi < param_count {
        if pi > 0 { print_str(", ") } else { 0 }
        let pn = list_item(p, pstart + pi * 2)
        let pt = list_item(p, pstart + pi * 2 + 1)
        print_str(tok_text(source, p_tokens(p), pn))
        print_str(": ")
        print_str(tok_text(source, p_tokens(p), pt))
        pi = pi + 1
    }
    print_str(")")
    let ret_tok = node_d3(p, off)
    if ret_tok >= 0 {
        print_str(" -> ")
        print_str(tok_text(source, p_tokens(p), ret_tok))
    } else {
        0
    }
    println_str("")
    let body_node = node_d4(p, off)
    print_node(p, source, body_node, depth + 1)
}

fn print_node_program(p: i64, source: str, off: i64, depth: i64) -> i64 {
    println_str("PROGRAM")
    let lstart = node_d0(p, off)
    let cnt = node_d1(p, off)
    let mut i = 0
    while i < cnt {
        let fn_node = list_item(p, lstart + i)
        print_node(p, source, fn_node, depth + 1)
        i = i + 1
    }
    0
}

// ============================================================================
// Tests
// ============================================================================

fn test_1() -> i64 {
    let source = "fn main() -> i64 { 42 }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    assert_eq(node_type(p, prog), N_PROGRAM())
    // Program should have 1 function
    assert_eq(node_d1(p, prog), 1)
    // Get the function node
    let fn_off = list_item(p, node_d0(p, prog))
    assert_eq(node_type(p, fn_off), N_FN())
    println_str("Test 1 passed: simple function")
    0
}

fn test_2() -> i64 {
    let source = "fn add(a: i64, b: i64) -> i64 { a + b }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let fn_off = list_item(p, node_d0(p, prog))
    assert_eq(node_type(p, fn_off), N_FN())
    // 2 parameters
    assert_eq(node_d2(p, fn_off), 2)
    println_str("Test 2 passed: function with params")
    0
}

fn test_3() -> i64 {
    let source = "fn f() -> i64 { let x = 10; let mut y = 20; x + y }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let fn_off = list_item(p, node_d0(p, prog))
    // Get body block
    let body = node_d4(p, fn_off)
    assert_eq(node_type(p, body), N_BLOCK())
    // Should have 3 statements
    assert_eq(node_d1(p, body), 3)
    println_str("Test 3 passed: let bindings")
    0
}

fn test_4() -> i64 {
    let source = "fn f() -> i64 { if x > 0 { 1 } else { 2 } }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let fn_off = list_item(p, node_d0(p, prog))
    let body = node_d4(p, fn_off)
    // First statement should be an if (not wrapped in EXPR_STMT)
    let first_stmt = list_item(p, node_d0(p, body))
    assert_eq(node_type(p, first_stmt), N_IF())
    // Should have else branch
    let else_br = node_d2(p, first_stmt)
    assert_true(else_br >= 0)
    println_str("Test 4 passed: if/else")
    0
}

fn test_5() -> i64 {
    let source = "fn f() -> i64 { while x > 0 { x = x - 1 } }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let fn_off = list_item(p, node_d0(p, prog))
    let body = node_d4(p, fn_off)
    let first_stmt = list_item(p, node_d0(p, body))
    assert_eq(node_type(p, first_stmt), N_WHILE())
    println_str("Test 5 passed: while loop")
    0
}

fn test_6() -> i64 {
    let source = "fn f() -> i64 { g(1, 2, 3) }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let fn_off = list_item(p, node_d0(p, prog))
    let body = node_d4(p, fn_off)
    let first_stmt = list_item(p, node_d0(p, body))
    // EXPR_STMT wrapping a CALL
    let call_node = node_d0(p, first_stmt)
    assert_eq(node_type(p, call_node), N_CALL())
    // 3 arguments
    assert_eq(node_d2(p, call_node), 3)
    println_str("Test 6 passed: function call 3 args")
    0
}

fn test_7() -> i64 {
    // Test operator precedence: 1 + 2 * 3 should parse as 1 + (2 * 3)
    let source = "fn f() -> i64 { 1 + 2 * 3 }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    let fn_off = list_item(p, node_d0(p, prog))
    let body = node_d4(p, fn_off)
    let first_stmt = list_item(p, node_d0(p, body))
    let expr = node_d0(p, first_stmt)
    // Should be BINOP(+)
    assert_eq(node_type(p, expr), N_BINOP())
    assert_eq(node_d0(p, expr), TK_PLUS())
    // Right child should be BINOP(*)
    let right = node_d2(p, expr)
    assert_eq(node_type(p, right), N_BINOP())
    assert_eq(node_d0(p, right), TK_STAR())
    println_str("Test 7 passed: precedence")
    0
}

fn test_8() -> i64 {
    // Full pretty-print test
    let source = "fn add(a: i64, b: i64) -> i64 { a + b }"
    let tokens = lex(source)
    let p = parse(tokens, source)
    let prog = p_nodes_used(p) - 8
    println_str("--- AST dump ---")
    print_node(p, source, prog, 0)
    println_str("--- end ---")
    println_str("Test 8 passed: AST dump")
    0
}

fn main() -> i64 {
    test_1()
    test_2()
    test_3()
    test_4()
    test_5()
    test_6()
    test_7()
    test_8()
    println_str("All parser tests passed!")
    0
}
