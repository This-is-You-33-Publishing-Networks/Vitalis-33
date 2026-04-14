// ============================================================================
// PROOF 3: Determinism -- Same input always produces identical output
// ============================================================================
// Compiles hello.sl TWICE and compares every byte of the output.
// If all bytes match, the compiler is deterministic.
// ============================================================================

fn main() -> i64 {
    println_str("==================================================")
    println_str("  Vitalis Self-Hosting Compiler v60 -- PROOF 3")
    println_str("  DETERMINISM: compile twice, compare bytes")
    println_str("==================================================")
    println_str("")

    let source = file_read("C:/Vitalis-V60/examples/hello.sl")
    let src_len = str_len(source)
    print_str("Source: ")
    print(src_len)
    println_str(" chars")
    println_str("")

    // Compile #1
    println_str("[1/3] Compilation #1...")
    let ctx1 = compile(source)
    let ok1 = compile_ok(ctx1)
    let size1 = compile_output_size(ctx1)
    let bytes1 = compile_pe_bytes(ctx1)
    print_str("       Result: ")
    if ok1 == 1 {
        print(size1)
        println_str(" bytes  OK")
    } else {
        println_str("FAILED")
        exit(1)
    }

    // Compile #2
    println_str("[2/3] Compilation #2...")
    let ctx2 = compile(source)
    let ok2 = compile_ok(ctx2)
    let size2 = compile_output_size(ctx2)
    let bytes2 = compile_pe_bytes(ctx2)
    print_str("       Result: ")
    if ok2 == 1 {
        print(size2)
        println_str(" bytes  OK")
    } else {
        println_str("FAILED")
        exit(1)
    }
    println_str("")

    // Compare
    println_str("[3/3] Byte-by-byte comparison...")
    if size1 != size2 {
        print_str("  FAIL: sizes differ (")
        print(size1)
        print_str(" vs ")
        print(size2)
        println_str(")")
        exit(1)
        0
    } else {
        print_str("       Size match: ")
        print(size1)
        println_str(" bytes")

        let mut mismatches = 0
        let mut i = 0
        while i < size1 {
            let b1 = array_get(bytes1, i)
            let b2 = array_get(bytes2, i)
            if b1 != b2 {
                mismatches = mismatches + 1
                if mismatches <= 5 {
                    print_str("  MISMATCH at offset ")
                    print(i)
                    print_str(": ")
                    print(b1)
                    print_str(" vs ")
                    println(b2)
                } else {
                    0
                }
            } else {
                0
            }
            i = i + 1
        }

        println_str("")
        if mismatches == 0 {
            println_str("==================================================")
            println_str("  PROOF 3 COMPLETE: DETERMINISTIC")
            println_str("  Two independent compilations produced")
            print_str("  IDENTICAL output: ")
            print(size1)
            println_str(" bytes, 0 mismatches")
            println_str("==================================================")
        } else {
            print_str("  FAIL: ")
            print(mismatches)
            println_str(" mismatches found")
        }
        0
    }
}
