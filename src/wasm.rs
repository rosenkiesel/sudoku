//! wasm exports (feature `wasm`, target `wasm32-unknown-unknown`; build with
//! `wasm-pack build --target web --no-default-features --features wasm`).
//!
//! The browser uses these for fully client-side operation (REQUIREMENTS §6
//! "Client-side mode (wasm)"): decode/encode the state cookies and run the
//! solver locally. The cookie codec is the same Rust code the axum server
//! uses, so cookies stay byte-identical. Without wasm the page falls back to
//! `POST /api/command` on the server.

use crate::model::game_rules::build_system;
use crate::model::given_grid::GivenGrid;
use crate::model::request::{default_start, SolveRequest};
use crate::model::{grid_indices, Size};
use crate::solver::central_path::CentralPathSolver;
use crate::solver::Solver;
use crate::ui::cookie::{
    encode_cookies, state_from_cookies, state_from_json, state_to_json, StateJson,
};
use crate::ui::{conflicts, winners, Rng, DEFAULT_SEED, GIVEN_WEIGHT};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Decode cookie pairs `[[name, value], ...]` into the plain state JSON.
#[wasm_bindgen]
pub fn cookie_decode(pairs: JsValue) -> Result<JsValue, JsValue> {
    let pairs: Vec<(String, String)> = serde_wasm_bindgen::from_value(pairs)
        .map_err(|e| JsValue::from_str(&format!("bad cookie pairs: {e}")))?;
    let (state, note) = state_from_cookies(&pairs);
    serde_wasm_bindgen::to_value(&state_to_json(&state, &note))
        .map_err(|e| JsValue::from_str(&format!("encode state: {e}")))
}

/// Encode a plain state JSON into cookie pairs `[[name, value], ...]`.
#[wasm_bindgen]
pub fn cookie_encode(state: JsValue) -> Result<JsValue, JsValue> {
    let j: StateJson = serde_wasm_bindgen::from_value(state)
        .map_err(|e| JsValue::from_str(&format!("bad state: {e}")))?;
    let ui = state_from_json(&j);
    let cookies = encode_cookies(&ui, &j.note);
    serde_wasm_bindgen::to_value(&cookies)
        .map_err(|e| JsValue::from_str(&format!("encode cookies: {e}")))
}

/// Random target base for a seed (same SplitMix64 as the server/console `rand`).
#[wasm_bindgen]
pub fn rand_base(seed: f64, num_vars: usize) -> Vec<f64> {
    let mut rng = Rng::new(seed as u64);
    (0..num_vars).map(|_| rng.next_f64() * 2.0 - 1.0).collect()
}

#[derive(Serialize)]
pub struct SolveJson {
    pub values: Vec<f64>,
    pub winners: Vec<u8>,
    pub conflict_rows: Vec<usize>,
    pub conflict_cols: Vec<usize>,
}

/// Solve a puzzle client-side: `givens` is the grid string (size inferred),
/// `base` the target base (null → from `seed` or zeros), `start` the warm-start
/// values (null → default 1/N).
#[wasm_bindgen]
pub fn solve(
    givens: &str,
    eps: f64,
    seed: Option<f64>,
    base: Option<Vec<f64>>,
    start: Option<Vec<f64>>,
) -> Result<JsValue, JsValue> {
    let grid = GivenGrid::parse_auto(givens).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let size: Size = grid.size();
    let mut target = match base {
        Some(b) if b.len() == size.num_vars() => b,
        _ => rand_base(seed.unwrap_or(DEFAULT_SEED as f64), size.num_vars()),
    };
    for (&(x, y), &d) in grid.givens() {
        target[grid_indices::idx(&size, x, y, d)] += GIVEN_WEIGHT;
    }
    let start = match start {
        Some(s) if s.len() == size.num_vars() => s,
        _ => default_start(&size),
    };
    let request = SolveRequest::with_start(size, target, eps, start)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let system = build_system(&size);
    let assignment = CentralPathSolver::new()
        .solve(&system, &request)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let wins = winners(&size, &assignment.values);
    let (conflict_rows, conflict_cols) = conflicts(&size, &wins);
    serde_wasm_bindgen::to_value(&SolveJson {
        values: assignment.values,
        winners: wins,
        conflict_rows,
        conflict_cols,
    })
    .map_err(|e| JsValue::from_str(&format!("encode result: {e}")))
}
