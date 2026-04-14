//! Vitalis Cross-Compilation Targets (v22 Roadmap)
//!
//! Provides cross-compilation support for multiple architectures:
//! - x86_64 (AMD64) — native host
//! - AArch64 (ARM64) — Apple Silicon, AWS Graviton, Raspberry Pi 4+
//! - RISC-V 64 (RV64GC) — SiFive, StarFive, emerging ISA
//!
//! # Architecture
//!
//! ```text
//! Vitalis IR → Target Selection → ISA Configuration → Cranelift Backend → Object Code
//!                   ↓                    ↓                    ↓
//!            TargetTriple        ISA Flags/Features     ABI Lowering
//! ```
//!
//! Cross-compilation handles:
//! - Target-specific ISA selection (register allocation, instruction selection)
//! - ABI differences (calling conventions, struct layout)
//! - Pointer width differences (32-bit vs 64-bit)
//! - Endianness considerations
//! - Runtime library selection per target

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::aot::{TargetTriple, Architecture, OperatingSystem, Environment, AotConfig, AotCompiler, AotResult};
use crate::ir::IrType;

// ═══════════════════════════════════════════════════════════════════════
//  Target Features & ISA Configuration
// ═══════════════════════════════════════════════════════════════════════

/// CPU features for a target architecture.
#[derive(Debug, Clone)]
pub struct TargetFeatures {
    /// Architecture-specific feature flags.
    pub features: Vec<String>,
    /// CPU model name (for tuning).
    pub cpu: String,
    /// Whether to use SIMD instructions.
    pub simd: bool,
    /// Whether to use atomic instructions.
    pub atomics: bool,
    /// Whether to enable floating-point operations.
    pub float: bool,
    /// Whether to enable hardware multiply/divide.
    pub mul_div: bool,
}

impl TargetFeatures {
    /// Default features for x86_64.
    pub fn x86_64_default() -> Self {
        Self {
            features: vec![
                "sse2".to_string(),
                "sse3".to_string(),
                "sse4.1".to_string(),
                "sse4.2".to_string(),
                "popcnt".to_string(),
                "bmi1".to_string(),
                "bmi2".to_string(),
            ],
            cpu: "x86-64-v2".to_string(),
            simd: true,
            atomics: true,
            float: true,
            mul_div: true,
        }
    }

    /// Default features for AArch64.
    pub fn aarch64_default() -> Self {
        Self {
            features: vec![
                "neon".to_string(),      // SIMD
                "fp-armv8".to_string(),   // Floating point
                "crc".to_string(),        // CRC instructions
                "lse".to_string(),        // Large System Extensions (atomics)
            ],
            cpu: "generic".to_string(),
            simd: true,
            atomics: true,
            float: true,
            mul_div: true,
        }
    }

    /// Default features for RISC-V 64.
    pub fn riscv64_default() -> Self {
        Self {
            features: vec![
                "m".to_string(),  // Integer multiply/divide
                "a".to_string(),  // Atomic instructions
                "f".to_string(),  // Single-precision float
                "d".to_string(),  // Double-precision float
                "c".to_string(),  // Compressed instructions
            ],
            cpu: "generic-rv64".to_string(),
            simd: false, // RISC-V V extension not yet standard
            atomics: true,
            float: true,
            mul_div: true,
        }
    }

