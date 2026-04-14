//! v79 backend-neutral GPU kernel IR lowering.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSpace {
    Global,
    Shared,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelValue {
    pub name: String,
    pub space: MemSpace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelInst {
    Load { dst: String, src: KernelValue, idx: String },
    Store { dst: KernelValue, idx: String, src: String },
    Add { dst: String, a: String, b: String },
    Mul { dst: String, a: String, b: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelIr {
    pub name: String,
    pub launch: [u32; 3],
    pub body: Vec<KernelInst>,
}

pub fn lower_elementwise_add(name: &str, launch: [u32; 3]) -> KernelIr {
    let a = KernelValue {
        name: "a".into(),
        space: MemSpace::Global,
    };
    let b = KernelValue {
        name: "b".into(),
        space: MemSpace::Global,
    };
    let out = KernelValue {
        name: "out".into(),
        space: MemSpace::Global,
    };

    KernelIr {
        name: name.to_string(),
        launch,
        body: vec![
            KernelInst::Load {
                dst: "v0".into(),
                src: a,
                idx: "gid".into(),
            },
            KernelInst::Load {
                dst: "v1".into(),
                src: b,
                idx: "gid".into(),
            },
            KernelInst::Add {
                dst: "v2".into(),
                a: "v0".into(),
                b: "v1".into(),
            },
            KernelInst::Store {
                dst: out,
                idx: "gid".into(),
                src: "v2".into(),
            },
        ],
    }
}

pub fn validate_launch_geometry(launch: [u32; 3]) -> bool {
    launch[0] > 0 && launch[1] > 0 && launch[2] > 0 && launch[0] <= 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowers_add_kernel_to_four_instruction_body() {
        let ir = lower_elementwise_add("vec_add", [256, 1, 1]);
        assert_eq!(ir.name, "vec_add");
        assert_eq!(ir.body.len(), 4);
        assert!(matches!(ir.body[2], KernelInst::Add { .. }));
    }

    #[test]
    fn validates_launch_bounds() {
        assert!(validate_launch_geometry([1024, 1, 1]));
        assert!(!validate_launch_geometry([0, 1, 1]));
        assert!(!validate_launch_geometry([2048, 1, 1]));
    }
}
