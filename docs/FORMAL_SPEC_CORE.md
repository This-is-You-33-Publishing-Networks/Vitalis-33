# Formal Spec Core (v91)

This document defines machine-checkable invariants for parser/type/IR layers and ties
those invariants to executable property-style tests in `src/formal_spec_core.rs`.

## Parser Invariants

- Program entrypoint must not contain duplicate top-level symbol declarations.
- Lexed token stream consumed by parser must be finite and deterministic for identical input.
- Expression nesting depth used for parse guards must remain within configured limits.

## Type Invariants

- Type preservation for assignment-like rewrites: source type equals destination type.
- `Never` is bottom: it is compatible with all target types but not vice versa unless equal.
- Tuple arity must be stable through inference round trips.

## IR Invariants

- SSA values are single-assignment within a basic block schedule.
- All branch targets must reference existing blocks.
- Instruction type signatures must match operand arity and expected output type.

## Executable Property Harness

The executable harness validates:

- Deterministic spec digest generation from sorted invariant declarations.
- Property sweeps over synthetic parser/type/IR specimens.
- Regression checks for violation detection and witness extraction.

## Verification Gate

- Module-level gate: `cargo test --release formal_spec_core::tests`
- Global gate: `cargo test --release` (subject to known linker-environment test exception)
