//! Cookie codec for the web UI (REQUIREMENTS §6) — implemented **once in Rust**
//! and shared by the axum server and the wasm build, so client-side and
//! server-side cookie writes stay byte-identical.
//!
//! Cookies (all values base64url): `sudoku_state` (manifest JSON v1: eps, view),
//! `sudoku_givens` (grid string), `sudoku_base` (N³ u8, quantize_signed),
//! `sudoku_start` (N³ u8, quantize_unit, optional), `sudoku_seeds` (JSON array,
//! ≤ MAX_SEEDS), `sudoku_note` (flash message).

use crate::model::given_grid::GivenGrid;
use crate::model::quantize::{dequantize_signed, dequantize_unit, quantize_signed, quantize_unit};
use crate::solver::Assignment;
use crate::ui::{UiState, ViewMode, DEFAULT_SEED, MAX_N_WEB, MAX_SEEDS};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Plain JSON-able view of the UI state — the interchange format between the
/// browser JS, the wasm exports and the axum JSON API.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateJson {
    pub grid: Option<String>, // GivenGrid string (size inferred via parse_auto)
    pub eps: f64,
    pub view: String,            // "winner" | "probs"
    pub base: Option<Vec<f64>>,  // random target base (null → zeros)
    pub start: Option<Vec<f64>>, // last assignment values (warm start)
    #[serde(default)]
    pub seeds: Vec<f64>, // recent rand seeds (as f64 for JS interop), MRU first
    #[serde(default)]
    pub note: String,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    v: u32,
    eps: f64,
    view: String,
}

pub fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn unb64(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD.decode(s)
}

pub fn state_to_json(state: &UiState, note: &str) -> StateJson {
    StateJson {
        grid: state.grid.as_ref().map(|g| g.format()),
        eps: state.epsilon,
        view: state.view.as_str().to_string(),
        base: (state.grid.is_some() && state.base.iter().any(|&v| v != 0.0))
            .then(|| state.base.clone()),
        start: state.assignment.as_ref().map(|a| a.values.clone()),
        seeds: state.seeds.iter().map(|&s| s as f64).collect(),
        note: note.to_string(),
    }
}

/// Rebuild a UiState from its JSON view (defensive: size cap, length checks).
pub fn state_from_json(j: &StateJson) -> UiState {
    let mut state = UiState::new();
    state.epsilon = j.eps.clamp(1e-12, 1.0);
    state.view = ViewMode::parse(&j.view).unwrap_or(ViewMode::Winner);
    state.seeds = j.seeds.iter().take(MAX_SEEDS).map(|&s| s as u64).collect();
    state.seed = state.seeds.first().copied().unwrap_or(DEFAULT_SEED);
    if let Some(text) = &j.grid {
        if let Ok(grid) = GivenGrid::parse_auto(text) {
            let size = grid.size();
            if size.n() <= MAX_N_WEB {
                state.base = vec![0.0; size.num_vars()];
                if let Some(base) = &j.base {
                    if base.len() == size.num_vars() {
                        state.base = base.clone();
                    }
                }
                if let Some(start) = &j.start {
                    if start.len() == size.num_vars() && start.iter().all(|v| v.is_finite()) {
                        let clamped: Vec<f64> =
                            start.iter().map(|&v| v.clamp(1e-12, 1.0 - 1e-12)).collect();
                        state.assignment = Some(Assignment { values: clamped });
                    }
                }
                state.grid = Some(grid);
            }
        }
    }
    state
}

/// Serialize the state into cookies (name, base64url value), per REQUIREMENTS §6.
pub fn encode_cookies(state: &UiState, note: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let manifest = serde_json::json!({
        "v": 1,
        "eps": state.epsilon,
        "view": state.view.as_str(),
    });
    out.push((
        "sudoku_state".to_string(),
        b64(manifest.to_string().as_bytes()),
    ));
    if let Some(grid) = &state.grid {
        out.push(("sudoku_givens".to_string(), b64(grid.format().as_bytes())));
        if state.base.iter().any(|&v| v != 0.0) {
            let packed: Vec<u8> = state.base.iter().map(|&v| quantize_signed(v)).collect();
            out.push(("sudoku_base".to_string(), b64(&packed)));
        }
        if let Some(a) = &state.assignment {
            let packed: Vec<u8> = a.values.iter().map(|&v| quantize_unit(v)).collect();
            out.push(("sudoku_start".to_string(), b64(&packed)));
        }
    }
    if !state.seeds.is_empty() {
        let seeds: Vec<u64> = state.seeds.iter().take(MAX_SEEDS).copied().collect();
        out.push((
            "sudoku_seeds".to_string(),
            b64(serde_json::to_string(&seeds).unwrap().as_bytes()),
        ));
    }
    if !note.is_empty() {
        out.push(("sudoku_note".to_string(), b64(note.as_bytes())));
    }
    out
}

/// Rebuild the state from cookies, with per-cookie fallbacks (REQUIREMENTS §6).
/// Returns (state, flash note).
pub fn state_from_cookies(cookies: &[(String, String)]) -> (UiState, String) {
    let get = |name: &str| {
        cookies
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let mut state = UiState::new();
    let mut note = String::new();

    let Some(manifest) = get("sudoku_state")
        .and_then(|v| unb64(&v).ok())
        .and_then(|b| serde_json::from_slice::<Manifest>(&b).ok())
    else {
        return (state, note);
    };
    if manifest.v != 1 {
        return (UiState::new(), note); // unknown version: start fresh
    }
    state.epsilon = manifest.eps.clamp(1e-12, 1.0);
    state.view = ViewMode::parse(&manifest.view).unwrap_or(ViewMode::Winner);

    if let Some(grid) = get("sudoku_givens")
        .and_then(|v| unb64(&v).ok())
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| GivenGrid::parse_auto(&s).ok())
    {
        if grid.size().n() <= MAX_N_WEB {
            state.base = vec![0.0; grid.size().num_vars()];
            state.grid = Some(grid);
        }
    }
    if let (Some(grid), Some(b)) = (&state.grid, get("sudoku_base")) {
        if let Ok(bytes) = unb64(&b) {
            if bytes.len() == grid.size().num_vars() {
                state.base = bytes.iter().map(|&u| dequantize_signed(u)).collect();
            }
        }
    }
    if let (Some(grid), Some(s)) = (&state.grid, get("sudoku_start")) {
        if let Ok(bytes) = unb64(&s) {
            if bytes.len() == grid.size().num_vars() {
                // dequantize_unit is strictly inside (0,1) → valid warm start
                state.assignment = Some(Assignment {
                    values: bytes.iter().map(|&u| dequantize_unit(u)).collect(),
                });
            }
        }
    }
    if let Some(seeds) = get("sudoku_seeds")
        .and_then(|v| unb64(&v).ok())
        .and_then(|b| serde_json::from_slice::<Vec<u64>>(&b).ok())
    {
        state.seeds = seeds.into_iter().take(MAX_SEEDS).collect();
        state.seed = state.seeds.first().copied().unwrap_or(DEFAULT_SEED);
    }
    if let Some(n) = get("sudoku_note")
        .and_then(|v| unb64(&v).ok())
        .and_then(|b| String::from_utf8(b).ok())
    {
        note = n;
    }
    (state, note)
}
