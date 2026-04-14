//! Stream Processing — Systems Infrastructure (v334)
//!
//! Windowing (tumbling, sliding), watermarks, and late data handling.

use std::sync::{LazyLock, Mutex};

struct StreamState {
    tumbling_window: Vec<i64>,
    tumbling_size: i64,
    sliding_window: Vec<i64>,
    sliding_size: i64,
    watermark: i64,
    late_count: i64,
}

static STREAMS: LazyLock<Mutex<Vec<StreamState>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Create stream with tumbling window of given size. Returns stream ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_create(tumbling_size: i64, sliding_size: i64) -> i64 {
    let mut streams = STREAMS.lock().unwrap();
    let id = streams.len() as i64;
    streams.push(StreamState {
        tumbling_window: Vec::new(), tumbling_size: tumbling_size.max(1),
        sliding_window: Vec::new(), sliding_size: sliding_size.max(1),
        watermark: 0, late_count: 0,
    });
    id
}

/// Add element to tumbling window. Returns window sum when full, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_window_tumbling(stream_id: i64, value: i64) -> i64 {
    let mut streams = STREAMS.lock().unwrap();
    let idx = stream_id as usize;
    if idx >= streams.len() { return -1; }
    let s = &mut streams[idx];
    s.tumbling_window.push(value);
    if s.tumbling_window.len() as i64 >= s.tumbling_size {
        let sum: i64 = s.tumbling_window.iter().sum();
        s.tumbling_window.clear();
        sum
    } else { 0 }
}

/// Add element to sliding window. Returns window sum (always available).
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_window_sliding(stream_id: i64, value: i64) -> i64 {
    let mut streams = STREAMS.lock().unwrap();
    let idx = stream_id as usize;
    if idx >= streams.len() { return -1; }
    let s = &mut streams[idx];
    s.sliding_window.push(value);
    if s.sliding_window.len() as i64 > s.sliding_size {
        s.sliding_window.remove(0);
    }
    s.sliding_window.iter().sum()
}

/// Advance watermark to given timestamp. Returns count of late events.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_watermark(stream_id: i64, timestamp: i64) -> i64 {
    let mut streams = STREAMS.lock().unwrap();
    let idx = stream_id as usize;
    if idx >= streams.len() { return -1; }
    let s = &mut streams[idx];
    if timestamp < s.watermark { s.late_count += 1; }
    else { s.watermark = timestamp; }
    s.late_count
}

