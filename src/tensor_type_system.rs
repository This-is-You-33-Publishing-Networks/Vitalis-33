//! v76 tensor type rules for shape compatibility and scalar promotion.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorDType {
    I32,
    I64,
    F32,
    F64,
}

impl TensorDType {
    pub fn promote(a: TensorDType, b: TensorDType) -> TensorDType {
        use TensorDType::*;
        match (a, b) {
            (F64, _) | (_, F64) => F64,
            (F32, _) | (_, F32) => F32,
            (I64, _) | (_, I64) => I64,
            _ => I32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorType {
    pub dtype: TensorDType,
    pub dims: Vec<usize>,
}

impl fmt::Display for TensorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tensor<{:?}, {:?}>", self.dtype, self.dims)
    }
}

pub fn parse_tensor_type(spec: &str) -> Option<TensorType> {
    // Expected shape: Tensor<f32,2,3>
    let s = spec.trim();
    if !s.starts_with("Tensor<") || !s.ends_with('>') {
        return None;
    }
    let inner = &s[7..s.len() - 1];
    let mut parts = inner.split(',').map(|p| p.trim());
    let dtype = match parts.next()? {
        "i32" => TensorDType::I32,
        "i64" => TensorDType::I64,
        "f32" => TensorDType::F32,
        "f64" => TensorDType::F64,
        _ => return None,
    };

    let mut dims = Vec::new();
    for part in parts {
        let dim: usize = part.parse().ok()?;
        dims.push(dim);
    }

    Some(TensorType { dtype, dims })
}

pub fn broadcast_shape(a: &[usize], b: &[usize]) -> Option<Vec<usize>> {
    let rank = a.len().max(b.len());
    let mut out = Vec::with_capacity(rank);

    for i in 0..rank {
        let da = *a.get(a.len().wrapping_sub(1 + i)).unwrap_or(&1);
        let db = *b.get(b.len().wrapping_sub(1 + i)).unwrap_or(&1);
        if da == db || da == 1 || db == 1 {
            out.push(da.max(db));
        } else {
            return None;
        }
    }

    out.reverse();
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tensor_form() {
        let ty = parse_tensor_type("Tensor<f32,64,128>").expect("parse");
        assert_eq!(ty.dtype, TensorDType::F32);
        assert_eq!(ty.dims, vec![64, 128]);
    }

    #[test]
    fn broadcast_shape_matches_numpy_rules() {
        assert_eq!(broadcast_shape(&[2, 1, 4], &[1, 3, 4]), Some(vec![2, 3, 4]));
        assert_eq!(broadcast_shape(&[5, 2], &[3, 2]), None);
    }

    #[test]
    fn scalar_promotion_prefers_wider_float() {
        assert_eq!(TensorDType::promote(TensorDType::I32, TensorDType::F32), TensorDType::F32);
        assert_eq!(TensorDType::promote(TensorDType::F32, TensorDType::F64), TensorDType::F64);
    }
}
