//! Neuromorphic Safety & Verification (v279–v284)
//!
//! Formal verification, safety rails, explainability,
//! adversarial robustness, fairness, certified inference.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v279: Formal Verification ────────────────────────────────────────────────

/// Verify spike timing bounds. Returns 1 if all within [lo, hi], 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_verify_timing(spike_time: i64, lo: i64, hi: i64) -> i64 {
    if spike_time >= lo && spike_time <= hi { 1 } else { 0 }
}

/// Verify membrane potential is bounded. Returns 1 if within range.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_verify_membrane(voltage: f64, v_min: f64, v_max: f64) -> i64 {
    if voltage >= v_min && voltage <= v_max { 1 } else { 0 }
}

/// Verify weight symmetry (|w_ij - w_ji| < eps). Returns violation count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_verify_symmetry(w_ij: f64, w_ji: f64, eps: f64) -> i64 {
    if (w_ij - w_ji).abs() < eps.abs().max(1e-10) { 0 } else { 1 }
}

/// Verify liveness: neuron must fire within deadline. Returns 1 if live.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_verify_liveness(last_spike: i64, deadline: i64, now: i64) -> i64 {
    if now - last_spike <= deadline { 1 } else { 0 }
}

// ── v280: Safety Rails ───────────────────────────────────────────────────────

static SAFETY_VIOLATIONS: AtomicI64 = AtomicI64::new(0);

/// Clamp firing rate to safe range [0, max_hz]. Returns clamped rate.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_safe_rate_clamp(rate: i64, max_hz: i64) -> i64 {
    let max = max_hz.max(1);
    if rate < 0 { SAFETY_VIOLATIONS.fetch_add(1, Ordering::SeqCst); 0 }
    else if rate > max { SAFETY_VIOLATIONS.fetch_add(1, Ordering::SeqCst); max }
    else { rate }
}

/// Detect runaway excitation. Returns 1 if rate exceeds threshold.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_safe_runaway(rate: i64, threshold: i64) -> i64 {
    if rate > threshold.max(1) { SAFETY_VIOLATIONS.fetch_add(1, Ordering::SeqCst); 1 } else { 0 }
}

/// Dead neuron detection. Returns 1 if neuron hasn't fired in window.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_safe_dead_neuron(spikes_in_window: i64) -> i64 {
    if spikes_in_window <= 0 { 1 } else { 0 }
}

/// Get total safety violation count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_safe_violations() -> i64 {
    SAFETY_VIOLATIONS.load(Ordering::SeqCst)
}

// ── v281: Explainability ─────────────────────────────────────────────────────

/// Compute spike contribution score. Returns score ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_explain_contribution(weight: f64, spike_count: i64) -> i64 {
    (weight * spike_count as f64 * 1000.0).round() as i64
}

/// Feature importance via ablation. Returns importance ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_explain_ablation(baseline_acc: f64, ablated_acc: f64) -> i64 {
    let importance = (baseline_acc - ablated_acc).max(0.0);
    (importance * 1000.0).round() as i64
}

/// Spike saliency map value. Returns saliency ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_explain_saliency(gradient: f64, activation: f64) -> i64 {
    let saliency = (gradient * activation).abs();
    (saliency * 1000.0).round() as i64
}

/// Layer-wise relevance propagation score. Returns LRP ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_explain_lrp(activation: f64, weight: f64, total: f64) -> i64 {
    if total.abs() < 1e-10 { return 0; }
    let lrp = activation * weight / total;
    (lrp * 1000.0).round() as i64
}

// ── v282: Adversarial Robustness ─────────────────────────────────────────────

/// Check if perturbation is within epsilon ball. Returns 1 if safe.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_robust_eps_check(perturbation: f64, epsilon: f64) -> i64 {
    if perturbation.abs() <= epsilon.abs() { 1 } else { 0 }
}

/// Compute adversarial margin. Returns margin ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_robust_margin(correct_logit: f64, best_other: f64) -> i64 {
    let margin = correct_logit - best_other;
    (margin * 1000.0).round() as i64
}

/// Certified radius for randomized smoothing. Returns radius ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_robust_certified_radius(sigma: f64, pa: f64) -> i64 {
    if pa <= 0.5 || pa >= 1.0 { return 0; }
    // Simplified: radius ≈ σ * Φ⁻¹(pA) ≈ σ * 2(pA-0.5) for rough estimate
    let inv_cdf = 2.0 * (pa - 0.5);
    (sigma * inv_cdf * 1000.0).round() as i64
}

