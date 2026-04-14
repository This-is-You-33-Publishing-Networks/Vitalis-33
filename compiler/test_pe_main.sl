// ============================================================================
// Phase 7 Tests - PE Executable Writer
// ============================================================================

fn test_pe_pass(name: str) -> i64 {
    print_str("[PASS] ")
    println_str(name)
    0
}

fn test_pe_fail(name: str, expected: i64, got: i64) -> i64 {
    print_str("[FAIL] ")
    print_str(name)
    print_str(" expected=")
    print_str(to_string_i64(expected))
    print_str(" got=")
    println_str(to_string_i64(got))
    0
}

fn pe_assert(name: str, expected: i64, got: i64) -> i64 {
    if expected == got {
        test_pe_pass(name)
    } else {
        test_pe_fail(name, expected, got)
    }
}

// ============================================================================
// Test 1: DOS header starts with MZ
// ============================================================================
fn test_dos_header() -> i64 {
    // Build a minimal PE with a tiny "code" section
    let code = array_new(16)
    // xor eax, eax; ret
    array_set(code, 0, 49)   // 0x31
    array_set(code, 1, 192)  // 0xC0
    array_set(code, 2, 195)  // 0xC3
    let code_size = 3

    let data = array_new(8)
    let data_size = 0

    let pe = new_pe_builder(code, code_size, data, data_size)
    build_pe(pe)

    // MZ signature
    pe_assert("dos: M", 77, pe_byte_at(pe, 0))
    pe_assert("dos: Z", 90, pe_byte_at(pe, 1))
    // e_lfanew at offset 60 should point to 64
    pe_assert("dos: e_lfanew", 64, pe_u32_at(pe, 60))
}

// ============================================================================
// Test 2: PE Signature
// ============================================================================
fn test_pe_signature() -> i64 {
    let code = array_new(4)
    array_set(code, 0, 195)  // RET
    let pe = new_pe_builder(code, 1, array_new(1), 0)
    build_pe(pe)

    // "PE\0\0" at offset 64
    pe_assert("sig: P", 80, pe_byte_at(pe, 64))
    pe_assert("sig: E", 69, pe_byte_at(pe, 65))
    pe_assert("sig: \\0", 0, pe_byte_at(pe, 66))
    pe_assert("sig: \\0", 0, pe_byte_at(pe, 67))
}

// ============================================================================
// Test 3: COFF header fields
// ============================================================================
fn test_coff_header() -> i64 {
    let code = array_new(4)
    array_set(code, 0, 195)
    let pe = new_pe_builder(code, 1, array_new(1), 0)
    build_pe(pe)

    // COFF starts at offset 68
    // Machine: 0x8664
    pe_assert("coff: machine", 34404, pe_u16_at(pe, 68))
    // NumberOfSections: 2
    pe_assert("coff: sections", 2, pe_u16_at(pe, 70))
    // SizeOfOptionalHeader: 240
    pe_assert("coff: opt_size", 240, pe_u16_at(pe, 84))
}

// ============================================================================
// Test 4: Optional header magic
// ============================================================================
fn test_optional_header() -> i64 {
    let code = array_new(4)
    array_set(code, 0, 195)
    let pe = new_pe_builder(code, 1, array_new(1), 0)
    build_pe(pe)

    // Optional header starts at offset 88
    // PE32+ magic: 0x020B
    pe_assert("opt: magic", 523, pe_u16_at(pe, 88))
    // Subsystem at offset 88 + 68 = 156: CONSOLE = 3
    pe_assert("opt: subsystem", 3, pe_u16_at(pe, 156))
}

// ============================================================================
// Test 5: Section headers present
// ============================================================================
fn test_section_headers() -> i64 {
    let code = array_new(16)
    array_set(code, 0, 195)
    let pe = new_pe_builder(code, 1, array_new(1), 0)
    build_pe(pe)

    // Section headers start after optional header
    // DOS(64) + PESig(4) + COFF(20) + Optional(240) = 328
    let sec_offset = 328

    // .text name: first byte = '.'(46), 't'(116)
    pe_assert("sec: .text dot", 46, pe_byte_at(pe, sec_offset))
    pe_assert("sec: .text t", 116, pe_byte_at(pe, sec_offset + 1))

    // .data section header at sec_offset + 40
    let data_off = sec_offset + 40
    pe_assert("sec: .data dot", 46, pe_byte_at(pe, data_off))
    pe_assert("sec: .data d", 100, pe_byte_at(pe, data_off + 1))  // 'd' = 100
}

