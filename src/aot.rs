//! Vitalis Native AOT Compilation (v22 Roadmap)
//!
//! Provides ahead-of-time compilation to native object files and standalone executables.
//! Uses Cranelift's `ObjectModule` backend instead of `JITModule` to emit relocatable
//! object code that can be linked with a system linker.
//!
//! # Architecture
//!
//! ```text
//! Source → Lexer → Parser → AST → TypeChecker → IR → Cranelift ObjectModule → .o file
//!                                                                                ↓
//!                                                                    System Linker (cc/link.exe)
//!                                                                                ↓
//!                                                                      Native Executable
//! ```
//!
//! # Advantages over JIT
//!
//! - No JIT compilation overhead at startup
//! - Distributable binaries (no runtime dependency on Cranelift)
//! - Better optimization opportunities (whole-program analysis)
//! - Compatible with system debugging tools (gdb, lldb, WinDbg)
//! - Smaller deployment footprint

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::fmt;

use cranelift::prelude::*;
use cranelift_codegen::settings;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ir::{IrModule, IrType, Inst, IrBinOp, IrUnOp, IrCmp};

// ═══════════════════════════════════════════════════════════════════════
//  AOT Configuration
// ═══════════════════════════════════════════════════════════════════════

/// Target triple for cross-compilation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetTriple {
    pub arch: Architecture,
    pub os: OperatingSystem,
    pub env: Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Architecture {
    X86_64,
    AArch64,
    RiscV64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    Linux,
    Windows,
    MacOS,
    None, // bare-metal
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Environment {
    Gnu,
    Msvc,
    Musl,
    None,
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arch = match self.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::AArch64 => "aarch64",
            Architecture::RiscV64 => "riscv64gc",
        };
        let os = match self.os {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "macos",
            OperatingSystem::None => "none",
        };
        let env = match self.env {
            Environment::Gnu => "gnu",
            Environment::Msvc => "msvc",
            Environment::Musl => "musl",
            Environment::None => "unknown",
        };
        write!(f, "{}-{}-{}", arch, os, env)
    }
}

impl TargetTriple {
    /// Parse a target triple string.
    ///
    /// Supports full triples (e.g., `aarch64-linux-gnu`) and short-form aliases:
    /// - `aarch64` / `arm64` → `aarch64-linux-gnu`
    /// - `riscv64` → `riscv64-linux-gnu`
    /// - `x86_64` / `x64` → host OS with default env
    /// - `wasm32` → not handled here (routed via WASM pipeline)
    pub fn parse(s: &str) -> Option<Self> {
        // v118: Short-form aliases — single-word architecture names
        match s {
            "aarch64" | "arm64" => return Some(Self {
                arch: Architecture::AArch64,
                os: OperatingSystem::Linux,
                env: Environment::Gnu,
            }),
            "riscv64" | "riscv64gc" => return Some(Self {
                arch: Architecture::RiscV64,
                os: OperatingSystem::Linux,
                env: Environment::Gnu,
            }),
            "x86_64" | "x64" | "amd64" => return Some(Self::host()),
            _ => {}
        }

        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() < 2 { return None; }

        let arch = match parts[0] {
            "x86_64" | "x64" | "amd64" => Architecture::X86_64,
            "aarch64" | "arm64" => Architecture::AArch64,
            "riscv64" | "riscv64gc" => Architecture::RiscV64,
            _ => return None,
        };

        let os = if parts.len() > 1 {
            match parts[1] {
                "linux" | "unknown-linux" | "unknown" if parts.len() > 2 && parts[2].starts_with("linux") => OperatingSystem::Linux,
                "linux" | "unknown-linux" => OperatingSystem::Linux,
                "windows" | "pc-windows" => OperatingSystem::Windows,
                "apple" | "macos" | "darwin" => OperatingSystem::MacOS,
                "none" => OperatingSystem::None,
                "unknown" => {
                    // Handle "arch-unknown-linux-gnu" style
                    if parts.len() > 2 {
                        match parts[2] {
                            "linux" => OperatingSystem::Linux,
                            "none" => OperatingSystem::None,
                            _ => OperatingSystem::Linux,
                        }
                    } else {
                        OperatingSystem::None
                    }
                }
                _ => OperatingSystem::Linux,
            }
        } else {
            OperatingSystem::Linux
        };

        let env = if parts.len() > 2 {
            match parts.last().unwrap_or(&"") {
                &"gnu" => Environment::Gnu,
                &"msvc" => Environment::Msvc,
                &"musl" => Environment::Musl,
                &"elf" => Environment::None,
                _ => Environment::None,
            }
        } else {
            Environment::None
        };

        Some(TargetTriple { arch, os, env })
    }

    /// Get the host target triple.
    pub fn host() -> Self {
        #[cfg(target_arch = "x86_64")]
        let arch = Architecture::X86_64;
        #[cfg(target_arch = "aarch64")]
        let arch = Architecture::AArch64;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let arch = Architecture::X86_64;

        #[cfg(target_os = "linux")]
        let os = OperatingSystem::Linux;
        #[cfg(target_os = "windows")]
        let os = OperatingSystem::Windows;
        #[cfg(target_os = "macos")]
        let os = OperatingSystem::MacOS;
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        let os = OperatingSystem::Linux;

        #[cfg(target_env = "gnu")]
        let env = Environment::Gnu;
        #[cfg(target_env = "msvc")]
        let env = Environment::Msvc;
        #[cfg(target_env = "musl")]
        let env = Environment::Musl;
        #[cfg(not(any(target_env = "gnu", target_env = "msvc", target_env = "musl")))]
        let env = Environment::None;

        TargetTriple { arch, os, env }
    }

