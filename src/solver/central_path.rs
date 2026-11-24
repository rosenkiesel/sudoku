use super::{Assignment, Solver, SolverError};
use crate::model::game_rules::{LinearSystem, SparseRow};
use crate::model::request::SolveRequest;

/// Primal-dual interior-point solver that follows the central path of
///
///   min  -target^T x   s.t.  A x = b
///
/// with the log-barrier  -mu * sum_i ( ln x_i + ln(1 - x_i) ), i.e. it solves
///
///   phi_mu(x) = -target^T x - mu * sum_i ( ln x_i + ln(1 - x_i) )   s.t.  A x = b
///
/// for a decreasing sequence of mu values (mu0, mu0*shrink, ..., epsilon),
/// warm-starting Newton from the previous point. The barrier keeps
/// 0 < x < 1 strictly, so the bounds need no explicit handling.
pub struct CentralPathSolver {
    pub mu0: f64,
    pub shrink: f64,
    pub newton_tol: f64,
    pub max_newton: usize,
    pub max_line_search: usize,
    pub accept_tol: f64,
    /// Small Tikhonov term on the Schur system for numerical safety.
    pub reg: f64,
}

impl Default for CentralPathSolver {
    fn default() -> Self {
        Self {
            mu0: 1.0,
            shrink: 0.1,
            newton_tol: 1e-10,
            max_newton: 50,
            max_line_search: 40,
            accept_tol: 1e-6,
            reg: 1e-12,
        }
    }
}

/// Problem data shared by all Newton steps of one solve.
struct Ctx {
    rows: Vec<(SparseRow, f64)>,
    num_vars: usize,
    c: Vec<f64>,
    col_rows: Vec<Vec<(usize, f64)>>,
}

impl CentralPathSolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// The rule matrix A has rank 4*N^2 - 3: the four rule families (cell,
    /// row, column, block) each sum to the same total, so the last row of
    /// the row/column/block families is linearly dependent and is dropped.
    fn make_ctx(system: &LinearSystem, request: &SolveRequest) -> Ctx {
        let n2 = system.size.n() * system.size.n();
        let rows: Vec<(SparseRow, f64)> = system
            .rows
            .iter()
            .enumerate()
            .filter(|(r, _)| *r != 2 * n2 - 1 && *r != 3 * n2 - 1 && *r != 4 * n2 - 1)
            .map(|(_, row)| row.clone())
            .collect();
        let num_vars = system.num_vars;
        let c: Vec<f64> = request.target.iter().map(|t| -t).collect();
        let mut col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); num_vars];
        for (r, (row, _)) in rows.iter().enumerate() {
            for &(j, a) in row {
                col_rows[j].push((r, a));
            }
        }
        Ctx {
            rows,
            num_vars,
            c,
            col_rows,
        }
    }

    /// Residuals of the barrier KKT system at fixed mu.
    /// r_d = c + g(x) + A^T lambda, r_p = A x - b, with c = -target and
    /// g_i(x) = -mu * (1/x_i - 1/(1-x_i)). Returns the Euclidean norm.
    fn residuals(
        ctx: &Ctx,
        x: &[f64],
        lambda: &[f64],
        mu: f64,
        r_d: &mut [f64],
        r_p: &mut [f64],
    ) -> f64 {
        for (i, &xi) in x.iter().enumerate() {
            r_d[i] = ctx.c[i] - mu * (1.0 / xi - 1.0 / (1.0 - xi));
        }
        for (r, (row, b)) in ctx.rows.iter().enumerate() {
            let mut s = -b;
            for &(j, a) in row {
                s += a * x[j];
                r_d[j] += a * lambda[r];
            }
            r_p[r] = s;
        }
        r_d.iter()
            .chain(r_p.iter())
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt()
    }

    /// One Newton step for the barrier problem at fixed mu.
    /// Solves  [ H  A^T ] [dx]     [ -r_d ]
    ///         [ A   0  ] [dl]  =  [ -r_p ]
    /// via the Schur complement  M = A H^-1 A^T,  H = diag(mu*(1/x^2 + 1/(1-x)^2)).
    fn newton_step(
        &self,
        ctx: &Ctx,
        x: &[f64],
        lambda: &[f64],
        mu: f64,
    ) -> Result<(Vec<f64>, Vec<f64>), SolverError> {
        let n = ctx.num_vars;
        let m = ctx.rows.len();
        let mut r_d = vec![0.0; n];
        let mut r_p = vec![0.0; m];
        Self::residuals(ctx, x, lambda, mu, &mut r_d, &mut r_p);

        let mut h_inv = vec![0.0; n];
        for (i, &xi) in x.iter().enumerate() {
            let h = mu * (1.0 / (xi * xi) + 1.0 / ((1.0 - xi) * (1.0 - xi)));
            h_inv[i] = 1.0 / h;
        }

        let mut mat = vec![0.0; m * m];
        let mut rhs = r_p.clone();
        for (r, (row, _)) in ctx.rows.iter().enumerate() {
            for &(j, a) in row {
                rhs[r] -= a * h_inv[j] * r_d[j];
            }
        }
        for (j, entries) in ctx.col_rows.iter().enumerate() {
            for &(r, a) in entries {
                for &(s, b) in entries {
                    mat[r * m + s] += a * h_inv[j] * b;
                }
            }
        }
        for i in 0..m {
            mat[i * m + i] += self.reg;
        }

        let d_lambda = solve_dense(&mat, &rhs, m)
            .ok_or_else(|| SolverError::Backend("singular Newton system".into()))?;

        let mut dx = vec![0.0; n];
        for (i, &rd) in r_d.iter().enumerate() {
            dx[i] = -h_inv[i] * rd;
        }
        for (r, (row, _)) in ctx.rows.iter().enumerate() {
            for &(j, a) in row {
                dx[j] -= h_inv[j] * a * d_lambda[r];
            }
        }
        Ok((dx, d_lambda))
    }

    /// Newton iterations at fixed mu, with backtracking line search that
    /// keeps x strictly inside (0, 1). Returns the final residual norm.
    fn newton(
        &self,
        ctx: &Ctx,
        x: &mut [f64],
        lambda: &mut [f64],
        mu: f64,
    ) -> Result<f64, SolverError> {
        let n = ctx.num_vars;
        let m = ctx.rows.len();
        let mut r_d = vec![0.0; n];
        let mut r_p = vec![0.0; m];
        let mut norm = Self::residuals(ctx, x, lambda, mu, &mut r_d, &mut r_p);
        for _ in 0..self.max_newton {
            if norm < self.newton_tol {
                return Ok(norm);
            }
            let (dx, d_lambda) = self.newton_step(ctx, x, lambda, mu)?;
            let mut alpha = 1.0_f64;
            for (i, &dxi) in dx.iter().enumerate() {
                if dxi > 0.0 {
                    alpha = alpha.min((1.0 - x[i]) / dxi);
                } else if dxi < 0.0 {
                    alpha = alpha.min(x[i] / -dxi);
                }
            }
            alpha = (0.99 * alpha).min(1.0);
            let mut accepted = false;
            for _ in 0..self.max_line_search {
                let x_new: Vec<f64> = x.iter().zip(&dx).map(|(xi, d)| xi + alpha * d).collect();
                let l_new: Vec<f64> = lambda
                    .iter()
                    .zip(&d_lambda)
                    .map(|(l, d)| l + alpha * d)
                    .collect();
                let mut rd2 = vec![0.0; n];
                let mut rp2 = vec![0.0; m];
                let norm2 = Self::residuals(ctx, &x_new, &l_new, mu, &mut rd2, &mut rp2);
                if norm2 < norm || alpha < 1e-12 {
                    x.copy_from_slice(&x_new);
                    lambda.copy_from_slice(&l_new);
                    norm = norm2;
                    accepted = true;
                    break;
                }
                alpha *= 0.5;
            }
            if !accepted {
                // numerical noise floor: accept if the residual is already small
                if norm < self.accept_tol {
                    return Ok(norm);
                }
                return Err(SolverError::Backend(format!(
                    "line search failed at mu={mu}, residual={norm:e}"
                )));
            }
        }
        if norm < self.accept_tol {
            Ok(norm)
        } else {
            Err(SolverError::Backend(format!(
                "newton did not converge at mu={mu}, residual={norm:e}"
            )))
        }
    }
}

