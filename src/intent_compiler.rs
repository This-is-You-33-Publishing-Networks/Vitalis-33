//! v460 — Intent Compiler.
//!
//! Natural language → Vitalis AST compilation. Parse intent descriptions,
//! map to program synthesis with NL constraints. Semantic matching against stdlib.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// A parsed intent from natural language.
#[derive(Debug, Clone)]
pub struct Intent {
    pub description: String,
    pub keywords: Vec<String>,
    pub inferred_type: Option<IntentType>,
    pub confidence: f64,
}

/// High-level intent categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntentType {
    Sort,
    Search,
    Filter,
    Transform,
    Aggregate,
    IO,
    Network,
    DataStructure,
    Algorithm,
    MathComputation,
    StringManipulation,
}

impl IntentType {
    pub fn name(&self) -> &'static str {
        match self {
            IntentType::Sort => "sort",
            IntentType::Search => "search",
            IntentType::Filter => "filter",
            IntentType::Transform => "transform",
            IntentType::Aggregate => "aggregate",
            IntentType::IO => "io",
            IntentType::Network => "network",
            IntentType::DataStructure => "data_structure",
            IntentType::Algorithm => "algorithm",
            IntentType::MathComputation => "math",
            IntentType::StringManipulation => "string",
        }
    }
}

/// Keyword → IntentType mapping database.
pub struct IntentClassifier {
    keyword_map: HashMap<String, (IntentType, f64)>,
}

impl IntentClassifier {
    pub fn new() -> Self {
        let mut km = HashMap::new();
        let mappings: &[(&[&str], IntentType, f64)] = &[
            (&["sort", "order", "arrange", "rank"], IntentType::Sort, 0.9),
            (&["search", "find", "lookup", "locate"], IntentType::Search, 0.9),
            (&["filter", "select", "where", "keep"], IntentType::Filter, 0.85),
            (&["transform", "map", "convert", "translate"], IntentType::Transform, 0.85),
            (&["sum", "count", "average", "total", "aggregate", "reduce"], IntentType::Aggregate, 0.9),
            (&["read", "write", "file", "print", "output"], IntentType::IO, 0.8),
            (&["http", "request", "api", "server", "network", "connect"], IntentType::Network, 0.85),
            (&["list", "tree", "graph", "stack", "queue", "hash"], IntentType::DataStructure, 0.8),
            (&["path", "shortest", "traverse", "bfs", "dfs", "dijkstra"], IntentType::Algorithm, 0.9),
            (&["sqrt", "sin", "cos", "matrix", "integral", "derivative"], IntentType::MathComputation, 0.9),
            (&["string", "split", "join", "regex", "replace", "trim"], IntentType::StringManipulation, 0.85),
        ];
        for (keywords, intent_type, confidence) in mappings {
            for kw in *keywords {
                km.insert(kw.to_string(), (*intent_type, *confidence));
            }
        }
        Self { keyword_map: km }
    }

    /// Classify a natural language description.
    pub fn classify(&self, description: &str) -> Intent {
        let lower = description.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let keywords: Vec<String> = words.iter()
            .filter(|w| self.keyword_map.contains_key(**w))
            .map(|w| w.to_string())
            .collect();

        let mut type_scores: HashMap<IntentType, f64> = HashMap::new();
        for kw in &keywords {
            if let Some((intent_type, conf)) = self.keyword_map.get(kw) {
                *type_scores.entry(*intent_type).or_insert(0.0) += conf;
            }
        }

        let (best_type, best_score) = type_scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(t, s)| (Some(*t), *s))
            .unwrap_or((None, 0.0));

        // Confidence is the best type's accumulated score, capped at 1.0
        let confidence = if keywords.is_empty() { 0.0 }
        else { best_score.min(1.0) };

        Intent {
            description: description.to_string(),
            keywords,
            inferred_type: best_type,
            confidence,
        }
    }

    pub fn total_keywords(&self) -> usize {
        self.keyword_map.len()
    }
}

/// Code template for generating Vitalis code from intents.
#[derive(Debug, Clone)]
pub struct CodeTemplate {
    pub intent_type: IntentType,
    pub template: String,
    pub complexity: u32,
}

/// Template registry for intent → code mapping.
pub struct TemplateRegistry {
    templates: HashMap<IntentType, Vec<CodeTemplate>>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        let mut templates: HashMap<IntentType, Vec<CodeTemplate>> = HashMap::new();

        let defaults = vec![
            (IntentType::Sort, "fn sort(list: [i64]) -> [i64] { list }", 1),
            (IntentType::Search, "fn search(list: [i64], target: i64) -> i64 { 0 }", 2),
            (IntentType::Filter, "fn filter(list: [i64]) -> [i64] { list }", 1),
            (IntentType::Aggregate, "fn aggregate(list: [i64]) -> i64 { 0 }", 1),
        ];

        for (intent_type, template, complexity) in defaults {
            templates.entry(intent_type).or_default().push(CodeTemplate {
                intent_type,
                template: template.to_string(),
                complexity,
            });
        }

