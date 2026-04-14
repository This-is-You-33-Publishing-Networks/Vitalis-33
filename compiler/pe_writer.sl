// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 7: PE Executable Writer
// ============================================================================
// Produces a minimal Windows PE32+ (64-bit) .exe from machine code + data.
//
// PE layout:
//   DOS Header (64 bytes, MZ stub)
//   PE Signature ("PE\0\0")
//   COFF File Header (20 bytes)
//   Optional Header (PE32+, 112 bytes standard + data directories)
//   Section Headers (.text, .data, .rdata)
//   [padding to file alignment]
//   .text section (machine code)
//   .data section (initialized data)
//   .rdata section (imports)
//
// Simplifications in this bootstrap compiler:
//   - No relocations (fixed base address 0x400000)
//   - Minimal imports: kernel32.dll (ExitProcess, GetStdHandle, WriteFile)
//   - No TLS, no exception handling, no debug info
//   - Single .text section for code, single .data for strings/constants
//
// All values are little-endian. We use the emit_* helpers from x86_emit.sl
// repurposed for the PE file buffer.
//
// NOTE: No `return` inside if-blocks (Cranelift bug).
// ============================================================================

// -- PE constants --
fn PE_DOS_HEADER_SIZE() -> i64 { 64 }
fn PE_SIGNATURE_SIZE() -> i64  { 4 }
fn PE_COFF_HEADER_SIZE() -> i64 { 20 }
fn PE_OPT_HEADER_SIZE() -> i64 { 240 }  // PE32+ optional header with data dirs
fn PE_SECTION_HEADER_SIZE() -> i64 { 40 }

fn PE_FILE_ALIGNMENT() -> i64 { 512 }   // 0x200
fn PE_SECTION_ALIGNMENT() -> i64 { 4096 }  // 0x1000
fn PE_IMAGE_BASE() -> i64 { 4194304 }  // 0x400000

// Characteristics
fn PE_CHARACTERISTIC_EXEC() -> i64 { 2 }       // IMAGE_FILE_EXECUTABLE_IMAGE
fn PE_CHARACTERISTIC_LARGE() -> i64 { 32 }      // IMAGE_FILE_LARGE_ADDRESS_AWARE
fn PE_SUBSYSTEM_CONSOLE() -> i64 { 3 }          // IMAGE_SUBSYSTEM_WINDOWS_CUI

// Section characteristics
fn SEC_CODE() -> i64     { 1610612768 }  // 0x60000020 = EXEC|READ|CODE
fn SEC_DATA() -> i64     { 3221225536 }  // 0xC0000040 = READ|WRITE|INITIALIZED
fn SEC_RDATA() -> i64    { 1073741888 }  // 0x40000040 = READ|INITIALIZED

// ============================================================================
// PE builder context
// ============================================================================
// pe[0]  = output buffer (byte array)
// pe[1]  = output position
// pe[2]  = capacity
// pe[3]  = code bytes (from x86_emit)
// pe[4]  = code size
// pe[5]  = data bytes (from x86_emit)
// pe[6]  = data size
// pe[7]  = entry point RVA
// pe[8]  = number of sections (2 or 3)
// pe[9]  = headers size (aligned)
// pe[10] = text section RVA
// pe[11] = data section RVA
// pe[12] = import table entries (names array)
// pe[13] = import count

fn pe_out(pe: i64) -> i64       { array_get(pe, 0) }
fn pe_pos(pe: i64) -> i64       { array_get(pe, 1) }
fn pe_cap(pe: i64) -> i64       { array_get(pe, 2) }
fn pe_code(pe: i64) -> i64      { array_get(pe, 3) }
fn pe_code_size(pe: i64) -> i64 { array_get(pe, 4) }
fn pe_data(pe: i64) -> i64      { array_get(pe, 5) }
fn pe_data_size(pe: i64) -> i64 { array_get(pe, 6) }
fn pe_entry(pe: i64) -> i64     { array_get(pe, 7) }
fn pe_num_sections(pe: i64) -> i64 { array_get(pe, 8) }
fn pe_headers_size(pe: i64) -> i64 { array_get(pe, 9) }
fn pe_text_rva(pe: i64) -> i64  { array_get(pe, 10) }
fn pe_data_rva(pe: i64) -> i64  { array_get(pe, 11) }

