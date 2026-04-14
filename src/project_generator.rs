//! Project Scaffolding Generator — Vitalis v861
//!
//! Generates project file structures from templates, enabling rapid
//! project initialization with best-practice layouts.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

static STATE: LazyLock<Mutex<ProjectGenerator>> = LazyLock::new(|| Mutex::new(ProjectGenerator::new()));

pub struct ProjectGenerator {
    generated_files: Vec<String>,
    templates: HashMap<String, Vec<String>>,
}

impl ProjectGenerator {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        templates.insert("lib".to_string(), vec![
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
            "README.md".to_string(),
        ]);
        templates.insert("bin".to_string(), vec![
            "src/main.rs".to_string(),
            "Cargo.toml".to_string(),
        ]);
        Self { generated_files: Vec::new(), templates }
    }

    pub fn generate(&mut self, name: &str, template: &str) -> Vec<String> {
        let base = self.templates.get(template).cloned().unwrap_or_else(|| {
            vec![format!("src/{name}.rs"), "Cargo.toml".to_string()]
        });
        let files: Vec<String> = base.iter()
            .map(|f| format!("{name}/{f}"))
            .collect();
        self.generated_files.extend(files.clone());
        files
    }

    pub fn file_count(&self) -> usize {
        self.generated_files.len()
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    pub fn reset(&mut self) {
        self.generated_files.clear();
    }
}

impl Default for ProjectGenerator {
    fn default() -> Self { Self::new() }
}

#[unsafe(no_mangle)]
pub extern "C" fn pg_generate(name_len: i64, use_lib: i64) -> i64 {
    let name = "proj_".to_string() + &"x".repeat(name_len.max(0) as usize);
    let template = if use_lib != 0 { "lib" } else { "bin" };
    let mut s = STATE.lock().unwrap();
    s.generate(&name, template).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn pg_file_count() -> i64 {
    STATE.lock().unwrap().file_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn pg_template_count() -> i64 {
    STATE.lock().unwrap().template_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn pg_reset() -> i64 {
    STATE.lock().unwrap().reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lib_template() {
        let mut pg = ProjectGenerator::new();
        let files = pg.generate("mylib", "lib");
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_generate_bin_template() {
        let mut pg = ProjectGenerator::new();
        let files = pg.generate("myapp", "bin");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_file_count_accumulates() {
        let mut pg = ProjectGenerator::new();
        pg.generate("proj1", "lib");
        pg.generate("proj2", "bin");
        assert_eq!(pg.file_count(), 5);
    }

    #[test]
    fn test_template_count() {
        let pg = ProjectGenerator::new();
        assert_eq!(pg.template_count(), 2);
    }

    #[test]
    fn test_reset() {
        let mut pg = ProjectGenerator::new();
        pg.generate("p", "lib");
        pg.reset();
        assert_eq!(pg.file_count(), 0);
    }

    #[test]
    fn test_ffi_pg_generate() {
        pg_reset();
        let count = pg_generate(3, 1);
        assert!(count > 0);
    }
}