/// Return total late event count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_stream_late_count(stream_id: i64) -> i64 {
    let streams = STREAMS.lock().unwrap();
    let idx = stream_id as usize;
    if idx >= streams.len() { return -1; }
    streams[idx].late_count
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { let id = slang_stream_create(5, 3); assert!(id >= 0); }
    #[test] fn test_tumbling_not_full() {
        let id = slang_stream_create(3, 1);
        assert_eq!(slang_stream_window_tumbling(id, 10), 0);
    }
    #[test] fn test_tumbling_full() {
        let id = slang_stream_create(3, 1);
        slang_stream_window_tumbling(id, 10);
        slang_stream_window_tumbling(id, 20);
        assert_eq!(slang_stream_window_tumbling(id, 30), 60);
    }
    #[test] fn test_sliding() {
        let id = slang_stream_create(1, 3);
        assert_eq!(slang_stream_window_sliding(id, 10), 10);
        assert_eq!(slang_stream_window_sliding(id, 20), 30);
        assert_eq!(slang_stream_window_sliding(id, 30), 60);
        assert_eq!(slang_stream_window_sliding(id, 40), 90); // slides: 20+30+40
    }
    #[test] fn test_watermark() {
        let id = slang_stream_create(1, 1);
        slang_stream_watermark(id, 100);
        slang_stream_watermark(id, 200);
        assert_eq!(slang_stream_watermark(id, 50), 1); // late!
    }
    #[test] fn test_late_count() {
        let id = slang_stream_create(1, 1);
        slang_stream_watermark(id, 100);
        slang_stream_watermark(id, 50);
        slang_stream_watermark(id, 30);
        assert_eq!(slang_stream_late_count(id), 2);
    }
    #[test] fn test_invalid_stream() {
        assert_eq!(slang_stream_window_tumbling(999, 1), -1);
        assert_eq!(slang_stream_window_sliding(999, 1), -1);
        assert_eq!(slang_stream_watermark(999, 1), -1);
        assert_eq!(slang_stream_late_count(999), -1);
    }
    #[test] fn test_tumbling_multiple_windows() {
        let id = slang_stream_create(2, 1);
        slang_stream_window_tumbling(id, 1);
        assert_eq!(slang_stream_window_tumbling(id, 2), 3);
        slang_stream_window_tumbling(id, 3);
        assert_eq!(slang_stream_window_tumbling(id, 4), 7);
    }
    #[test] fn test_sliding_window_one() {
        let id = slang_stream_create(1, 1);
        assert_eq!(slang_stream_window_sliding(id, 10), 10);
        assert_eq!(slang_stream_window_sliding(id, 20), 20); // replaces
    }
    #[test] fn test_no_late_events() {
        let id = slang_stream_create(1, 1);
        slang_stream_watermark(id, 10);
        slang_stream_watermark(id, 20);
        slang_stream_watermark(id, 30);
        assert_eq!(slang_stream_late_count(id), 0);
    }
    #[test] fn test_tumbling_size_one() {
        let id = slang_stream_create(1, 1);
        assert_eq!(slang_stream_window_tumbling(id, 42), 42);
    }
    #[test] fn test_watermark_equal() {
        let id = slang_stream_create(1, 1);
        slang_stream_watermark(id, 100);
        assert_eq!(slang_stream_watermark(id, 100), 0); // not late
    }
    #[test] fn test_negative_values() {
        let id = slang_stream_create(2, 2);
        slang_stream_window_tumbling(id, -5);
        assert_eq!(slang_stream_window_tumbling(id, 10), 5);
    }
    #[test] fn test_large_window() {
        let id = slang_stream_create(100, 1);
        for i in 1..100 { slang_stream_window_tumbling(id, 1); }
        assert_eq!(slang_stream_window_tumbling(id, 1), 100);
    }
    #[test] fn test_sliding_large() {
        let id = slang_stream_create(1, 10);
        for i in 1..=10 { slang_stream_window_sliding(id, i); }
        assert_eq!(slang_stream_window_sliding(id, 11), 65); // 2..11 sum
    }
    #[test] fn test_watermark_monotonic() {
        let id = slang_stream_create(1, 1);
        for i in 0..10 { slang_stream_watermark(id, i * 10); }
        assert_eq!(slang_stream_late_count(id), 0);
    }
    #[test] fn test_multiple_streams() {
        let a = slang_stream_create(2, 1);
        let b = slang_stream_create(3, 1);
        slang_stream_window_tumbling(a, 5);
        assert_eq!(slang_stream_window_tumbling(a, 5), 10);
        slang_stream_window_tumbling(b, 5);
        slang_stream_window_tumbling(b, 5);
        assert_eq!(slang_stream_window_tumbling(b, 5), 15);
    }
    #[test] fn test_zero_tumbling_clamped() {
        let id = slang_stream_create(0, 1); // clamped to 1
        assert_eq!(slang_stream_window_tumbling(id, 42), 42);
    }
    #[test] fn test_sliding_empty() {
        let id = slang_stream_create(1, 5);
        assert_eq!(slang_stream_window_sliding(id, 0), 0);
    }
    #[test] fn test_late_count_accumulates() {
        let id = slang_stream_create(1, 1);
        slang_stream_watermark(id, 100);
        for _ in 0..5 { slang_stream_watermark(id, 1); } // 5 late events
        assert_eq!(slang_stream_late_count(id), 5);
    }
}
