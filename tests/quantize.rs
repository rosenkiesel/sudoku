use sudoku_rs::model::quantize::{
    dequantize_signed, dequantize_unit, quantize_signed, quantize_unit,
};
use sudoku_rs::model::Size;

#[test]
fn unit_quantize_roundtrip_stays_interior() {
    for &v in &[0.0, 0.25, 0.5, 0.99, 1.0] {
        let d = dequantize_unit(quantize_unit(v));
        assert!((d - v).abs() <= 0.5 / 255.0 + 1e-12, "v={v} d={d}");
        assert!(
            d > 0.0 && d < 1.0,
            "dequantized value must be a valid start: {d}"
        );
    }
}

#[test]
fn signed_quantize_roundtrip() {
    for &v in &[-1.0, -0.3, 0.0, 0.7, 1.0] {
        let d = dequantize_signed(quantize_signed(v));
        assert!((d - v).abs() <= 1.0 / 127.5 + 1e-12, "v={v} d={d}");
    }
}

#[test]
fn base_vector_quantizes_to_n_cubed_bytes() {
    let s = Size::new(2, 3).unwrap();
    let base: Vec<f64> = (0..s.num_vars())
        .map(|i| ((i as f64 * 0.37) % 2.0) - 1.0)
        .collect();
    let packed: Vec<u8> = base.iter().map(|&v| quantize_signed(v)).collect();
    assert_eq!(packed.len(), 216); // N^3 bytes for 6x6
    let back: Vec<f64> = packed.iter().map(|&q| dequantize_signed(q)).collect();
    assert!(base.iter().zip(&back).all(|(a, b)| (a - b).abs() < 0.01));
}
