//! Message Queue (Pub/Sub) — Systems Infrastructure (v335)
//!
//! Topic-based pub/sub with consumer groups and offset tracking.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

struct Topic {
    messages: Vec<i64>,
    offsets: HashMap<i64, i64>, // consumer_id → offset
}

static TOPICS: LazyLock<Mutex<HashMap<i64, Topic>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static TOPIC_COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Create a topic. Returns topic ID.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mq_create_topic() -> i64 {
    let id = TOPIC_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    TOPICS.lock().unwrap().insert(id, Topic { messages: Vec::new(), offsets: HashMap::new() });
    id
}

/// Publish a message to a topic. Returns message count.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mq_publish(topic_id: i64, message: i64) -> i64 {
    let mut topics = TOPICS.lock().unwrap();
    if let Some(t) = topics.get_mut(&topic_id) {
        t.messages.push(message);
        t.messages.len() as i64
    } else { -1 }
}

/// Subscribe a consumer. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mq_subscribe(topic_id: i64, consumer_id: i64) -> i64 {
    let mut topics = TOPICS.lock().unwrap();
    if let Some(t) = topics.get_mut(&topic_id) {
        t.offsets.entry(consumer_id).or_insert(0);
        1
    } else { -1 }
}

/// Consume the next message for a consumer. Returns message or -1 if none.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mq_consume(topic_id: i64, consumer_id: i64) -> i64 {
    let mut topics = TOPICS.lock().unwrap();
    if let Some(t) = topics.get_mut(&topic_id) {
        let offset = t.offsets.get(&consumer_id).copied().unwrap_or(0) as usize;
        if offset < t.messages.len() {
            let msg = t.messages[offset];
            t.offsets.insert(consumer_id, (offset + 1) as i64);
            msg
        } else { -1 }
    } else { -1 }
}

/// Get current offset for a consumer.
#[unsafe(no_mangle)]
pub extern "C" fn slang_mq_offset(topic_id: i64, consumer_id: i64) -> i64 {
    let topics = TOPICS.lock().unwrap();
    if let Some(t) = topics.get(&topic_id) {
        t.offsets.get(&consumer_id).copied().unwrap_or(-1)
    } else { -1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create_topic() { let id = slang_mq_create_topic(); assert!(id >= 0); }
    #[test] fn test_publish() { let id = slang_mq_create_topic(); assert_eq!(slang_mq_publish(id, 42), 1); }
    #[test] fn test_subscribe() { let id = slang_mq_create_topic(); assert_eq!(slang_mq_subscribe(id, 1), 1); }
    #[test] fn test_consume() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_publish(id, 42);
        assert_eq!(slang_mq_consume(id, 1), 42);
    }
    #[test] fn test_consume_empty() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        assert_eq!(slang_mq_consume(id, 1), -1);
    }
    #[test] fn test_offset() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_publish(id, 10);
        slang_mq_consume(id, 1);
        assert_eq!(slang_mq_offset(id, 1), 1);
    }
    #[test] fn test_multiple_consumers() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_subscribe(id, 2);
        slang_mq_publish(id, 42);
        assert_eq!(slang_mq_consume(id, 1), 42);
        assert_eq!(slang_mq_consume(id, 2), 42); // both get it
    }
    #[test] fn test_multiple_messages() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_publish(id, 10);
        slang_mq_publish(id, 20);
        slang_mq_publish(id, 30);
        assert_eq!(slang_mq_consume(id, 1), 10);
        assert_eq!(slang_mq_consume(id, 1), 20);
        assert_eq!(slang_mq_consume(id, 1), 30);
        assert_eq!(slang_mq_consume(id, 1), -1);
    }
    #[test] fn test_invalid_topic() {
        assert_eq!(slang_mq_publish(999, 1), -1);
        assert_eq!(slang_mq_subscribe(999, 1), -1);
        assert_eq!(slang_mq_consume(999, 1), -1);
    }
    #[test] fn test_offset_unsubscribed() {
        let id = slang_mq_create_topic();
        assert_eq!(slang_mq_offset(id, 99), -1);
    }
    #[test] fn test_publish_returns_count() {
        let id = slang_mq_create_topic();
        for i in 1..=5 { assert_eq!(slang_mq_publish(id, i), i); }
    }
    #[test] fn test_multiple_topics() {
        let a = slang_mq_create_topic();
        let b = slang_mq_create_topic();
        slang_mq_subscribe(a, 1);
        slang_mq_subscribe(b, 1);
        slang_mq_publish(a, 10);
        slang_mq_publish(b, 20);
        assert_eq!(slang_mq_consume(a, 1), 10);
        assert_eq!(slang_mq_consume(b, 1), 20);
    }
    #[test] fn test_consume_without_subscribe() {
        let id = slang_mq_create_topic();
        slang_mq_publish(id, 42);
        assert_eq!(slang_mq_consume(id, 99), 42); // auto offset 0
    }
    #[test] fn test_offset_advances() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        for i in 0..10 { slang_mq_publish(id, i); }
        for _ in 0..5 { slang_mq_consume(id, 1); }
        assert_eq!(slang_mq_offset(id, 1), 5);
    }
    #[test] fn test_negative_message() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_publish(id, -42);
        assert_eq!(slang_mq_consume(id, 1), -42);
    }
    #[test] fn test_zero_message() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_publish(id, 0);
        assert_eq!(slang_mq_consume(id, 1), 0);
    }
    #[test] fn test_subscribe_idempotent() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_publish(id, 10);
        slang_mq_consume(id, 1);
        slang_mq_subscribe(id, 1); // re-subscribe shouldn't reset
        assert_eq!(slang_mq_offset(id, 1), 1); // still at 1
    }
    #[test] fn test_large_batch() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        for i in 0..100 { slang_mq_publish(id, i); }
        let mut sum = 0i64;
        loop { let m = slang_mq_consume(id, 1); if m == -1 { break; } sum += m; }
        assert_eq!(sum, 4950);
    }
    #[test] fn test_consumer_independence() {
        let id = slang_mq_create_topic();
        slang_mq_subscribe(id, 1);
        slang_mq_subscribe(id, 2);
        slang_mq_publish(id, 100);
        slang_mq_consume(id, 1);
        assert_eq!(slang_mq_offset(id, 1), 1);
        assert_eq!(slang_mq_offset(id, 2), 0); // consumer 2 hasn't consumed
    }
}
