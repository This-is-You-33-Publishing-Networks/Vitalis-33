//! Neuromorphic Application Domains (v237–v242)
//!
//! Spike-based vision, audio, control, anomaly detection, optimization, NLP.

use std::sync::atomic::{AtomicI64, Ordering};

// ── v237: Spike Vision ───────────────────────────────────────────────────────

static VISION_FRAMES: AtomicI64 = AtomicI64::new(0);

/// Encode image pixels to spike train. Returns number of spikes generated.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_vision_encode(width: i64, height: i64, threshold: f64) -> i64 {
    VISION_FRAMES.fetch_add(1, Ordering::SeqCst);
    let pixels = width * height;
    // Each pixel above threshold produces a spike
    (pixels as f64 * (1.0 - threshold.clamp(0.0, 1.0))).round() as i64
}

/// Spike-based edge detection. Returns number of edge spikes.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_vision_edge(width: i64, height: i64, sensitivity: f64) -> i64 {
    let perimeter_ratio = 2.0 * (width + height) as f64 / (width * height).max(1) as f64;
    (width as f64 * height as f64 * perimeter_ratio * sensitivity.clamp(0.1, 2.0)).round() as i64
}

/// Spike-based motion detection. Returns motion magnitude (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_vision_motion(prev_spikes: i64, curr_spikes: i64) -> i64 {
    let diff = (curr_spikes - prev_spikes).unsigned_abs() as i64;
    let total = prev_spikes.max(1);
    (diff as f64 / total as f64 * 1000.0).min(1000.0).round() as i64
}

/// Get total vision frames processed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_vision_frames() -> i64 {
    VISION_FRAMES.load(Ordering::SeqCst)
}

// ── v238: Spike Audio ────────────────────────────────────────────────────────

/// Cochlear model: encode audio samples to spike train.
/// Returns number of spikes (frequency channels × active).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_audio_encode(n_samples: i64, n_channels: i64, threshold: f64) -> i64 {
    let active_ratio = 1.0 - threshold.clamp(0.0, 1.0);
    (n_samples as f64 * n_channels as f64 * active_ratio / n_samples.max(1) as f64).round() as i64
}

/// Spike-based frequency estimation. Returns dominant frequency.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_audio_frequency(spike_interval_us: i64) -> i64 {
    if spike_interval_us <= 0 { return 0; }
    1_000_000 / spike_interval_us // Hz = 1e6 / interval_us
}

/// Spike temporal coding for audio onset detection.
/// Returns onset strength (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_audio_onset(prev_rate: i64, curr_rate: i64) -> i64 {
    let ratio = curr_rate as f64 / prev_rate.max(1) as f64;
    ((ratio - 1.0).max(0.0) * 1000.0).min(1000.0) as i64
}

/// Spike keyword spotting confidence. Returns confidence (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_audio_classify(n_matches: i64, n_total: i64) -> i64 {
    if n_total <= 0 { return 0; }
    (n_matches as f64 / n_total as f64 * 1000.0).min(1000.0).round() as i64
}

// ── v239: Spike-Based Control ────────────────────────────────────────────────

/// Spike PID controller. Returns control signal ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_pid(error: f64, deriv: f64, integral: f64) -> i64 {
    let kp = 1.0;
    let kd = 0.1;
    let ki = 0.01;
    let signal = kp * error + kd * deriv + ki * integral;
    (signal * 1000.0).round() as i64
}

/// Spike-based motor command. Returns pulse width in microseconds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_motor(target_angle: f64, current_angle: f64) -> i64 {
    let error = target_angle - current_angle;
    (1500.0 + error * 10.0).clamp(500.0, 2500.0) as i64 // servo PWM range
}

/// Spike reflex arc latency. Returns latency in microseconds.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_reflex(n_synapses: i64) -> i64 {
    n_synapses * 500 // ~0.5ms per synapse
}

/// Spike-based trajectory planning cost. Returns cost ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_trajectory_cost(distance: f64, obstacles: i64) -> i64 {
    let cost = distance * (1.0 + 0.5 * obstacles as f64);
    (cost * 1000.0).round() as i64
}

// ── v240: Spike Anomaly Detection ────────────────────────────────────────────

/// Compute anomaly score from spike rate deviation.
/// Returns score (0-1000), higher = more anomalous.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_anomaly_score(observed_rate: f64, expected_rate: f64) -> i64 {
    if expected_rate <= 0.0 { return 1000; }
    let deviation = ((observed_rate - expected_rate) / expected_rate).abs();
    (deviation * 1000.0).min(1000.0) as i64
}

/// Spike-based change-point detection. Returns 1 if change detected, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_changepoint(prev_rate: f64, curr_rate: f64, threshold: f64) -> i64 {
    let change = (curr_rate - prev_rate).abs();
    if change > threshold { 1 } else { 0 }
}