// ============================================================================
// Test 6: File is aligned to 512 bytes
// ============================================================================
fn test_alignment() -> i64 {
    let code = array_new(4)
    array_set(code, 0, 195)
    let pe = new_pe_builder(code, 1, array_new(1), 0)
    build_pe(pe)

    let size = pe_file_size(pe)
    pe_assert("align: multiple of 512", 0, size % 512)
    // Should be at least 1024 (headers 512 + text 512)
    pe_assert("align: >= 1024", 1, if size >= 1024 { 1 } else { 0 })
}

// ============================================================================
// Test 7: Code is placed at correct file offset
// ============================================================================
fn test_code_placement() -> i64 {
    let code = array_new(8)
    array_set(code, 0, 144)  // NOP
    array_set(code, 1, 144)  // NOP
    array_set(code, 2, 195)  // RET
    let pe = new_pe_builder(code, 3, array_new(1), 0)
    build_pe(pe)

    // Headers are 512 bytes (aligned), so code starts at 512
    pe_assert("code: nop at 512", 144, pe_byte_at(pe, 512))
    pe_assert("code: nop at 513", 144, pe_byte_at(pe, 513))
    pe_assert("code: ret at 514", 195, pe_byte_at(pe, 514))
}

// ============================================================================
// Test 8: Data section with content
// ============================================================================
fn test_data_section() -> i64 {
    let code = array_new(4)
    array_set(code, 0, 195)

    let data = array_new(8)
    array_set(data, 0, 72)   // 'H'
    array_set(data, 1, 105)  // 'i'
    array_set(data, 2, 0)    // null

    let pe = new_pe_builder(code, 1, data, 3)
    build_pe(pe)

    // Data at offset = headers(512) + text_aligned(512) = 1024
    pe_assert("data: H", 72, pe_byte_at(pe, 1024))
    pe_assert("data: i", 105, pe_byte_at(pe, 1025))
    pe_assert("data: null", 0, pe_byte_at(pe, 1026))
}

// ============================================================================
// Test 9: align_up utility
// ============================================================================
fn test_align_up() -> i64 {
    pe_assert("align_up: 0", 0, align_up(0, 512))
    pe_assert("align_up: 1->512", 512, align_up(1, 512))
    pe_assert("align_up: 512->512", 512, align_up(512, 512))
    pe_assert("align_up: 513->1024", 1024, align_up(513, 512))
    pe_assert("align_up: 4096", 4096, align_up(4096, 4096))
    pe_assert("align_up: 100->4096", 4096, align_up(100, 4096))
}

// ============================================================================
// Test 10: Entry point RVA
// ============================================================================
fn test_entry_point() -> i64 {
    let code = array_new(16)
    // 5 bytes of NOPs then RET (entry at offset 5)
    let mut i = 0
    while i < 5 {
        array_set(code, i, 144)
        i = i + 1
    }
    array_set(code, 5, 195)

    let pe = new_pe_builder(code, 6, array_new(1), 0)
    // Set entry to offset 5 within code
    array_set(pe, 7, 5)
    build_pe(pe)

    // AddressOfEntryPoint is at offset 88 + 16 = 104
    // Should be text_rva(0x1000) + 5 = 4101
    pe_assert("entry: rva", 4101, pe_u32_at(pe, 104))
}

// ============================================================================
// Test 11: Image base
// ============================================================================
fn test_image_base() -> i64 {
    let code = array_new(4)
    array_set(code, 0, 195)
    let pe = new_pe_builder(code, 1, array_new(1), 0)
    build_pe(pe)

    // ImageBase at optional header offset + 24 = 88 + 24 = 112
    // Lower 32 bits should be 0x400000 = 4194304
    pe_assert("imgbase: low32", 4194304, pe_u32_at(pe, 112))
    // Upper 32 bits should be 0
    pe_assert("imgbase: high32", 0, pe_u32_at(pe, 116))
}

// ============================================================================
// Test 12: File output total size consistency
// ============================================================================
fn test_total_size() -> i64 {
    let code = array_new(600)
    let mut i = 0
    while i < 600 {
        array_set(code, i, 144)  // fill with NOPs
        i = i + 1
    }

    let data = array_new(200)
    i = 0
    while i < 200 {
        array_set(data, i, 65)  // fill with 'A'
        i = i + 1
    }

    let pe = new_pe_builder(code, 600, data, 200)
    build_pe(pe)

    // headers(512) + text(align_up(600,512)=1024) + data(align_up(200,512)=512) = 2048
    pe_assert("total: size", 2048, pe_file_size(pe))
}

// ============================================================================
// Main
// ============================================================================
fn main() -> i64 {
    println_str("=== PE Writer Tests ===")

    test_dos_header()
    test_pe_signature()
    test_coff_header()
    test_optional_header()
    test_section_headers()
    test_alignment()
    test_code_placement()
    test_data_section()
    test_align_up()
    test_entry_point()
    test_image_base()
    test_total_size()

    println_str("=== All PE writer tests done ===")
    0
}
