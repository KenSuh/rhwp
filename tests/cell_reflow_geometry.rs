// 각 컨테이너 형태에서 글자 크기·줄 간격·입력이 현재 안쪽 용량으로 줄을 나누는지 시험한다.
#[path = "common/reflow_geometry.rs"]
mod geometry;

use geometry::{fixture, Fixture, Form, TEXT};
use rhwp::model::control::Control;
use rhwp::model::shape::CaptionDirection::{Bottom, Left, Right, Top};

#[derive(Clone, Copy, Debug)]
enum Edit {
    FontSize,
    LineSpacing,
    Input,
}

fn check(form: Form, edit: Edit) {
    check_text(form, edit, TEXT);
}

fn check_text(form: Form, edit: Edit, text: &str) {
    let mut f = fixture(
        form,
        if matches!(edit, Edit::Input) {
            None
        } else {
            Some(text)
        },
    );
    apply_edit(&mut f, edit, text);
    let para = f.paragraph();
    assert_eq!(para.text, text, "{form:?} {edit:?}");
    assert!(
        para.line_segs.len() > 1,
        "{form:?} {edit:?}: fixture must wrap"
    );
    for line in &para.line_segs {
        assert!(
            (f.capacity - 1..=f.capacity).contains(&line.segment_width),
            "{form:?} {edit:?}: capacity {}, got {}",
            f.capacity,
            line.segment_width
        );
        match edit {
            Edit::FontSize => assert!(
                (899..=901).contains(&line.line_height),
                "{form:?}: new font metrics {}",
                line.line_height
            ),
            Edit::LineSpacing => assert_eq!(
                line.line_spacing,
                line.line_height * 2,
                "{form:?}: new spacing"
            ),
            Edit::Input => {}
        }
    }
    let tree = f.doc.get_page_render_tree(0).unwrap();
    let rendered: String = tree
        .split("\"text\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    assert_eq!(rendered, text, "{form:?} {edit:?}: native rendered text");
    assert!(
        f.doc.render_page_svg(0).unwrap().contains("<svg"),
        "{form:?} {edit:?}: SVG render"
    );
}

fn apply_edit(f: &mut Fixture, edit: Edit, text: &str) {
    let end = text.chars().count();
    match (edit, &f.path) {
        (Edit::FontSize, Some(path)) => {
            f.doc
                .apply_char_format_in_cell_by_path(
                    0,
                    f.para_idx,
                    path,
                    0,
                    end,
                    "{\"fontSize\":900}",
                )
                .unwrap();
        }
        (Edit::LineSpacing, Some(path)) => {
            f.doc
                .apply_para_format_in_cell_by_path(
                    0,
                    f.para_idx,
                    path,
                    "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
                )
                .unwrap();
        }
        (Edit::Input, Some(path)) => {
            f.doc
                .insert_text_in_cell_by_path_api(0, f.para_idx.try_into().unwrap(), path, 0, text)
                .unwrap();
        }
        (Edit::FontSize, None) => {
            f.doc
                .apply_char_format_in_cell(
                    0,
                    f.para_idx,
                    f.control_idx,
                    f.cell_idx,
                    0,
                    0,
                    end,
                    "{\"fontSize\":900}",
                )
                .unwrap();
        }
        (Edit::LineSpacing, None) => {
            f.doc
                .apply_para_format_in_cell(
                    0,
                    f.para_idx,
                    f.control_idx,
                    f.cell_idx,
                    0,
                    "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
                )
                .unwrap();
        }
        (Edit::Input, None) => {
            f.doc
                .insert_text_in_cell(
                    0,
                    f.para_idx.try_into().unwrap(),
                    f.control_idx.try_into().unwrap(),
                    f.cell_idx.try_into().unwrap(),
                    0,
                    0,
                    text,
                )
                .unwrap();
        }
    }
}

fn remove_capacity(f: &mut Fixture, form: Form, both_axes: bool) {
    let mut model = f.doc.document().clone();
    let mut control = &mut model.sections[0].paragraphs[f.para_idx].controls[f.control_idx];
    if matches!(form, Form::Cell { nested: true, .. }) {
        let Control::Table(table) = control else {
            panic!("outer table")
        };
        control = &mut table.cells[0].paragraphs[0].controls[0];
    }
    match control {
        Control::Table(table) if matches!(form, Form::Cell { .. }) => {
            table.common.height = 0;
            table.dirty = true;
            for cell in &mut table.cells {
                if cell.text_direction == 0 || both_axes {
                    cell.width = 0;
                }
                if cell.text_direction != 0 || both_axes {
                    cell.height = 0;
                }
            }
        }
        Control::Table(table) => match table.caption.as_ref().unwrap().direction {
            Top | Bottom => {
                for cell in &mut table.cells {
                    cell.width = 0;
                }
            }
            Left | Right => table.caption.as_mut().unwrap().width = 0,
        },
        Control::Shape(shape) => {
            shape.common_mut().width = 0;
            if both_axes {
                shape.common_mut().height = 0;
            }
        }
        Control::Picture(pic) => {
            match pic.caption.as_ref().unwrap().direction {
                Top | Bottom => pic.common.width = 0,
                Left | Right => pic.caption.as_mut().unwrap().width = 0,
            }
            if both_axes {
                pic.common.height = 0;
            }
        }
        _ => panic!("fixture geometry"),
    }
    f.doc.set_document(model);
}

#[test]
fn unavailable_geometry_matrix_preserves_segments() {
    let mut forms = Vec::new();
    for direction in [0, 1, 2] {
        for merged in [false, true] {
            for nested in [false, true] {
                for inherited in [false, true] {
                    forms.push(Form::Cell {
                        direction,
                        merged,
                        nested,
                        inherited,
                    });
                }
            }
        }
    }
    forms.extend([
        Form::TextBox { rotation: 0 },
        Form::TextBox { rotation: 90 },
    ]);
    for direction in [Top, Bottom, Left, Right] {
        forms.push(Form::TableCaption {
            direction,
            merged: false,
            spacing: 0,
        });
        forms.push(Form::TableCaption {
            direction,
            merged: true,
            spacing: 200,
        });
        forms.push(Form::PictureCaption(direction));
    }
    let mut scenarios = 0;
    for form in forms {
        for both_axes in [false, true] {
            for by_index in [false, true] {
                for edit in [Edit::FontSize, Edit::LineSpacing, Edit::Input] {
                    let mut f = fixture(form, Some(TEXT));
                    if by_index {
                        if matches!(form, Form::Cell { nested: true, .. }) {
                            continue;
                        }
                        f.path = None;
                    }
                    apply_edit(&mut f, Edit::FontSize, TEXT);
                    remove_capacity(&mut f, form, both_axes);
                    let mut expected_para = f.paragraph().clone();
                    if matches!(edit, Edit::Input) {
                        expected_para.insert_text_at(0, TEXT);
                    }
                    let old = format!("{:?}", expected_para.line_segs);
                    apply_edit(&mut f, edit, TEXT);
                    assert_eq!(
                        format!("{:?}", f.paragraph().line_segs),
                        old,
                        "{form:?} {edit:?} both={both_axes} index={by_index}"
                    );
                    let expected = if matches!(edit, Edit::Input) {
                        TEXT.repeat(2)
                    } else {
                        TEXT.to_string()
                    };
                    assert_eq!(f.paragraph().text, expected);
                    f.doc.reflow_linesegs();
                    assert_eq!(
                        format!("{:?}", f.paragraph().line_segs),
                        old,
                        "{form:?}: document reflow must preserve unavailable geometry"
                    );
                    scenarios += 1;
                }
            }
        }
    }
    assert_eq!(scenarios, 384);
}

#[test]
fn zero_cross_axis_still_reflows_at_resolved_capacity() {
    for direction in [0, 1, 2] {
        for by_index in [false, true] {
            let form = Form::Cell {
                direction,
                merged: false,
                nested: false,
                inherited: false,
            };
            let mut f = fixture(form, Some(TEXT));
            if by_index {
                f.path = None;
            }
            let mut model = f.doc.document().clone();
            let Control::Table(table) =
                &mut model.sections[0].paragraphs[f.para_idx].controls[f.control_idx]
            else {
                panic!("table")
            };
            for cell in &mut table.cells {
                if direction == 0 {
                    cell.height = 0;
                } else {
                    cell.width = 0;
                }
            }
            f.doc.set_document(model);
            apply_edit(&mut f, Edit::FontSize, TEXT);
            assert!(f.paragraph().line_segs.len() > 1);
            assert!(f.paragraph().line_segs.iter().all(|line| {
                (f.capacity - 1..=f.capacity).contains(&line.segment_width)
                    && (899..=901).contains(&line.line_height)
            }));
        }
    }
}

#[test]
fn automatic_height_vertical_cell_keeps_rendered_text() {
    for direction in [1, 2] {
        for by_index in [false, true] {
            let form = Form::Cell {
                direction,
                merged: false,
                nested: false,
                inherited: false,
            };
            let mut f = fixture(form, Some(TEXT));
            if by_index {
                f.path = None;
            }
            apply_edit(&mut f, Edit::FontSize, TEXT);
            remove_capacity(&mut f, form, false);
            let before = f.doc.get_page_render_tree(0).unwrap();
            apply_edit(&mut f, Edit::FontSize, TEXT);
            let after = f.doc.get_page_render_tree(0).unwrap();
            let text = |tree: &str| -> String {
                tree.split("\"text\":\"")
                    .skip(1)
                    .map(|s| s.split('"').next().unwrap())
                    .collect()
            };
            assert_eq!(text(&before), TEXT);
            assert_eq!(text(&after), TEXT);
        }
    }
}

#[test]
fn vertical_latin_rotated_and_upright_geometry_preserves_rendered_text() {
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    for direction in [1, 2] {
        for edit in [Edit::FontSize, Edit::LineSpacing, Edit::Input] {
            check_text(
                Form::Cell {
                    direction,
                    merged: true,
                    nested: false,
                    inherited: false,
                },
                edit,
                text,
            );
        }
    }
}

macro_rules! matrix {
    ($font:ident, $spacing:ident, $input:ident, $form:expr) => {
        #[test]
        fn $font() {
            check($form, Edit::FontSize);
        }
        #[test]
        fn $spacing() {
            check($form, Edit::LineSpacing);
        }
        #[test]
        fn $input() {
            check($form, Edit::Input);
        }
    };
}

matrix!(
    cell_font,
    cell_spacing,
    cell_input,
    Form::Cell {
        direction: 0,
        merged: false,
        nested: false,
        inherited: false
    }
);
matrix!(
    vertical_rotate_font,
    vertical_rotate_spacing,
    vertical_rotate_input,
    Form::Cell {
        direction: 1,
        merged: false,
        nested: false,
        inherited: false
    }
);
matrix!(
    vertical_upright_font,
    vertical_upright_spacing,
    vertical_upright_input,
    Form::Cell {
        direction: 2,
        merged: false,
        nested: false,
        inherited: false
    }
);
matrix!(
    inherited_vertical_font,
    inherited_vertical_spacing,
    inherited_vertical_input,
    Form::Cell {
        direction: 1,
        merged: false,
        nested: false,
        inherited: true
    }
);
matrix!(
    merged_font,
    merged_spacing,
    merged_input,
    Form::Cell {
        direction: 0,
        merged: true,
        nested: false,
        inherited: false
    }
);
matrix!(
    merged_vertical_font,
    merged_vertical_spacing,
    merged_vertical_input,
    Form::Cell {
        direction: 1,
        merged: true,
        nested: false,
        inherited: false
    }
);
matrix!(
    nested_font,
    nested_spacing,
    nested_input,
    Form::Cell {
        direction: 0,
        merged: false,
        nested: true,
        inherited: false
    }
);
matrix!(
    nested_vertical_font,
    nested_vertical_spacing,
    nested_vertical_input,
    Form::Cell {
        direction: 1,
        merged: false,
        nested: true,
        inherited: false
    }
);
matrix!(
    nested_upright_font,
    nested_upright_spacing,
    nested_upright_input,
    Form::Cell {
        direction: 2,
        merged: false,
        nested: true,
        inherited: false
    }
);
matrix!(
    textbox_font,
    textbox_spacing,
    textbox_input,
    Form::TextBox { rotation: 0 }
);
matrix!(
    rotated_textbox_font,
    rotated_textbox_spacing,
    rotated_textbox_input,
    Form::TextBox { rotation: 90 }
);
matrix!(
    caption_top_font,
    caption_top_spacing,
    caption_top_input,
    Form::TableCaption {
        direction: Top,
        merged: false,
        spacing: 0
    }
);
matrix!(
    caption_bottom_font,
    caption_bottom_spacing,
    caption_bottom_input,
    Form::TableCaption {
        direction: Bottom,
        merged: false,
        spacing: 0
    }
);
matrix!(
    caption_left_font,
    caption_left_spacing,
    caption_left_input,
    Form::TableCaption {
        direction: Left,
        merged: false,
        spacing: 0
    }
);
matrix!(
    caption_right_font,
    caption_right_spacing,
    caption_right_input,
    Form::TableCaption {
        direction: Right,
        merged: false,
        spacing: 0
    }
);
matrix!(
    caption_merged_font,
    caption_merged_spacing,
    caption_merged_input,
    Form::TableCaption {
        direction: Bottom,
        merged: true,
        spacing: 0
    }
);
matrix!(
    caption_spaced_font,
    caption_spaced_spacing,
    caption_spaced_input,
    Form::TableCaption {
        direction: Top,
        merged: false,
        spacing: 200
    }
);
matrix!(
    picture_top_font,
    picture_top_spacing,
    picture_top_input,
    Form::PictureCaption(Top)
);
matrix!(
    picture_bottom_font,
    picture_bottom_spacing,
    picture_bottom_input,
    Form::PictureCaption(Bottom)
);
matrix!(
    picture_left_font,
    picture_left_spacing,
    picture_left_input,
    Form::PictureCaption(Left)
);
matrix!(
    picture_right_font,
    picture_right_spacing,
    picture_right_input,
    Form::PictureCaption(Right)
);
