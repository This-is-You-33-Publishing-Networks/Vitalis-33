//! v77 autodiff core with finite-difference validation helpers.

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Input,
    Add(usize, usize),
    Mul(usize, usize),
    Sin(usize),
}

#[derive(Debug, Clone)]
pub struct TapeNode {
    pub op: Op,
    pub value: f64,
    pub grad: f64,
}

#[derive(Debug, Default)]
pub struct Tape {
    nodes: Vec<TapeNode>,
}

impl Tape {
    pub fn input(&mut self, value: f64) -> usize {
        self.nodes.push(TapeNode {
            op: Op::Input,
            value,
            grad: 0.0,
        });
        self.nodes.len() - 1
    }

    pub fn add(&mut self, a: usize, b: usize) -> usize {
        let value = self.nodes[a].value + self.nodes[b].value;
        self.nodes.push(TapeNode {
            op: Op::Add(a, b),
            value,
            grad: 0.0,
        });
        self.nodes.len() - 1
    }

    pub fn mul(&mut self, a: usize, b: usize) -> usize {
        let value = self.nodes[a].value * self.nodes[b].value;
        self.nodes.push(TapeNode {
            op: Op::Mul(a, b),
            value,
            grad: 0.0,
        });
        self.nodes.len() - 1
    }

    pub fn sin(&mut self, x: usize) -> usize {
        let value = self.nodes[x].value.sin();
        self.nodes.push(TapeNode {
            op: Op::Sin(x),
            value,
            grad: 0.0,
        });
        self.nodes.len() - 1
    }

    pub fn value(&self, id: usize) -> f64 {
        self.nodes[id].value
    }

    pub fn grad(&self, id: usize) -> f64 {
        self.nodes[id].grad
    }

    pub fn backward(&mut self, out: usize) {
        if let Some(node) = self.nodes.get_mut(out) {
            node.grad = 1.0;
        }

        for i in (0..=out).rev() {
            let g = self.nodes[i].grad;
            match self.nodes[i].op {
                Op::Input => {}
                Op::Add(a, b) => {
                    self.nodes[a].grad += g;
                    self.nodes[b].grad += g;
                }
                Op::Mul(a, b) => {
                    let av = self.nodes[a].value;
                    let bv = self.nodes[b].value;
                    self.nodes[a].grad += g * bv;
                    self.nodes[b].grad += g * av;
                }
                Op::Sin(x) => {
                    self.nodes[x].grad += g * self.nodes[x].value.cos();
                }
            }
        }
    }
}

pub fn finite_difference(f: impl Fn(f64) -> f64, x: f64) -> f64 {
    let eps = 1e-6;
    (f(x + eps) - f(x - eps)) / (2.0 * eps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_mode_matches_known_gradient() {
        // f(x) = x^2 + sin(x), f'(x) = 2x + cos(x)
        let x0 = 1.25;
        let mut tape = Tape::default();
        let x = tape.input(x0);
        let x2 = tape.mul(x, x);
        let sx = tape.sin(x);
        let y = tape.add(x2, sx);

        tape.backward(y);

        let expected = 2.0 * x0 + x0.cos();
        assert!((tape.grad(x) - expected).abs() < 1e-9);
    }

    #[test]
    fn finite_difference_cross_validation() {
        let x0 = 0.77;
        let numerical = finite_difference(|x| x * x + x.sin(), x0);
        let analytic = 2.0 * x0 + x0.cos();
        assert!((numerical - analytic).abs() < 1e-5);
    }
}
