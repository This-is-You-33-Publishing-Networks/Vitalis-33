//! Brain-Inspired Architectures (v225–v230)
//!
//! Predictive coding, HTM, neural oscillations, neuromodulation,
//! cortical columns, spike-based attention.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};

// ── v225: Predictive Coding Networks ─────────────────────────────────────────

/// Predictive coding forward: top-down prediction vs bottom-up input.
/// Returns prediction error as fixed-point ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_coding_forward(input: f64, prediction: f64, precision: f64) -> i64 {
    let error = (input - prediction) * precision;
    (error * 1000.0).round() as i64
}

/// Compute prediction error for hierarchical layer.
/// error = precision * (input - prediction)^2.
/// Returns error ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_coding_error(input: f64, prediction: f64, precision: f64) -> i64 {
    let err = precision * (input - prediction).powi(2);
    (err * 1000.0).round() as i64
}

/// Update prediction: pred += lr * error.
/// Returns updated prediction ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_coding_update(prediction: f64, error: f64, lr: f64) -> i64 {
    let updated = prediction + lr * error;
    (updated * 1000.0).round() as i64
}

/// Get effective layer count for predictive coding hierarchy.
/// depth = log2(input_size / min_layer_size).
#[unsafe(no_mangle)]
pub extern "C" fn slang_pred_coding_layers(input_size: i64, min_size: i64) -> i64 {
    if min_size <= 0 || input_size <= 0 { return 0; }
    let ratio = input_size as f64 / min_size as f64;
    ratio.log2().ceil() as i64
}

// ── v226: Hierarchical Temporal Memory (HTM) ─────────────────────────────────

static HTM_COLUMNS: AtomicI64 = AtomicI64::new(0);

/// HTM spatial pooler: compute column activations from input overlap.
/// Returns number of active columns (top-k by overlap score).
#[unsafe(no_mangle)]
pub extern "C" fn slang_htm_spatial_pool(n_columns: i64, n_active_bits: i64, sparsity_pct: i64) -> i64 {
    let target_active = (n_columns as f64 * sparsity_pct as f64 / 100.0) as i64;
    HTM_COLUMNS.store(n_columns, Ordering::SeqCst);
    target_active.min(n_active_bits).max(1)
}

/// HTM temporal memory: predict next active columns based on context.
/// Returns number of predicted columns.
#[unsafe(no_mangle)]
pub extern "C" fn slang_htm_temporal_memory(active_columns: i64, n_cells_per_col: i64) -> i64 {
    // Each active column activates cells; predicted = cells that were previously active + context
    let predicted = (active_columns as f64 * 0.7) as i64; // ~70% prediction accuracy
    predicted * n_cells_per_col
}

/// HTM anomaly score: 1.0 = fully anomalous, 0.0 = fully predicted.
/// Returns score ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_htm_anomaly_score(predicted: i64, actual: i64) -> i64 {
    if actual <= 0 { return 0; }
    let overlap = predicted.min(actual);
    let score = 1.0 - (overlap as f64 / actual as f64);
    (score * 1000.0).round() as i64
}

/// Get total HTM column count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_htm_column_count() -> i64 {
    HTM_COLUMNS.load(Ordering::SeqCst)
}

// ── v227: Neural Oscillation Networks ────────────────────────────────────────

/// Gamma oscillation (30-100 Hz): compute phase at time t.
/// Returns phase in milliradians.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_osc_gamma(t: f64, freq: f64, amplitude: f64) -> i64 {
    let phase = amplitude * (2.0 * std::f64::consts::PI * freq * t).sin();
    (phase * 1000.0).round() as i64
}

/// Theta oscillation (4-8 Hz): compute phase at time t.
/// Returns phase in milliradians.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_osc_theta(t: f64, freq: f64, amplitude: f64) -> i64 {
    let phase = amplitude * (2.0 * std::f64::consts::PI * freq * t).sin();
    (phase * 1000.0).round() as i64
}

/// Couple two oscillators: compute coupling term.
/// coupling_force = K * sin(phase_b - phase_a).
/// Returns coupling ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_osc_couple(phase_a: f64, phase_b: f64, coupling_k: f64) -> i64 {
    let coupling = coupling_k * (phase_b - phase_a).sin();
    (coupling * 1000.0).round() as i64
}

