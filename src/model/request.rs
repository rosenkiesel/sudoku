use crate::model::{ModelError, Size};

/// Input to the solver: the rule system is implicit in `size`; the target
/// vector (length N^3, arbitrary reals) and epsilon (barrier weight mu > 0)
/// steer the central-path solve.
#[derive(Clone, Debug)]
pub struct SolveRequest {
    pub size: Size,
    pub target: Vec<f64>,
    pub epsilon: f64,
}

impl SolveRequest {
    pub fn new(size: Size, target: Vec<f64>, epsilon: f64) -> Result<Self, ModelError> {
        if target.len() != size.num_vars() {
            return Err(ModelError::TargetLength {
                expected: size.num_vars(),
                actual: target.len(),
            });
        }
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(ModelError::InvalidEpsilon { epsilon });
        }
        Ok(Self {
            size,
            target,
            epsilon,
        })
    }
}
