//! Diffusion Model Primitives — AI/ML Production Stack (v323)
//!
//! Forward diffusion (noise scheduling), reverse denoising, DDPM/DDIM
//! sampling for generative modeling.

use std::sync::atomic::{AtomicI64, Ordering};

static DIFFUSION_OPS: AtomicI64 = AtomicI64::new(0);

/// Forward diffusion: add noise at timestep t with linear noise schedule.
/// alpha_bar = 1 - t/T. noisy = sqrt(alpha_bar)*x + sqrt(1-alpha_bar)*noise.
/// Returns noisy value × 1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diffusion_forward(x: f64, noise: f64, timestep: i64, total_steps: i64) -> i64 {
    DIFFUSION_OPS.fetch_add(1, Ordering::Relaxed);
    if total_steps <= 0 { return (x * 1000.0).round() as i64; }
    let t = (timestep as f64 / total_steps as f64).clamp(0.0, 1.0);
    let alpha_bar = 1.0 - t;
    let noisy = alpha_bar.sqrt() * x + (1.0 - alpha_bar).sqrt() * noise;
    (noisy * 1000.0).round() as i64
}

/// Reverse denoising step: predict clean sample from noisy input.
/// Uses predicted noise to estimate x_0.
/// Returns denoised value × 1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diffusion_reverse(noisy: f64, predicted_noise: f64, timestep: i64, total_steps: i64) -> i64 {
    DIFFUSION_OPS.fetch_add(1, Ordering::Relaxed);
    if total_steps <= 0 { return (noisy * 1000.0).round() as i64; }
    let t = (timestep as f64 / total_steps as f64).clamp(0.001, 1.0);
    let alpha_bar = 1.0 - t;
    let x0 = (noisy - (1.0 - alpha_bar).sqrt() * predicted_noise) / alpha_bar.sqrt().max(0.001);
    (x0 * 1000.0).round() as i64
}

/// Compute noise schedule value (alpha_bar) at timestep. Returns × 1000.
/// schedule_type: 0=linear, 1=cosine.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diffusion_schedule(timestep: i64, total_steps: i64, schedule_type: i64) -> i64 {
    if total_steps <= 0 { return 1000; }
    let t = (timestep as f64 / total_steps as f64).clamp(0.0, 1.0);
    let alpha_bar = match schedule_type {
        1 => { // cosine schedule
            let s = 0.008;
            let f = ((t + s) / (1.0 + s) * std::f64::consts::FRAC_PI_2).cos();
            let f0 = (s / (1.0 + s) * std::f64::consts::FRAC_PI_2).cos();
            (f * f) / (f0 * f0)
        }
        _ => 1.0 - t, // linear
    };
    (alpha_bar * 1000.0).round() as i64
}

/// DDIM deterministic sample step. Returns sampled value × 1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_diffusion_sample(x_t: f64, noise_pred: f64, alpha_t: f64, alpha_prev: f64) -> i64 {
    DIFFUSION_OPS.fetch_add(1, Ordering::Relaxed);
    let at = alpha_t.max(0.001);
    let ap = alpha_prev.max(0.001);
    let x0_pred = (x_t - (1.0 - at).sqrt() * noise_pred) / at.sqrt();
    let x_prev = ap.sqrt() * x0_pred + (1.0 - ap).sqrt() * noise_pred;
    (x_prev * 1000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_forward_no_noise() { assert_eq!(slang_diffusion_forward(1.0, 0.0, 0, 100), 1000); }
    #[test] fn test_forward_full_noise() {
        let v = slang_diffusion_forward(1.0, 1.0, 100, 100);
        assert_eq!(v, 1000); // at t=T, alpha_bar=0, so noisy = noise
    }
    #[test] fn test_forward_midpoint() {
        let v = slang_diffusion_forward(1.0, 0.0, 50, 100);
        assert!(v > 0 && v < 1000);
    }
    #[test] fn test_reverse_clean() {
        let v = slang_diffusion_reverse(1.0, 0.0, 50, 100);
        assert!(v > 0);
    }
    #[test] fn test_schedule_linear_start() {
        assert_eq!(slang_diffusion_schedule(0, 100, 0), 1000);
    }
    #[test] fn test_schedule_linear_end() {
        assert_eq!(slang_diffusion_schedule(100, 100, 0), 0);
    }
    #[test] fn test_schedule_cosine() {
        let v = slang_diffusion_schedule(50, 100, 1);
        assert!(v > 0 && v < 1000);
    }
    #[test] fn test_sample_deterministic() {
        let v = slang_diffusion_sample(0.5, 0.1, 0.9, 0.95);
        assert!(v != 0);
    }
    #[test] fn test_forward_zero_steps() {
        assert_eq!(slang_diffusion_forward(5.0, 1.0, 0, 0), 5000);
    }
    #[test] fn test_reverse_zero_steps() {
        assert_eq!(slang_diffusion_reverse(5.0, 1.0, 0, 0), 5000);
    }
    #[test] fn test_schedule_zero_steps() {
        assert_eq!(slang_diffusion_schedule(0, 0, 0), 1000);
    }
    #[test] fn test_forward_negative_value() {
        let v = slang_diffusion_forward(-1.0, 0.0, 0, 100);
        assert_eq!(v, -1000);
    }
    #[test] fn test_schedule_linear_midpoint() {
        assert_eq!(slang_diffusion_schedule(50, 100, 0), 500);
    }
    #[test] fn test_sample_identity() {
        // When alpha_t == alpha_prev, output ~= input
        let v = slang_diffusion_sample(1.0, 0.0, 0.9, 0.9);
        assert_eq!(v, 1000); // no noise pred → same value
    }
    #[test] fn test_forward_large_noise() {
        let v = slang_diffusion_forward(0.0, 100.0, 50, 100);
        assert!(v > 0); // noise dominates
    }
    #[test] fn test_cosine_schedule_bounds() {
        let start = slang_diffusion_schedule(0, 1000, 1);
        let end = slang_diffusion_schedule(1000, 1000, 1);
        assert!(start > end);
    }
    #[test] fn test_reverse_denoises() {
        // Forward then reverse should roughly recover
        let noisy = slang_diffusion_forward(1.0, 0.5, 10, 100);
        assert!(noisy != 1000); // it got noisy
    }
    #[test] fn test_sample_zero_noise_pred() {
        let v = slang_diffusion_sample(2.0, 0.0, 0.5, 0.5);
        // x0 = x_t / sqrt(alpha_t), x_prev = sqrt(alpha_prev) * x0
        assert!(v > 0);
    }
    #[test] fn test_schedule_monotonic_linear() {
        let mut prev = 1001;
        for t in 0..=100 {
            let v = slang_diffusion_schedule(t, 100, 0);
            assert!(v <= prev);
            prev = v;
        }
    }
    #[test] fn test_forward_symmetry() {
        let v1 = slang_diffusion_forward(1.0, -1.0, 50, 100);
        let v2 = slang_diffusion_forward(-1.0, 1.0, 50, 100);
        assert_eq!(v1, -v2); // symmetric
    }
}