impl Solver for CentralPathSolver {
    fn solve(
        &self,
        system: &LinearSystem,
        request: &SolveRequest,
    ) -> Result<Assignment, SolverError> {
        if system.size != request.size
            || system.num_vars != request.target.len()
            || system.num_vars != request.start.len()
        {
            return Err(SolverError::Backend(
                "linear system and solve request do not match".into(),
            ));
        }
        let ctx = Self::make_ctx(system, request);
        let m = ctx.rows.len();

        // Warm start from the request's start state (default 1/N per variable,
        // strictly interior; a previous Assignment can be passed to continue
        // along the central path).
        let mut x = request.start.clone();
        let mut lambda = vec![0.0; m];

        // sweep mu down to epsilon, but never below the numerical floor
        let eps = request.epsilon.max(1e-10);
        let debug = std::env::var_os("SUDOKU_DEBUG").is_some();
        let mut mu = self.mu0;
        loop {
            let norm = self.newton(&ctx, &mut x, &mut lambda, mu)?;
            if debug {
                let obj: f64 = ctx.c.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
                eprintln!("mu={mu:e}  residual={norm:e}  c^T x={obj:.6}");
            }
            if mu <= eps {
                break;
            }
            mu = (mu * self.shrink).max(eps);
        }
        // clamp into the open interval so the result is a valid start state
        // for the next SolveRequest
        for v in &mut x {
            *v = v.clamp(1e-12, 1.0 - 1e-12);
        }
        Ok(Assignment { values: x })
    }
}

/// Gaussian elimination with partial pivoting: solves `a * x = b` for a
/// row-major `n x n` matrix `a`. Returns None if a pivot underflows to zero.
fn solve_dense(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut m = a.to_vec();
    let mut x = b.to_vec();
    for col in 0..n {
        let mut piv = col;
        let mut best = m[col * n + col].abs();
        for (r, row) in m.chunks_exact(n).enumerate().skip(col + 1) {
            let v = row[col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if best == 0.0 {
            return None;
        }
        if piv != col {
            for c in 0..n {
                m.swap(col * n + c, piv * n + c);
            }
            x.swap(col, piv);
        }
        let d = m[col * n + col];
        for r in col + 1..n {
            let f = m[r * n + col] / d;
            if f == 0.0 {
                continue;
            }
            for c in col..n {
                m[r * n + c] -= f * m[col * n + c];
            }
            x[r] -= f * x[col];
        }
    }
    let mut res = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = x[r];
        for c in r + 1..n {
            s -= m[r * n + c] * res[c];
        }
        res[r] = s / m[r * n + r];
    }
    Some(res)
}
