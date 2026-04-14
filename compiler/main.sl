// ============================================================================
// Vitalis Self-Hosting Compiler - Phase 10: CLI Driver (main.sl)
// ============================================================================
// Usage:  vtc.exe run bootstrap.sl -- build <input.sl> -o <output.exe>
//         vtc.exe run bootstrap.sl -- check <input.sl>
//
// This is the top-level entry point that:
//   1. Parses command-line arguments
//   2. Reads source from disk
//   3. Calls compile(source) -> PE bytes
//   4. Writes PE bytes to output file
//
// Combined with all compiler modules into bootstrap.sl for execution.
// ============================================================================

fn print_usage() -> i64 {
    println_str("Vitalis Self-Hosting Compiler v60")
    println_str("Usage:")
    println_str("  vtc run bootstrap.sl -- build <input.sl> -o <output.exe>")
    println_str("  vtc run bootstrap.sl -- check <input.sl>")
    println_str("")
    println_str("Commands:")
    println_str("  build   Compile .sl source to Windows PE executable")
    println_str("  check   Type-check source without generating code")
    0
}

// Find the index of our "--" separator in args
// vtc.exe run bootstrap.sl -- build input.sl -o output.exe
// args: [vtc.exe, run, bootstrap.sl, --, build, input.sl, -o, output.exe]
// We want the args AFTER "--"
fn find_dash_dash() -> i64 {
    let ac = args_count()
    let mut i = 0
    let mut found = -1
    while i < ac {
        if str_eq(args_get(i), "--") == 1 {
            found = i
            i = ac  // break
        } else {
            0
        }
        i = i + 1
    }
    found
}

// Get arg at position relative to after "--"
fn user_arg(idx: i64) -> str {
    let dd = find_dash_dash()
    if dd < 0 {
        // No --, just use raw position (args_get(idx))
        args_get(idx)
    } else {
        args_get(dd + 1 + idx)
    }
}

fn user_arg_count() -> i64 {
    let dd = find_dash_dash()
    if dd < 0 {
        args_count()
    } else {
        args_count() - dd - 1
    }
}

// ============================================================================
// Commands
// ============================================================================

fn cmd_build(input_path: str, output_path: str) -> i64 {
    print_str("[vtc60] Reading ")
    println_str(input_path)

    let source = file_read(input_path)
    let src_len = str_len(source)
    if src_len <= 0 {
        print_str("[vtc60] ERROR: Could not read file: ")
        println_str(input_path)
        exit(1)
        0
    } else {
        print_str("[vtc60] Source: ")
        print(src_len)
        println_str(" chars")

        // Compile
        println_str("[vtc60] Compiling...")
        let ctx = compile(source)

        if compile_ok(ctx) == 1 {
            let size = compile_output_size(ctx)
            print_str("[vtc60] Compilation OK: ")
            print(size)
            println_str(" bytes PE output")

            // Write to file
            print_str("[vtc60] Writing ")
            println_str(output_path)
            let pe_bytes = compile_pe_bytes(ctx)
            file_write_bytes(output_path, pe_bytes)

            print_str("[vtc60] Done: ")
            println_str(output_path)
            0
        } else {
            print_str("[vtc60] COMPILATION FAILED at stage: ")
            println_str(compile_error_stage(ctx))
            exit(1)
            0
        }
    }
}

fn cmd_check(input_path: str) -> i64 {
    print_str("[vtc60] Checking ")
    println_str(input_path)

    let source = file_read(input_path)
    let src_len = str_len(source)
    if src_len <= 0 {
        print_str("[vtc60] ERROR: Could not read file: ")
        println_str(input_path)
        exit(1)
        0
    } else {
        print_str("[vtc60] Source: ")
        print(src_len)
        println_str(" chars")

        // Run stages up to typecheck only
        let ctx = new_compile_ctx(source)
        stage_lex(ctx, source)
        if ctx_error(ctx) != ERR_NONE() {
            println_str("[vtc60] LEX ERROR")
            exit(1)
            0
        } else {
            print_str("[vtc60] Lexed: ")
            print(ctx_tok_count(ctx))
            println_str(" tokens")

            stage_parse(ctx, source)
            if ctx_error(ctx) != ERR_NONE() {
                println_str("[vtc60] PARSE ERROR")
                exit(1)
                0
            } else {
                println_str("[vtc60] Parsed OK")

                stage_typecheck(ctx, source)
                if ctx_error(ctx) != ERR_NONE() {
                    println_str("[vtc60] TYPE ERROR")
                    exit(1)
                    0
                } else {
                    println_str("[vtc60] Type check OK")
                    println_str("[vtc60] All checks passed")
                    0
                }
            }
        }
    }
}

// ============================================================================
// Main entry point
// ============================================================================
fn main() -> i64 {
    let uac = user_arg_count()

    if uac < 1 {
        print_usage()
        0
    } else {
        let cmd = user_arg(0)

        if str_eq(cmd, "build") == 1 {
            if uac < 4 {
                println_str("[vtc60] ERROR: build requires: build <input.sl> -o <output.exe>")
                exit(1)
                0
            } else {
                let input = user_arg(1)
                // user_arg(2) should be "-o"
                let output = user_arg(3)
                cmd_build(input, output)
            }
        } else if str_eq(cmd, "check") == 1 {
            if uac < 2 {
                println_str("[vtc60] ERROR: check requires: check <input.sl>")
                exit(1)
                0
            } else {
                let input = user_arg(1)
                cmd_check(input)
            }
        } else {
            print_str("[vtc60] Unknown command: ")
            println_str(cmd)
            print_usage()
            exit(1)
            0
        }
    }
}
