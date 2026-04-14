// ============================================================================
// PROOF 1: Compile hello.sl -> hello.exe
// ============================================================================
// This script reads examples/hello.sl, compiles it through the full
// self-hosting pipeline, and writes a Windows PE executable to disk.
// ============================================================================

fn main() -> i64 {
    println_str("==================================================")
    println_str("  Vitalis Self-Hosting Compiler v60 - PROOF 1")
    println_str("  Compile hello.sl -> hello.exe")
    println_str("==================================================")
    println_str("")

    // Read source
    let input = "C:/Vitalis-V60/examples/hello.sl"
    let output = "C:/Vitalis-V60/compiler/hello.exe"

    print_str("[1/4] Reading source: ")
    println_str(input)
    let source = file_read(input)
    let src_len = str_len(source)
    print_str("       Source size: ")
    print(src_len)
    println_str(" chars")

    if src_len <= 0 {
        println_str("ERROR: Could not read source file!")
        exit(1)
        0
    } else {
        // Show first line of source
        print_str("       First chars: ")
        println_str(str_substr(source, 0, 60))
        println_str("")

        // Stage-by-stage compilation with diagnostics
        println_str("[2/4] Compiling through all stages...")
        let ctx = new_compile_ctx(source)

        // Stage 1: Lex
        print_str("  Stage 1 (Lex).......... ")
        stage_lex(ctx, source)
        if ctx_error(ctx) != ERR_NONE() {
            println_str("FAIL")
            exit(1)
            0
        } else {
            print(ctx_tok_count(ctx))
            println_str(" tokens  OK")

            // Stage 2: Parse
            print_str("  Stage 2 (Parse)........ ")
            stage_parse(ctx, source)
            if ctx_error(ctx) != ERR_NONE() {
                println_str("FAIL")
                exit(1)
                0
            } else {
                print_str("prog_off=")
                print(ctx_prog_off(ctx))
                println_str("  OK")

                // Stage 3: TypeCheck
                print_str("  Stage 3 (TypeCheck).... ")
                stage_typecheck(ctx, source)
                if ctx_error(ctx) != ERR_NONE() {
                    println_str("FAIL")
                    exit(1)
                    0
                } else {
                    println_str("OK")

                    // Stage 4: IR Gen
                    print_str("  Stage 4 (IR Gen)....... ")
                    stage_ir_gen(ctx, source)
                    if ctx_error(ctx) != ERR_NONE() {
                        println_str("FAIL")
                        exit(1)
                        0
                    } else {
                        println_str("OK")

                        // Stage 5: RegAlloc
                        print_str("  Stage 5 (RegAlloc)..... ")
                        stage_regalloc(ctx)
                        if ctx_error(ctx) != ERR_NONE() {
                            println_str("FAIL")
                            exit(1)
                            0
                        } else {
                            println_str("OK")

                            // Stage 6: x86 Emit
                            print_str("  Stage 6 (x86 Emit)..... ")
                            stage_x86_emit(ctx)
                            if ctx_error(ctx) != ERR_NONE() {
                                println_str("FAIL")
                                exit(1)
                                0
                            } else {
                                let code_size = buf_pos(ctx_codebuf(ctx))
                                print(code_size)
                                println_str(" bytes code  OK")

                                // Stage 7: PE Build
                                print_str("  Stage 7 (PE Build)..... ")
                                stage_pe_build(ctx)
                                if ctx_error(ctx) != ERR_NONE() {
                                    println_str("FAIL")
                                    exit(1)
                                    0
                                } else {
                                    let pe_size = compile_output_size(ctx)
                                    print(pe_size)
                                    println_str(" bytes PE  OK")
                                    println_str("")

                                    // Write PE
                                    print_str("[3/4] Writing: ")
                                    println_str(output)
                                    let pe_bytes = compile_pe_bytes(ctx)
                                    // Create right-sized output array
                                    let out_buf = array_new(pe_size)
                                    let mut bi = 0
                                    while bi < pe_size {
                                        array_set(out_buf, bi, array_get(pe_bytes, bi))
                                        bi = bi + 1
                                    }
                                    file_write_bytes(output, out_buf)
                                    println_str("       Written successfully")
                                    println_str("")

                                    // Verify
                                    println_str("[4/4] Verification:")
                                    print_str("       MZ signature:  0x")
                                    print(array_get(pe_bytes, 0))
                                    print_str(" 0x")
                                    println(array_get(pe_bytes, 1))
                                    print_str("       PE signature:  0x")
                                    print(pe_byte_at(ctx_pe(ctx), 64))
                                    print_str(" 0x")
                                    println(pe_byte_at(ctx_pe(ctx), 65))
                                    print_str("       Machine type:  0x")
                                    println(pe_u16_at(ctx_pe(ctx), 68))
                                    print_str("       PE32+ magic:   0x")
                                    println(pe_u16_at(ctx_pe(ctx), 88))
                                    print_str("       File size:     ")
                                    print(pe_size)
                                    println_str(" bytes")
                                    print_str("       Alignment:     ")
                                    print(pe_size % 512)
                                    println_str(" (should be 0)")
                                    println_str("")
                                    println_str("==================================================")
                                    println_str("  PROOF 1 COMPLETE: hello.sl -> hello.exe")
                                    println_str("==================================================")
                                    0
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