/// Phase-locking value between two oscillators.
/// PLV = |mean(exp(i*(phase_a - phase_b)))|. Approximated.
/// Returns PLV ×1000 (1000 = perfect locking).
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuro_osc_phase_lock(_phase_diff_mean: f64, phase_diff_var: f64) -> i64 {
    // PLV ≈ exp(-variance/2) for von Mises distribution
    let plv = (-phase_diff_var / 2.0).exp();
    (plv * 1000.0).round() as i64
}

// ── v228: Neuromodulation System ─────────────────────────────────────────────

/// Dopamine signal: reward prediction error modulation.
/// da = reward - expected_reward. Returns DA level ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuromod_dopamine(reward: f64, expected: f64) -> i64 {
    let da = reward - expected;
    (da * 1000.0).round() as i64
}

/// Serotonin signal: modulates patience/temporal discounting.
/// 5HT = baseline + mood_factor. Returns level ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuromod_serotonin(baseline: f64, mood: f64) -> i64 {
    let level = (baseline + mood).clamp(0.0, 2.0);
    (level * 1000.0).round() as i64
}

/// Acetylcholine signal: attention modulation.
/// ACh = stimulus_salience * arousal. Returns level ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuromod_acetylcholine(salience: f64, arousal: f64) -> i64 {
    let level = (salience * arousal).clamp(0.0, 2.0);
    (level * 1000.0).round() as i64
}

/// Apply neuromodulatory gain to synaptic weight.
/// effective_weight = weight * (1.0 + modulator_level * gain_factor).
/// Returns effective weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_neuromod_apply(weight: f64, modulator: f64, gain: f64) -> i64 {
    let effective = weight * (1.0 + modulator * gain);
    (effective * 1000.0).round() as i64
}

// ── v229: Cortical Column Models ─────────────────────────────────────────────

static CORTICAL_COLUMNS: LazyLock<Mutex<Vec<[f64; 6]>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Create a cortical column with 6 layers (I-VI). Returns column ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cortical_column_create() -> i64 {
    let mut cols = CORTICAL_COLUMNS.lock().unwrap();
    let id = cols.len() as i64;
    cols.push([0.0; 6]); // 6 layers, initially silent
    id
}

/// Step a cortical column: inject input to layer 4, propagate up/down.
/// Returns total activity across all layers ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cortical_column_step(col_id: i64, input: f64) -> i64 {
    let mut cols = CORTICAL_COLUMNS.lock().unwrap();
    let idx = col_id as usize;
    if idx >= cols.len() { return -1; }
    let col = &mut cols[idx];
    // Layer 4 receives thalamic input
    col[3] = input; // L4
    // Feedforward: L4 → L2/3
    col[1] = col[3] * 0.8;
    // L2/3 → L5
    col[4] = col[1] * 0.7;
    // L5 → L6 (feedback to thalamus)
    col[5] = col[4] * 0.6;
    // L6 → L4 (modulatory feedback)
    col[3] += col[5] * 0.1;
    // L1 (apical dendrites, top-down)
    col[0] = col[1] * 0.3;
    // L2/3 → output
    col[2] = col[1] * 0.9;
    let total: f64 = col.iter().sum();
    (total * 1000.0).round() as i64
}

/// Get activity at a specific layer (0-5) of a column.
/// Returns activity ×1000, or -1 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cortical_column_layer_activity(col_id: i64, layer: i64) -> i64 {
    let cols = CORTICAL_COLUMNS.lock().unwrap();
    let idx = col_id as usize;
    let l = layer as usize;
    if idx >= cols.len() || l >= 6 { return -1; }
    (cols[idx][l] * 1000.0).round() as i64
}

/// Return number of cortical columns.
#[unsafe(no_mangle)]
pub extern "C" fn slang_cortical_column_count() -> i64 {
    CORTICAL_COLUMNS.lock().unwrap().len() as i64
}

// ── v230: Spike-Based Attention ──────────────────────────────────────────────

