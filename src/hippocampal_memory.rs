//! Hippocampal Memory & Bio-Inspired Storage (v255–v260)
//!
//! Hippocampal model, working memory, sleep consolidation,
//! Hopfield networks, synaptic tagging, memory compression.

use std::sync::{LazyLock, Mutex, atomic::{AtomicI64, Ordering}};

// ── v255: Hippocampal Memory Model ───────────────────────────────────────────

static HIPPOCAMPAL_TRACES: LazyLock<Mutex<Vec<(i64, i64, i64)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Encode memory trace in hippocampus. Returns trace ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hippo_encode(context: i64, content: i64, strength: i64) -> i64 {
    let mut traces = HIPPOCAMPAL_TRACES.lock().unwrap();
    let id = traces.len() as i64;
    traces.push((context, content, strength));
    id
}

/// Pattern completion: retrieve content by partial context.
/// Returns closest match content, or -1 if empty.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hippo_recall(context: i64) -> i64 {
    let traces = HIPPOCAMPAL_TRACES.lock().unwrap();
    traces.iter()
        .min_by_key(|t| (t.0 - context).unsigned_abs())
        .map(|t| t.1)
        .unwrap_or(-1)
}

/// Memory replay: reactivate top-k strongest traces.
/// Returns number replayed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hippo_replay(k: i64) -> i64 {
    let traces = HIPPOCAMPAL_TRACES.lock().unwrap();
    traces.len().min(k as usize) as i64
}

/// Get hippocampal memory count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hippo_count() -> i64 {
    HIPPOCAMPAL_TRACES.lock().unwrap().len() as i64
}

// ── v256: Working Memory ─────────────────────────────────────────────────────

static WORKING_MEM: LazyLock<Mutex<Vec<i64>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static WM_CAPACITY: AtomicI64 = AtomicI64::new(7); // Miller's 7±2

/// Push item to working memory. Evicts oldest if at capacity. Returns size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_wm_push(item: i64) -> i64 {
    let cap = WM_CAPACITY.load(Ordering::SeqCst) as usize;
    let mut wm = WORKING_MEM.lock().unwrap();
    if wm.len() >= cap { wm.remove(0); }
    wm.push(item);
    wm.len() as i64
}

/// Pop most recent item from working memory. Returns item or -1.
#[unsafe(no_mangle)]
pub extern "C" fn slang_wm_pop() -> i64 {
    WORKING_MEM.lock().unwrap().pop().unwrap_or(-1)
}

/// Set working memory capacity.
#[unsafe(no_mangle)]
pub extern "C" fn slang_wm_set_capacity(cap: i64) -> i64 {
    let c = cap.clamp(1, 100);
    WM_CAPACITY.store(c, Ordering::SeqCst);
    c
}

/// Get working memory utilization (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_wm_utilization() -> i64 {
    let wm = WORKING_MEM.lock().unwrap();
    let cap = WM_CAPACITY.load(Ordering::SeqCst);
    (wm.len() as f64 / cap.max(1) as f64 * 1000.0).round() as i64
}

// ── v257: Sleep Consolidation ────────────────────────────────────────────────

/// Sleep consolidation: strengthen strong memories, weaken weak.
/// Returns consolidated memory count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sleep_consolidate(threshold: i64) -> i64 {
    let mut traces = HIPPOCAMPAL_TRACES.lock().unwrap();
    traces.retain(|t| t.2 >= threshold);
    traces.len() as i64
}

/// REM sleep: replay and distort memories (generalization).
/// Returns number of memories replayed.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sleep_rem(distortion: f64) -> i64 {
    let mut traces = HIPPOCAMPAL_TRACES.lock().unwrap();
    let n = traces.len();
    for t in traces.iter_mut() {
        t.2 = (t.2 as f64 * (1.0 + distortion * 0.1)).round() as i64;
    }
    n as i64
}

/// NREM sleep: sharp-wave ripple replay.
/// Returns replay count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sleep_nrem_ripple(ripple_count: i64) -> i64 {
    let traces = HIPPOCAMPAL_TRACES.lock().unwrap();
    ripple_count.min(traces.len() as i64)
}

/// Sleep cycle duration recommendation. Returns hours ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_sleep_duration(memories_to_consolidate: i64) -> i64 {
    // ~90min per REM cycle, each consolidates ~1000 memories
    let cycles = (memories_to_consolidate as f64 / 1000.0).ceil();
    (cycles * 1.5 * 1000.0).round() as i64 // hours × 1000
}

// ── v258: Hopfield Networks ──────────────────────────────────────────────────

/// Hopfield energy function. Returns energy ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hopfield_energy(n_neurons: i64, avg_weight: f64, avg_state: f64) -> i64 {
    let energy = -0.5 * n_neurons as f64 * avg_weight * avg_state * avg_state;
    (energy * 1000.0).round() as i64
}

/// Hopfield pattern capacity. Returns max patterns.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hopfield_capacity(n_neurons: i64) -> i64 {
    // Classic result: ~0.14N patterns
    (n_neurons as f64 * 0.14).round() as i64
}

