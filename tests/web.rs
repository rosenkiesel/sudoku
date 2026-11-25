#![cfg(feature = "web")]

use base64::Engine;
use sudoku_rs::solver::Assignment;
use sudoku_rs::ui::web::{encode_cookies, state_from_cookies};
use sudoku_rs::ui::{Command, UiState, ViewMode, MAX_N_CONSOLE, MAX_N_WEB};

const GRID_6: &str = "1.,3.,5.;.4,.6,.2;5.,.2,.4;.3,4.,.1;4.,.1,2.;.1,.3,.5";

#[test]
fn cookie_roundtrip() {
    let mut state = UiState::new();
    state
        .apply(&Command::New { bx: 2, by: 3 }, MAX_N_CONSOLE)
        .unwrap();
    state
        .apply(&Command::Load(GRID_6.to_string()), MAX_N_CONSOLE)
        .unwrap();
    state
        .apply(&Command::Rand { seed: Some(42) }, MAX_N_CONSOLE)
        .unwrap();
    state
        .apply(&Command::Rand { seed: Some(7) }, MAX_N_CONSOLE)
        .unwrap();
    state.apply(&Command::EpsMinus, MAX_N_CONSOLE).unwrap();
    state
        .apply(&Command::View(ViewMode::Probabilities), MAX_N_CONSOLE)
        .unwrap();
    state.assignment = Some(Assignment {
        values: vec![0.5; 216],
    });

    let cookies = encode_cookies(&state, "hello");
    let (back, note) = state_from_cookies(&cookies);

    assert_eq!(note, "hello");
    let grid = back.grid.unwrap();
    assert_eq!(grid.dimensions(), (2, 3));
    assert_eq!(grid.format(), GRID_6);
    assert_eq!(back.view, ViewMode::Probabilities);
    assert_eq!(back.seeds, vec![7, 42]); // seed history, most recent first
    assert!((back.epsilon - 1e-7).abs() < 1e-18);
    assert_eq!(back.base.len(), 216);
    assert!(back.base.iter().any(|&v| v != 0.0));
    let a = back.assignment.unwrap();
    assert_eq!(a.values.len(), 216);
    assert!(a.values.iter().all(|&v| v > 0.0 && v < 1.0)); // valid warm start
    assert!((a.values[0] - 0.5).abs() < 0.01);
}

#[test]
fn web_size_limit_12x12() {
    let mut state = UiState::new();
    assert!(state
        .apply(&Command::New { bx: 3, by: 4 }, MAX_N_WEB)
        .is_ok()); // 12 ok
    let mut state = UiState::new();
    assert!(state
        .apply(&Command::New { bx: 3, by: 5 }, MAX_N_WEB)
        .is_err()); // 15 rejected
    assert!(state
        .apply(&Command::New { bx: 4, by: 4 }, MAX_N_WEB)
        .is_err()); // 16 rejected
}

#[test]
fn unknown_version_cookie_starts_fresh() {
    let cookies = vec![(
        "sudoku_state".to_string(),
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"v":99,"eps":1e-3,"view":"winner"}"#),
    )];
    let (state, note) = state_from_cookies(&cookies);
    assert!(state.grid.is_none());
    assert!(note.is_empty());
}

#[test]
fn corrupt_cookies_fall_back_per_field() {
    let cookies = vec![
        (
            "sudoku_state".to_string(),
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(r#"{"v":1,"eps":1e-6,"view":"bogus"}"#),
        ),
        ("sudoku_givens".to_string(), "!!!not-base64!!!".to_string()),
        ("sudoku_base".to_string(), "AAAA".to_string()), // wrong length
    ];
    let (state, _) = state_from_cookies(&cookies);
    assert!(state.grid.is_none()); // givens undecodable -> fresh session
    assert_eq!(state.view, ViewMode::Winner); // unknown view -> default
}

#[test]
fn server_loopback_get_and_api() {
    use std::io::{Read, Write};
    std::thread::spawn(|| {
        let _ = sudoku_rs::ui::web::serve();
    });
    std::thread::sleep(std::time::Duration::from_millis(700));
    let mut s = std::net::TcpStream::connect("127.0.0.1:8080").expect("loopback connect");
    s.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut resp = String::new();
    s.read_to_string(&mut resp).unwrap();
    assert!(
        resp.starts_with("HTTP/1.1 200"),
        "got: {}",
        &resp[..40.min(resp.len())]
    );
    assert!(resp.contains("sudoku-rs"));
    assert!(resp.contains("no puzzle yet")); // server-rendered, works without JS
    assert!(resp.contains("<form"));

    // POST /api/command: no wasm in this test -> server computes
    let body = r#"{"cmd":"new 3 3","cookies":[]}"#;
    let req = format!(
        "POST /api/command HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );
    let mut s = std::net::TcpStream::connect("127.0.0.1:8080").expect("loopback connect");
    s.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    s.read_to_string(&mut resp).unwrap();
    assert!(
        resp.starts_with("HTTP/1.1 200"),
        "got: {}",
        &resp[..40.min(resp.len())]
    );
    assert!(
        resp.contains("sudoku_state"),
        "api must return cookie pairs"
    );
    assert!(!resp.contains("530,070,000")); // fresh 3x3 has empty grid
    assert!(resp.contains("\"grid\":\"...") || resp.contains("null") || resp.contains("grid"));
}
