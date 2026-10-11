use rhwp::model::control::Control;
use rhwp::model::paragraph::Paragraph;
use rhwp::model::shape::{Caption, RectangleShape, ShapeObject, TextBox};
use rhwp::model::table::Table;
use rhwp::model::Padding;
use rhwp::wasm_api::HwpDocument;

pub(super) const TEXT: &str = "가나다라마바사아자차카타파하가나다라마바사아자차카타파하가나다라마바사아자차카타파하가나다라마바사아자차카타파하";

#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub(super) width: &'static str,
    pub(super) padding: i16,
    pub(super) margin: i32,
    pub(super) depth: usize,
    pub(super) directions: usize,
    pub(super) direction: u8,
    pub(super) leaf_width: u32,
    pub(super) api: &'static str,
    pub(super) container: &'static str,
}

pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for width in ["positive", "zero", "auto"] {
        for padding in [0, 7500] {
            for margin in [0, 6000] {
                for depth in 1..=3 {
                    for directions in 0..(1 << depth) {
                        for direction in [1, 2] {
                            for leaf_width in [0, 20000] {
                                for api in [
                                    "target-path",
                                    "ancestor-index",
                                    "caption-index",
                                    "textbox-index",
                                ] {
                                    for container in ["table", "textbox", "caption"] {
                                        cases.push(Case {
                                            width,
                                            padding,
                                            margin,
                                            depth,
                                            directions,
                                            direction,
                                            leaf_width,
                                            api,
                                            container,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    cases
}

pub(super) fn nested(table: &Table, depth: usize) -> &Table {
    if depth == 0 {
        return table;
    }
    let Control::Table(child) = &table.cells[0].paragraphs[0].controls[0] else {
        panic!("child table")
    };
    nested(child, depth - 1)
}

pub(super) fn root(doc: &HwpDocument, para: usize, control: usize) -> &Table {
    match &doc.document().sections[0].paragraphs[para].controls[control] {
        Control::Table(table) => {
            if let Some(caption) = &table.caption {
                if let Some(Control::Table(child)) = caption.paragraphs[0].controls.first() {
                    return child;
                }
            }
            table
        }
        Control::Shape(shape) => {
            let Control::Table(table) = &shape
                .drawing()
                .unwrap()
                .text_box
                .as_ref()
                .unwrap()
                .paragraphs[0]
                .controls[0]
            else {
                panic!("textbox table")
            };
            table
        }
        _ => panic!("root container"),
    }
}

pub(super) fn prepared(case: Case) -> (HwpDocument, usize, usize) {
    let mut doc = HwpDocument::create_empty();
    doc.create_blank_document().unwrap();
    doc.create_table_ex("{\"sectionIdx\":0,\"paraIdx\":0,\"charOffset\":0,\"rowCount\":1,\"colCount\":1,\"treatAsChar\":true,\"colWidths\":[20000]}").unwrap();
    let mut model = doc.document().clone();
    let para_idx = model.sections[0]
        .paragraphs
        .iter()
        .position(|p| p.controls.iter().any(|c| matches!(c, Control::Table(_))))
        .unwrap();
    let control_idx = model.sections[0].paragraphs[para_idx]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    let mut ancestor_style = model.doc_info.para_shapes[0].clone();
    ancestor_style.margin_left = case.margin;
    ancestor_style.margin_right = case.margin;
    let style_id = model.doc_info.para_shapes.len().try_into().unwrap();
    model.doc_info.para_shapes.push(ancestor_style);
    let Control::Table(table) = &mut model.sections[0].paragraphs[para_idx].controls[control_idx]
    else {
        panic!("root table")
    };
    table.row_count = 1;
    table.col_count = 1;
    table.cells.truncate(1);
    table.cells[0].width = case.leaf_width;
    table.cells[0].text_direction = case.direction;
    table.cells[0].height = 0;
    table.cells[0].padding = Padding::default();
    table.padding = Padding::default();
    table.common.height = 0;
    table.cells[0].paragraphs[0].insert_text_at(0, TEXT);
    table.cells[0].paragraphs[0].line_segs[0].line_height = 0;
    let leaf = table.as_ref().clone();
    let mut chain = leaf.clone();
    for level in (0..case.depth).rev() {
        let mut parent = leaf.clone();
        parent.cells[0].width = if case.width == "positive" { 45000 } else { 0 };
        parent.cells[0].height = if case.width == "auto" { 0 } else { 60000 };
        parent.cells[0].text_direction = if case.directions & (1 << level) == 0 {
            0
        } else {
            case.direction
        };
        parent.cells[0].padding = Padding::default();
        parent.padding.left = case.padding;
        parent.padding.right = case.padding;
        let mut p = Paragraph::new_empty();
        p.char_count = 9;
        p.line_segs[0].line_height = 0;
        p.para_shape_id = style_id;
        p.controls.push(Control::Table(Box::new(chain)));
        parent.cells[0].paragraphs = vec![p];
        chain = parent;
    }
    chain.caption = Some(Caption {
        width: 10000,
        max_width: 45000,
        paragraphs: vec![Paragraph::new_empty()],
        ..Default::default()
    });
    **table = chain;
    let mut shape = RectangleShape::default();
    shape.common.width = 45000;
    shape.common.height = 60000;
    shape.common.treat_as_char = true;
    shape.drawing.text_box = Some(TextBox {
        paragraphs: vec![Paragraph::new_empty()],
        ..Default::default()
    });
    model.sections[0].paragraphs[para_idx]
        .controls
        .push(Control::Shape(Box::new(ShapeObject::Rectangle(shape))));
    model.sections[0].paragraphs[para_idx].char_count += 8;
    let host = &mut model.sections[0].paragraphs[para_idx].controls[control_idx];
    if case.container != "table" {
        let Control::Table(chain) = host else {
            panic!("chain")
        };
        let mut p = Paragraph::new_empty();
        p.char_count = 9;
        p.controls.push(Control::Table(chain.clone()));
        match case.container {
            "textbox" => {
                let mut shape = RectangleShape::default();
                shape.common.width = 45000;
                shape.common.height = 60000;
                shape.common.treat_as_char = true;
                shape.drawing.text_box = Some(TextBox {
                    paragraphs: vec![p],
                    ..Default::default()
                });
                *host = Control::Shape(Box::new(ShapeObject::Rectangle(shape)));
            }
            "caption" => {
                let mut outer = chain.as_ref().clone();
                outer.cells[0].paragraphs = vec![Paragraph::new_empty()];
                outer.cells[0].text_direction = 0;
                outer.cells[0].width = 45000;
                outer.caption.as_mut().unwrap().paragraphs = vec![p];
                **chain = outer;
            }
            _ => panic!("container"),
        }
    }
    if case.container == "textbox" {
        let mut caption_host = leaf;
        caption_host.cells[0].paragraphs = vec![Paragraph::new_empty()];
        caption_host.cells[0].text_direction = 0;
        caption_host.cells[0].width = 45000;
        caption_host.caption = Some(Caption {
            width: 10000,
            max_width: 45000,
            paragraphs: vec![Paragraph::new_empty()],
            ..Default::default()
        });
        model.sections[0].paragraphs[para_idx]
            .controls
            .push(Control::Table(Box::new(caption_host)));
        model.sections[0].paragraphs[para_idx].char_count += 8;
    }
    let page = &mut model.sections[0].section_def.page_def;
    page.width = 45000 + page.margin_left + page.margin_right;
    doc.set_document(model);
    (doc, para_idx, control_idx)
}