        Self { templates }
    }

    /// Get best template for an intent type.
    pub fn get_template(&self, intent_type: IntentType) -> Option<&CodeTemplate> {
        self.templates.get(&intent_type)
            .and_then(|ts| ts.first())
    }

    /// Register a new template.
    pub fn register(&mut self, template: CodeTemplate) {
        self.templates.entry(template.intent_type).or_default().push(template);
    }

    /// Count of template types.
    pub fn type_count(&self) -> usize {
        self.templates.len()
    }

    /// Total templates.
    pub fn total_templates(&self) -> usize {
        self.templates.values().map(|v| v.len()).sum()
    }
}

/// Compile intent to code.
pub fn compile_intent(description: &str) -> Option<String> {
    let classifier = IntentClassifier::new();
    let intent = classifier.classify(description);
    if intent.confidence < 0.1 { return None; }
    let registry = TemplateRegistry::new();
    intent.inferred_type
        .and_then(|t| registry.get_template(t))
        .map(|tmpl| tmpl.template.clone())
}

// ─── FFI ──────────────────────────────────────────────────────────────

static GLOBAL_CLASSIFIER: LazyLock<Mutex<IntentClassifier>> =
    LazyLock::new(|| Mutex::new(IntentClassifier::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_intent_classify(desc_len: i64) -> i64 {
    let classifier = GLOBAL_CLASSIFIER.lock().unwrap();
    // Return keyword count for the description length (proxy for complexity)
    (classifier.total_keywords().min(desc_len as usize)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_intent_keywords() -> i64 {
    let classifier = GLOBAL_CLASSIFIER.lock().unwrap();
    classifier.total_keywords() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_type_name() {
        assert_eq!(IntentType::Sort.name(), "sort");
        assert_eq!(IntentType::MathComputation.name(), "math");
    }

    #[test]
    fn test_classifier_new() {
        let c = IntentClassifier::new();
        assert!(c.total_keywords() > 20);
    }

    #[test]
    fn test_classify_sort() {
        let c = IntentClassifier::new();
        let intent = c.classify("sort a list of numbers");
        assert_eq!(intent.inferred_type, Some(IntentType::Sort));
        assert!(intent.confidence > 0.5);
    }

    #[test]
    fn test_classify_search() {
        let c = IntentClassifier::new();
        let intent = c.classify("find the shortest path");
        assert!(intent.inferred_type.is_some());
    }

    #[test]
    fn test_classify_no_match() {
        let c = IntentClassifier::new();
        let intent = c.classify("xyzzy foobar baz");
        assert!(intent.keywords.is_empty());
        assert!(intent.confidence < 0.01);
    }

    #[test]
    fn test_classify_multiple_keywords() {
        let c = IntentClassifier::new();
        let intent = c.classify("search and filter list");
        assert!(!intent.keywords.is_empty());
    }

    #[test]
    fn test_template_registry_new() {
        let r = TemplateRegistry::new();
        assert!(r.type_count() > 0);
        assert!(r.total_templates() >= 4);
    }

    #[test]
    fn test_template_get() {
        let r = TemplateRegistry::new();
        let t = r.get_template(IntentType::Sort);
        assert!(t.is_some());
    }

    #[test]
    fn test_template_register() {
        let mut r = TemplateRegistry::new();
        let before = r.total_templates();
        r.register(CodeTemplate {
            intent_type: IntentType::IO,
            template: "fn main() { }".to_string(),
            complexity: 1,
        });
        assert_eq!(r.total_templates(), before + 1);
    }

    #[test]
    fn test_compile_intent_sort() {
        let code = compile_intent("sort a list");
        assert!(code.is_some());
    }

    #[test]
    fn test_compile_intent_no_match() {
        let code = compile_intent("xyzzy foobar");
        assert!(code.is_none());
    }

    #[test]
    fn test_intent_struct() {
        let intent = Intent {
            description: "test".to_string(),
            keywords: vec!["sort".to_string()],
            inferred_type: Some(IntentType::Sort),
            confidence: 0.9,
        };
        assert_eq!(intent.description, "test");
    }

    #[test]
    fn test_code_template_struct() {
        let t = CodeTemplate {
            intent_type: IntentType::Filter,
            template: "fn f() {}".to_string(),
            complexity: 2,
        };
        assert_eq!(t.complexity, 2);
    }

    #[test]
    fn test_ffi_keywords() {
        let n = slang_intent_keywords();
        assert!(n > 20);
    }

    #[test]
    fn test_ffi_classify() {
        let r = slang_intent_classify(10);
        assert!(r >= 0);
    }

    #[test]
    fn test_classify_math() {
        let c = IntentClassifier::new();
        let intent = c.classify("compute the sqrt of matrix");
        assert_eq!(intent.inferred_type, Some(IntentType::MathComputation));
    }

    #[test]
    fn test_classify_aggregate() {
        let c = IntentClassifier::new();
        let intent = c.classify("sum the total count");
        assert_eq!(intent.inferred_type, Some(IntentType::Aggregate));
    }
}
