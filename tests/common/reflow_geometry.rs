// 셀·중첩 표·글상자·캡션의 진행 방향 용량을 서로 다른 기하로 준비하는 네이티브 회귀 표본.
use rhwp::model::control::Control;
use rhwp::model::image::Picture;
use rhwp::model::paragraph::Paragraph;
use rhwp::model::shape::{Caption, CaptionDirection, RectangleShape, ShapeObject, TextBox};
use rhwp::model::table::Table;
use rhwp::model::Padding;
use rhwp::wasm_api::HwpDocument;

pub const TEXT: &str = "가나다라마바사아자차카타파하가나다라마바사아자차카타파하가나다라마바사아자차카타파하가나다라마바사아자차카타파하";

#[derive(Clone, Copy, Debug)]
pub enum Form {
    Cell {
        direction: u8,
        merged: bool,
        nested: bool,
        inherited: bool,
    },
    TextBox {
        rotation: i16,
    },
    TableCaption {
        direction: CaptionDirection,
        merged: bool,
        spacing: i16,
    },
    PictureCaption(CaptionDirection),
}

pub struct Fixture {
    pub doc: HwpDocument,
    pub para_idx: usize,
    pub control_idx: usize,
    pub cell_idx: usize,
    pub path: Option<String>,
    pub capacity: i32,
}

fn number(json: &str, key: &str) -> usize {
    let start = json.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
    json[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

fn caption(direction: CaptionDirection) -> Caption {
    Caption {
        direction,
        width: 7000,
        max_width: 10582,
        paragraphs: vec![Paragraph::new_empty()],
        ..Default::default()
    }
}

pub fn fixture(form: Form, seed: Option<&str>) -> Fixture {
    let mut doc = HwpDocument::create_empty();
    doc.create_blank_document().unwrap();
    let created = doc.create_table_ex(
        "{\"sectionIdx\":0,\"paraIdx\":0,\"charOffset\":0,\"rowCount\":2,\"colCount\":2,\"treatAsChar\":true,\"colWidths\":[5291,5291]}"
    ).unwrap();
    let para_idx = number(&created, "paraIdx");
    let control_idx = number(&created, "controlIdx");
    let mut model = doc.document().clone();
    let control = &mut model.sections[0].paragraphs[para_idx].controls[control_idx];
    let Control::Table(base) = control else {
        panic!("table fixture");
    };
    let mut path = None;
    let (cell_idx, capacity) = match form {
        Form::Cell {
            direction,
            merged,
            nested,
            inherited,
        } => {
            let mut target = base.as_ref().clone();
            if merged {
                target.merge_cells(0, 0, 1, 1).unwrap();
            }
            if nested {
                for cell in &mut target.cells {
                    cell.width = if direction == 0 { 3000 } else { 5291 };
                }
            }
            target.padding = Padding {
                left: 137,
                right: 311,
                top: 1200,
                bottom: 2300,
            };
            target.dirty = true;
            for cell in &mut target.cells {
                cell.height = 32000;
                cell.text_direction = direction;
                cell.padding = if inherited {
                    Padding::default()
                } else {
                    target.padding
                };
            }
            let capacity = if direction == 0 {
                if nested {
                    2552
                } else if merged {
                    10134
                } else {
                    4843
                }
            } else {
                28500
            };
            let outer =
                format!("{{\"controlIndex\":{control_idx},\"cellIndex\":0,\"cellParaIndex\":0}}");
            if nested {
                base.dirty = true;
                base.cells[0].width = 15000;
                base.cells[0].height = 40000;
                base.cells[0].paragraphs[0]
                    .controls
                    .push(Control::Table(Box::new(target)));
                path = Some(format!(
                    "[{outer},{{\"controlIndex\":0,\"cellIndex\":0,\"cellParaIndex\":0}}]"
                ));
            } else {
                **base = target;
                path = Some(format!("[{outer}]"));
            }
            (0, capacity)
        }
        Form::TextBox { rotation } => {
            let mut shape = RectangleShape::default();
            shape.common.width = 10000;
            shape.common.height = 32000;
            shape.common.treat_as_char = true;
            shape.drawing.shape_attr.rotation_angle = rotation;
            shape.drawing.text_box = Some(TextBox {
                margin_left: 137,
                margin_right: 311,
                paragraphs: vec![Paragraph::new_empty()],
                ..Default::default()
            });
            *control = Control::Shape(Box::new(ShapeObject::Rectangle(shape)));
            (0, 9552)
        }
        Form::TableCaption {
            direction,
            merged,
            spacing,
        } => {
            if merged {
                base.merge_cells(0, 0, 1, 1).unwrap();
            }
            base.cell_spacing = spacing;
            base.caption = Some(caption(direction));
            let capacity = match direction {
                CaptionDirection::Left | CaptionDirection::Right => 7000,
                CaptionDirection::Top | CaptionDirection::Bottom => 10582 + i32::from(spacing),
            };
            (65534, capacity)
        }
        Form::PictureCaption(direction) => {
            let mut pic = Picture::default();
            pic.common.width = 18000;
            pic.common.height = 20000;
            pic.common.treat_as_char = true;
            pic.caption = Some(caption(direction));
            *control = Control::Picture(Box::new(pic));
            let capacity = match direction {
                CaptionDirection::Left | CaptionDirection::Right => 7000,
                CaptionDirection::Top | CaptionDirection::Bottom => 18000,
            };
            (65534, capacity)
        }
    };
    let mut result = Fixture {
        doc,
        para_idx,
        control_idx,
        cell_idx,
        path,
        capacity,
    };
    if let Some(text) = seed {
        paragraph_mut(
            &mut model.sections[0].paragraphs[para_idx].controls[control_idx],
            form,
        )
        .insert_text_at(0, text);
    }
    result.doc.set_document(model);
    result
}

fn paragraph_mut(control: &mut Control, form: Form) -> &mut Paragraph {
    match control {
        Control::Table(table) => match form {
            Form::TableCaption { .. } => &mut table.caption.as_mut().unwrap().paragraphs[0],
            Form::Cell { nested: true, .. } => {
                let Control::Table(nested) = &mut table.cells[0].paragraphs[0].controls[0] else {
                    panic!("nested table");
                };
                &mut nested.cells[0].paragraphs[0]
            }
            _ => &mut table.cells[0].paragraphs[0],
        },
        Control::Shape(shape) => {
            &mut shape
                .drawing_mut()
                .unwrap()
                .text_box
                .as_mut()
                .unwrap()
                .paragraphs[0]
        }
        Control::Picture(pic) => &mut pic.caption.as_mut().unwrap().paragraphs[0],
        _ => panic!("fixture paragraph"),
    }
}

impl Fixture {
    pub fn paragraph(&self) -> &Paragraph {
        let mut control =
            &self.doc.document().sections[0].paragraphs[self.para_idx].controls[self.control_idx];
        if self.path.as_ref().is_some_and(|p| p.contains("},{")) {
            let Control::Table(table) = control else {
                panic!("outer table");
            };
            control = &table.cells[0].paragraphs[0].controls[0];
        }
        match control {
            Control::Table(table) if self.cell_idx == 65534 => {
                &table.caption.as_ref().unwrap().paragraphs[0]
            }
            Control::Table(table) => &table.cells[0].paragraphs[0],
            Control::Shape(shape) => {
                &shape
                    .drawing()
                    .unwrap()
                    .text_box
                    .as_ref()
                    .unwrap()
                    .paragraphs[0]
            }
            Control::Picture(pic) => &pic.caption.as_ref().unwrap().paragraphs[0],
            _ => panic!("fixture paragraph"),
        }
    }
}
