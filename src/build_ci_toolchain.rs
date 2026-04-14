//! v87 Build + CI toolchain helpers: graph diagnostics, deterministic manifests, and CI triage formatting.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildNode {
    pub name: String,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildGraphSnapshot {
    pub node_count: usize,
    pub edge_count: usize,
    pub roots: Vec<String>,
    pub leaves: Vec<String>,
    pub topo_hint: Vec<String>,
}

pub fn inspect_build_graph(nodes: &[BuildNode]) -> BuildGraphSnapshot {
    let mut all = BTreeSet::new();
    let mut incoming: BTreeMap<String, usize> = BTreeMap::new();
    let mut outgoing: BTreeMap<String, usize> = BTreeMap::new();

    for node in nodes {
        all.insert(node.name.clone());
        outgoing.insert(node.name.clone(), node.deps.len());
        incoming.entry(node.name.clone()).or_insert(0);
        for dep in &node.deps {
            all.insert(dep.clone());
            *incoming.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    let mut roots: Vec<String> = all
        .iter()
        .filter(|n| incoming.get(*n).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    roots.sort();

    let mut leaves: Vec<String> = all
        .iter()
        .filter(|n| outgoing.get(*n).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    leaves.sort();

    // Deterministic topological hint: lexical Kahn-like approximation.
    let mut topo_hint: Vec<String> = nodes.iter().map(|n| n.name.clone()).collect();
    topo_hint.sort();

    BuildGraphSnapshot {
        node_count: all.len(),
        edge_count: nodes.iter().map(|n| n.deps.len()).sum(),
        roots,
        leaves,
        topo_hint,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildManifest {
    pub profile: String,
    pub target: String,
    pub env: BTreeMap<String, String>,
    pub features: Vec<String>,
    pub artifact_stamp: String,
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn deterministic_manifest(
    profile: &str,
    target: &str,
    env: &BTreeMap<String, String>,
    features: &[String],
) -> BuildManifest {
    let mut feature_list = features.to_vec();
    feature_list.sort();

    let mut canonical = format!("profile={};target={};", profile, target);
    for (k, v) in env {
        canonical.push_str(&format!("{}={};", k, v));
    }
    for feat in &feature_list {
        canonical.push_str(&format!("feature={};", feat));
    }

    let artifact_stamp = format!("stamp:{:016x}", fnv1a64(canonical.as_bytes()));

    BuildManifest {
        profile: profile.to_string(),
        target: target.to_string(),
        env: env.clone(),
        features: feature_list,
        artifact_stamp,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CiPreset {
    Fast,
    Full,
    ReleaseCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiPlan {
    pub preset: CiPreset,
    pub commands: Vec<String>,
}

pub fn build_ci_plan(preset: CiPreset) -> CiPlan {
    let commands = match preset {
        CiPreset::Fast => vec![
            "cargo test --release package_manager::tests".to_string(),
            "cargo test --release concurrency::tests".to_string(),
        ],
        CiPreset::Full => vec![
            "cargo test --release".to_string(),
            "cargo run --release -- check examples/hello.sl".to_string(),
        ],
        CiPreset::ReleaseCandidate => vec![
            "cargo test --release".to_string(),
            "cargo run --release -- build examples/hello.sl -o hello.exe".to_string(),
            "cargo run --release -- run examples/hello.sl".to_string(),
        ],
    };

    CiPlan { preset, commands }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureRecord {
    pub module: String,
    pub test: String,
    pub message: String,
}

pub fn format_failure_triage(records: &[FailureRecord]) -> String {
    if records.is_empty() {
        return "no failures".to_string();
    }

    let mut grouped: BTreeMap<String, Vec<&FailureRecord>> = BTreeMap::new();
    for record in records {
        grouped.entry(record.module.clone()).or_default().push(record);
    }

    let mut out = String::new();
    for (module, items) in grouped {
        out.push_str(&format!("[{}]\n", module));
        for item in items {
            out.push_str(&format!("- {} :: {}\n", item.test, item.message));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_build_graph() {
        let nodes = vec![
            BuildNode {
                name: "a".to_string(),
                deps: vec!["b".to_string(), "c".to_string()],
            },
            BuildNode {
                name: "b".to_string(),
                deps: vec!["d".to_string()],
            },
            BuildNode {
                name: "c".to_string(),
                deps: vec![],
            },
        ];

        let snapshot = inspect_build_graph(&nodes);
        assert_eq!(snapshot.node_count, 4);
        assert_eq!(snapshot.edge_count, 3);
        assert!(snapshot.roots.contains(&"a".to_string()));
        assert!(snapshot.leaves.contains(&"d".to_string()));
    }

    #[test]
    fn test_deterministic_manifest_stable() {
        let mut env = BTreeMap::new();
        env.insert("RUSTFLAGS".to_string(), "-Ctarget-cpu=native".to_string());
        env.insert("CI".to_string(), "true".to_string());

        let features = vec!["gpu".to_string(), "simd".to_string()];
        let m1 = deterministic_manifest("release", "x86_64-windows-msvc", &env, &features);
        let m2 = deterministic_manifest("release", "x86_64-windows-msvc", &env, &features);
        assert_eq!(m1.artifact_stamp, m2.artifact_stamp);
    }

    #[test]
    fn test_ci_plan_presets() {
        let fast = build_ci_plan(CiPreset::Fast);
        let full = build_ci_plan(CiPreset::Full);
        let rc = build_ci_plan(CiPreset::ReleaseCandidate);

        assert_eq!(fast.commands.len(), 2);
        assert_eq!(full.commands.len(), 2);
        assert_eq!(rc.commands.len(), 3);
    }

    #[test]
    fn test_format_failure_triage_grouped() {
        let triage = format_failure_triage(&[
            FailureRecord {
                module: "concurrency".to_string(),
                test: "test_deadlock_cycle".to_string(),
                message: "assertion failed".to_string(),
            },
            FailureRecord {
                module: "concurrency".to_string(),
                test: "test_work_stealing".to_string(),
                message: "timed out".to_string(),
            },
            FailureRecord {
                module: "networking".to_string(),
                test: "test_frame_decode".to_string(),
                message: "length mismatch".to_string(),
            },
        ]);

        assert!(triage.contains("[concurrency]"));
        assert!(triage.contains("[networking]"));
        assert!(triage.contains("test_work_stealing"));
    }
}
