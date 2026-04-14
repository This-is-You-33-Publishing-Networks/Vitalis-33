//! Alias Analysis — v377
//! Points-to analysis with must/may/no alias queries and type-based alias analysis.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    pub id: i64,
    pub base: i64,
    pub offset: i64,
    pub size: i64,
    pub type_tag: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasResult {
    MustAlias,
    MayAlias,
    NoAlias,
}

#[derive(Debug)]
pub struct AliasAnalysis {
    locations: HashMap<i64, MemoryLocation>,
    points_to: HashMap<i64, HashSet<i64>>,
    type_tags: HashMap<i64, i64>,
    next_id: i64,
}

impl AliasAnalysis {
    pub fn new() -> Self {
        Self {
            locations: HashMap::new(),
            points_to: HashMap::new(),
            type_tags: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_location(&mut self, base: i64, offset: i64, size: i64, type_tag: i64) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.locations.insert(id, MemoryLocation { id, base, offset, size, type_tag });
        self.type_tags.insert(id, type_tag);
        id
    }

    pub fn add_points_to(&mut self, pointer: i64, target: i64) {
        self.points_to.entry(pointer).or_default().insert(target);
    }

    pub fn query(&self, a: i64, b: i64) -> AliasResult {
        let loc_a = match self.locations.get(&a) {
            Some(l) => l,
            None => return AliasResult::MayAlias,
        };
        let loc_b = match self.locations.get(&b) {
            Some(l) => l,
            None => return AliasResult::MayAlias,
        };

        // TBAA: different type tags → no alias (checked first)
        if loc_a.type_tag != loc_b.type_tag && loc_a.type_tag != 0 && loc_b.type_tag != 0 {
            return AliasResult::NoAlias;
        }

        // Same location
        if loc_a.base == loc_b.base && loc_a.offset == loc_b.offset && loc_a.size == loc_b.size {
            return AliasResult::MustAlias;
        }

        // Different base → no alias
        if loc_a.base != loc_b.base {
            return AliasResult::NoAlias;
        }

        // Same base, check overlap
        let a_end = loc_a.offset + loc_a.size;
        let b_end = loc_b.offset + loc_b.size;
        if loc_a.offset >= b_end || loc_b.offset >= a_end {
            return AliasResult::NoAlias;
        }

        AliasResult::MayAlias
    }

    pub fn must_alias(&self, a: i64, b: i64) -> bool {
        self.query(a, b) == AliasResult::MustAlias
    }

    pub fn may_alias(&self, a: i64, b: i64) -> bool {
        self.query(a, b) == AliasResult::MayAlias
    }

    pub fn no_alias(&self, a: i64, b: i64) -> bool {
        self.query(a, b) == AliasResult::NoAlias
    }

    pub fn points_to_set(&self, pointer: i64) -> Vec<i64> {
        self.points_to.get(&pointer).map(|s| s.iter().copied().collect()).unwrap_or_default()
    }

    pub fn alias_set_count(&self) -> i64 {
        self.points_to.len() as i64
    }

    pub fn tbaa_check(&self, a: i64, b: i64) -> bool {
        let tag_a = self.type_tags.get(&a).copied().unwrap_or(0);
        let tag_b = self.type_tags.get(&b).copied().unwrap_or(0);
        tag_a != 0 && tag_b != 0 && tag_a != tag_b
    }

    /// Check if a pointer escapes (is in points-to set of any other pointer).
    pub fn escapes(&self, location: i64) -> bool {
        self.points_to.values().any(|targets| targets.contains(&location))
    }

    pub fn location_count(&self) -> i64 {
        self.locations.len() as i64
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static ALIAS: LazyLock<Mutex<AliasAnalysis>> = LazyLock::new(|| Mutex::new(AliasAnalysis::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_analyze(base: i64, offset: i64, size: i64, type_tag: i64) -> i64 {
    ALIAS.lock().unwrap().add_location(base, offset, size, type_tag)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_may_alias(a: i64, b: i64) -> i64 {
    if ALIAS.lock().unwrap().may_alias(a, b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_must_alias(a: i64, b: i64) -> i64 {
    if ALIAS.lock().unwrap().must_alias(a, b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_no_alias(a: i64, b: i64) -> i64 {
    if ALIAS.lock().unwrap().no_alias(a, b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_points_to(pointer: i64) -> i64 {
    ALIAS.lock().unwrap().points_to_set(pointer).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_set_count() -> i64 {
    ALIAS.lock().unwrap().alias_set_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_tbaa_check(a: i64, b: i64) -> i64 {
    if ALIAS.lock().unwrap().tbaa_check(a, b) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_alias_escape_check(location: i64) -> i64 {
    if ALIAS.lock().unwrap().escapes(location) { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_analysis() {
        let aa = AliasAnalysis::new();
        assert_eq!(aa.location_count(), 0);
    }

    #[test]
    fn test_add_location() {
        let mut aa = AliasAnalysis::new();
        let id = aa.add_location(100, 0, 8, 1);
        assert!(id > 0);
        assert_eq!(aa.location_count(), 1);
    }

    #[test]
    fn test_must_alias_same() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        let b = aa.add_location(100, 0, 8, 1);
        assert!(aa.must_alias(a, b));
    }

    #[test]
    fn test_no_alias_different_base() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        let b = aa.add_location(200, 0, 8, 1);
        assert!(aa.no_alias(a, b));
    }

    #[test]
    fn test_no_alias_different_type() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        let b = aa.add_location(100, 0, 8, 2);
        assert!(aa.no_alias(a, b));
    }

    #[test]
    fn test_no_alias_no_overlap() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 4, 1);
        let b = aa.add_location(100, 8, 4, 1);
        assert!(aa.no_alias(a, b));
    }

    #[test]
    fn test_may_alias_overlap() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        let b = aa.add_location(100, 4, 8, 1);
        assert!(aa.may_alias(a, b));
    }

    #[test]
    fn test_tbaa_check_different_types() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        let b = aa.add_location(100, 0, 8, 2);
        assert!(aa.tbaa_check(a, b));
    }

    #[test]
    fn test_tbaa_check_same_type() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        let b = aa.add_location(100, 0, 8, 1);
        assert!(!aa.tbaa_check(a, b));
    }

    #[test]
    fn test_tbaa_zero_type() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 0);
        let b = aa.add_location(100, 0, 8, 1);
        assert!(!aa.tbaa_check(a, b));
    }

    #[test]
    fn test_points_to() {
        let mut aa = AliasAnalysis::new();
        aa.add_points_to(1, 10);
        aa.add_points_to(1, 20);
        let pts = aa.points_to_set(1);
        assert_eq!(pts.len(), 2);
    }

    #[test]
    fn test_points_to_empty() {
        let aa = AliasAnalysis::new();
        assert!(aa.points_to_set(999).is_empty());
    }

    #[test]
    fn test_escapes() {
        let mut aa = AliasAnalysis::new();
        aa.add_points_to(1, 42);
        assert!(aa.escapes(42));
        assert!(!aa.escapes(99));
    }

    #[test]
    fn test_alias_set_count() {
        let mut aa = AliasAnalysis::new();
        aa.add_points_to(1, 10);
        aa.add_points_to(2, 20);
        assert_eq!(aa.alias_set_count(), 2);
    }

    #[test]
    fn test_unknown_location() {
        let aa = AliasAnalysis::new();
        assert!(aa.may_alias(999, 998));
    }

    #[test]
    fn test_adjacent_no_alias() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 4, 1);
        let b = aa.add_location(100, 4, 4, 1);
        assert!(aa.no_alias(a, b));
    }

    #[test]
    fn test_contained_may_alias() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 16, 1);
        let b = aa.add_location(100, 4, 4, 1);
        assert!(aa.may_alias(a, b));
    }

    #[test]
    fn test_multiple_locations() {
        let mut aa = AliasAnalysis::new();
        for i in 0..10 {
            aa.add_location(i * 100, 0, 8, i);
        }
        assert_eq!(aa.location_count(), 10);
    }

    #[test]
    fn test_self_alias() {
        let mut aa = AliasAnalysis::new();
        let a = aa.add_location(100, 0, 8, 1);
        assert!(aa.must_alias(a, a));
    }

    #[test]
    fn test_escape_not_in_any_set() {
        let mut aa = AliasAnalysis::new();
        aa.add_location(100, 0, 8, 1);
        assert!(!aa.escapes(100));
    }
}