fn pe_set_pos(pe: i64, v: i64) -> i64 { array_set(pe, 1, v); 0 }

fn new_pe_builder(code: i64, code_size: i64, data: i64, data_size: i64) -> i64 {
    let cap = 65536  // 64KB should be enough for minimal exe
    let pe = array_new(14)
    array_set(pe, 0, array_new(cap))
    array_set(pe, 1, 0)
    array_set(pe, 2, cap)
    array_set(pe, 3, code)
    array_set(pe, 4, code_size)
    array_set(pe, 5, data)
    array_set(pe, 6, data_size)
    array_set(pe, 7, 0)  // entry point, set later
    array_set(pe, 8, 2)  // .text + .data
    array_set(pe, 9, 0)  // headers size, computed later
    array_set(pe, 10, PE_SECTION_ALIGNMENT())  // .text RVA = 0x1000
    array_set(pe, 11, 0)  // .data RVA, computed later
    array_set(pe, 12, array_new(32))
    array_set(pe, 13, 0)
    pe
}

// ============================================================================
// Low-level byte emission into PE output buffer
// ============================================================================
fn pe_emit_byte(pe: i64, b: i64) -> i64 {
    let out = pe_out(pe)
    let pos = pe_pos(pe)
    array_set(out, pos, b % 256)
    pe_set_pos(pe, pos + 1)
    0
}

fn pe_emit_u16(pe: i64, val: i64) -> i64 {
    pe_emit_byte(pe, val % 256)
    pe_emit_byte(pe, (val / 256) % 256)
}

fn pe_emit_u32(pe: i64, val: i64) -> i64 {
    let u = if val < 0 { val + 4294967296 } else { val % 4294967296 }
    pe_emit_byte(pe, u % 256)
    pe_emit_byte(pe, (u / 256) % 256)
    pe_emit_byte(pe, (u / 65536) % 256)
    pe_emit_byte(pe, (u / 16777216) % 256)
}

fn pe_emit_u64(pe: i64, val: i64) -> i64 {
    pe_emit_u32(pe, val % 4294967296)
    pe_emit_u32(pe, val / 4294967296)
}

fn pe_emit_zeros(pe: i64, count: i64) -> i64 {
    let mut i = 0
    while i < count {
        pe_emit_byte(pe, 0)
        i = i + 1
    }
    0
}

// Emit a string (no null terminator)
fn pe_emit_string(pe: i64, s: str) -> i64 {
    let slen = str_len(s)
    let mut i = 0
    while i < slen {
        pe_emit_byte(pe, char_to_int(str_char_at(s, i)))
        i = i + 1
    }
    0
}

// Emit a fixed-width string (padded with zeros)
fn pe_emit_name8(pe: i64, s: str) -> i64 {
    let slen = str_len(s)
    let mut i = 0
    while i < 8 {
        if i < slen {
            pe_emit_byte(pe, char_to_int(str_char_at(s, i)))
        } else {
            pe_emit_byte(pe, 0)
        }
        i = i + 1
    }
    0
}

// Pad to alignment boundary
fn pe_pad_to(pe: i64, alignment: i64) -> i64 {
    let pos = pe_pos(pe)
    let remainder = pos % alignment
    if remainder != 0 {
        pe_emit_zeros(pe, alignment - remainder)
    } else {
        0
    }
}

// Copy raw bytes from source array into PE output
fn pe_copy_bytes(pe: i64, src: i64, count: i64) -> i64 {
    let mut i = 0
    while i < count {
        pe_emit_byte(pe, array_get(src, i))
        i = i + 1
    }
    0
}

// ============================================================================
// Align size up to boundary
// ============================================================================
fn align_up(size: i64, alignment: i64) -> i64 {
    let r = size % alignment
    if r != 0 {
        size + (alignment - r)
    } else {
        size
    }
}

// ============================================================================
// Write DOS Header (64 bytes)
// ============================================================================
fn write_dos_header(pe: i64) -> i64 {
    // "MZ" magic
    pe_emit_byte(pe, 77)   // 'M'
    pe_emit_byte(pe, 90)   // 'Z'

    // e_cblp through e_ovno (29 words = 58 bytes of DOS header fields)
    // We zero most fields, but set e_lfanew (offset to PE sig) at offset 60
    pe_emit_zeros(pe, 58)

    // e_lfanew at offset 60: points to PE signature (right after DOS header)
    pe_emit_u32(pe, PE_DOS_HEADER_SIZE())
    0
}

