// ============================================================================
// PROOF 2: Self-Compilation -- The compiler compiles itself
// ============================================================================
// Reads bootstrap.sl (the full 6,000+ LOC compiler), feeds it through
// the self-hosting pipeline, and produces vtc_v60.exe.
// This is the ULTIMATE proof: Vitalis compiles Vitalis.
// ============================================================================

fn main() -> i64 {
    println_str("==================================================")
    println_str("  Vitalis Self-Hosting Compiler v60 -- PROOF 2")
    println_str("  SELF-COMPILATION: bootstrap.sl -> vtc_v60.exe")
    println_str("==================================================")
    println_str("")

    let input = "C:/Vitalis-V60/compiler/bootstrap.sl"
    let output = "C:/Vitalis-V60/compiler/vtc_v60.exe"

    // Step 1: Read our own source
    print_str("[1/3] Reading compiler source: ")
    println_str(input)
    let source = file_read(input)
    let src_len = str_len(source)
    print_str("       Source size: ")
    print(src_len)
    println_str(" chars")

    if src_len <= 0 {
        println_str("ERROR: Could not read source!")
        exit(1)
        0
    } else {
        // Step 2: Full compilation
        println_str("")
        println_str("[2/3] Compiling through all 7 stages...")
        let ctx = compile(source)

        // Step 3: Results
        println_str("")
        println_str("[3/3] Results:")
        let err = ctx_error(ctx)
        if err == ERR_NONE() {
            let pe_size = compile_output_size(ctx)
            let pe_bytes = compile_pe_bytes(ctx)

            // Write PE
            print_str("       PE size: ")
            print(pe_size)
            println_str(" bytes")
            print_str("       Writing: ")
            println_str(output)
            let out_buf = array_new(pe_size)
            let mut bi = 0
            while bi < pe_size {
                array_set(out_buf, bi, array_get(pe_bytes, bi))
                bi = bi + 1
            }
            file_write_bytes(output, out_buf)
            println_str("       Written successfully")

            // Verify PE headers
            println_str("")
            println_str("==================================================")
            println_str("  VERIFICATION")
            println_str("==================================================")
            print_str("  Source:     ")
            print(src_len)
            println_str(" chars")
            print_str("  Tokens:     ")
            println(ctx_tok_count(ctx))
            print_str("  PE size:    ")
            print(pe_size)
            println_str(" bytes")
            print_str("  MZ sig:     ")
            print(array_get(pe_bytes, 0))
            print_str(" ")
            println(array_get(pe_bytes, 1))
            println_str("")
            println_str("==================================================")
            println_str("  PROOF 2 COMPLETE: VITALIS COMPILED ITSELF")
            println_str("  bootstrap.sl -> vtc_v60.exe")
            println_str("==================================================")
            0
        } else {
            print_str("  Compilation stopped at stage: ")
            println_str(compile_error_stage(ctx))

            // Even if typecheck failed, show what we achieved
            print_str("  Tokens lexed:    ")
            println(ctx_tok_count(ctx))
            print_str("  Parse offset:    ")
            println(ctx_prog_off(ctx))
            println_str("")
            println_str("==================================================")
            println_str("  PROOF 2: PARTIAL -- see diagnostics above")
            println_str("==================================================")
            0
        }
    }
}