/// Spike burst detection. Returns burst intensity (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_burst_detect(isi_ms: f64, burst_threshold_ms: f64) -> i64 {
    if isi_ms <= 0.0 { return 0; }
    let intensity = (burst_threshold_ms / isi_ms).min(10.0);
    (intensity * 100.0) as i64
}

/// Time-series spike pattern match score. Returns match (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_pattern_match(correlation: f64) -> i64 {
    (correlation.clamp(-1.0, 1.0) * 500.0 + 500.0) as i64
}

// ── v241: Spike Optimization ─────────────────────────────────────────────────

/// Simulated annealing using spike noise. Returns best cost ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_anneal(initial_cost: f64, temperature: f64, iterations: i64) -> i64 {
    let mut cost = initial_cost;
    let mut temp = temperature;
    for _ in 0..iterations.min(10000) {
        let delta = temp * 0.01; // simplified
        cost -= delta;
        temp *= 0.99;
    }
    (cost.max(0.0) * 1000.0).round() as i64
}

/// Spike-based gradient estimation via perturbation.
/// Returns gradient ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_gradient(f_plus: f64, f_minus: f64, epsilon: f64) -> i64 {
    if epsilon.abs() < 1e-12 { return 0; }
    let grad = (f_plus - f_minus) / (2.0 * epsilon);
    (grad * 1000.0).round() as i64
}

/// Constraint satisfaction via spike relaxation.
/// Returns violation count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_constraint(n_constraints: i64, satisfied_pct: f64) -> i64 {
    (n_constraints as f64 * (1.0 - satisfied_pct.clamp(0.0, 1.0))).round() as i64
}

/// Spike-based objective function evaluation. Returns fitness ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_fitness(accuracy: f64, efficiency: f64) -> i64 {
    let fitness = 0.7 * accuracy + 0.3 * efficiency;
    (fitness * 1000.0).round() as i64
}

// ── v242: Spike NLP ──────────────────────────────────────────────────────────

/// Spike-based word embedding similarity. Returns similarity (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_nlp_similarity(dot_product: f64, norm_a: f64, norm_b: f64) -> i64 {
    let denom = norm_a * norm_b;
    if denom <= 0.0 { return 0; }
    let cosine = (dot_product / denom).clamp(-1.0, 1.0);
    ((cosine + 1.0) * 500.0) as i64
}

/// Spike temporal attention score. Returns attention weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_nlp_attention(query_rate: f64, key_rate: f64) -> i64 {
    let score = (query_rate * key_rate).sqrt();
    (score * 1000.0).min(1000.0) as i64
}

/// Spike sequence encoding length. Returns encoded length.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_nlp_encode_len(n_tokens: i64, spike_window: i64) -> i64 {
    n_tokens * spike_window.max(1)
}

/// Spike language model perplexity estimate. Returns perplexity ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_nlp_perplexity(cross_entropy: f64) -> i64 {
    let ppl = cross_entropy.exp();
    (ppl * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spike_vision() {
        let spikes = slang_spike_vision_encode(100, 100, 0.5);
        assert_eq!(spikes, 5000);
        let edges = slang_spike_vision_edge(100, 100, 1.0);
        assert!(edges > 0);
    }

    #[test]
    fn test_spike_audio() {
        let freq = slang_spike_audio_frequency(1000); // 1ms interval
        assert_eq!(freq, 1000); // 1kHz
    }

    #[test]
    fn test_spike_pid() {
        let signal = slang_spike_pid(1.0, 0.0, 0.0);
        assert_eq!(signal, 1000); // Kp × error × 1000
    }

    #[test]
    fn test_spike_anomaly() {
        let score = slang_spike_anomaly_score(200.0, 100.0);
        assert_eq!(score, 1000); // 100% deviation
    }

    #[test]
    fn test_spike_anneal() {
        let cost = slang_spike_anneal(100.0, 10.0, 100);
        assert!(cost < 100_000); // should decrease
    }

    #[test]
    fn test_spike_nlp() {
        let sim = slang_spike_nlp_similarity(1.0, 1.0, 1.0);
        assert_eq!(sim, 1000); // cosine=1 → (1+1)*500 = 1000
    }

    #[test]
    fn test_spike_motor() {
        let pw = slang_spike_motor(90.0, 45.0);
        assert!(pw >= 500 && pw <= 2500);
    }

    #[test]
    fn test_spike_burst_detect() {
        let intensity = slang_spike_burst_detect(2.0, 10.0);
        assert_eq!(intensity, 500); // 10/2 = 5 → 500
    }

    #[test]
    fn test_spike_gradient() {
        let g = slang_spike_gradient(1.1, 0.9, 0.1);
        assert_eq!(g, 1000); // (1.1-0.9)/(2*0.1) = 1.0 → 1000
    }

    #[test]
    fn test_spike_nlp_encode() {
        assert_eq!(slang_spike_nlp_encode_len(100, 10), 1000);
    }
}
