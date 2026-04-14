//! v100 Overhaul validation: architecture report and release sign-off checklist.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDelta {
    pub area: String,
    pub correctness_delta: i32,
    pub perf_delta_percent: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignoffChecklist {
    pub full_tests_green: bool,
    pub integration_green: bool,
    pub reproducibility_green: bool,
    pub docs_published: bool,
}

pub fn compile_report(deltas: &[ValidationDelta]) -> String {
    let mut lines = vec!["# Overhaul Validation Report".to_string()];
    for d in deltas {
        lines.push(format!(
            "- {}: correctness_delta={}, perf_delta={}%%",
            d.area, d.correctness_delta, d.perf_delta_percent
        ));
    }
    lines.join("\n")
}

pub fn signoff_ready(c: &SignoffChecklist) -> bool {
    c.full_tests_green && c.integration_green && c.reproducibility_green && c.docs_published
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_contains_areas() {
        let r = compile_report(&[
            ValidationDelta { area: "compiler".into(), correctness_delta: 0, perf_delta_percent: 5 },
            ValidationDelta { area: "runtime".into(), correctness_delta: 0, perf_delta_percent: 3 },
        ]);
        assert!(r.contains("compiler"));
        assert!(r.contains("runtime"));
    }

    #[test]
    fn test_signoff_gate() {
        let ok = SignoffChecklist {
            full_tests_green: true,
            integration_green: true,
            reproducibility_green: true,
            docs_published: true,
        };
        assert!(signoff_ready(&ok));
    }
}
