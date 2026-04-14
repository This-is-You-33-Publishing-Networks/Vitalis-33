//! Search Index — v374
//! Inverted index with BM25 and TF-IDF scoring for full-text search.

use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct SearchIndex {
    /// doc_id → word list
    documents: HashMap<i64, Vec<String>>,
    /// term → set of doc_ids
    inverted: HashMap<String, HashSet<i64>>,
    /// Total document count
    next_id: i64,
    /// Average document length
    avg_dl: f64,
    /// BM25 parameters
    k1: f64,
    b: f64,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            inverted: HashMap::new(),
            next_id: 1,
            avg_dl: 0.0,
            k1: 1.2,
            b: 0.75,
        }
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_string())
            .collect()
    }

    pub fn add_document(&mut self, text: &str) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let words = Self::tokenize(text);
        for word in &words {
            self.inverted.entry(word.clone()).or_default().insert(id);
        }
        self.documents.insert(id, words);
        self.recalc_avg_dl();
        id
    }

    pub fn remove_document(&mut self, doc_id: i64) -> bool {
        if let Some(words) = self.documents.remove(&doc_id) {
            for word in &words {
                if let Some(set) = self.inverted.get_mut(word) {
                    set.remove(&doc_id);
                    if set.is_empty() {
                        self.inverted.remove(word);
                    }
                }
            }
            self.recalc_avg_dl();
            true
        } else {
            false
        }
    }

    fn recalc_avg_dl(&mut self) {
        if self.documents.is_empty() {
            self.avg_dl = 0.0;
        } else {
            let total: usize = self.documents.values().map(|d| d.len()).sum();
            self.avg_dl = total as f64 / self.documents.len() as f64;
        }
    }

    pub fn doc_count(&self) -> i64 {
        self.documents.len() as i64
    }

    pub fn term_count(&self) -> i64 {
        self.inverted.len() as i64
    }

    /// Term frequency of `term` in `doc_id`.
    pub fn tf(&self, term: &str, doc_id: i64) -> f64 {
        let term_lower = term.to_lowercase();
        self.documents.get(&doc_id)
            .map(|words| words.iter().filter(|w| *w == &term_lower).count() as f64)
            .unwrap_or(0.0)
    }

    /// Inverse document frequency of `term`.
    pub fn idf(&self, term: &str) -> f64 {
        let term_lower = term.to_lowercase();
        let n = self.documents.len() as f64;
        let df = self.inverted.get(&term_lower).map(|s| s.len()).unwrap_or(0) as f64;
        if df == 0.0 || n == 0.0 {
            0.0
        } else {
            ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
        }
    }

    /// TF-IDF score for term in doc.
    pub fn tfidf(&self, term: &str, doc_id: i64) -> f64 {
        self.tf(term, doc_id) * self.idf(term)
    }

    /// BM25 score for a term in a document.
    pub fn bm25(&self, term: &str, doc_id: i64) -> f64 {
        let tf = self.tf(term, doc_id);
        let idf = self.idf(term);
        let dl = self.documents.get(&doc_id).map(|d| d.len() as f64).unwrap_or(0.0);
        let numerator = tf * (self.k1 + 1.0);
        let denominator = tf + self.k1 * (1.0 - self.b + self.b * dl / self.avg_dl.max(1.0));
        idf * numerator / denominator
    }

    /// Search for a query, return doc_id with highest BM25 score.
    pub fn search(&self, query: &str) -> Option<i64> {
        let terms = Self::tokenize(query);
        if terms.is_empty() {
            return None;
        }
        let mut scores: HashMap<i64, f64> = HashMap::new();
        for term in &terms {
            if let Some(docs) = self.inverted.get(term) {
                for &doc_id in docs {
                    let score = self.bm25(term, doc_id);
                    *scores.entry(doc_id).or_insert(0.0) += score;
                }
            }
        }
        scores.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }

    /// Search returning top-N results.
    pub fn search_top_n(&self, query: &str, n: usize) -> Vec<(i64, f64)> {
        let terms = Self::tokenize(query);
        let mut scores: HashMap<i64, f64> = HashMap::new();
        for term in &terms {
            if let Some(docs) = self.inverted.get(term) {
                for &doc_id in docs {
                    let score = self.bm25(term, doc_id);
                    *scores.entry(doc_id).or_insert(0.0) += score;
                }
            }
        }
        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(n);
        results
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static INDEX: LazyLock<Mutex<SearchIndex>> = LazyLock::new(|| Mutex::new(SearchIndex::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_create() -> i64 {
    *INDEX.lock().unwrap() = SearchIndex::new();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_add_doc(text_hash: i64) -> i64 {
    let mut idx = INDEX.lock().unwrap();
    let text = format!("document content {}", text_hash);
    idx.add_document(&text)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_search(query_hash: i64) -> i64 {
    let idx = INDEX.lock().unwrap();
    let query = format!("document {}", query_hash);
    idx.search(&query).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_doc_count() -> i64 {
    INDEX.lock().unwrap().doc_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_term_count() -> i64 {
    INDEX.lock().unwrap().term_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_bm25_score(doc_id: i64) -> f64 {
    let idx = INDEX.lock().unwrap();
    idx.bm25("document", doc_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_tfidf_score(doc_id: i64) -> f64 {
    let idx = INDEX.lock().unwrap();
    idx.tfidf("document", doc_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_idx_remove_doc(doc_id: i64) -> i64 {
    if INDEX.lock().unwrap().remove_document(doc_id) { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_index() {
        let idx = SearchIndex::new();
        assert_eq!(idx.doc_count(), 0);
        assert_eq!(idx.term_count(), 0);
    }

    #[test]
    fn test_add_document() {
        let mut idx = SearchIndex::new();
        let id = idx.add_document("hello world");
        assert_eq!(id, 1);
        assert_eq!(idx.doc_count(), 1);
    }

    #[test]
    fn test_tokenize() {
        let tokens = SearchIndex::tokenize("Hello, World! Test-123");
        assert_eq!(tokens, vec!["hello", "world", "test", "123"]);
    }

    #[test]
    fn test_inverted_index() {
        let mut idx = SearchIndex::new();
        idx.add_document("hello world");
        idx.add_document("hello rust");
        assert_eq!(idx.inverted["hello"].len(), 2);
        assert_eq!(idx.inverted["world"].len(), 1);
    }

    #[test]
    fn test_tf() {
        let mut idx = SearchIndex::new();
        let doc = idx.add_document("the cat sat on the mat");
        assert_eq!(idx.tf("the", doc), 2.0);
        assert_eq!(idx.tf("cat", doc), 1.0);
        assert_eq!(idx.tf("dog", doc), 0.0);
    }

    #[test]
    fn test_idf() {
        let mut idx = SearchIndex::new();
        idx.add_document("hello world");
        idx.add_document("hello rust");
        idx.add_document("goodbye world");
        let idf_hello = idx.idf("hello");
        let idf_rust = idx.idf("rust");
        assert!(idf_rust > idf_hello); // rarer term has higher IDF
    }

    #[test]
    fn test_tfidf() {
        let mut idx = SearchIndex::new();
        let doc = idx.add_document("rust is great");
        idx.add_document("python is great");
        let score = idx.tfidf("rust", doc);
        assert!(score > 0.0);
    }

    #[test]
    fn test_bm25() {
        let mut idx = SearchIndex::new();
        let doc = idx.add_document("information retrieval systems");
        idx.add_document("database management systems");
        let score = idx.bm25("retrieval", doc);
        assert!(score > 0.0);
    }

    #[test]
    fn test_search_basic() {
        let mut idx = SearchIndex::new();
        let d1 = idx.add_document("rust programming language");
        let _d2 = idx.add_document("python scripting");
        let result = idx.search("rust").unwrap();
        assert_eq!(result, d1);
    }

    #[test]
    fn test_search_no_results() {
        let mut idx = SearchIndex::new();
        idx.add_document("hello world");
        assert!(idx.search("xyz").is_none());
    }

    #[test]
    fn test_search_empty_query() {
        let idx = SearchIndex::new();
        assert!(idx.search("").is_none());
    }

    #[test]
    fn test_remove_document() {
        let mut idx = SearchIndex::new();
        let id = idx.add_document("to be removed");
        assert!(idx.remove_document(id));
        assert_eq!(idx.doc_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut idx = SearchIndex::new();
        assert!(!idx.remove_document(999));
    }

    #[test]
    fn test_remove_cleans_inverted() {
        let mut idx = SearchIndex::new();
        let id = idx.add_document("unique");
        idx.remove_document(id);
        assert!(!idx.inverted.contains_key("unique"));
    }

    #[test]
    fn test_search_top_n() {
        let mut idx = SearchIndex::new();
        idx.add_document("rust language systems");
        idx.add_document("rust programming");
        idx.add_document("python programming");
        let results = idx.search_top_n("rust", 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_avg_dl() {
        let mut idx = SearchIndex::new();
        idx.add_document("one two three");
        idx.add_document("four five");
        assert!((idx.avg_dl - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_term_count() {
        let mut idx = SearchIndex::new();
        idx.add_document("a b c d e");
        assert_eq!(idx.term_count(), 5);
    }

    #[test]
    fn test_multiple_documents_search() {
        let mut idx = SearchIndex::new();
        for i in 0..10 {
            idx.add_document(&format!("document number {}", i));
        }
        assert_eq!(idx.doc_count(), 10);
        let result = idx.search("document");
        assert!(result.is_some());
    }

    #[test]
    fn test_case_insensitive() {
        let mut idx = SearchIndex::new();
        let id = idx.add_document("Hello World");
        let result = idx.search("hello");
        assert_eq!(result, Some(id));
    }

    #[test]
    fn test_idf_nonexistent_term() {
        let mut idx = SearchIndex::new();
        idx.add_document("hello");
        assert_eq!(idx.idf("nonexistent"), 0.0);
    }

    #[test]
    fn test_bm25_zero_for_absent_term() {
        let mut idx = SearchIndex::new();
        let doc = idx.add_document("hello world");
        assert_eq!(idx.bm25("nothere", doc), 0.0);
    }
}
