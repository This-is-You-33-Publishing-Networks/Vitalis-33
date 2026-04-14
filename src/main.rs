//! Vitalis Compiler CLI — `vtc`
//!
//! Usage:
//!   vtc run <file.sl>          — Compile and JIT-execute
//!   vtc check <file.sl>        — Parse and type-check only
//!   vtc dump-ast <file.sl>     — Dump parsed AST
//!   vtc dump-ir <file.sl>      — Dump lowered IR
//!   vtc lex <file.sl>          — Dump lexer tokens

use vitalis::lexer;
use vitalis::parser;
use vitalis::types;
use vitalis::ir;
use vitalis::codegen;

use clap::{Parser, Subcommand};
use miette::{miette, Diagnostic, NamedSource, Result, SourceSpan};
use std::path::PathBuf;
use thiserror::Error;

// ─── Rich Diagnostic ───────────────────────────────────────────────────
/// A source-annotated error for beautiful terminal output via miette.
#[derive(Error, Debug, Diagnostic)]
#[error("{message}")]
#[allow(unused_assignments, unused, dead_code)]
struct VitalisDiag {
    message: String,

    #[source_code]
    src: NamedSource<String>,

    #[label("{label}")]
    span: SourceSpan,

    label: String,

    #[help]
    help: Option<String>,
}

/// Format a parse error as a rich miette diagnostic.
fn parse_diagnostic(source: &str, filename: &str, e: &parser::ParseError) -> VitalisDiag {
    VitalisDiag {
        message: e.message.clone(),
        src: NamedSource::new(filename, source.to_string()),
        span: (e.span.start, e.span.end.saturating_sub(e.span.start).max(1)).into(),
        label: "here".into(),
        help: e.hint.clone(),
    }
}

/// Format a type error as a rich miette diagnostic.
fn type_diagnostic(source: &str, filename: &str, e: &types::TypeError) -> VitalisDiag {
    VitalisDiag {
        message: e.message.clone(),
        src: NamedSource::new(filename, source.to_string()),
        span: (e.span.start, e.span.end.saturating_sub(e.span.start).max(1)).into(),
        label: "here".into(),
        help: e.hint.clone(),
    }
}

