#[path = "common/vertical_baseline.rs"]
mod baseline;
#[path = "common/reflow_geometry.rs"]
mod geometry;

#[test]
fn vertical_cells_match_pre_l1_rendered_text_after_actual_edits() {
    let actual = baseline::census();
    let expected = include_str!("fixtures/vertical-cell-dd1600342.txt");
    let actual: Vec<_> = actual.lines().collect();
    let expected: Vec<_> = expected.lines().collect();
    assert_eq!(actual.len(), 1128);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert_eq!(actual, expected);
    }
}

#[test]
fn vertical_positive_geometry_renders_all_text_after_input_and_different_font_size() {
    use geometry::{fixture, Form, TEXT};
    use rhwp::model::control::Control;
    for direction in [1, 2] {
        for by_index in [false, true] {
            let form = Form::Cell {
                direction,
                merged: false,
                nested: false,
                inherited: false,
            };
            let mut f = fixture(form, Some(TEXT));
            let mut model = f.doc.document().clone();
            let Control::Table(table) =
                &mut model.sections[0].paragraphs[f.para_idx].controls[f.control_idx]
            else {
                panic!("table")
            };
            table.row_count = 1;
            table.col_count = 1;
            table.cells.retain(|cell| cell.row == 0 && cell.col == 0);
            table.cells[0].width = 20000;
            table.cells[0].height = 60000;
            table.dirty = true;
            table.update_ctrl_dimensions();
            f.doc.set_document(model);
            if by_index {
                f.path = None;
            }
            baseline::edit(&mut f, "900", TEXT);
            baseline::edit(&mut f, "input", TEXT);
            let expected = TEXT.repeat(2);
            let rendered = baseline::render_text(&f);
            assert_eq!(rendered.chars().count(), 112);
            assert_eq!(rendered, expected);
            baseline::edit(&mut f, "600", TEXT);
            let shape_id = usize::try_from(f.paragraph().char_shape_id_at(0).unwrap()).unwrap();
            assert_eq!(
                f.doc.document().doc_info.char_shapes[shape_id].base_size,
                600
            );
            let rendered = baseline::render_text(&f);
            assert_eq!(rendered.chars().count(), 112);
            assert_eq!(rendered, expected);
        }
    }
}
