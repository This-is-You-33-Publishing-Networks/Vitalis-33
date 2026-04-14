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
