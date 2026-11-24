use crate::model::{ModelError, Size};

/// Input to the solver: the target vector (length N³, arbitrary reals),
/// epsilon (barrier weight μ > 0), and a start state for the central-path
/// sweep. The default start is `1/(bx*by)` in every variable (strictly
/// interior); pass the previous `Assignment` values as `start` to warm-start
/// the next solve.
#[derive(Clone, Debug)]
pub struct SolveRequest {
    pub size: Size,
    pub target: Vec<f64>,
    pub epsilon: f64,
    pub start: Vec<f64>,
}

/// Default start state: `1/(bx*by)` in every variable.
pub fn default_start(size: &Size) -> Vec<f64> {
    let n = size.n();
    let mut v = 1.0 / n as f64;
    if v >= 1.0 {
        v = 0.5; // N = 1 edge case
    }
    vec![v; size.num_vars()]
}

impl SolveRequest {
    /// Request with the default start state (1/N everywhere).
    pub fn new(size: Size, target: Vec<f64>, epsilon: f64) -> Result<Self, ModelError> {
        Self::with_start(size, target, epsilon, default_start(&size))
    }

    pub fn with_start(
        size: Size,
        target: Vec<f64>,
        epsilon: f64,
        start: Vec<f64>,
    ) -> Result<Self, ModelError> {
        if target.len() != size.num_vars() {
            return Err(ModelError::TargetLength {
                expected: size.num_vars(),
                actual: target.len(),
            });
        }
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(ModelError::InvalidEpsilon { epsilon });
        }
        if start.len() != size.num_vars() {
            return Err(ModelError::StartLength {
                expected: size.num_vars(),
                actual: start.len(),
            });
        }
        if start.iter().any(|v| !v.is_finite()) {
            return Err(ModelError::InvalidStart);
        }
        // the barrier needs a strictly interior point: clamp into (0, 1), so a
        // previous solver result (which may contain exact 0.0/1.0) is a valid start
        let start: Vec<f64> = start.iter().map(|&v| v.clamp(1e-12, 1.0 - 1e-12)).collect();
        Ok(Self {
            size,
            target,
            epsilon,
            start,
        })
    }
}
