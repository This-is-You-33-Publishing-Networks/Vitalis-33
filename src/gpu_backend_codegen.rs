//! v80 multi-backend GPU codegen over shared kernel IR.

use crate::gpu_compute::GpuBackend;
use crate::gpu_kernel_lowering::{KernelInst, KernelIr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendProgram {
    pub backend: GpuBackend,
    pub source: String,
}

pub fn emit_kernel(ir: &KernelIr, backend: GpuBackend) -> BackendProgram {
    let header = match backend {
        GpuBackend::Cuda => "// CUDA",
        GpuBackend::Vulkan => "// Vulkan (GLSL compute)",
        GpuBackend::Metal => "// Metal",
        GpuBackend::WebGpu => "// WebGPU (WGSL)",
        GpuBackend::Software => "// Software",
    };

    let mut lines = vec![
        header.to_string(),
        format!("kernel {} launch={:?}", ir.name, ir.launch),
    ];

    for inst in &ir.body {
        match inst {
            KernelInst::Load { dst, src, idx } => {
                lines.push(format!("{dst} = load {}[{idx}]", src.name));
            }
            KernelInst::Store { dst, idx, src } => {
                lines.push(format!("store {}[{idx}] = {src}", dst.name));
            }
            KernelInst::Add { dst, a, b } => {
                lines.push(format!("{dst} = {a} + {b}"));
            }
            KernelInst::Mul { dst, a, b } => {
                lines.push(format!("{dst} = {a} * {b}"));
            }
        }
    }

    BackendProgram {
        backend,
        source: lines.join("\n"),
    }
}

pub fn cpu_reference_add(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu_kernel_lowering::lower_elementwise_add;

    #[test]
    fn emits_backend_specific_headers() {
        let ir = lower_elementwise_add("vadd", [64, 1, 1]);
        let cuda = emit_kernel(&ir, GpuBackend::Cuda);
        let metal = emit_kernel(&ir, GpuBackend::Metal);
        assert!(cuda.source.starts_with("// CUDA"));
        assert!(metal.source.starts_with("// Metal"));
    }

    #[test]
    fn cpu_reference_parity_for_add() {
        let a = vec![1.0, 2.5, -3.0, 4.25];
        let b = vec![0.5, -2.5, 3.0, 1.75];
        let out = cpu_reference_add(&a, &b);
        assert_eq!(out, vec![1.5, 0.0, 0.0, 6.0]);
    }
}
