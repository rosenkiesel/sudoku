//! Web UI (feature `web`): axum server on 127.0.0.1:8080 (loopback only).
//! `GET /` renders the current state as HTML, `POST /command` applies one
//! command. **No server-side state** — the whole `UiState` lives in cookies
//! (REQUIREMENTS §6): `sudoku_state` manifest (version + eps + view) plus
//! `sudoku_givens` / `sudoku_base` / `sudoku_start` / `sudoku_note`, all
//! base64url-encoded. Grid sizes above 12×12 are rejected (cookie limit).

pub mod render;

use crate::app;
use crate::model::given_grid::GivenGrid;
use crate::model::quantize::{dequantize_signed, dequantize_unit, quantize_signed, quantize_unit};
use crate::solver::central_path::CentralPathSolver;
use crate::solver::Assignment;
use crate::ui::{parse_command, Command, UiState, ViewMode, MAX_N_WEB};
use axum::extract::Form;
use axum::http::{header::SET_COOKIE, HeaderMap, HeaderValue};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Start the web UI on http://127.0.0.1:8080 (loopback only).
pub fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Runtime::new()?.block_on(async {
        let app = Router::new()
            .route("/", get(index))
            .route("/command", post(command));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
        println!("web UI on http://127.0.0.1:8080");
        axum::serve(listener, app).await?;
        Ok(())
    })
}

#[derive(Deserialize)]
struct CmdForm {
    cmd: String,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    v: u32,
    eps: f64,
    view: String,
}

async fn index(headers: HeaderMap) -> Response {
    let (state, note) = state_from_cookies(&cookie_map(&headers));
    let mut resp = Html(render::page(&state, &note)).into_response();
    if !note.is_empty() {
        // flash note: clear it again right away
        set_cookie(&mut resp, "sudoku_note", "", 0);
    }
    resp
}

async fn command(headers: HeaderMap, Form(form): Form<CmdForm>) -> Response {
    let mut state = state_from_cookies(&cookie_map(&headers)).0;
    let (note, cmd) = match parse_command(form.cmd.trim()) {
        Ok(c) => (String::new(), c),
        Err(e) => (format!("error: {e}"), Command::Show),
    };
    let msg = app::step(&mut state, &CentralPathSolver::new(), &cmd, MAX_N_WEB);
    let note = match (note.is_empty(), msg) {
        (true, m) => m.unwrap_or_default(),
        (false, Some(m)) => format!("{note}; {m}"),
        (false, None) => note,
    };
    let mut resp = Redirect::to("/").into_response();
    for (name, value) in encode_cookies(&state, &note) {
        set_cookie(&mut resp, &name, &value, 60 * 60 * 24 * 30);
    }
    resp
}

fn set_cookie(resp: &mut Response, name: &str, value: &str, max_age: u32) {
    let v = format!("{name}={value}; Path=/; SameSite=Lax; Max-Age={max_age}");
    if let Ok(hv) = HeaderValue::from_str(&v) {
        resp.headers_mut().append(SET_COOKIE, hv);
    }
}

fn cookie_map(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for h in headers.get_all(axum::http::header::COOKIE) {
        if let Ok(s) = h.to_str() {
            for pair in s.split(';') {
                if let Some((k, v)) = pair.trim().split_once('=') {
                    out.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
        }
    }
    out
}

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD.decode(s)
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
    if let Some(n) = get("sudoku_note")
        .and_then(|v| unb64(&v).ok())
        .and_then(|b| String::from_utf8(b).ok())
    {
        note = n;
    }
    (state, note)
}