/// Spike-based attention: compute query-key similarity via spike timing.
/// Closer spike times = higher attention. Returns score ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_attention_query(query_time: f64, key_time: f64, tau: f64) -> i64 {
    let dt = (query_time - key_time).abs();
    let score = (-dt / tau).exp();
    (score * 1000.0).round() as i64
}

/// Compute attention key from spike pattern.
/// key = hash of (neuron_id, spike_time). Returns key value.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_attention_key(neuron_id: i64, spike_time: i64) -> i64 {
    neuron_id.wrapping_mul(2654435761).wrapping_add(spike_time)
}

/// Compute attention value: weighted spike count.
/// Returns value ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_attention_value(spike_count: i64, weight: f64) -> i64 {
    (spike_count as f64 * weight * 1000.0).round() as i64
}

/// Compute softmax-style attention score from unnormalized logits.
/// score = exp(logit) / (exp(logit) + n_others). Returns ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_spike_attention_score(logit: f64, n_others: i64) -> i64 {
    let exp_logit = logit.exp();
    let score = exp_logit / (exp_logit + n_others as f64);
    (score * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // v225
    #[test]
    fn test_pred_coding() {
        let err = slang_pred_coding_forward(1.0, 0.8, 1.0);
        assert_eq!(err, 200); // (1.0 - 0.8) * 1000
        let updated = slang_pred_coding_update(0.8, 0.2, 0.1);
        assert_eq!(updated, 820); // (0.8 + 0.1*0.2) * 1000
    }

    #[test]
    fn test_pred_coding_layers() {
        assert_eq!(slang_pred_coding_layers(1024, 16), 6); // log2(64)=6
    }

    // v226
    #[test]
    fn test_htm_spatial_pool() {
        let active = slang_htm_spatial_pool(2048, 100, 2);
        assert!(active > 0 && active <= 100);
    }

    #[test]
    fn test_htm_anomaly() {
        assert_eq!(slang_htm_anomaly_score(0, 10), 1000); // fully anomalous
        assert_eq!(slang_htm_anomaly_score(10, 10), 0);   // fully predicted
    }

    // v227
    #[test]
    fn test_osc_gamma() {
        let phase = slang_neuro_osc_gamma(0.0, 40.0, 1.0);
        assert_eq!(phase, 0); // sin(0) = 0
    }

    #[test]
    fn test_osc_phase_lock() {
        let plv = slang_neuro_osc_phase_lock(0.0, 0.0);
        assert_eq!(plv, 1000); // perfect locking
    }

    // v228
    #[test]
    fn test_neuromod_dopamine() {
        assert_eq!(slang_neuromod_dopamine(1.0, 0.5), 500); // positive RPE
        assert_eq!(slang_neuromod_dopamine(0.0, 1.0), -1000); // negative RPE
    }

    #[test]
    fn test_neuromod_apply() {
        let w = slang_neuromod_apply(1.0, 0.5, 1.0);
        assert_eq!(w, 1500); // 1.0 * (1 + 0.5*1.0) = 1.5
    }

    // v229
    #[test]
    fn test_cortical_column() {
        let col = slang_cortical_column_create();
        let activity = slang_cortical_column_step(col, 1.0);
        assert!(activity > 0);
        let l4 = slang_cortical_column_layer_activity(col, 3);
        assert!(l4 > 0);
        assert!(slang_cortical_column_count() > 0);
    }

    #[test]
    fn test_cortical_column_invalid() {
        assert_eq!(slang_cortical_column_step(999999, 1.0), -1);
        assert_eq!(slang_cortical_column_layer_activity(999999, 0), -1);
    }

    // v230
    #[test]
    fn test_spike_attention() {
        let score = slang_spike_attention_query(10.0, 10.0, 1.0);
        assert_eq!(score, 1000); // same time → max attention
        let score2 = slang_spike_attention_query(10.0, 15.0, 1.0);
        assert!(score2 < score); // different time → less attention
    }

    #[test]
    fn test_spike_attention_score() {
        let s = slang_spike_attention_score(0.0, 1);
        assert_eq!(s, 500); // e^0 / (e^0 + 1) = 0.5
    }
}
