//! Web UI (feature `web`): axum server on 127.0.0.1:8080 (loopback only).
//!
//! - `GET /` renders the **full page server-side** from the cookies — the UI
//!   works entirely without JavaScript (forms POST to `/command`).
//! - With wasm available, `static/app.js` takes over as progressive
//!   enhancement: local render, local solve via the wasm exports
//!   (`src/wasm.rs`), self-updated cookies (same Rust codec → byte-identical)
//!   and the §7 mouse interaction.
//! - `POST /api/command` is the JSON API (for JS clients without wasm).
//! - **No server-side state** — everything lives in the cookies (REQUIREMENTS §6).

pub mod render;

use crate::app;
use crate::solver::central_path::CentralPathSolver;
pub use crate::ui::cookie::{encode_cookies, state_from_cookies};
use crate::ui::cookie::{state_to_json, StateJson};
use crate::ui::{parse_command, Command, MAX_N_WEB};
use axum::extract::Form;
use axum::http::{header::SET_COOKIE, HeaderMap, HeaderValue};
use axum::response::{Html, IntoResponse, Json, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};

/// Start the web UI on http://127.0.0.1:8080 (loopback only).
pub fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Runtime::new()?.block_on(async {
        let app = Router::new()
            .route("/", get(index))
            .route("/static/app.js", get(app_js))
            .route("/command", post(command_form))
            .route("/api/command", post(api_command));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
        println!("web UI on http://127.0.0.1:8080");
        axum::serve(listener, app).await?;
        Ok(())
    })
}

/// Full server-rendered page from the cookies — works without any JavaScript.
async fn index(headers: HeaderMap) -> Response {
    let (state, note) = state_from_cookies(&cookie_map(&headers));
    let mut resp = Html(render::page(&state, &note)).into_response();
    if !note.is_empty() {
        // flash note: clear it again right away
        set_cookie(&mut resp, "sudoku_note", "", 0);
    }
    resp
}

async fn app_js() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("static/app.js"),
    )
        .into_response()
}

#[derive(Deserialize)]
struct CmdForm {
    cmd: String,
}

/// No-JS form POST → run command → set cookies → redirect to /.
async fn command_form(headers: HeaderMap, Form(form): Form<CmdForm>) -> Response {
    let mut state = state_from_cookies(&cookie_map(&headers)).0;
    let cmd = match parse_command(form.cmd.trim()) {
        Ok(c) => c,
        Err(e) => {
            let mut resp = Redirect::to("/").into_response();
            set_cookie(
                &mut resp,
                "sudoku_note",
                &b64_str(&format!("error: {e}")),
                60,
            );
            return resp;
        }
    };
    // `rand` without seed (no-JS button): time-based seed, generated server-side
    let cmd = match cmd {
        Command::Rand { seed: None } => Command::Rand {
            seed: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(crate::ui::DEFAULT_SEED),
            ),
        },
        other => other,
    };
    let msg = app::step(&mut state, &CentralPathSolver::new(), &cmd, MAX_N_WEB);
    let mut resp = Redirect::to("/").into_response();
    for (name, value) in encode_cookies(&state, &msg.unwrap_or_default()) {
        set_cookie(&mut resp, &name, &value, 60 * 60 * 24 * 30);
    }
    resp
}

#[derive(Deserialize)]
struct ApiRequest {
    cmd: String,
    /// raw cookie pairs [[name, value], ...] sent by the browser
    cookies: Vec<(String, String)>,
}

#[derive(Serialize)]
struct ApiResponse {
    note: String,
    state: StateJson,
    /// cookie pairs [[name, value], ...] for the browser to write itself
    cookies: Vec<(String, String)>,
}

/// JSON API used by the page when wasm is unavailable: the browser sends its
/// cookies, the server runs the same `app::step` and returns the new state +
/// cookies (encoded by the same Rust codec → byte-identical).
async fn api_command(Json(req): Json<ApiRequest>) -> Response {
    let mut state = state_from_cookies(&req.cookies).0;
    let (note, cmd) = match parse_command(req.cmd.trim()) {
        Ok(c) => (String::new(), c),
        Err(e) => (format!("error: {e}"), Command::Show),
    };
    let msg = app::step(&mut state, &CentralPathSolver::new(), &cmd, MAX_N_WEB);
    let note = match (note.is_empty(), msg) {
        (true, m) => m.unwrap_or_default(),
        (false, Some(m)) => format!("{note}; {m}"),
        (false, None) => note,
    };
    Json(ApiResponse {
        note: note.clone(),
        state: state_to_json(&state, &note),
        cookies: encode_cookies(&state, &note),
    })
    .into_response()
}

fn set_cookie(resp: &mut Response, name: &str, value: &str, max_age: u32) {
    let v = format!("{name}={value}; Path=/; SameSite=Lax; Max-Age={max_age}");
    if let Ok(hv) = HeaderValue::from_str(&v) {
        resp.headers_mut().append(SET_COOKIE, hv);
    }
}

fn b64_str(s: &str) -> String {
    crate::ui::cookie::b64(s.as_bytes())
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