// ============================================================================
// Write COFF File Header (20 bytes)
// ============================================================================
fn write_coff_header(pe: i64) -> i64 {
    // Machine: x64 = 0x8664
    pe_emit_u16(pe, 34404)  // 0x8664
    // NumberOfSections
    pe_emit_u16(pe, pe_num_sections(pe))
    // TimeDateStamp
    pe_emit_u32(pe, 0)
    // PointerToSymbolTable
    pe_emit_u32(pe, 0)
    // NumberOfSymbols
    pe_emit_u32(pe, 0)
    // SizeOfOptionalHeader
    pe_emit_u16(pe, PE_OPT_HEADER_SIZE())
    // Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
    pe_emit_u16(pe, PE_CHARACTERISTIC_EXEC() + PE_CHARACTERISTIC_LARGE())
}

// ============================================================================
// Write Optional Header (PE32+, 240 bytes)
// ============================================================================
fn write_optional_header(pe: i64, image_size: i64) -> i64 {
    let num_sects = pe_num_sections(pe)
    let headers_size = pe_headers_size(pe)
    let text_rva = pe_text_rva(pe)
    let code_size = pe_code_size(pe)
    let data_rva = pe_data_rva(pe)
    let data_size = pe_data_size(pe)

    // Magic: PE32+ = 0x020B
    pe_emit_u16(pe, 523)  // 0x020B
    // LinkerVersion (major, minor)
    pe_emit_byte(pe, 1)
    pe_emit_byte(pe, 0)
    // SizeOfCode
    pe_emit_u32(pe, align_up(code_size, PE_FILE_ALIGNMENT()))
    // SizeOfInitializedData
    pe_emit_u32(pe, align_up(data_size, PE_FILE_ALIGNMENT()))
    // SizeOfUninitializedData
    pe_emit_u32(pe, 0)
    // AddressOfEntryPoint (RVA)
    pe_emit_u32(pe, text_rva + pe_entry(pe))
    // BaseOfCode
    pe_emit_u32(pe, text_rva)

    // --- PE32+ specific fields ---
    // ImageBase (64-bit)
    pe_emit_u64(pe, PE_IMAGE_BASE())
    // SectionAlignment
    pe_emit_u32(pe, PE_SECTION_ALIGNMENT())
    // FileAlignment
    pe_emit_u32(pe, PE_FILE_ALIGNMENT())
    // OS Version (major, minor)
    pe_emit_u16(pe, 6)
    pe_emit_u16(pe, 0)
    // Image Version
    pe_emit_u16(pe, 0)
    pe_emit_u16(pe, 0)
    // Subsystem Version (6.0)
    pe_emit_u16(pe, 6)
    pe_emit_u16(pe, 0)
    // Win32VersionValue
    pe_emit_u32(pe, 0)
    // SizeOfImage
    pe_emit_u32(pe, image_size)
    // SizeOfHeaders
    pe_emit_u32(pe, headers_size)
    // CheckSum
    pe_emit_u32(pe, 0)
    // Subsystem: CONSOLE
    pe_emit_u16(pe, PE_SUBSYSTEM_CONSOLE())
    // DllCharacteristics: NX_COMPAT | DYNAMIC_BASE
    pe_emit_u16(pe, 352)  // 0x0160
    // SizeOfStackReserve (64-bit)
    pe_emit_u64(pe, 1048576)  // 1MB
    // SizeOfStackCommit
    pe_emit_u64(pe, 4096)
    // SizeOfHeapReserve
    pe_emit_u64(pe, 1048576)
    // SizeOfHeapCommit
    pe_emit_u64(pe, 4096)
    // LoaderFlags
    pe_emit_u32(pe, 0)
    // NumberOfRvaAndSizes (16)
    pe_emit_u32(pe, 16)

    // Data directories (16 entries x 8 bytes = 128 bytes)
    // All zeros for now (no imports in this minimal version)
    pe_emit_zeros(pe, 128)
}

