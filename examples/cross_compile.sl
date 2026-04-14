// Vitalis — Cross-compilation
//
// This example can be compiled for different architectures:
//
//   vtc build examples/cross_compile.sl --target aarch64
//   vtc build examples/cross_compile.sl --target riscv64
//   vtc build examples/cross_compile.sl --target wasm
//
// The same source code compiles to x86-64, AArch64, RISC-V, or WebAssembly.

fn fibonacci(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn main() -> i64 {
    fibonacci(20)  // 6765
}