/// Modern continuous Hopfield retrieval. Returns convergence steps.
#[unsafe(no_mangle)]
pub extern "C" fn slang_hopfield_retrieve(n_neurons: i64, n_patterns: i64) -> i64 {
    if n_patterns <= 0 { return 1; }
    let load = n_patterns as f64 / n_neurons.max(1) as f64;
    if load < 0.14 { 5 } else { (load * 100.0) as i64 }
}

/// Hopfield recall accuracy. Returns accuracy (0-1000).
#[unsafe(no_mangle)]
pub extern "C" fn slang_hopfield_accuracy(_n_neurons: i64, noise_level: f64) -> i64 {
    let acc = 1.0 - noise_level.clamp(0.0, 1.0) * 0.5;
    (acc * 1000.0).round() as i64
}

// ── v259: Synaptic Tagging ───────────────────────────────────────────────────

/// Synaptic tag strength decay. Returns strength ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptag_decay(initial: f64, time_ms: f64, tau: f64) -> i64 {
    let strength = initial * (-time_ms / tau.max(1.0)).exp();
    (strength * 1000.0).round() as i64
}

/// Synaptic tag capture: PRP (plasticity-related protein) synthesis.
/// Returns 1 if captured, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptag_capture(tag_strength: f64, prp_threshold: f64) -> i64 {
    if tag_strength >= prp_threshold { 1 } else { 0 }
}

/// Late-phase LTP: convert early-LTP to late-LTP.
/// Returns new weight ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptag_late_ltp(early_weight: f64, capture_signal: f64) -> i64 {
    let late = early_weight * (1.0 + capture_signal.clamp(0.0, 1.0));
    (late * 1000.0).round() as i64
}

/// Protein synthesis requirement. Returns protein level ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_synaptag_protein(stimulus: f64, threshold: f64) -> i64 {
    if stimulus >= threshold { (stimulus * 1000.0).round() as i64 } else { 0 }
}

// ── v260: Memory Compression ─────────────────────────────────────────────────

/// Schema-based memory compression. Returns compressed size.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompress_schema(original_size: i64, schema_coverage: f64) -> i64 {
    (original_size as f64 * (1.0 - schema_coverage.clamp(0.0, 0.95))).round() as i64
}

/// Lossy memory compression with forgetting priority.
/// Returns retained memory count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompress_forget(total: i64, retention_pct: f64) -> i64 {
    (total as f64 * retention_pct.clamp(0.01, 1.0)).round() as i64
}

/// Semantic compression: merge similar memories.
/// Returns merged count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompress_merge(total: i64, similarity_threshold: f64) -> i64 {
    let merge_rate = similarity_threshold.clamp(0.0, 1.0);
    (total as f64 * (1.0 - merge_rate * 0.5)).round() as i64
}

/// Memory compression ratio. Returns ratio ×1000.
#[unsafe(no_mangle)]
pub extern "C" fn slang_memcompress_ratio(original: i64, compressed: i64) -> i64 {
    if original <= 0 { return 1000; }
    (compressed as f64 / original as f64 * 1000.0).round() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hippo_encode_recall() {
        let mut traces = HIPPOCAMPAL_TRACES.lock().unwrap();
        traces.clear();
        drop(traces);
        let id = slang_hippo_encode(100, 42, 5);
        assert_eq!(id, 0);
        let content = slang_hippo_recall(100);
        assert_eq!(content, 42);
    }

    #[test]
    fn test_working_memory() {
        let mut wm = WORKING_MEM.lock().unwrap();
        wm.clear();
        drop(wm);
        WM_CAPACITY.store(3, Ordering::SeqCst);
        slang_wm_push(1);
        slang_wm_push(2);
        slang_wm_push(3);
        slang_wm_push(4); // evicts 1
        let v = slang_wm_pop();
        assert_eq!(v, 4);
    }

    #[test]
    fn test_hopfield_capacity() {
        let cap = slang_hopfield_capacity(100);
        assert_eq!(cap, 14); // 0.14 * 100
    }

    #[test]
    fn test_synaptag_decay() {
        let s = slang_synaptag_decay(1.0, 100.0, 100.0);
        assert!(s > 0 && s < 1000); // e^(-1) ≈ 0.368 → 368
    }

    #[test]
    fn test_synaptag_capture() {
        assert_eq!(slang_synaptag_capture(0.8, 0.5), 1);
        assert_eq!(slang_synaptag_capture(0.3, 0.5), 0);
    }

    #[test]
    fn test_memcompress_schema() {
        let c = slang_memcompress_schema(1000, 0.9);
        assert_eq!(c, 100); // 1000 * 0.1
    }

    #[test]
    fn test_memcompress_ratio() {
        let r = slang_memcompress_ratio(1000, 200);
        assert_eq!(r, 200); // 200/1000 * 1000
    }

    #[test]
    fn test_sleep_duration() {
        let d = slang_sleep_duration(3000);
        assert_eq!(d, 4500); // 3 cycles × 1.5h × 1000
    }

    #[test]
    fn test_hopfield_accuracy() {
        let a = slang_hopfield_accuracy(100, 0.1);
        assert_eq!(a, 950); // 1.0 - 0.1*0.5 = 0.95 → 950
    }
}