    /// Get features for a given architecture.
    pub fn for_arch(arch: Architecture) -> Self {
        match arch {
            Architecture::X86_64 => Self::x86_64_default(),
            Architecture::AArch64 => Self::aarch64_default(),
            Architecture::RiscV64 => Self::riscv64_default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ABI Configuration
// ═══════════════════════════════════════════════════════════════════════

/// Calling convention and ABI details for a target.
#[derive(Debug, Clone)]
pub struct AbiConfig {
    pub calling_convention: CallingConvention,
    pub pointer_width: u8,
    pub stack_alignment: u8,
    pub int_arg_registers: u8,
    pub float_arg_registers: u8,
    pub endianness: Endianness,
    pub red_zone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CallingConvention {
    SystemV,    // Linux, macOS (x86_64, AArch64)
    Win64,      // Windows x86_64
    Aapcs64,    // AArch64 (ARM procedure call standard)
    RiscvIlp32, // RISC-V 32-bit
    RiscvLp64,  // RISC-V 64-bit
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endianness {
    Little,
    Big,
}

impl AbiConfig {
    /// Get ABI config for a target triple.
    pub fn for_target(target: &TargetTriple) -> Self {
        match (target.arch, target.os) {
            (Architecture::X86_64, OperatingSystem::Windows) => Self {
                calling_convention: CallingConvention::Win64,
                pointer_width: 8,
                stack_alignment: 16,
                int_arg_registers: 4,   // rcx, rdx, r8, r9
                float_arg_registers: 4, // xmm0-xmm3
                endianness: Endianness::Little,
                red_zone: false,
            },
            (Architecture::X86_64, _) => Self {
                calling_convention: CallingConvention::SystemV,
                pointer_width: 8,
                stack_alignment: 16,
                int_arg_registers: 6,   // rdi, rsi, rdx, rcx, r8, r9
                float_arg_registers: 8, // xmm0-xmm7
                endianness: Endianness::Little,
                red_zone: true,
            },
            (Architecture::AArch64, _) => Self {
                calling_convention: CallingConvention::Aapcs64,
                pointer_width: 8,
                stack_alignment: 16,
                int_arg_registers: 8,   // x0-x7
                float_arg_registers: 8, // v0-v7
                endianness: Endianness::Little,
                red_zone: false,
            },
            (Architecture::RiscV64, _) => Self {
                calling_convention: CallingConvention::RiscvLp64,
                pointer_width: 8,
                stack_alignment: 16,
                int_arg_registers: 8,   // a0-a7
                float_arg_registers: 8, // fa0-fa7
                endianness: Endianness::Little,
                red_zone: false,
            },
        }
    }

    /// Get the byte size of a pointer on this target.
    pub fn pointer_bytes(&self) -> u8 {
        self.pointer_width
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Type Layout for Target
// ═══════════════════════════════════════════════════════════════════════

/// Compute type sizes and alignments for a specific target.
#[derive(Debug, Clone)]
pub struct TypeLayout {
    abi: AbiConfig,
}

impl TypeLayout {
    pub fn new(abi: AbiConfig) -> Self {
        Self { abi }
    }

    /// Size of an IR type in bytes on this target.
    pub fn size_of(&self, ty: &IrType) -> u32 {
        match ty {
            IrType::I32 => 4,
            IrType::I64 => 8,
            IrType::F32 => 4,
            IrType::F64 => 8,
            IrType::Bool => 1,
            IrType::Ptr => self.abi.pointer_width as u32,
            IrType::Void => 0,
        }
    }

    /// Alignment of an IR type in bytes on this target.
    pub fn align_of(&self, ty: &IrType) -> u32 {
        match ty {
            IrType::I32 => 4,
            IrType::I64 => 8,
            IrType::F32 => 4,
            IrType::F64 => 8,
            IrType::Bool => 1,
            IrType::Ptr => self.abi.pointer_width as u32,
            IrType::Void => 1,
        }
    }

    /// Stack slot size (rounded up to alignment).
    pub fn stack_slot_size(&self, ty: &IrType) -> u32 {
        let size = self.size_of(ty);
        let align = self.align_of(ty);
        (size + align - 1) / align * align
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Cross-Compilation Manager
// ═══════════════════════════════════════════════════════════════════════

/// Manages cross-compilation for multiple targets.
pub struct CrossCompiler {
    /// Available targets.
    targets: HashMap<String, CrossTarget>,
    /// Default target (usually host).
    default_target: String,
}

/// A configured cross-compilation target.
#[derive(Debug, Clone)]
pub struct CrossTarget {
    pub triple: TargetTriple,
    pub features: TargetFeatures,
    pub abi: AbiConfig,
    pub layout: TypeLayout,
    pub sysroot: Option<PathBuf>,
    pub linker: Option<String>,
}

impl CrossTarget {
    pub fn new(triple: TargetTriple) -> Self {
        let features = TargetFeatures::for_arch(triple.arch);
        let abi = AbiConfig::for_target(&triple);
        let layout = TypeLayout::new(abi.clone());
        Self {
            triple,
            features,
            abi,
            layout,
            sysroot: None,
            linker: None,
        }
    }

    /// Set a custom sysroot path.
    pub fn with_sysroot(mut self, path: PathBuf) -> Self {
        self.sysroot = Some(path);
        self
    }

    /// Set a custom linker.
    pub fn with_linker(mut self, linker: &str) -> Self {
        self.linker = Some(linker.to_string());
        self
    }
}

impl CrossCompiler {
    /// Create a new cross-compiler with default targets.
    pub fn new() -> Self {
        let mut targets = HashMap::new();
        let host = TargetTriple::host();
        let host_name = format!("{}", host);

        // Register host target
        targets.insert(host_name.clone(), CrossTarget::new(host));

        // Register known cross-compilation targets
        let cross_targets = vec![
            TargetTriple { arch: Architecture::X86_64, os: OperatingSystem::Linux, env: Environment::Gnu },
            TargetTriple { arch: Architecture::X86_64, os: OperatingSystem::Linux, env: Environment::Musl },
            TargetTriple { arch: Architecture::X86_64, os: OperatingSystem::Windows, env: Environment::Msvc },
            TargetTriple { arch: Architecture::AArch64, os: OperatingSystem::Linux, env: Environment::Gnu },
            TargetTriple { arch: Architecture::AArch64, os: OperatingSystem::Linux, env: Environment::Musl },
            TargetTriple { arch: Architecture::AArch64, os: OperatingSystem::MacOS, env: Environment::None },
            TargetTriple { arch: Architecture::RiscV64, os: OperatingSystem::Linux, env: Environment::Gnu },
            TargetTriple { arch: Architecture::RiscV64, os: OperatingSystem::None, env: Environment::None },
        ];

        for triple in cross_targets {
            let name = format!("{}", triple);
            if !targets.contains_key(&name) {
                targets.insert(name, CrossTarget::new(triple));
            }
        }

        Self {
            targets,
            default_target: host_name,
        }
    }

    /// List all available targets.
    pub fn available_targets(&self) -> Vec<&str> {
        self.targets.keys().map(|k| k.as_str()).collect()
    }

    /// Get a specific target configuration.
    pub fn get_target(&self, name: &str) -> Option<&CrossTarget> {
        self.targets.get(name)
    }

    /// Get the default (host) target.
    pub fn default_target(&self) -> &CrossTarget {
        self.targets.get(&self.default_target).unwrap()
    }

    /// Add a custom target.
    pub fn add_target(&mut self, name: &str, target: CrossTarget) {
        self.targets.insert(name.to_string(), target);
    }

    /// v118: Resolve a target string — supports full triples, short aliases, and
    /// partial names. Returns the canonical target name and the CrossTarget.
    pub fn resolve_target(&self, name: &str) -> Option<(&str, &CrossTarget)> {
        // Exact match first
        if let Some(target) = self.targets.get(name) {
            return Some((self.targets.get_key_value(name).unwrap().0.as_str(), target));
        }

        // Try short-form alias via TargetTriple::parse (handles aarch64, riscv64, etc.)
        if let Some(parsed) = TargetTriple::parse(name) {
            let canonical = format!("{}", parsed);
            if let Some(target) = self.targets.get(&canonical) {
                return Some((self.targets.get_key_value(&canonical).unwrap().0.as_str(), target));
            }
        }

        // Try prefix match (e.g., "aarch64-linux" matches "aarch64-linux-gnu")
        let matches: Vec<_> = self.targets.iter()
            .filter(|(k, _)| k.starts_with(name))
            .collect();
        if matches.len() == 1 {
            return Some((matches[0].0.as_str(), matches[0].1));
        }

        None
    }

    /// Compile source for a specific target.
    pub fn compile_for_target(
        &self,
        source: &str,
        target_name: &str,
        output: &Path,
    ) -> Result<AotResult, String> {
        let (_, target) = self.resolve_target(target_name)
            .ok_or_else(|| format!("unknown target: '{}'", target_name))?;

        let config = AotConfig {
            target: target.triple.clone(),
            output: output.to_path_buf(),
            link: target.triple.is_host(), // only link on host
            verbose: true,
            ..Default::default()
        };

        let mut compiler = AotCompiler::new(config);
        compiler.compile_source(source)
    }

    /// Compile source for all available targets.
    pub fn compile_for_all(
        &self,
        source: &str,
        output_dir: &Path,
    ) -> Vec<(String, Result<AotResult, String>)> {
        let mut results = Vec::new();

        for (name, target) in &self.targets {
            let output = output_dir.join(format!("vitalis_{}", name));
            let config = AotConfig {
                target: target.triple.clone(),
                output,
                link: false, // Don't link for cross-compile targets
                verbose: false,
                ..Default::default()
            };

            let mut compiler = AotCompiler::new(config);
            let result = compiler.compile_source(source);
            results.push((name.clone(), result));
        }

        results
    }

    /// Get target info as a formatted string.
    pub fn target_info(&self, name: &str) -> Option<String> {
        let target = self.targets.get(name)?;
        Some(format!(
            "Target: {}\n  Arch: {:?}\n  OS: {:?}\n  ABI: {:?}\n  Pointer: {} bytes\n  \
             Stack align: {} bytes\n  Int regs: {}\n  Float regs: {}\n  SIMD: {}\n  \
             Atomics: {}\n  CPU: {}",
            target.triple,
            target.triple.arch,
            target.triple.os,
            target.abi.calling_convention,
            target.abi.pointer_width,
            target.abi.stack_alignment,
            target.abi.int_arg_registers,
            target.abi.float_arg_registers,
            target.features.simd,
            target.features.atomics,
            target.features.cpu,
        ))
    }
}

impl fmt::Display for CrossCompiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Vitalis Cross-Compiler — {} targets available:", self.targets.len())?;
        for (name, target) in &self.targets {
            let marker = if name == &self.default_target { " (host)" } else { "" };
            writeln!(f, "  {} — {:?} / {:?}{}",
                     name, target.triple.arch, target.triple.os, marker)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_features_x86_64() {
        let features = TargetFeatures::x86_64_default();
        assert!(features.simd);
        assert!(features.atomics);
        assert!(features.features.contains(&"sse2".to_string()));
    }

    #[test]
    fn test_target_features_aarch64() {
        let features = TargetFeatures::aarch64_default();
        assert!(features.simd);
        assert!(features.features.contains(&"neon".to_string()));
    }

    #[test]
    fn test_target_features_riscv64() {
        let features = TargetFeatures::riscv64_default();
        assert!(!features.simd); // V extension not standard yet
        assert!(features.atomics);
        assert!(features.features.contains(&"m".to_string()));
        assert!(features.features.contains(&"a".to_string()));
    }

    #[test]
    fn test_abi_config_x86_64_linux() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        let abi = AbiConfig::for_target(&target);
        assert_eq!(abi.calling_convention, CallingConvention::SystemV);
        assert_eq!(abi.pointer_width, 8);
        assert_eq!(abi.int_arg_registers, 6);
        assert!(abi.red_zone);
    }

    #[test]
    fn test_abi_config_x86_64_windows() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            os: OperatingSystem::Windows,
            env: Environment::Msvc,
        };
        let abi = AbiConfig::for_target(&target);
        assert_eq!(abi.calling_convention, CallingConvention::Win64);
        assert_eq!(abi.int_arg_registers, 4);
        assert!(!abi.red_zone);
    }

    #[test]
    fn test_abi_config_aarch64() {
        let target = TargetTriple {
            arch: Architecture::AArch64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        let abi = AbiConfig::for_target(&target);
        assert_eq!(abi.calling_convention, CallingConvention::Aapcs64);
        assert_eq!(abi.int_arg_registers, 8);
    }

    #[test]
    fn test_abi_config_riscv64() {
        let target = TargetTriple {
            arch: Architecture::RiscV64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        let abi = AbiConfig::for_target(&target);
        assert_eq!(abi.calling_convention, CallingConvention::RiscvLp64);
        assert_eq!(abi.pointer_width, 8);
    }

    #[test]
    fn test_type_layout() {
        let target = TargetTriple::host();
        let abi = AbiConfig::for_target(&target);
        let layout = TypeLayout::new(abi);

        assert_eq!(layout.size_of(&IrType::I32), 4);
        assert_eq!(layout.size_of(&IrType::I64), 8);
        assert_eq!(layout.size_of(&IrType::F64), 8);
        assert_eq!(layout.size_of(&IrType::Bool), 1);
        assert_eq!(layout.size_of(&IrType::Ptr), 8);
        assert_eq!(layout.size_of(&IrType::Void), 0);
    }

    #[test]
    fn test_type_alignment() {
        let target = TargetTriple::host();
        let abi = AbiConfig::for_target(&target);
        let layout = TypeLayout::new(abi);

        assert_eq!(layout.align_of(&IrType::I32), 4);
        assert_eq!(layout.align_of(&IrType::I64), 8);
        assert_eq!(layout.align_of(&IrType::Ptr), 8);
    }

    #[test]
    fn test_cross_compiler_creation() {
        let cc = CrossCompiler::new();
        let targets = cc.available_targets();
        assert!(!targets.is_empty());
        // Should have at least host + cross targets
        assert!(targets.len() >= 2);
    }

    #[test]
    fn test_cross_compiler_host_target() {
        let cc = CrossCompiler::new();
        let host = cc.default_target();
        assert!(host.triple.is_host());
    }

    #[test]
    fn test_cross_target_creation() {
        let triple = TargetTriple {
            arch: Architecture::AArch64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        let target = CrossTarget::new(triple.clone());
        assert_eq!(target.triple, triple);
        assert!(target.features.simd);
        assert_eq!(target.abi.pointer_width, 8);
    }

    #[test]
    fn test_cross_target_with_sysroot() {
        let triple = TargetTriple {
            arch: Architecture::RiscV64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        let target = CrossTarget::new(triple)
            .with_sysroot(PathBuf::from("/opt/riscv/sysroot"))
            .with_linker("riscv64-linux-gnu-gcc");
        assert_eq!(target.sysroot, Some(PathBuf::from("/opt/riscv/sysroot")));
        assert_eq!(target.linker, Some("riscv64-linux-gnu-gcc".to_string()));
    }

    #[test]
    fn test_target_info() {
        let cc = CrossCompiler::new();
        // Should be able to get info for at least the host target
        let targets = cc.available_targets();
        if !targets.is_empty() {
            let info = cc.target_info(targets[0]);
            assert!(info.is_some());
            let info_str = info.unwrap();
            assert!(info_str.contains("Target:"));
            assert!(info_str.contains("Pointer:"));
        }
    }

    #[test]
    fn test_cross_compiler_display() {
        let cc = CrossCompiler::new();
        let display = format!("{}", cc);
        assert!(display.contains("targets available"));
        assert!(display.contains("host"));
    }

    #[test]
    fn test_compile_for_host() {
        let cc = CrossCompiler::new();
        let host_name = format!("{}", TargetTriple::host());
        let output = PathBuf::from("target/test_cross_output");
        let result = cc.compile_for_target(
            "fn main() -> i64 { 42 }",
            &host_name,
            &output,
        );
        match result {
            Ok(aot) => {
                // If the linker is available, compilation should fully succeed.
                // If linking failed due to missing linker/libs, that's acceptable
                // in environments without a full toolchain installed.
                if !aot.is_success() {
                    let is_linker_issue = aot.errors.iter().any(|e| {
                        e.contains("link") || e.contains("linker")
                            || e.contains("not found") || e.contains("os error")
                    });
                    assert!(is_linker_issue,
                        "AOT failed for non-linker reason: {:?}", aot.errors);
                }
                // Cleanup
                let _ = std::fs::remove_file(&aot.object_path);
            }
            Err(e) => {
                // compile_for_target itself may error when linker is absent
                let is_linker_issue = e.contains("link") || e.contains("linker")
                    || e.contains("not found") || e.contains("os error");
                assert!(is_linker_issue,
                    "compile_for_target failed for non-linker reason: {}", e);
            }
        }
    }

    #[test]
    fn test_endianness() {
        // All our current targets are little-endian
        let targets = vec![
            TargetTriple { arch: Architecture::X86_64, os: OperatingSystem::Linux, env: Environment::Gnu },
            TargetTriple { arch: Architecture::AArch64, os: OperatingSystem::Linux, env: Environment::Gnu },
            TargetTriple { arch: Architecture::RiscV64, os: OperatingSystem::Linux, env: Environment::Gnu },
        ];
        for triple in targets {
            let abi = AbiConfig::for_target(&triple);
            assert_eq!(abi.endianness, Endianness::Little);
        }
    }

    #[test]
    fn test_calling_conventions() {
        assert_ne!(CallingConvention::SystemV, CallingConvention::Win64);
        assert_ne!(CallingConvention::Aapcs64, CallingConvention::RiscvLp64);
    }

    // ── v118: resolve_target tests ──

    #[test]
    fn test_resolve_target_exact_match() {
        let cc = CrossCompiler::new();
        let result = cc.resolve_target("aarch64-linux-gnu");
        assert!(result.is_some());
        let (name, target) = result.unwrap();
        assert_eq!(name, "aarch64-linux-gnu");
        assert_eq!(target.triple.arch, Architecture::AArch64);
    }

    #[test]
    fn test_resolve_target_short_alias_aarch64() {
        let cc = CrossCompiler::new();
        let result = cc.resolve_target("aarch64");
        assert!(result.is_some());
        let (_, target) = result.unwrap();
        assert_eq!(target.triple.arch, Architecture::AArch64);
        assert_eq!(target.triple.os, OperatingSystem::Linux);
    }

    #[test]
    fn test_resolve_target_short_alias_arm64() {
        let cc = CrossCompiler::new();
        let result = cc.resolve_target("arm64");
        assert!(result.is_some());
        let (_, target) = result.unwrap();
        assert_eq!(target.triple.arch, Architecture::AArch64);
    }

    #[test]
    fn test_resolve_target_short_alias_riscv64() {
        let cc = CrossCompiler::new();
        let result = cc.resolve_target("riscv64");
        assert!(result.is_some());
        let (_, target) = result.unwrap();
        assert_eq!(target.triple.arch, Architecture::RiscV64);
    }

    #[test]
    fn test_resolve_target_unknown() {
        let cc = CrossCompiler::new();
        assert!(cc.resolve_target("mips64").is_none());
        assert!(cc.resolve_target("z80").is_none());
    }

    #[test]
    fn test_cross_compile_aarch64_object() {
        let cc = CrossCompiler::new();
        let output = PathBuf::from("target/test_cross_aarch64_obj");
        let result = cc.compile_for_target(
            "fn main() -> i64 { 99 }",
            "aarch64-linux-gnu",
            &output,
        );
        // Should succeed — we skip linking for cross targets
        assert!(result.is_ok(), "cross-compile aarch64 failed: {:?}", result.err());
        let aot = result.unwrap();
        assert!(aot.is_success());
        // Verify ELF for AArch64
        let bytes = std::fs::read(&aot.object_path).unwrap();
        assert_eq!(bytes[0], 0x7F);
        assert_eq!(bytes[1], b'E');
        let _ = std::fs::remove_file(&aot.object_path);
    }

    #[test]
    fn test_cross_compile_riscv64_object() {
        let cc = CrossCompiler::new();
        let output = PathBuf::from("target/test_cross_riscv64_obj");
        let result = cc.compile_for_target(
            "fn main() -> i64 { 77 }",
            "riscv64-linux-gnu",
            &output,
        );
        assert!(result.is_ok(), "cross-compile riscv64 failed: {:?}", result.err());
        let aot = result.unwrap();
        assert!(aot.is_success());
        let bytes = std::fs::read(&aot.object_path).unwrap();
        assert_eq!(bytes[0], 0x7F);
        let _ = std::fs::remove_file(&aot.object_path);
    }

    #[test]
    fn test_type_layout_riscv64() {
        let target = TargetTriple {
            arch: Architecture::RiscV64,
            os: OperatingSystem::Linux,
            env: Environment::Gnu,
        };
        let abi = AbiConfig::for_target(&target);
        let layout = TypeLayout::new(abi);
        assert_eq!(layout.size_of(&IrType::I64), 8);
        assert_eq!(layout.size_of(&IrType::Ptr), 8);
        assert_eq!(layout.align_of(&IrType::F64), 8);
    }

    #[test]
    fn test_features_for_arch() {
        let x86 = TargetFeatures::for_arch(Architecture::X86_64);
        assert!(x86.simd);
        assert_eq!(x86.cpu, "x86-64-v2");

        let arm = TargetFeatures::for_arch(Architecture::AArch64);
        assert!(arm.simd);
        assert!(arm.features.contains(&"neon".to_string()));

        let rv = TargetFeatures::for_arch(Architecture::RiscV64);
        assert!(!rv.simd);
        assert!(rv.features.contains(&"c".to_string()));
    }
}
