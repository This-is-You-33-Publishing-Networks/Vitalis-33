//! ML Pipeline — v396
//! ML pipeline stages, composition, validation, and execution.

use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum StageKind {
    Normalize,
    Scale,
    PCA,
    Impute,
    Encode,
    Split,
    Train,
    Evaluate,
    Custom(String),
}

impl StageKind {
    pub fn from_i64(v: i64) -> Self {
        match v {
            0 => StageKind::Normalize,
            1 => StageKind::Scale,
            2 => StageKind::PCA,
            3 => StageKind::Impute,
            4 => StageKind::Encode,
            5 => StageKind::Split,
            6 => StageKind::Train,
            7 => StageKind::Evaluate,
            _ => StageKind::Custom(format!("stage_{}", v)),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            StageKind::Normalize => "normalize",
            StageKind::Scale => "scale",
            StageKind::PCA => "pca",
            StageKind::Impute => "impute",
            StageKind::Encode => "encode",
            StageKind::Split => "split",
            StageKind::Train => "train",
            StageKind::Evaluate => "evaluate",
            StageKind::Custom(name) => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub id: usize,
    pub kind: StageKind,
    pub params: HashMap<String, f64>,
    pub executed: bool,
}

impl PipelineStage {
    pub fn new(id: usize, kind: StageKind) -> Self {
        Self { id, kind, params: HashMap::new(), executed: false }
    }

    pub fn with_param(mut self, key: &str, value: f64) -> Self {
        self.params.insert(key.to_string(), value);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStatus {
    Created,
    Validated,
    Running,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct MlPipeline {
    stages: Vec<PipelineStage>,
    status: PipelineStatus,
    results: Vec<f64>,
    next_stage_id: usize,
}

impl MlPipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            status: PipelineStatus::Created,
            results: Vec::new(),
            next_stage_id: 1,
        }
    }

    pub fn add_stage(&mut self, kind: StageKind) -> usize {
        let id = self.next_stage_id;
        self.next_stage_id += 1;
        self.stages.push(PipelineStage::new(id, kind));
        self.status = PipelineStatus::Created;
        id
    }

    pub fn remove_stage(&mut self, stage_id: usize) -> bool {
        let before = self.stages.len();
        self.stages.retain(|s| s.id != stage_id);
        if self.stages.len() < before {
            self.status = PipelineStatus::Created;
            true
        } else {
            false
        }
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn validate(&mut self) -> bool {
        if self.stages.is_empty() {
            self.status = PipelineStatus::Failed("No stages defined".into());
            return false;
        }
        // Check for duplicate stage kinds (warning but not fatal)
        let mut seen = std::collections::HashSet::new();
        for stage in &self.stages {
            if !seen.insert(stage.kind.name().to_string()) {
                // Duplicate, but we allow it
            }
        }
        self.status = PipelineStatus::Validated;
        true
    }

    pub fn execute(&mut self, input: &[f64]) -> Vec<f64> {
        if self.stages.is_empty() {
            self.status = PipelineStatus::Failed("No stages to execute".into());
            return Vec::new();
        }
        self.status = PipelineStatus::Running;
        let mut data = input.to_vec();
        for stage in &mut self.stages {
            data = Self::execute_stage(stage, &data);
            stage.executed = true;
        }
        self.results = data.clone();
        self.status = PipelineStatus::Completed;
        data
    }

    fn execute_stage(stage: &PipelineStage, data: &[f64]) -> Vec<f64> {
        match &stage.kind {
            StageKind::Normalize => {
                if data.is_empty() { return Vec::new(); }
                let min = data.iter().copied().fold(f64::INFINITY, f64::min);
                let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let range = max - min;
                if range == 0.0 {
                    vec![0.0; data.len()]
                } else {
                    data.iter().map(|&x| (x - min) / range).collect()
                }
            }
            StageKind::Scale => {
                let factor = stage.params.get("factor").copied().unwrap_or(1.0);
                data.iter().map(|&x| x * factor).collect()
            }
            StageKind::PCA => {
                // Simplified: center the data (subtract mean)
                if data.is_empty() { return Vec::new(); }
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                data.iter().map(|&x| x - mean).collect()
            }
            StageKind::Impute => {
                // Replace NaN with mean of non-NaN values
                let valid: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
                if valid.is_empty() { return vec![0.0; data.len()]; }
                let mean = valid.iter().sum::<f64>() / valid.len() as f64;
                data.iter().map(|&x| if x.is_nan() { mean } else { x }).collect()
            }
            StageKind::Encode => {
                // One-hot like: map distinct values to consecutive integers
                let mut mapping: HashMap<i64, f64> = HashMap::new();
                let mut next = 0.0;
                data.iter().map(|&x| {
                    let key = x as i64;
                    *mapping.entry(key).or_insert_with(|| { let v = next; next += 1.0; v })
                }).collect()
            }
            StageKind::Split => {
                // Return first 80% of data (train split)
                let split_point = (data.len() as f64 * 0.8).ceil() as usize;
                data[..split_point.min(data.len())].to_vec()
            }
            StageKind::Train => {
                // Simple linear regression coefficients: [slope, intercept]
                if data.len() < 2 {
                    return data.to_vec();
                }
                let n = data.len() as f64;
                let x_mean = (n - 1.0) / 2.0;
                let y_mean = data.iter().sum::<f64>() / n;
                let mut num = 0.0;
                let mut den = 0.0;
                for (i, &y) in data.iter().enumerate() {
                    let x = i as f64;
                    num += (x - x_mean) * (y - y_mean);
                    den += (x - x_mean) * (x - x_mean);
                }
                let slope = if den != 0.0 { num / den } else { 0.0 };
                let intercept = y_mean - slope * x_mean;
                vec![slope, intercept]
            }
            StageKind::Evaluate => {
                // Compute MSE if data alternates [predicted, actual, ...]
                if data.len() < 2 { return vec![0.0]; }
                let pairs = data.len() / 2;
                let mse: f64 = (0..pairs)
                    .map(|i| {
                        let pred = data[i * 2];
                        let actual = data[i * 2 + 1];
                        (pred - actual).powi(2)
                    })
                    .sum::<f64>() / pairs as f64;
                vec![mse]
            }
            StageKind::Custom(_) => {
                // Identity transform.
                data.to_vec()
            }
        }
    }

    pub fn status(&self) -> &PipelineStatus {
        &self.status
    }

    pub fn results(&self) -> &[f64] {
        &self.results
    }

    pub fn reset(&mut self) {
        for stage in &mut self.stages {
            stage.executed = false;
        }
        self.results.clear();
        self.status = PipelineStatus::Created;
    }
}

static PIPE_STORE: LazyLock<Mutex<HashMap<i64, MlPipeline>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static PIPE_NEXT_ID: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(1));

fn pipe_alloc() -> i64 {
    let mut next = PIPE_NEXT_ID.lock().unwrap();
    let id = *next;
    *next += 1;
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_create() -> i64 {
    let id = pipe_alloc();
    PIPE_STORE.lock().unwrap().insert(id, MlPipeline::new());
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_add_stage(id: i64, kind: i64) -> i64 {
    let mut store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get_mut(&id) {
        pipeline.add_stage(StageKind::from_i64(kind)) as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_execute(id: i64, data_len: i64) -> i64 {
    let mut store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get_mut(&id) {
        // Generate simple input data of given length
        let input: Vec<f64> = (0..data_len.max(0) as usize).map(|i| i as f64).collect();
        let result = pipeline.execute(&input);
        result.len() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_stage_count(id: i64) -> i64 {
    let store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get(&id) {
        pipeline.stage_count() as i64
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_remove_stage(id: i64, stage_id: i64) -> i64 {
    let mut store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get_mut(&id) {
        if pipeline.remove_stage(stage_id as usize) { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_validate(id: i64) -> i64 {
    let mut store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get_mut(&id) {
        if pipeline.validate() { 1 } else { 0 }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_status(id: i64) -> i64 {
    let store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get(&id) {
        match pipeline.status() {
            PipelineStatus::Created => 0,
            PipelineStatus::Validated => 1,
            PipelineStatus::Running => 2,
            PipelineStatus::Completed => 3,
            PipelineStatus::Failed(_) => -2,
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_pipe_reset(id: i64) -> i64 {
    let mut store = PIPE_STORE.lock().unwrap();
    if let Some(pipeline) = store.get_mut(&id) {
        pipeline.reset();
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_new() {
        let p = MlPipeline::new();
        assert_eq!(p.stage_count(), 0);
        assert_eq!(*p.status(), PipelineStatus::Created);
    }

    #[test]
    fn test_add_stage() {
        let mut p = MlPipeline::new();
        let id = p.add_stage(StageKind::Normalize);
        assert_eq!(id, 1);
        assert_eq!(p.stage_count(), 1);
    }

    #[test]
    fn test_remove_stage() {
        let mut p = MlPipeline::new();
        let id = p.add_stage(StageKind::Scale);
        assert!(p.remove_stage(id));
        assert_eq!(p.stage_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut p = MlPipeline::new();
        assert!(!p.remove_stage(999));
    }

    #[test]
    fn test_validate_empty() {
        let mut p = MlPipeline::new();
        assert!(!p.validate());
    }

    #[test]
    fn test_validate_with_stages() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Normalize);
        assert!(p.validate());
        assert_eq!(*p.status(), PipelineStatus::Validated);
    }

    #[test]
    fn test_normalize_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Normalize);
        let result = p.execute(&[0.0, 5.0, 10.0]);
        assert_eq!(result, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn test_scale_stage() {
        let mut p = MlPipeline::new();
        let id = p.add_stage(StageKind::Scale);
        p.stages.iter_mut().find(|s| s.id == id).unwrap().params.insert("factor".into(), 2.0);
        let result = p.execute(&[1.0, 2.0, 3.0]);
        assert_eq!(result, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_pca_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::PCA);
        let result = p.execute(&[2.0, 4.0, 6.0]);
        // Mean = 4.0, so centered = [-2, 0, 2]
        assert!((result[0] - (-2.0)).abs() < 1e-10);
        assert!((result[1]).abs() < 1e-10);
        assert!((result[2] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_impute_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Impute);
        let result = p.execute(&[1.0, f64::NAN, 3.0]);
        assert!((result[1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_split_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Split);
        let result = p.execute(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(result.len(), 4); // 80% of 5 = 4
    }

    #[test]
    fn test_train_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Train);
        let result = p.execute(&[0.0, 1.0, 2.0, 3.0]);
        // Should return [slope, intercept]
        assert_eq!(result.len(), 2);
        assert!((result[0] - 1.0).abs() < 1e-10); // slope = 1.0
        assert!(result[1].abs() < 1e-10); // intercept = 0.0
    }

    #[test]
    fn test_evaluate_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Evaluate);
        // pred=1, actual=2, pred=3, actual=4
        let result = p.execute(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(result.len(), 1);
        assert!((result[0] - 1.0).abs() < 1e-10); // MSE = ((1-2)^2 + (3-4)^2)/2 = 1.0
    }

    #[test]
    fn test_custom_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Custom("my_stage".into()));
        let result = p.execute(&[1.0, 2.0]);
        assert_eq!(result, vec![1.0, 2.0]); // identity
    }

    #[test]
    fn test_pipeline_chain() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Normalize);
        p.add_stage(StageKind::Scale);
        // Scale defaults to factor=1.0, so normalize then scale=1.0 -> normalize
        let result = p.execute(&[0.0, 5.0, 10.0]);
        assert_eq!(result, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn test_pipeline_status_after_execute() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Normalize);
        p.execute(&[1.0, 2.0]);
        assert_eq!(*p.status(), PipelineStatus::Completed);
    }

    #[test]
    fn test_pipeline_reset() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Normalize);
        p.execute(&[1.0, 2.0]);
        p.reset();
        assert_eq!(*p.status(), PipelineStatus::Created);
        assert!(p.results().is_empty());
    }

    #[test]
    fn test_encode_stage() {
        let mut p = MlPipeline::new();
        p.add_stage(StageKind::Encode);
        let result = p.execute(&[10.0, 20.0, 10.0, 30.0]);
        // 10->0, 20->1, 10->0, 30->2
        assert_eq!(result[0], result[2]); // same encoding
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_stage_kind_from_i64() {
        assert_eq!(StageKind::from_i64(0), StageKind::Normalize);
        assert_eq!(StageKind::from_i64(6), StageKind::Train);
        match StageKind::from_i64(99) {
            StageKind::Custom(_) => {}
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn test_pipeline_empty_execute() {
        let mut p = MlPipeline::new();
        let result = p.execute(&[1.0]);
        assert!(result.is_empty());
    }
}
