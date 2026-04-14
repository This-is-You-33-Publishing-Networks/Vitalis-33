# Vitalis Language Guide

A comprehensive reference for the Vitalis programming language.

## Table of Contents

1. [Overview](#overview)
2. [Types](#types)
3. [Variables](#variables)
4. [Functions](#functions)
5. [Control Flow](#control-flow)
6. [Pattern Matching](#pattern-matching)
7. [Structs](#structs)
8. [Enums](#enums)
9. [Traits and Impl Blocks](#traits-and-impl-blocks)
10. [Closures and Lambdas](#closures-and-lambdas)
11. [Modules and Imports](#modules-and-imports)
12. [Error Handling](#error-handling)
13. [Operators](#operators)
14. [Pipe Operator](#pipe-operator)
15. [Arrays](#arrays)
16. [Constants and Type Aliases](#constants-and-type-aliases)
17. [Async and Concurrency](#async-and-concurrency)
18. [Standard Library](#standard-library)
19. [Code Evolution](#code-evolution)
20. [CLI Reference](#cli-reference)

---

## Overview

Vitalis is a statically-typed, JIT-compiled language. Source files use the `.sl` extension.

Every program needs a `main` function that returns `i64`:

```rust
fn main() -> i64 {
    42
}
```

The return value of `main` is the program's exit code / result.

---

## Types

| Type | Description | Example |
|------|-------------|---------|
| `i64` | 64-bit signed integer | `42`, `-1`, `0` |
| `f64` | 64-bit floating point | `3.14`, `-0.5`, `1.0` |
| `bool` | Boolean | `true`, `false` |
| `string` | UTF-8 string | `"hello"` |

---

## Variables

Variables are declared with `let` and are immutable by default:

```rust
let x: i64 = 42;          // explicit type
let y = 3.14;              // type inferred as f64
let name: string = "Vitalis";
let active: bool = true;
```

---

## Functions

Functions are declared with `fn`, require typed parameters, and a return type:

```rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn greet(name: string) -> i64 {
    println_str(name);
    0
}
```

The last expression in a block is the implicit return value (no `return` keyword needed).

### Calling Functions

```rust
fn main() -> i64 {
    let result = add(20, 22);
    result
}
```

---

## Control Flow

### If/Else

`if/else` is an expression — it returns a value:

```rust
fn abs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}
```

### While Loops

```rust
fn sum_to(n: i64) -> i64 {
    let total = 0;
    let i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    }
    total
}
```

### For Loops

```rust
fn factorial(n: i64) -> i64 {
    let result = 1;
    for i in 1..n+1 {
        result = result * i;
    }
    result
}
```

---

## Structs

Define custom data types with named fields:

```rust
struct Point {
    x: i64,
    y: i64,
}

fn distance_sq(p: Point) -> i64 {
    p.x * p.x + p.y * p.y
}

fn main() -> i64 {
    let p = Point { x: 3, y: 4 };
    distance_sq(p)
}
```

---

## Enums

Define types with multiple variants:

```rust
enum Color {
    Red,
    Green,
    Blue,
}

fn color_code(c: Color) -> i64 {
    match c {
        Color::Red => 1,
        Color::Green => 2,
        Color::Blue => 3,
    }
}

fn main() -> i64 {
    color_code(Color::Green)
}
```

---

## Pattern Matching

The `match` expression provides exhaustive pattern matching:

```rust
fn describe(x: i64) -> i64 {
    match x {
        0 => 100,
        1 => 200,
        _ => 0,
    }
}
```

### Matching Enum Variants

```rust
enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rect(w, h) => w * h,
    }
}
```

### Literal and Wildcard Patterns

- Integer/float/string/bool literals match exact values.
- `_` is the wildcard pattern — matches anything.
- Identifiers in patterns bind the matched value.

---

## Traits and Impl Blocks

### Defining Traits

Traits declare a set of methods that types can implement:

```rust
trait Describe {
    fn label(self) -> i64;
}
```

Traits can include default implementations:

```rust
trait HasDefault {
    fn value(self) -> i64 { 42 }
}
```

### Impl Blocks

Implement methods on a struct:

```rust
struct Counter {
    count: i64,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }

    fn increment(self) -> i64 {
        self.count + 1
    }
}
```

Implement a trait for a struct:

```rust
impl Describe for Counter {
    fn label(self) -> i64 {
        self.count
    }
}
```

### Method Calls

Methods are called with dot syntax:

```rust
fn main() -> i64 {
    let c = Counter::new();
    c.increment()
}
```

---

## Closures and Lambdas

Lambda expressions create anonymous functions using `|params| body` syntax:

```rust
fn main() -> i64 {
    let double = |x: i64| -> i64 { x * 2 };
    double(21)
}
```

Lambdas can capture variables from their enclosing scope:

```rust
fn main() -> i64 {
    let offset = 10;
    let add_offset = |x: i64| -> i64 { x + offset };
    add_offset(32)
}
```

Lambdas work with higher-order functions:

```rust
fn apply(f: fn(i64) -> i64, x: i64) -> i64 {
    f(x)
}

fn main() -> i64 {
    let triple = |x: i64| -> i64 { x * 3 };
    apply(triple, 14)
}
```

---

## Modules and Imports

### Defining Modules

Group related code into modules:

```rust
module math {
    fn add(a: i64, b: i64) -> i64 { a + b }
    fn mul(a: i64, b: i64) -> i64 { a * b }
}

fn main() -> i64 {
    math::add(20, 22)
}
```

### Imports

Import items from other modules or files:

```rust
import std::io;
import math::vector;
```

### Visibility

Use `pub` to make items visible outside their module:

```rust
module utils {
    pub fn helper() -> i64 { 1 }
    fn internal() -> i64 { 2 }  // private
}
```

---

## Error Handling

### Try/Catch

Handle errors with `try/catch` blocks:

```rust
fn safe_divide(a: i64, b: i64) -> i64 {
    try {
        if b == 0 { throw "division by zero"; }
        a / b
    } catch e {
        0
    }
}
```

### Throw

Use `throw` to raise an error:

```rust
fn validate(x: i64) -> i64 {
    if x < 0 { throw "negative value"; }
    x
}
```

### Error Propagation

Use `?` to propagate errors to the caller:

```rust
fn process(x: i64) -> i64 {
    let value = validate(x)?;
    value * 2
}
```

---

## Arrays

Array literals use square brackets:

```rust
fn main() -> i64 {
    let arr = [10, 20, 30];
    arr[1]  // 20
}
```

---

## Constants and Type Aliases

### Constants

Declare compile-time constants:

```rust
const MAX_SIZE: i64 = 100;
const PI: f64 = 3.14159265;
```

### Type Aliases

Create shorthand names for types:

```rust
type Distance = f64;
type Callback = fn(i64) -> i64;
```

---

## Async and Concurrency

### Async/Await

```rust
async fn fetch_data() -> i64 {
    // async operation
    42
}

fn main() -> i64 {
    let result = await fetch_data();
    result
}
```

### Spawn

Launch concurrent tasks:

```rust
fn main() -> i64 {
    spawn(background_task());
    0
}
```

### Parallel Blocks

Execute multiple expressions concurrently:

```rust
fn main() -> i64 {
    parallel {
        task_a(),
        task_b(),
    }
}
```

---

## Operators

### Arithmetic
| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `a + b` |
| `-` | Subtraction | `a - b` |
| `*` | Multiplication | `a * b` |
| `/` | Division | `a / b` |
| `%` | Modulo | `a % b` |

### Comparison
| Operator | Description |
|----------|-------------|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less or equal |
| `>=` | Greater or equal |

### Logical
| Operator | Description |
|----------|-------------|
| `&&` | Logical AND |
| `\|\|` | Logical OR |
| `!` | Logical NOT |

### Bitwise
| Operator | Description |
|----------|-------------|
| `&` | Bitwise AND |
| `\|` | Bitwise OR |
| `^` | Bitwise XOR |
| `<<` | Left shift |
| `>>` | Right shift |

---

## Pipe Operator

Chain function calls with `|>`. The result of the left side is passed as the first argument to the right:

```rust
fn double(x: i64) -> i64 { x * 2 }
fn inc(x: i64) -> i64 { x + 1 }

fn main() -> i64 {
    5 |> double |> inc  // double(5) = 10, inc(10) = 11
}
```

This compiles to the same code as `inc(double(5))` but reads left-to-right.

---

## Standard Library

### I/O Functions
```rust
print(42);                // print i64
println(42);              // print i64 + newline
print_f64(3.14);         // print f64
println_f64(3.14);       // print f64 + newline
print_bool(true);        // print bool
println_bool(true);      // print bool + newline
print_str("hello");      // print string
println_str("hello");    // print string + newline
```

### Math Functions (f64)
```rust
sqrt(2.0)                // 1.4142...
sin(3.14)                // ~0.0
cos(0.0)                 // 1.0
exp(1.0)                 // 2.7182...
ln(2.718)                // ~1.0
pow(2.0, 10.0)           // 1024.0
floor(3.7)               // 3.0
ceil(3.2)                // 4.0
round(3.5)               // 4.0
abs_f64(-3.14)           // 3.14
min_f64(1.0, 2.0)        // 1.0
max_f64(1.0, 2.0)        // 2.0
```

### Math Functions (i64)
```rust
abs(-42)                 // 42
min(3, 7)                // 3
max(3, 7)                // 7
gcd(12, 8)               // 4
lcm(4, 6)                // 12
factorial(10)            // 3628800
fibonacci(10)            // 55
```

### AI Activation Functions
```rust
sigmoid(0.0)             // 0.5
relu(-1.0)               // 0.0
relu(2.0)                // 2.0
gelu(1.0)                // ~0.8413
tanh(1.0)                // ~0.7616
swish(1.0)               // ~0.7311
softplus(1.0)            // ~1.3133
leaky_relu(-1.0, 0.01)   // -0.01
elu(-1.0, 1.0)           // ~-0.6321
```

### Type Conversions
```rust
to_f64(42)               // 42.0
to_i64(3.7)              // 3
```

### String Operations
```rust
str_len("hello")         // 5
str_eq("a", "a")         // true
str_cat("hello ", "world") // "hello world"
```

### Numeric Utilities
```rust
lerp(0.0, 10.0, 0.5)    // 5.0
smoothstep(0.0, 1.0, 0.5) // smooth interpolation
clamp(5.0, 0.0, 3.0)    // 3.0
map_range(0.5, 0.0, 1.0, 0.0, 100.0) // 50.0
```

### Time & Random
```rust
clock_ns()               // nanosecond timestamp
clock_ms()               // millisecond timestamp
epoch_secs()             // Unix epoch seconds
rand_f64()               // random f64 in [0, 1)
rand_i64()               // random i64
```

### Assertions
```rust
assert_eq(2 + 2, 4);    // passes
assert_true(10 > 5);    // passes
```

---

## Code Evolution

The `@evolvable` annotation marks functions that can be mutated and evolved at runtime.

### Vitalis Source
```rust
@evolvable
fn strategy(x: i64) -> i64 {
    x * 2
}

fn main() -> i64 {
    strategy(21)
}
```

### Evolution via Python API
```python
import vitalis

# Register an evolvable function
vitalis.evo_register("strategy", """
@evolvable
fn strategy(x: i64) -> i64 { x * 2 }
""")

# Evolve to a new generation
vitalis.evo_evolve("strategy", """
@evolvable
fn strategy(x: i64) -> i64 { x * 3 }
""")

# Track fitness
vitalis.evo_set_fitness("strategy", 0.95)

# Rollback if needed
vitalis.evo_rollback("strategy", 1)  # back to generation 1
```

---

---

## CLI Reference

```bash
# Run a source file
vtc run examples/hello.sl

# Evaluate an expression
vtc eval -e "21 * 2"

# Type-check without executing
vtc check examples/arithmetic.sl

# Dump the AST
vtc dump-ast examples/structs.sl

# Dump the IR
vtc dump-ir examples/arithmetic.sl

# Tokenize
vtc lex examples/hello.sl

# Build a standalone native executable (AOT)
vtc build examples/hello.sl -o hello.exe

# Build for a different architecture
vtc build examples/hello.sl --target aarch64
vtc build examples/hello.sl --target riscv64

# Build to WebAssembly
vtc build examples/hello.sl --target wasm

# List available cross-compilation targets
vtc targets

# Start interactive REPL
vtc repl

# Format source code
vtc fmt examples/hello.sl
vtc fmt --check examples/hello.sl  # check without modifying

# Lint source code
vtc lint examples/hello.sl

# Start LSP server (for IDE integration)
vtc lsp

# Start Debug Adapter Protocol server
vtc debug

# Run the bootstrap self-compilation pipeline
vtc bootstrap
```

---

## What's Next?

See [CONTRIBUTING.md](../CONTRIBUTING.md) for how to add new features.  
See [EXTENDING.md](EXTENDING.md) for a guide on extending the compiler and stdlib.