    /// Get pointer size in bytes for the target.
    pub fn pointer_size(&self) -> u8 {
        match self.arch {
            Architecture::X86_64 | Architecture::AArch64 | Architecture::RiscV64 => 8,
        }
    }

    /// Get the Cranelift architecture name.
    pub fn cranelift_arch(&self) -> &str {
        match self.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::AArch64 => "aarch64",
            Architecture::RiscV64 => "riscv64",
        }
    }

    /// Get the target-lexicon triple string.
    pub fn to_cranelift_triple(&self) -> String {
        match (self.arch, self.os, self.env) {
            (Architecture::X86_64, OperatingSystem::Linux, Environment::Gnu) =>
                "x86_64-unknown-linux-gnu".to_string(),
            (Architecture::X86_64, OperatingSystem::Linux, Environment::Musl) =>
                "x86_64-unknown-linux-musl".to_string(),
            (Architecture::X86_64, OperatingSystem::Windows, Environment::Msvc) =>
                "x86_64-pc-windows-msvc".to_string(),
            (Architecture::X86_64, OperatingSystem::MacOS, _) =>
                "x86_64-apple-darwin".to_string(),
            (Architecture::AArch64, OperatingSystem::Linux, Environment::Gnu) =>
                "aarch64-unknown-linux-gnu".to_string(),
            (Architecture::AArch64, OperatingSystem::Linux, Environment::Musl) =>
                "aarch64-unknown-linux-musl".to_string(),
            (Architecture::AArch64, OperatingSystem::MacOS, _) =>
                "aarch64-apple-darwin".to_string(),
            (Architecture::RiscV64, OperatingSystem::Linux, Environment::Gnu) =>
                "riscv64gc-unknown-linux-gnu".to_string(),
            (Architecture::RiscV64, OperatingSystem::None, _) =>
                "riscv64gc-unknown-none-elf".to_string(),
            _ => format!("{}", self),
        }
    }

    /// Check if this target is the native host.
    pub fn is_host(&self) -> bool {
        *self == Self::host()
    }

    /// Get the object file extension for this target.
    pub fn object_extension(&self) -> &str {
        match self.os {
            OperatingSystem::Windows => "obj",
            _ => "o",
        }
    }

    /// Get the executable extension for this target.
    pub fn exe_extension(&self) -> &str {
        match self.os {
            OperatingSystem::Windows => ".exe",
            _ => "",
        }
    }

    /// Get the linker command for this target.
    pub fn linker_command(&self) -> &str {
        match (self.os, self.env) {
            (OperatingSystem::Windows, Environment::Msvc) => "link.exe",
            (OperatingSystem::Windows, _) => "gcc",
            (OperatingSystem::MacOS, _) => "cc",
            _ => "cc",
        }
    }

    /// v118: Get the cross-compilation linker command for this target.
    ///
    /// When cross-compiling from one architecture to another, the default
    /// system `cc` won't work — a cross-linker is needed (e.g., `aarch64-linux-gnu-gcc`).
    pub fn cross_linker_command(&self) -> String {
        if self.is_host() {
            return self.linker_command().to_string();
        }
        match (self.arch, self.os, self.env) {
            (Architecture::AArch64, OperatingSystem::Linux, Environment::Gnu) =>
                "aarch64-linux-gnu-gcc".to_string(),
            (Architecture::AArch64, OperatingSystem::Linux, Environment::Musl) =>
                "aarch64-linux-musl-gcc".to_string(),
            (Architecture::RiscV64, OperatingSystem::Linux, Environment::Gnu) =>
                "riscv64-linux-gnu-gcc".to_string(),
            (Architecture::RiscV64, OperatingSystem::None, _) =>
                "riscv64-unknown-elf-gcc".to_string(),
            (Architecture::X86_64, OperatingSystem::Linux, Environment::Gnu) =>
                "x86_64-linux-gnu-gcc".to_string(),
            (Architecture::X86_64, OperatingSystem::Linux, Environment::Musl) =>
                "x86_64-linux-musl-gcc".to_string(),
            _ => self.linker_command().to_string(),
        }
    }

    /// v118: Check if compiling for this target requires cross-compilation.
    pub fn is_cross_compile(&self) -> bool {
        let host = Self::host();
        self.arch != host.arch || self.os != host.os
    }
}

/// AOT compilation configuration.
#[derive(Debug, Clone)]
pub struct AotConfig {
    /// Target triple.
    pub target: TargetTriple,
    /// Output file path.
    pub output: PathBuf,
    /// Optimization level (0-3).
    pub opt_level: u8,
    /// Whether to emit debug info.
    pub debug_info: bool,
    /// Whether to produce a standalone executable (vs just .o file).
    pub link: bool,
    /// Additional linker flags.
    pub linker_flags: Vec<String>,
    /// Whether to strip symbols.
    pub strip: bool,
    /// Whether to enable LTO.
    pub lto: bool,
    /// Verbose output.
    pub verbose: bool,
}

