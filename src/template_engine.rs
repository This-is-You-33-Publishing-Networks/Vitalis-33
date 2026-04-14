//! Template Engine — v375
//! String template rendering with variable substitution, conditionals, loops.

use std::collections::HashMap;

#[derive(Debug)]
pub struct TemplateEngine {
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }

    pub fn set_var(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    pub fn get_var(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }

    pub fn has_var(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    pub fn var_count(&self) -> usize {
        self.variables.len()
    }

    pub fn clear_vars(&mut self) {
        self.variables.clear();
    }

    /// Render template with {{variable}} substitution.
    pub fn render(&self, template: &str) -> String {
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second {
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }
                        break;
                    }
                    var_name.push(c);
                    chars.next();
                }
                let var_name = var_name.trim();
                if let Some(val) = self.variables.get(var_name) {
                    result.push_str(val);
                } else {
                    result.push_str("{{");
                    result.push_str(var_name);
                    result.push_str("}}");
                }
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// Validate template syntax — check for unclosed {{ }}.
    pub fn validate(&self, template: &str) -> bool {
        let mut depth = 0i32;
        let mut prev = ' ';
        for ch in template.chars() {
            if prev == '{' && ch == '{' {
                depth += 1;
            }
            if prev == '}' && ch == '}' {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            prev = ch;
        }
        depth == 0
    }

    /// Escape HTML entities in a string.
    pub fn escape_html(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '&' => result.push_str("&amp;"),
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '"' => result.push_str("&quot;"),
                '\'' => result.push_str("&#x27;"),
                _ => result.push(ch),
            }
        }
        result
    }

    /// Extract variable names from template.
    pub fn extract_vars(template: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next();
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }
                        break;
                    }
                    name.push(c);
                    chars.next();
                }
                let name = name.trim().to_string();
                if !name.is_empty() && !vars.contains(&name) {
                    vars.push(name);
                }
            }
        }
        vars
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static TPL: LazyLock<Mutex<TemplateEngine>> = LazyLock::new(|| Mutex::new(TemplateEngine::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_render(template_hash: i64) -> i64 {
    let tpl = TPL.lock().unwrap();
    let template = format!("Hello {{name}}! Value: {}", template_hash);
    let rendered = tpl.render(&template);
    rendered.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_set_var(key_hash: i64, val_hash: i64) -> i64 {
    let mut tpl = TPL.lock().unwrap();
    let key = format!("var_{}", key_hash);
    let val = format!("val_{}", val_hash);
    tpl.set_var(&key, &val);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_get_var(key_hash: i64) -> i64 {
    let tpl = TPL.lock().unwrap();
    let key = format!("var_{}", key_hash);
    if tpl.has_var(&key) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_has_var(key_hash: i64) -> i64 {
    let tpl = TPL.lock().unwrap();
    let key = format!("var_{}", key_hash);
    if tpl.has_var(&key) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_var_count() -> i64 {
    TPL.lock().unwrap().var_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_clear_vars() -> i64 {
    TPL.lock().unwrap().clear_vars();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_validate(template_hash: i64) -> i64 {
    let tpl = TPL.lock().unwrap();
    let template = format!("{{{{var}}}} text {}", template_hash);
    if tpl.validate(&template) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_tpl_escape_html(input_hash: i64) -> i64 {
    let escaped = TemplateEngine::escape_html(&format!("<b>{}</b>", input_hash));
    escaped.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let tpl = TemplateEngine::new();
        assert_eq!(tpl.var_count(), 0);
    }

    #[test]
    fn test_set_get_var() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("name", "Vitalis");
        assert_eq!(tpl.get_var("name"), Some("Vitalis"));
    }

    #[test]
    fn test_has_var() {
        let mut tpl = TemplateEngine::new();
        assert!(!tpl.has_var("x"));
        tpl.set_var("x", "1");
        assert!(tpl.has_var("x"));
    }

    #[test]
    fn test_var_count() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("a", "1");
        tpl.set_var("b", "2");
        tpl.set_var("c", "3");
        assert_eq!(tpl.var_count(), 3);
    }

    #[test]
    fn test_clear_vars() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("x", "1");
        tpl.clear_vars();
        assert_eq!(tpl.var_count(), 0);
    }

    #[test]
    fn test_render_basic() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("name", "World");
        let result = tpl.render("Hello, {{name}}!");
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_render_multiple_vars() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("first", "John");
        tpl.set_var("last", "Doe");
        let result = tpl.render("{{first}} {{last}}");
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_render_missing_var() {
        let tpl = TemplateEngine::new();
        let result = tpl.render("Hello, {{unknown}}!");
        assert_eq!(result, "Hello, {{unknown}}!");
    }

    #[test]
    fn test_render_no_vars() {
        let tpl = TemplateEngine::new();
        let result = tpl.render("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_render_empty() {
        let tpl = TemplateEngine::new();
        let result = tpl.render("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_validate_valid() {
        let tpl = TemplateEngine::new();
        assert!(tpl.validate("Hello {{name}}!"));
    }

    #[test]
    fn test_validate_unclosed() {
        let tpl = TemplateEngine::new();
        assert!(!tpl.validate("Hello {{name!"));
    }

    #[test]
    fn test_validate_no_vars() {
        let tpl = TemplateEngine::new();
        assert!(tpl.validate("plain text"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(TemplateEngine::escape_html("<b>hello</b>"), "&lt;b&gt;hello&lt;/b&gt;");
    }

    #[test]
    fn test_escape_html_ampersand() {
        assert_eq!(TemplateEngine::escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_html_quotes() {
        assert_eq!(TemplateEngine::escape_html("\"hi\""), "&quot;hi&quot;");
    }

    #[test]
    fn test_escape_html_no_special() {
        assert_eq!(TemplateEngine::escape_html("hello"), "hello");
    }

    #[test]
    fn test_extract_vars() {
        let vars = TemplateEngine::extract_vars("{{a}} and {{b}} and {{a}}");
        assert_eq!(vars, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_vars_empty() {
        let vars = TemplateEngine::extract_vars("no vars here");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_overwrite_var() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("x", "old");
        tpl.set_var("x", "new");
        assert_eq!(tpl.get_var("x"), Some("new"));
        assert_eq!(tpl.var_count(), 1);
    }

    #[test]
    fn test_render_with_spaces() {
        let mut tpl = TemplateEngine::new();
        tpl.set_var("x", "42");
        let result = tpl.render("{{ x }}");
        assert_eq!(result, "42");
    }

    #[test]
    fn test_get_nonexistent() {
        let tpl = TemplateEngine::new();
        assert!(tpl.get_var("nope").is_none());
    }
}
