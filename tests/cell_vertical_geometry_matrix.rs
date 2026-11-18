#[path = "common/vertical_geometry_matrix.rs"]
mod matrix;

#[test]
fn vertical_geometry_matrix_matches_pre_l1_segments_text_and_content_height() {
    use std::io::Read;
    let mut expected = String::new();
    flate2::read::GzDecoder::new(
        &include_bytes!("fixtures/vertical-geometry-dd1600342.txt.gz")[..],
    )
    .read_to_string(&mut expected)
    .unwrap();
    assert_eq!(matrix::cases().len(), 8064);
    assert_eq!(expected.lines().count(), 40320);
    let mut rows = expected.lines();
    for case in matrix::cases() {
        for action in ["document", "input", "900", "600", "growth"] {
            let actual = matrix::snapshot(case, action);
            assert_eq!(Some(actual.as_str()), rows.next());
        }
    }
    assert_eq!(rows.next(), None);
}