// ============================================================================
// Write Section Header (40 bytes each)
// ============================================================================
fn write_section_header(pe: i64, name: str, virtual_size: i64, virtual_addr: i64,
                         raw_size: i64, raw_offset: i64, characteristics: i64) -> i64 {
    pe_emit_name8(pe, name)
    pe_emit_u32(pe, virtual_size)     // VirtualSize
    pe_emit_u32(pe, virtual_addr)     // VirtualAddress (RVA)
    pe_emit_u32(pe, raw_size)         // SizeOfRawData
    pe_emit_u32(pe, raw_offset)       // PointerToRawData
    pe_emit_u32(pe, 0)               // PointerToRelocations
    pe_emit_u32(pe, 0)               // PointerToLinenumbers
    pe_emit_u16(pe, 0)               // NumberOfRelocations
    pe_emit_u16(pe, 0)               // NumberOfLinenumbers
    pe_emit_u32(pe, characteristics)  // Characteristics
}

// ============================================================================
// Build the complete PE file
// ============================================================================
fn build_pe(pe: i64) -> i64 {
    let code_size = pe_code_size(pe)
    let data_size = pe_data_size(pe)
    let num_sects = pe_num_sections(pe)

    // Compute layout sizes
    let headers_raw = PE_DOS_HEADER_SIZE() + PE_SIGNATURE_SIZE() +
                      PE_COFF_HEADER_SIZE() + PE_OPT_HEADER_SIZE() +
                      num_sects * PE_SECTION_HEADER_SIZE()
    let headers_aligned = align_up(headers_raw, PE_FILE_ALIGNMENT())
    array_set(pe, 9, headers_aligned)

    // .text section
    let text_rva = PE_SECTION_ALIGNMENT()  // 0x1000
    array_set(pe, 10, text_rva)
    let text_raw_size = align_up(code_size, PE_FILE_ALIGNMENT())
    let text_file_offset = headers_aligned

    // .data section
    let data_rva = text_rva + align_up(code_size, PE_SECTION_ALIGNMENT())
    array_set(pe, 11, data_rva)
    let data_raw_size = align_up(data_size, PE_FILE_ALIGNMENT())
    let data_file_offset = text_file_offset + text_raw_size

    // Total image size
    let image_size = data_rva + align_up(data_size, PE_SECTION_ALIGNMENT())

    // === Write headers ===
    write_dos_header(pe)

    // PE signature: "PE\0\0"
    pe_emit_byte(pe, 80)   // 'P'
    pe_emit_byte(pe, 69)   // 'E'
    pe_emit_byte(pe, 0)
    pe_emit_byte(pe, 0)

    write_coff_header(pe)
    write_optional_header(pe, image_size)

    // Section headers
    write_section_header(pe, ".text", code_size, text_rva,
                         text_raw_size, text_file_offset, SEC_CODE())
    write_section_header(pe, ".data", data_size, data_rva,
                         data_raw_size, data_file_offset, SEC_DATA())

    // Pad headers to alignment
    pe_pad_to(pe, PE_FILE_ALIGNMENT())

    // === Write .text section ===
    pe_copy_bytes(pe, pe_code(pe), code_size)
    pe_pad_to(pe, PE_FILE_ALIGNMENT())

    // === Write .data section ===
    if data_size > 0 {
        pe_copy_bytes(pe, pe_data(pe), data_size)
        pe_pad_to(pe, PE_FILE_ALIGNMENT())
    } else {
        0
    }

    0
}

// ============================================================================
// Query helpers
// ============================================================================

// Get the total size of the generated PE file
fn pe_file_size(pe: i64) -> i64 {
    pe_pos(pe)
}

// Get a byte from the PE output at a given offset
fn pe_byte_at(pe: i64, offset: i64) -> i64 {
    array_get(pe_out(pe), offset)
}

// Extract a little-endian u16 from PE output
fn pe_u16_at(pe: i64, offset: i64) -> i64 {
    let out = pe_out(pe)
    array_get(out, offset) + array_get(out, offset + 1) * 256
}

// Extract a little-endian u32 from PE output
fn pe_u32_at(pe: i64, offset: i64) -> i64 {
    let out = pe_out(pe)
    array_get(out, offset) +
    array_get(out, offset + 1) * 256 +
    array_get(out, offset + 2) * 65536 +
    array_get(out, offset + 3) * 16777216
}