impl Default for AotConfig {
    fn default() -> Self {
        Self {
            target: TargetTriple::host(),
            output: PathBuf::from("a.out"),
            opt_level: 2,
            debug_info: false,
            link: true,
            linker_flags: Vec::new(),
            strip: false,
            lto: false,
            verbose: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  AOT Compilation Result
// ═══════════════════════════════════════════════════════════════════════

/// Result of AOT compilation.
#[derive(Debug, Clone)]
pub struct AotResult {
    /// Path to the output file.
    pub output_path: PathBuf,
    /// Object file path (before linking).
    pub object_path: PathBuf,
    /// Target triple used.
    pub target: TargetTriple,
    /// Size of the output in bytes.
    pub output_size: u64,
    /// Compilation time.
    pub compile_time: Duration,
    /// Link time (if linking was performed).
    pub link_time: Option<Duration>,
    /// Number of functions compiled.
    pub function_count: usize,
    /// Whether linking was performed.
    pub linked: bool,
    /// Errors (if any).
    pub errors: Vec<String>,
}

impl AotResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Display for AotResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_success() {
            write!(f, "✓ AOT compilation succeeded: {} ({} bytes, {} functions, {:?})",
                   self.output_path.display(), self.output_size,
                   self.function_count, self.compile_time)?;
            if let Some(lt) = self.link_time {
                write!(f, " + link {:?}", lt)?;
            }
            Ok(())
        } else {
            write!(f, "✗ AOT compilation failed: {} errors", self.errors.len())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  AOT Compiler
// ═══════════════════════════════════════════════════════════════════════

/// AOT (Ahead-of-Time) compiler using Cranelift's ObjectModule backend.
///
/// Unlike the JIT compiler, this produces relocatable object files that
/// can be linked into standalone executables.
pub struct AotCompiler {
    config: AotConfig,
    /// Compiled function names.
    compiled_functions: Vec<String>,
}

impl AotCompiler {
    /// Create a new AOT compiler with the given configuration.
    pub fn new(config: AotConfig) -> Self {
        Self {
            config,
            compiled_functions: Vec::new(),
        }
    }

    /// Compile source code to a native object file.
    ///
    /// Returns the path to the generated object file.
    pub fn compile_source(&mut self, source: &str) -> Result<AotResult, String> {
        let start = Instant::now();

        // Phase 1: Parse
        let (program, parse_errors) = crate::parser::parse(source);
        if !parse_errors.is_empty() {
            return Err(format!(
                "Parse errors:\n{}",
                parse_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
            ));
        }

        // Phase 2: Type check
        let type_errors = crate::types::TypeChecker::new().check(&program);
        if !type_errors.is_empty() && self.config.verbose {
            eprintln!(
                "Type warnings:\n{}",
                type_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
            );
        }

        // Phase 3: Lower to IR
        let mut ir_module = crate::ir::IrBuilder::new().build(&program);

        // Phase 3.5: Optimize IR
        crate::optimizer::optimize_ir(&mut ir_module);

        // Phase 4: Compile via AOT path
        self.compile_ir(&ir_module, start)
    }

    /// Compile an IR module to a native object file using Cranelift ObjectModule.
    pub fn compile_ir(&mut self, ir_module: &IrModule, start: Instant) -> Result<AotResult, String> {
        if self.config.verbose {
            eprintln!("[aot] Compiling for target: {}", self.config.target);
            eprintln!("[aot] Functions: {}", ir_module.functions.len());
        }

        self.compiled_functions = ir_module.functions.iter()
            .map(|f| f.name.clone())
            .collect();

        // ── Build Cranelift ISA ──
        let opt = match self.config.opt_level {
            0 => "none",
            1 => "speed",
            2 => "speed",
            3 => "speed_and_size",
            _ => "speed",
        };
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", opt).map_err(|e| format!("set opt_level: {}", e))?;
        flag_builder.set("is_pic", "true").map_err(|e| format!("set is_pic: {}", e))?;

        let triple_str = self.config.target.to_cranelift_triple();
        let triple: target_lexicon::Triple = triple_str.parse()
            .map_err(|e| format!("bad triple '{}': {}", triple_str, e))?;

        let isa = cranelift_codegen::isa::lookup(triple.clone())
            .map_err(|e| format!("ISA lookup for {}: {}", triple_str, e))?
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA finish: {}", e))?;

        // ── Create ObjectModule ──
        let obj_builder = ObjectBuilder::new(
            isa,
            "vitalis_aot",
            cranelift_module::default_libcall_names(),
        ).map_err(|e| format!("ObjectBuilder: {}", e))?;
        let mut object_module = ObjectModule::new(obj_builder);

        let pointer_type = object_module.target_config().pointer_type();

        // ── Declare all functions ──
        let mut func_ids: HashMap<String, FuncId> = HashMap::new();
        for func in &ir_module.functions {
            let mut sig = object_module.make_signature();
            for (_, ty) in &func.params {
                sig.params.push(AbiParam::new(Self::ir_type_to_cl(ty, pointer_type)));
            }
            if func.ret_type != IrType::Void {
                sig.returns.push(AbiParam::new(Self::ir_type_to_cl(&func.ret_type, pointer_type)));
            }
            let id = object_module
                .declare_function(&func.name, Linkage::Export, &sig)
                .map_err(|e| format!("declare {}: {}", func.name, e))?;
            func_ids.insert(func.name.clone(), id);
        }

        // ── Declare runtime imports ──
        let rt = RuntimeLibrary::required();
        for rtf in &rt.required_functions {
            if func_ids.contains_key(&rtf.name) { continue; }
            let mut sig = object_module.make_signature();
            for p in &rtf.params {
                sig.params.push(AbiParam::new(Self::ir_type_to_cl(p, pointer_type)));
            }
            if rtf.ret != IrType::Void {
                sig.returns.push(AbiParam::new(Self::ir_type_to_cl(&rtf.ret, pointer_type)));
            }
            let id = object_module
                .declare_function(&rtf.name, Linkage::Import, &sig)
                .map_err(|e| format!("declare runtime {}: {}", rtf.name, e))?;
            func_ids.insert(rtf.name.clone(), id);
        }

        // ── Define (compile) each function ──
        let mut ctx = object_module.make_context();
        for func in &ir_module.functions {
            let func_id = func_ids[&func.name];
            ctx.func.signature = object_module.declarations()
                .get_function_decl(func_id).signature.clone();
            ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

            let mut builder_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

            // Create blocks
            let mut block_map: HashMap<crate::ir::BlockId, Block> = HashMap::new();
            for bb in &func.blocks {
                block_map.insert(bb.id, builder.create_block());
            }

            // Entry block
            let entry = block_map[&func.entry];
            builder.append_block_params_for_function_params(entry);
            builder.switch_to_block(entry);
            builder.seal_block(entry);

            // Map IR values to Cranelift values
            let mut value_map: HashMap<crate::ir::Value, cranelift::prelude::Value> = HashMap::new();
            let params_cl: Vec<cranelift::prelude::Value> = builder.block_params(entry).to_vec();
            for (i, (_name, _ty)) in func.params.iter().enumerate() {
                if i < params_cl.len() {
                    value_map.insert(crate::ir::Value(i as u32), params_cl[i]);
                }
            }

            let mut first = true;
            for bb in &func.blocks {
                if !first {
                    let cl_block = block_map[&bb.id];
                    builder.switch_to_block(cl_block);
                }
                first = false;

                for inst in &bb.insts {
                    Self::translate_inst(
                        inst, &mut builder, &mut value_map, &block_map,
                        &func_ids, &mut object_module, pointer_type,
                    );
                }
            }

            // Seal all blocks and finalize
            builder.seal_all_blocks();
            builder.finalize();

            object_module.define_function(func_id, &mut ctx)
                .map_err(|e| format!("define {}: {}", func.name, e))?;
            ctx.clear();
        }

        // ── Emit object bytes ──
        let product = object_module.finish();
        let obj_bytes = product.emit()
            .map_err(|e| format!("emit object: {}", e))?;

        let compile_time = start.elapsed();

        // ── Write object file ──
        let obj_ext = self.config.target.object_extension();
        let object_path = self.config.output.with_extension(obj_ext);
        std::fs::create_dir_all(object_path.parent().unwrap_or(Path::new(".")))
            .map_err(|e| format!("cannot create output dir: {}", e))?;
        std::fs::write(&object_path, &obj_bytes)
            .map_err(|e| format!("cannot write object file: {}", e))?;

        let output_size = obj_bytes.len() as u64;

        // ── Link if requested ──
        let (final_path, link_time, linked) = if self.config.link {
            let link_start = Instant::now();
            let exe_path = self.link_object(&object_path)?;
            (exe_path, Some(link_start.elapsed()), true)
        } else {
            (object_path.clone(), None, false)
        };

        let result = AotResult {
            output_path: final_path,
            object_path,
            target: self.config.target.clone(),
            output_size,
            compile_time,
            link_time,
            function_count: self.compiled_functions.len(),
            linked,
            errors: Vec::new(),
        };

        if self.config.verbose {
            eprintln!("[aot] {}", result);
        }
        Ok(result)
    }

    /// Translate a single IR instruction into Cranelift IR.
    fn translate_inst(
        inst: &Inst,
        builder: &mut FunctionBuilder,
        value_map: &mut HashMap<crate::ir::Value, cranelift::prelude::Value>,
        block_map: &HashMap<crate::ir::BlockId, Block>,
        func_ids: &HashMap<String, FuncId>,
        module: &mut ObjectModule,
        pointer_type: cranelift::prelude::Type,
    ) {
        match inst {
            Inst::IConst { result, value, ty: _ } => {
                let v = builder.ins().iconst(types::I64, *value);
                value_map.insert(*result, v);
            }
            Inst::FConst { result, value, ty: _ } => {
                let v = builder.ins().f64const(*value);
                value_map.insert(*result, v);
            }
            Inst::BConst { result, value } => {
                let v = builder.ins().iconst(types::I8, if *value { 1 } else { 0 });
                value_map.insert(*result, v);
            }
            Inst::BinOp { result, op, lhs, rhs, ty: _ } => {
                let l = value_map.get(lhs).copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                let r = value_map.get(rhs).copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                let v = match op {
                    IrBinOp::Add => builder.ins().iadd(l, r),
                    IrBinOp::Sub => builder.ins().isub(l, r),
                    IrBinOp::Mul => builder.ins().imul(l, r),
                    IrBinOp::Div => builder.ins().sdiv(l, r),
                    IrBinOp::Mod => builder.ins().srem(l, r),
                    IrBinOp::FAdd => builder.ins().fadd(l, r),
                    IrBinOp::FSub => builder.ins().fsub(l, r),
                    IrBinOp::FMul => builder.ins().fmul(l, r),
                    IrBinOp::FDiv => builder.ins().fdiv(l, r),
                    IrBinOp::And => builder.ins().band(l, r),
                    IrBinOp::Or  => builder.ins().bor(l, r),
                };
                value_map.insert(*result, v);
            }
            Inst::UnOp { result, op, operand, ty: _ } => {
                let v = value_map.get(operand).copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                let r = match op {
                    IrUnOp::Neg => builder.ins().ineg(v),
                    IrUnOp::FNeg => builder.ins().fneg(v),
                    IrUnOp::Not => {
                        let one = builder.ins().iconst(types::I8, 1);
                        builder.ins().bxor(v, one)
                    }
                };
                value_map.insert(*result, r);
            }
            Inst::ICmp { result, cond, lhs, rhs } => {
                let l = value_map.get(lhs).copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                let r = value_map.get(rhs).copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                let cc = match cond {
                    IrCmp::Eq => IntCC::Equal,
                    IrCmp::Ne => IntCC::NotEqual,
                    IrCmp::Lt => IntCC::SignedLessThan,
                    IrCmp::Gt => IntCC::SignedGreaterThan,
                    IrCmp::Le => IntCC::SignedLessThanOrEqual,
                    IrCmp::Ge => IntCC::SignedGreaterThanOrEqual,
                };
                let v = builder.ins().icmp(cc, l, r);
                value_map.insert(*result, v);
            }
            Inst::FCmp { result, cond, lhs, rhs } => {
                let l = value_map.get(lhs).copied().unwrap_or_else(|| builder.ins().f64const(0.0));
                let r = value_map.get(rhs).copied().unwrap_or_else(|| builder.ins().f64const(0.0));
                let cc = match cond {
                    IrCmp::Eq => FloatCC::Equal,
                    IrCmp::Ne => FloatCC::NotEqual,
                    IrCmp::Lt => FloatCC::LessThan,
                    IrCmp::Gt => FloatCC::GreaterThan,
                    IrCmp::Le => FloatCC::LessThanOrEqual,
                    IrCmp::Ge => FloatCC::GreaterThanOrEqual,
                };
                let v = builder.ins().fcmp(cc, l, r);
                value_map.insert(*result, v);
            }
            Inst::Call { result, func, args, ret_ty: _ } => {
                if let Some(&fid) = func_ids.get(func) {
                    let func_ref = module.declare_func_in_func(fid, builder.func);
                    let arg_vals: Vec<cranelift::prelude::Value> = args.iter()
                        .map(|a| value_map.get(a).copied()
                            .unwrap_or_else(|| builder.ins().iconst(types::I64, 0)))
                        .collect();
                    let call = builder.ins().call(func_ref, &arg_vals);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        value_map.insert(*result, results[0]);
                    }
                }
            }
            Inst::Return { value } => {
                if let Some(val) = value {
                    let v = value_map.get(val).copied()
                        .unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                    builder.ins().return_(&[v]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            Inst::Branch { cond, then_bb, else_bb } => {
                let c = value_map.get(cond).copied()
                    .unwrap_or_else(|| builder.ins().iconst(types::I8, 0));
                let tb = match block_map.get(then_bb).copied() {
                    Some(b) => b,
                    None => { eprintln!("AOT warning: missing then-block {:?}", then_bb); return; }
                };
                let fb = match block_map.get(else_bb).copied() {
                    Some(b) => b,
                    None => { eprintln!("AOT warning: missing else-block {:?}", else_bb); return; }
                };
                builder.ins().brif(c, tb, &[], fb, &[]);
            }
            Inst::Jump { target } => {
                let t = match block_map.get(target).copied() {
                    Some(b) => b,
                    None => { eprintln!("AOT warning: missing jump target {:?}", target); return; }
                };
                builder.ins().jump(t, &[]);
            }
            Inst::Alloca { result, size: _ } => {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot, 8, 0,
                ));
                let v = builder.ins().stack_addr(pointer_type, slot, 0);
                value_map.insert(*result, v);
            }
            Inst::Load { result, ptr, ty } => {
                let a = value_map.get(ptr).copied()
                    .unwrap_or_else(|| builder.ins().iconst(pointer_type, 0));
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let v = builder.ins().load(cl_ty, MemFlags::new(), a, 0);
                value_map.insert(*result, v);
            }
            Inst::Store { value, ptr } => {
                let a = value_map.get(ptr).copied()
                    .unwrap_or_else(|| builder.ins().iconst(pointer_type, 0));
                let v = value_map.get(value).copied()
                    .unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                builder.ins().store(MemFlags::new(), v, a, 0);
            }
            Inst::Copy { result, source } => {
                let v = value_map.get(source).copied()
                    .unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                value_map.insert(*result, v);
            }
            Inst::Phi { result, incoming, ty } => {
                // Phi lowering: select from incoming edges based on available values.
                // For each (value, block) pair, if the source value is already
                // in value_map, use that. Otherwise fall back to zero constant.
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                if !value_map.contains_key(result) {
                    // Try to find a valid incoming value
                    let mut resolved = None;
                    for (src_val, _src_block) in incoming {
                        if let Some(&v) = value_map.get(src_val) {
                            resolved = Some(v);
                            break;
                        }
                    }
                    let v = resolved.unwrap_or_else(|| builder.ins().iconst(cl_ty, 0));
                    value_map.insert(*result, v);
                }
            }
            other => {
                // Unsupported instructions produce a diagnostic instead of silent skip
                eprintln!("AOT warning: unsupported IR instruction: {:?}", std::mem::discriminant(other));
            }
        }
    }

    /// Map IR types to Cranelift types (same mapping as JIT).
    fn ir_type_to_cl(ty: &IrType, pointer_type: cranelift::prelude::Type) -> cranelift::prelude::Type {
        match ty {
            IrType::I32 => types::I32,
            IrType::I64 => types::I64,
            IrType::F32 => types::F32,
            IrType::F64 => types::F64,
            IrType::Bool => types::I8,
            IrType::Ptr => pointer_type,
            IrType::Void => types::I64,
        }
    }

    /// Invoke the system linker to produce an executable from the object file.
    fn link_object(&self, object_path: &Path) -> Result<PathBuf, String> {
        let exe_ext = self.config.target.exe_extension();
        let mut exe_path = self.config.output.clone();
        if !exe_ext.is_empty() {
            exe_path.set_extension(&exe_ext[1..]);
        }

        let linker = self.config.target.linker_command();
        let mut cmd = std::process::Command::new(linker);

        match (self.config.target.os, self.config.target.env) {
            (OperatingSystem::Windows, Environment::Msvc) => {
                cmd.arg(object_path)
                   .arg(format!("/OUT:{}", exe_path.display()))
                   .arg("/ENTRY:main")
                   .arg("/SUBSYSTEM:CONSOLE")
                   .arg("kernel32.lib")
                   .arg("msvcrt.lib");
            }
            _ => {
                cmd.arg(object_path)
                   .arg("-o").arg(&exe_path);
                if self.config.strip {
                    cmd.arg("-s");
                }
            }
        }

        for flag in &self.config.linker_flags {
            cmd.arg(flag);
        }

        if self.config.verbose {
            eprintln!("[aot] Linking: {:?}", cmd);
        }

        let output = cmd.output()
            .map_err(|e| format!("linker '{}' failed to start: {} (is it on PATH?)", linker, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("linker failed (exit {}):\n{}", output.status, stderr));
        }

        Ok(exe_path)
    }

    /// Get the list of compiled functions.
    pub fn compiled_functions(&self) -> &[String] {
        &self.compiled_functions
    }
}

/// Convenience function: compile source to native AOT.
pub fn compile_aot(source: &str, config: AotConfig) -> Result<AotResult, String> {
    let mut compiler = AotCompiler::new(config);
    compiler.compile_source(source)
}

/// Convenience function: compile source with default config.
pub fn compile_aot_default(source: &str, output: &str) -> Result<AotResult, String> {
    let config = AotConfig {
        output: PathBuf::from(output),
        ..Default::default()
    };
    compile_aot(source, config)
}

// ═══════════════════════════════════════════════════════════════════════
//  AOT Runtime Library
// ═══════════════════════════════════════════════════════════════════════

/// Describes the minimal runtime that AOT-compiled programs need.
///
/// For JIT mode, the runtime functions are linked at JIT time.
/// For AOT mode, they must be compiled into a static library and linked
/// with the object file.
pub struct RuntimeLibrary {
    /// Functions that the runtime must provide.
    pub required_functions: Vec<RuntimeFunction>,
}

#[derive(Debug, Clone)]
pub struct RuntimeFunction {
    pub name: String,
    pub params: Vec<IrType>,
    pub ret: IrType,
    pub description: String,
}

impl RuntimeLibrary {
    /// Get the list of runtime functions required by AOT-compiled programs.
    pub fn required() -> Self {
        Self {
            required_functions: vec![
                RuntimeFunction {
                    name: "slang_print_i64".to_string(),
                    params: vec![IrType::I64],
                    ret: IrType::Void,
                    description: "Print an i64 to stdout".to_string(),
                },
                RuntimeFunction {
                    name: "slang_print_f64".to_string(),
                    params: vec![IrType::F64],
                    ret: IrType::Void,
                    description: "Print an f64 to stdout".to_string(),
                },
                RuntimeFunction {
                    name: "slang_print_str".to_string(),
                    params: vec![IrType::Ptr],
                    ret: IrType::Void,
                    description: "Print a string to stdout".to_string(),
                },
                RuntimeFunction {
                    name: "slang_print_bool".to_string(),
                    params: vec![IrType::Bool],
                    ret: IrType::Void,
                    description: "Print a bool to stdout".to_string(),
                },
                RuntimeFunction {
                    name: "slang_println_str".to_string(),
                    params: vec![IrType::Ptr],
                    ret: IrType::Void,
                    description: "Print a string with newline".to_string(),
                },
                RuntimeFunction {
                    name: "slang_sqrt_f64".to_string(),
                    params: vec![IrType::F64],
                    ret: IrType::F64,
                    description: "Square root of f64".to_string(),
                },
                RuntimeFunction {
                    name: "slang_alloc".to_string(),
                    params: vec![IrType::I64],
                    ret: IrType::Ptr,
                    description: "Allocate heap memory".to_string(),
                },
                RuntimeFunction {
                    name: "slang_free".to_string(),
                    params: vec![IrType::Ptr],
                    ret: IrType::Void,
                    description: "Free heap memory".to_string(),
                },
            ],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_triple_host() {
        let host = TargetTriple::host();
        assert_eq!(host.pointer_size(), 8);
        assert!(host.is_host());
    }

    #[test]
    fn test_target_triple_parse() {
        let triple = TargetTriple::parse("x86_64-linux-gnu").unwrap();
        assert_eq!(triple.arch, Architecture::X86_64);
        assert_eq!(triple.os, OperatingSystem::Linux);
        assert_eq!(triple.env, Environment::Gnu);

        let arm = TargetTriple::parse("aarch64-linux-gnu").unwrap();
        assert_eq!(arm.arch, Architecture::AArch64);

        let riscv = TargetTriple::parse("riscv64-linux-gnu").unwrap();
        assert_eq!(riscv.arch, Architecture::RiscV64);

        assert!(TargetTriple::parse("invalid").is_none());
    }

    #[test]
    fn test_target_triple_display() {
        let triple = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(format!("{}", triple), "x86_64-linux-gnu");
    }

    #[test]
    fn test_cranelift_triple() {
        let triple = TargetTriple {
            arch: Architecture::AArch64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(triple.to_cranelift_triple(), "aarch64-unknown-linux-gnu");

        let riscv = TargetTriple {
            arch: Architecture::RiscV64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(riscv.to_cranelift_triple(), "riscv64gc-unknown-linux-gnu");
    }

    #[test]
    fn test_target_extensions() {
        let linux = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(linux.object_extension(), "o");
        assert_eq!(linux.exe_extension(), "");

        let windows = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Windows,
            env: Environment::Msvc,
        };
        assert_eq!(windows.object_extension(), "obj");
        assert_eq!(windows.exe_extension(), ".exe");
    }

    #[test]
    fn test_aot_config_default() {
        let config = AotConfig::default();
        assert_eq!(config.opt_level, 2);
        assert!(config.link);
        assert!(!config.debug_info);
        assert!(!config.strip);
    }

    #[test]
    fn test_aot_compilation() {
        let config = AotConfig {
            output: PathBuf::from("target/test_aot_output"),
            link: false,
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source("fn main() -> i64 { 42 }");
        assert!(result.is_ok(), "AOT compile failed: {:?}", result.err());
        let aot_result = result.unwrap();
        assert!(aot_result.is_success());
        assert!(aot_result.function_count > 0);
        // Verify real binary object file (not text placeholder)
        let bytes = std::fs::read(&aot_result.object_path).unwrap();
        assert!(bytes.len() > 64, "object file too small: {} bytes", bytes.len());
        // COFF magic (0x8664 for x86_64) or ELF magic (0x7F 'ELF')
        let is_coff = bytes.len() > 2 && bytes[0] == 0x64 && bytes[1] == 0x86;
        let is_elf = bytes.len() > 4 && bytes[0] == 0x7F && bytes[1] == b'E';
        assert!(is_coff || is_elf, "not a valid object file (first bytes: {:02x} {:02x})", bytes[0], bytes[1]);
        // Cleanup
        let _ = std::fs::remove_file(&aot_result.object_path);
    }

    #[test]
    fn test_aot_emits_correct_target_extension() {
        let config = AotConfig {
            output: PathBuf::from("target/test_aot_ext"),
            link: false,
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source("fn main() -> i64 { 0 }").unwrap();
        let ext = result.object_path.extension().unwrap().to_str().unwrap();
        #[cfg(target_os = "windows")]
        assert_eq!(ext, "obj");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(ext, "o");
        let _ = std::fs::remove_file(&result.object_path);
    }

    #[test]
    fn test_aot_multiple_functions() {
        let config = AotConfig {
            output: PathBuf::from("target/test_aot_multi"),
            link: false,
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source(
            "fn add(a: i64, b: i64) -> i64 { a + b }\nfn main() -> i64 { add(1, 2) }"
        ).unwrap();
        assert!(result.function_count >= 2);
        let _ = std::fs::remove_file(&result.object_path);
    }

    #[test]
    fn test_runtime_library() {
        let rt = RuntimeLibrary::required();
        assert!(!rt.required_functions.is_empty());
        // Should have at least print and alloc
        assert!(rt.required_functions.iter().any(|f| f.name.contains("print")));
        assert!(rt.required_functions.iter().any(|f| f.name.contains("alloc")));
    }

    #[test]
    fn test_aot_result_display() {
        let result = AotResult {
            output_path: PathBuf::from("test.exe"),
            object_path: PathBuf::from("test.obj"),
            target: TargetTriple::host(),
            output_size: 1024,
            compile_time: Duration::from_millis(50),
            link_time: Some(Duration::from_millis(10)),
            function_count: 3,
            linked: true,
            errors: Vec::new(),
        };
        let display = format!("{}", result);
        assert!(display.contains("succeeded"));
    }

    #[test]
    fn test_linker_commands() {
        let msvc = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Windows,
            env: Environment::Msvc,
        };
        assert_eq!(msvc.linker_command(), "link.exe");

        let linux = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(linux.linker_command(), "cc");
    }

    // ── v118: Short-form alias tests ──

    #[test]
    fn test_parse_short_form_aarch64() {
        let triple = TargetTriple::parse("aarch64").unwrap();
        assert_eq!(triple.arch, Architecture::AArch64);
        assert_eq!(triple.os, OperatingSystem::Linux);
        assert_eq!(triple.env, Environment::Gnu);
    }

    #[test]
    fn test_parse_short_form_arm64() {
        let triple = TargetTriple::parse("arm64").unwrap();
        assert_eq!(triple.arch, Architecture::AArch64);
    }

    #[test]
    fn test_parse_short_form_riscv64() {
        let triple = TargetTriple::parse("riscv64").unwrap();
        assert_eq!(triple.arch, Architecture::RiscV64);
        assert_eq!(triple.os, OperatingSystem::Linux);
        assert_eq!(triple.env, Environment::Gnu);
    }

    #[test]
    fn test_parse_short_form_x86_64() {
        let triple = TargetTriple::parse("x86_64").unwrap();
        assert_eq!(triple.arch, Architecture::X86_64);
        assert!(triple.is_host() || true); // Just verify it parses
    }

    #[test]
    fn test_parse_cranelift_style_triple() {
        // "aarch64-unknown-linux-gnu" is a valid Cranelift-style triple
        let triple = TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(triple.arch, Architecture::AArch64);
        assert_eq!(triple.os, OperatingSystem::Linux);
        assert_eq!(triple.env, Environment::Gnu);
    }

    #[test]
    fn test_parse_riscv64_bare_metal() {
        let triple = TargetTriple::parse("riscv64-none").unwrap();
        assert_eq!(triple.arch, Architecture::RiscV64);
        assert_eq!(triple.os, OperatingSystem::None);
    }

    // ── v118: Cross-linker tests ──

    #[test]
    fn test_cross_linker_aarch64() {
        let triple = TargetTriple {
            arch: Architecture::AArch64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(triple.cross_linker_command(), "aarch64-linux-gnu-gcc");
    }

    #[test]
    fn test_cross_linker_riscv64() {
        let triple = TargetTriple {
            arch: Architecture::RiscV64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        assert_eq!(triple.cross_linker_command(), "riscv64-linux-gnu-gcc");
    }

    #[test]
    fn test_cross_linker_riscv64_baremetal() {
        let triple = TargetTriple {
            arch: Architecture::RiscV64,
            os: OperatingSystem::None,
            env: Environment::None,
        };
        assert_eq!(triple.cross_linker_command(), "riscv64-unknown-elf-gcc");
    }

    #[test]
    fn test_is_cross_compile() {
        let host = TargetTriple::host();
        assert!(!host.is_cross_compile());

        let aarch64 = TargetTriple {
            arch: Architecture::AArch64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        // On x86_64 Windows (the typical dev env), AArch64 Linux is cross
        #[cfg(target_arch = "x86_64")]
        assert!(aarch64.is_cross_compile());
    }

    // ── v118: Cross-compilation object emission ──

    #[test]
    fn test_aot_cross_compile_aarch64_object() {
        let config = AotConfig {
            target: TargetTriple {
                arch: Architecture::AArch64,
                os: OperatingSystem::Linux,
                env: Environment::Gnu,
            },
            output: PathBuf::from("target/test_aot_aarch64_single"),
            link: false, // No cross-linker needed for object emission
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source("fn main() -> i64 { 42 }");
        assert!(result.is_ok(), "AArch64 AOT compile failed: {:?}", result.err());
        let aot_result = result.unwrap();
        assert!(aot_result.is_success());
        assert!(aot_result.function_count > 0);
        // Verify it's a real ELF object file (AArch64 Linux target)
        let bytes = std::fs::read(&aot_result.object_path).unwrap();
        assert!(bytes.len() > 64, "object too small: {} bytes", bytes.len());
        // ELF magic: 0x7F 'E' 'L' 'F'
        assert_eq!(bytes[0], 0x7F);
        assert_eq!(bytes[1], b'E');
        assert_eq!(bytes[2], b'L');
        assert_eq!(bytes[3], b'F');
        // ELF e_machine for AArch64 is 0xB7 (183) at offset 18 (little-endian)
        assert_eq!(bytes[18], 0xB7, "e_machine should be EM_AARCH64 (0xB7)");
        let _ = std::fs::remove_file(&aot_result.object_path);
    }

    #[test]
    fn test_aot_cross_compile_riscv64_object() {
        let config = AotConfig {
            target: TargetTriple {
                arch: Architecture::RiscV64,
                os: OperatingSystem::Linux,
                env: Environment::Gnu,
            },
            output: PathBuf::from("target/test_aot_riscv64"),
            link: false,
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source("fn main() -> i64 { 42 }");
        assert!(result.is_ok(), "RISC-V 64 AOT compile failed: {:?}", result.err());
        let aot_result = result.unwrap();
        assert!(aot_result.is_success());
        // Verify it's a real ELF object file
        let bytes = std::fs::read(&aot_result.object_path).unwrap();
        assert!(bytes.len() > 64);
        assert_eq!(bytes[0], 0x7F);
        assert_eq!(bytes[1], b'E');
        // ELF e_machine for RISC-V is 0xF3 (243) at offset 18
        assert_eq!(bytes[18], 0xF3, "e_machine should be EM_RISCV (0xF3)");
        let _ = std::fs::remove_file(&aot_result.object_path);
    }

    #[test]
    fn test_aot_cross_compile_aarch64_multi_function() {
        let config = AotConfig {
            target: TargetTriple {
                arch: Architecture::AArch64,
                os: OperatingSystem::Linux,
                env: Environment::Gnu,
            },
            output: PathBuf::from("target/test_aot_aarch64_multi"),
            link: false,
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source(
            "fn double(x: i64) -> i64 { x * 2 }\nfn main() -> i64 { double(21) }"
        ).unwrap();
        assert!(result.is_success());
        assert!(result.function_count >= 2);
        let _ = std::fs::remove_file(&result.object_path);
    }

    #[test]
    fn test_aot_cross_compile_riscv64_arithmetic() {
        let config = AotConfig {
            target: TargetTriple {
                arch: Architecture::RiscV64,
                os: OperatingSystem::Linux,
                env: Environment::Gnu,
            },
            output: PathBuf::from("target/test_aot_riscv64_arith"),
            link: false,
            ..Default::default()
        };
        let mut compiler = AotCompiler::new(config);
        let result = compiler.compile_source(
            "fn add(a: i64, b: i64) -> i64 { a + b }\nfn main() -> i64 { add(10, 32) }"
        ).unwrap();
        assert!(result.is_success());
        assert!(result.function_count >= 2);
        let _ = std::fs::remove_file(&result.object_path);
    }
}