/// Noise injection for smoothing defense. Returns perturbed ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_robust_inject_noise(value: f64, sigma: f64) -> i64 {
    // Deterministic proxy for testing: add σ/2
    let noisy = value + sigma * 0.5;
    (noisy * 1000.0).round() as i64
}

// ── v283: Fairness ───────────────────────────────────────────────────────────

/// Demographic parity gap. Returns |rate_a - rate_b| ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_fair_dp_gap(rate_a: f64, rate_b: f64) -> i64 {
    ((rate_a - rate_b).abs() * 1000.0).round() as i64
}

/// Equalized odds gap. Returns gap ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_fair_eo_gap(tpr_a: f64, tpr_b: f64) -> i64 {
    ((tpr_a - tpr_b).abs() * 1000.0).round() as i64
}

/// Calibration error for group. Returns |predicted - actual| ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_fair_calibration(predicted: f64, actual: f64) -> i64 {
    ((predicted - actual).abs() * 1000.0).round() as i64
}

/// Individual fairness score (Lipschitz). Returns score ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_fair_lipschitz(output_diff: f64, input_diff: f64) -> i64 {
    if input_diff.abs() < 1e-10 { return 0; }
    let ratio = output_diff.abs() / input_diff.abs();
    (ratio * 1000.0).round() as i64
}

// ── v284: Certified Inference ────────────────────────────────────────────────

/// Verify output is within certified bounds. Returns 1 if certified.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cert_bounds(output: f64, lower: f64, upper: f64) -> i64 {
    if output >= lower && output <= upper { 1 } else { 0 }
}

/// Interval bound propagation width. Returns width ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cert_ibp_width(lower: f64, upper: f64) -> i64 {
    ((upper - lower).abs() * 1000.0).round() as i64
}

/// CROWN bound tightness score. Returns tightness ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cert_crown(ibp_width: f64, crown_width: f64) -> i64 {
    if ibp_width.abs() < 1e-10 { return 1000; }
    let tightness = 1.0 - (crown_width / ibp_width).clamp(0.0, 1.0);
    (tightness * 1000.0).round() as i64
}

/// Certified accuracy: fraction with verified predictions. Returns ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_cert_accuracy(certified_count: i64, total: i64) -> i64 {
    if total <= 0 { return 0; }
    (certified_count as f64 / total as f64 * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_timing() {
        assert_eq!(slang_neuro_verify_timing(50, 0, 100), 1);
        assert_eq!(slang_neuro_verify_timing(150, 0, 100), 0);
    }

    #[test]
    fn test_verify_membrane() {
        assert_eq!(slang_neuro_verify_membrane(-70.0, -80.0, 40.0), 1);
        assert_eq!(slang_neuro_verify_membrane(50.0, -80.0, 40.0), 0);
    }

    #[test]
    fn test_safe_rate_clamp() {
        assert_eq!(slang_neuro_safe_rate_clamp(50, 100), 50);
        assert_eq!(slang_neuro_safe_rate_clamp(200, 100), 100);
    }

    #[test]
    fn test_explain_contribution() {
        assert_eq!(slang_neuro_explain_contribution(0.5, 10), 5000);
    }

    #[test]
    fn test_explain_ablation() {
        assert_eq!(slang_neuro_explain_ablation(0.9, 0.7), 200);
    }

    #[test]
    fn test_robust_eps_check() {
        assert_eq!(slang_neuro_robust_eps_check(0.05, 0.1), 1);
        assert_eq!(slang_neuro_robust_eps_check(0.2, 0.1), 0);
    }

    #[test]
    fn test_fair_dp_gap() {
        assert_eq!(slang_neuro_fair_dp_gap(0.8, 0.7), 100);
    }

    #[test]
    fn test_cert_bounds() {
        assert_eq!(slang_neuro_cert_bounds(0.5, 0.0, 1.0), 1);
        assert_eq!(slang_neuro_cert_bounds(1.5, 0.0, 1.0), 0);
    }

    #[test]
    fn test_cert_accuracy() {
        assert_eq!(slang_neuro_cert_accuracy(80, 100), 800);
    }

    #[test]
    fn test_robust_margin() {
        assert_eq!(slang_neuro_robust_margin(3.0, 1.0), 2000);
    }
}
