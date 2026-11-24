//! Quantization helpers for compact state storage (e.g. cookies in a future
//! web UI): probabilities and the random base are stored as u8 (N³ bytes for
//! an N×N grid) instead of f64 (8·N³ bytes).

/// Quantize a probability in [0, 1] to u8 (used e.g. for a warm-start assignment).
pub fn quantize_unit(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Dequantize back (same 255 scale as `quantize_unit`). The result is clamped
/// strictly inside (0, 1), so a dequantized assignment is always a valid
/// `start` for the next SolveRequest.
pub fn dequantize_unit(u: u8) -> f64 {
    (u as f64 / 255.0).clamp(1e-12, 1.0 - 1e-12)
}

/// Quantize a value in [-1, 1] to u8 (used e.g. for the random target base).
pub fn quantize_signed(v: f64) -> u8 {
    ((v.clamp(-1.0, 1.0) + 1.0) * 127.5).round() as u8
}

/// Dequantize a [-1, 1] value stored as u8.
pub fn dequantize_signed(u: u8) -> f64 {
    u as f64 / 127.5 - 1.0
}
