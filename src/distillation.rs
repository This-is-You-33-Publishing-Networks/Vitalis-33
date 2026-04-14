//! Model Distillation — AI/ML Production Stack (v321)
//!
//! Knowledge distillation (Hinton et al.), feature distillation, and
//! attention transfer for compressing large teacher models into smaller students.

use std::sync::atomic::{AtomicI64, Ordering};

static DISTILL_OPS: AtomicI64 = AtomicI64::new(0);

/// KL-divergence knowledge distillation loss: KL(softmax(teacher/T) || softmax(student/T)).
/// Simplified: |teacher - student| * temperature^2 * 1000 (fixed-point).
#[unsafe(no_mangle)]
pub extern "C" fn slang_distill_kd_loss(teacher_logit: f64, student_logit: f64, temperature: f64) -> i64 {
    DISTILL_OPS.fetch_add(1, Ordering::Relaxed);
    let t = temperature.max(0.01);
    let loss = (teacher_logit / t - student_logit / t).abs() * t * t;
    (loss * 1000.0).round() as i64
}

/// Feature distillation loss: MSE between teacher and student feature maps.
/// Returns loss × 1000 (fixed-point).
#[unsafe(no_mangle)]
pub extern "C" fn slang_distill_feature_loss(teacher_feat: f64, student_feat: f64) -> i64 {
    DISTILL_OPS.fetch_add(1, Ordering::Relaxed);
    let d = teacher_feat - student_feat;
    (d * d * 1000.0).round() as i64
}

/// Attention transfer: normalized attention map distance.
/// Returns distance × 1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_distill_attention_transfer(teacher_attn: f64, student_attn: f64) -> i64 {
    DISTILL_OPS.fetch_add(1, Ordering::Relaxed);
    let t_norm = teacher_attn.abs();
    let s_norm = student_attn.abs();
    ((t_norm - s_norm).abs() * 1000.0).round() as i64
}

/// Calculate effective temperature for softmax. Returns temperature × 1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_distill_temperature(base_temp: f64, epoch: i64, total_epochs: i64) -> i64 {
    // Annealing: temperature decreases linearly from base_temp to 1.0
    if total_epochs <= 0 { return (base_temp * 1000.0).round() as i64; }
    let progress = (epoch as f64 / total_epochs as f64).min(1.0);
    let temp = base_temp * (1.0 - progress) + 1.0 * progress;
    (temp * 1000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_kd_loss_identical() { assert_eq!(slang_distill_kd_loss(5.0, 5.0, 1.0), 0); }
    #[test] fn test_kd_loss_different() { let l = slang_distill_kd_loss(5.0, 3.0, 1.0); assert!(l > 0); }
    #[test] fn test_kd_loss_temperature_scaling() {
        let l1 = slang_distill_kd_loss(5.0, 3.0, 1.0);
        let l2 = slang_distill_kd_loss(5.0, 3.0, 2.0);
        assert!(l2 > l1); // higher temperature → higher scaled loss
    }
    #[test] fn test_feature_loss_same() { assert_eq!(slang_distill_feature_loss(3.0, 3.0), 0); }
    #[test] fn test_feature_loss_different() {
        let l = slang_distill_feature_loss(3.0, 1.0);
        assert_eq!(l, 4000); // (3-1)^2 * 1000 = 4000
    }
    #[test] fn test_attention_transfer_same() { assert_eq!(slang_distill_attention_transfer(0.5, 0.5), 0); }
    #[test] fn test_attention_transfer_diff() {
        let d = slang_distill_attention_transfer(0.8, 0.3);
        assert_eq!(d, 500); // |0.8 - 0.3| * 1000
    }
    #[test] fn test_temperature_start() {
        assert_eq!(slang_distill_temperature(5.0, 0, 100), 5000);
    }
    #[test] fn test_temperature_end() {
        assert_eq!(slang_distill_temperature(5.0, 100, 100), 1000); // annealed to 1.0
    }
    #[test] fn test_temperature_midpoint() {
        let t = slang_distill_temperature(5.0, 50, 100);
        assert_eq!(t, 3000); // midpoint between 5.0 and 1.0
    }
    #[test] fn test_temperature_zero_epochs() {
        assert_eq!(slang_distill_temperature(3.0, 0, 0), 3000);
    }
    #[test] fn test_kd_loss_negative() { let l = slang_distill_kd_loss(-3.0, 3.0, 1.0); assert!(l > 0); }
    #[test] fn test_feature_loss_negative() { let l = slang_distill_feature_loss(-2.0, 2.0); assert_eq!(l, 16000); }
    #[test] fn test_attention_negative() { let d = slang_distill_attention_transfer(-0.5, 0.5); assert_eq!(d, 0); }
    #[test] fn test_kd_loss_zero_temp() { let l = slang_distill_kd_loss(5.0, 3.0, 0.0); assert!(l > 0); }
    #[test] fn test_temperature_beyond_epochs() {
        assert_eq!(slang_distill_temperature(5.0, 200, 100), 1000); // clamped
    }
    #[test] fn test_feature_loss_zero() { assert_eq!(slang_distill_feature_loss(0.0, 0.0), 0); }
    #[test] fn test_kd_loss_symmetry() {
        let l1 = slang_distill_kd_loss(5.0, 3.0, 1.0);
        let l2 = slang_distill_kd_loss(3.0, 5.0, 1.0);
        assert_eq!(l1, l2);
    }
    #[test] fn test_feature_loss_large() {
        let l = slang_distill_feature_loss(100.0, 0.0);
        assert_eq!(l, 10000000); // 100^2 * 1000
    }
    #[test] fn test_temperature_negative_epoch() {
        let t = slang_distill_temperature(5.0, -1, 100);
        assert!(t >= 4000); // negative progress → above midpoint
    }
}