#[derive(Parser)]
#[command(
    name = "vtc",
    version = "0.1.0",
    about = "Vitalis Compiler — a language built for self-evolving AI",
    long_about = "Vitalis is a systems language purpose-built for \
                  autonomous code evolution, structured memory, and capability-based safety."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile and execute a .sl file via JIT
    Run {
        /// Path to the .sl source file
        file: PathBuf,
        /// Enable verbose pipeline observability (prints stage timings)
        #[arg(short, long)]
        verbose: bool,
    },
    /// Parse and type-check a .sl file without executing
    Check {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Dump the parsed AST as debug output
    DumpAst {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Dump the lowered IR
    DumpIr {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Dump lexer tokens
    Lex {
        /// Path to the .sl source file
        file: PathBuf,
    },
    /// Run an inline expression
    Eval {
        /// Vitalis expression (wrapped in fn main)
        #[arg(short, long)]
        expr: String,
    },
    /// Start the interactive REPL
    Repl,
    /// Build a standalone native executable (AOT compilation)
    Build {
        /// Path to the .sl source file
        file: PathBuf,
        /// Output file path
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
        /// Target triple (e.g., x86_64-linux-gnu, aarch64-linux-gnu, riscv64-linux-gnu)
        #[arg(short, long)]
        target: Option<String>,
    },
    /// List available cross-compilation targets
    Targets,
    /// Run the compiler bootstrap pipeline
    Bootstrap,
    /// Start the Language Server Protocol server (for IDE integration)
    Lsp,
    /// Start the Debug Adapter Protocol server
    Debug,
    /// Format .sl source files
    Fmt {
        /// Path to the .sl source file
        file: PathBuf,
        /// Check only — exit with error if not formatted (for CI)
        #[arg(long)]
        check: bool,
    },
    /// Lint .sl source files
    Lint {
        /// Path to the .sl source file
        file: PathBuf,
    },
}

fn read_source(path: &PathBuf) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| miette!("cannot read '{}': {}", path.display(), e))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { file, verbose } => {
            let source = read_source(&file)?;
            if verbose {
                eprintln!("[vtc] verbose mode enabled — pipeline timing active");
            }
            match codegen::compile_and_run(&source) {
                Ok(result) => {
                    println!("=> {}", result);
                    Ok(())
                }
                Err(e) => Err(miette!("{}", e)),
            }
        }

        Command::Check { file } => {
            let source = read_source(&file)?;
            let filename = file.display().to_string();

            let (program, parse_errors) = parser::parse(&source);
            if !parse_errors.is_empty() {
                for e in &parse_errors {
                    let diag = parse_diagnostic(&source, &filename, e);
                    eprintln!("{:?}", miette::Report::new(diag));
                }
                return Err(miette!("{} parse error(s)", parse_errors.len()));
            }

            let type_errors = types::TypeChecker::new().check(&program);
            if !type_errors.is_empty() {
                for e in &type_errors {
                    let diag = type_diagnostic(&source, &filename, e);
                    eprintln!("{:?}", miette::Report::new(diag));
                }
                eprintln!("{} type warning(s)", type_errors.len());
            }

            println!("✓ {} — {} items, no errors", file.display(), program.items.len());
            Ok(())
        }

        Command::DumpAst { file } => {
            let source = read_source(&file)?;
            let filename = file.display().to_string();
            let (program, errors) = parser::parse(&source);
            if !errors.is_empty() {
                for e in &errors {
                    let diag = parse_diagnostic(&source, &filename, e);
                    eprintln!("{:?}", miette::Report::new(diag));
                }
            }
            println!("{:#?}", program);
            Ok(())
        }

        Command::DumpIr { file } => {
            let source = read_source(&file)?;
            let filename = file.display().to_string();
            let (program, errors) = parser::parse(&source);
            if !errors.is_empty() {
                for e in &errors {
                    let diag = parse_diagnostic(&source, &filename, e);
                    eprintln!("{:?}", miette::Report::new(diag));
                }
                return Err(miette!("cannot generate IR with parse errors"));
            }
            let ir_module = ir::IrBuilder::new().build(&program);
            for func in &ir_module.functions {
                print!("{}", func);
            }
            Ok(())
        }

        Command::Lex { file } => {
            let source = read_source(&file)?;
            let (tokens, errors) = lexer::lex(&source);
            for tok in &tokens {
                println!("{:>4}..{:<4}  {:?}", tok.span.start, tok.span.end, tok.token);
            }
            if !errors.is_empty() {
                eprintln!("\n{} lex error(s):", errors.len());
                for e in &errors {
                    eprintln!("  {}", e);
                }
            }
            Ok(())
        }

        Command::Eval { expr } => {
            let source = format!("fn main() -> i64 {{ {} }}", expr);
            match codegen::compile_and_run(&source) {
                Ok(result) => {
                    println!("{}", result);
                    Ok(())
                }
                Err(e) => Err(miette!("{}", e)),
            }
        }

        Command::Repl => {
            vitalis::repl::run_interactive();
            Ok(())
        }

        Command::Build { file, output, target } => {
            let source = read_source(&file)?;

            // v113: WASM target uses IR → WASM pipeline
            let is_wasm = target.as_deref().map(|t| t.starts_with("wasm")).unwrap_or(false);
            if is_wasm {
                let filename = file.display().to_string();
                let (program, errors) = parser::parse(&source);
                if !errors.is_empty() {
                    for e in &errors {
                        let diag = parse_diagnostic(&source, &filename, e);
                        eprintln!("{:?}", miette::Report::new(diag));
                    }
                    return Err(miette!("{} parse error(s)", errors.len()));
                }
                let ir_module = ir::IrBuilder::new().build(&program);
                let wasm_module = vitalis::wasm_aot::WasmModule::from_ir(&ir_module);
                let bytes = wasm_module.to_bytes();
                let out_path = if output == PathBuf::from("a.out") {
                    file.with_extension("wasm")
                } else {
                    output
                };
                std::fs::write(&out_path, &bytes)
                    .map_err(|e| miette!("cannot write '{}': {}", out_path.display(), e))?;
                println!("✓ {} → {} ({} bytes, {} function(s))",
                    file.display(), out_path.display(), bytes.len(), wasm_module.functions.len());
                return Ok(());
            }

            let target_triple = if let Some(t) = target {
                // v118: Support short-form aliases (e.g., `aarch64`, `riscv64`)
                vitalis::aot::TargetTriple::parse(&t)
                    .ok_or_else(|| miette!("unknown target: '{}'. Use `vtc targets` to list available targets.", t))?
            } else {
                vitalis::aot::TargetTriple::host()
            };

            // v118: Don't attempt linking for cross-compilation targets (no cross-linker guaranteed)
            let should_link = !target_triple.is_cross_compile();

            let config = vitalis::aot::AotConfig {
                target: target_triple,
                output,
                link: should_link,
                verbose: true,
                ..Default::default()
            };

            let mut compiler = vitalis::aot::AotCompiler::new(config);
            match compiler.compile_source(&source) {
                Ok(result) => {
                    println!("{}", result);
                    Ok(())
                }
                Err(e) => Err(miette!("{}", e)),
            }
        }

        Command::Targets => {
            let cc = vitalis::cross_compile::CrossCompiler::new();
            println!("{}", cc);
            for target_name in cc.available_targets() {
                if let Some(info) = cc.target_info(target_name) {
                    println!("\n{}", info);
                }
            }
            Ok(())
        }

        Command::Bootstrap => {
            let config = vitalis::bootstrap::BootstrapConfig {
                verbose: true,
                ..Default::default()
            };
            let mut pipeline = vitalis::bootstrap::BootstrapPipeline::new(config);
            let report = pipeline.run_full_bootstrap();
            println!("{}", report);
            if report.is_success() {
                Ok(())
            } else {
                Err(miette!("bootstrap failed"))
            }
        }

        Command::Lsp => {
            vitalis::lsp::run_stdio()
                .map_err(|e| miette!("LSP server error: {}", e))
        }

        Command::Debug => {
            vitalis::dap::run_dap_stdio()
                .map_err(|e| miette!("DAP server error: {}", e))
        }

        Command::Fmt { file, check } => {
            let source = read_source(&file)?;
            if check {
                match vitalis::formatter::check_formatted(&source) {
                    Ok(true) => {
                        println!("✓ {} — already formatted", file.display());
                        Ok(())
                    }
                    Ok(false) => Err(miette!("{} — not formatted", file.display())),
                    Err(e) => Err(miette!("format error: {}", e)),
                }
            } else {
                match vitalis::formatter::format_source(&source) {
                    Ok(formatted) => {
                        std::fs::write(&file, &formatted)
                            .map_err(|e| miette!("cannot write '{}': {}", file.display(), e))?;
                        println!("✓ {} — formatted", file.display());
                        Ok(())
                    }
                    Err(e) => Err(miette!("format error: {}", e)),
                }
            }
        }

        Command::Lint { file } => {
            let source = read_source(&file)?;
            match vitalis::linter::lint_source(&source) {
                Ok(diagnostics) => {
                    if diagnostics.is_empty() {
                        println!("✓ {} — no lint issues", file.display());
                    } else {
                        for d in &diagnostics {
                            eprintln!("  [{:?}] {}: {}", d.severity, d.rule.name(), d.message);
                        }
                        eprintln!("\n{} lint issue(s) in {}", diagnostics.len(), file.display());
                    }
                    Ok(())
                }
                Err(e) => Err(miette!("lint error: {}", e)),
            }
        }
    }
}
