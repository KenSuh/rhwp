use super::geometry::{fixture, Fixture, Form, TEXT};
use rhwp::model::control::Control;

pub const LATIN: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

pub fn render_text(f: &Fixture) -> String {
    (0..f.doc.page_count())
        .map(|page| {
            let tree = f.doc.get_page_render_tree(page).unwrap();
            tree.split("\"text\":\"")
                .skip(1)
                .map(|part| part.split('"').next().unwrap())
                .collect::<String>()
        })
        .collect()
}

pub fn edit(f: &mut Fixture, action: &str, text: &str) {
    let end = f.paragraph().text.chars().count();
    match (action, &f.path) {
        ("input", Some(path)) => {
            f.doc
                .insert_text_in_cell_by_path_api(0, f.para_idx.try_into().unwrap(), path, 0, text)
                .unwrap();
        }
        ("input", None) => {
            f.doc
                .insert_text_in_cell(
                    0,
                    f.para_idx.try_into().unwrap(),
                    f.control_idx.try_into().unwrap(),
                    0,
                    0,
                    0,
                    text,
                )
                .unwrap();
        }
        ("spacing", Some(path)) => {
            f.doc
                .apply_para_format_in_cell_by_path(
                    0,
                    f.para_idx,
                    path,
                    "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
                )
                .unwrap();
        }
        ("spacing", None) => {
            f.doc
                .apply_para_format_in_cell(
                    0,
                    f.para_idx,
                    f.control_idx,
                    0,
                    0,
                    "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
                )
                .unwrap();
        }
        (font, path) => {
            let props = format!("{{\"fontSize\":{font}}}");
            if let Some(path) = path {
                f.doc
                    .apply_char_format_in_cell_by_path(0, f.para_idx, path, 0, end, &props)
                    .unwrap();
            } else {
                f.doc
                    .apply_char_format_in_cell(0, f.para_idx, f.control_idx, 0, 0, 0, end, &props)
                    .unwrap();
            }
        }
    }
}

pub fn set_auto(f: &mut Fixture, form: Form, both_axes: bool) {
    let mut model = f.doc.document().clone();
    let Control::Table(outer) =
        &mut model.sections[0].paragraphs[f.para_idx].controls[f.control_idx]
    else {
        panic!("table")
    };
    let target = if matches!(form, Form::Cell { nested: true, .. }) {
        let Control::Table(inner) = &mut outer.cells[0].paragraphs[0].controls[0] else {
            panic!("nested table")
        };
        inner
    } else {
        outer
    };
    target.common.height = 0;
    target.dirty = true;
    for cell in &mut target.cells {
        cell.height = 0;
        if both_axes {
            cell.width = 0;
        }
    }
    f.doc.set_document(model);
}

pub fn row(id: &str, f: &Fixture) -> String {
    let rendered = render_text(f);
    format!(
        "{id}|{}|{}|{rendered}",
        f.paragraph().text.chars().count(),
        rendered.chars().count()
    )
}

pub fn single(form: Form, action: &str, text: &str) -> String {
    let mut f = fixture(form, if action == "input" { None } else { Some(text) });
    edit(&mut f, action, text);
    row(
        &format!(
            "single:{form:?}:{action}:{}",
            if text == TEXT { "cjk" } else { "latin" }
        ),
        &f,
    )
}

pub fn unavailable(form: Form, by_index: bool, both: bool, action: &str) -> String {
    let mut f = fixture(form, Some(TEXT));
    if by_index {
        f.path = None;
    }
    edit(&mut f, "900", TEXT);
    set_auto(&mut f, form, both);
    edit(&mut f, action, TEXT);
    f.doc.reflow_linesegs();
    row(
        &format!("unavailable:{form:?}:{by_index}:{both}:{action}"),
        &f,
    )
}

pub fn census() -> String {
    let mut rows = Vec::new();
    for direction in [1, 2] {
        for merged in [false, true] {
            for nested in [false, true] {
                for inherited in [false, true] {
                    let form = Form::Cell {
                        direction,
                        merged,
                        nested,
                        inherited,
                    };
                    for text in [TEXT, LATIN] {
                        for action in ["900", "spacing", "input"] {
                            rows.push(single(form, action, text));
                        }
                    }
                    for by_index in [false, true] {
                        if by_index && nested {
                            continue;
                        }
                        rows.push(cross(form, by_index));
                        for both in [false, true] {
                            for action in ["900", "spacing", "input"] {
                                rows.push(unavailable(form, by_index, both, action));
                            }
                        }
                        for geometry in ["fixed", "auto", "both"] {
                            for text in [TEXT, LATIN] {
                                let mut f = fixture(form, Some(text));
                                if by_index {
                                    f.path = None;
                                }
                                edit(&mut f, "900", text);
                                if geometry != "fixed" {
                                    set_auto(&mut f, form, geometry == "both");
                                }
                                for stage in [
                                    "seed",
                                    "input",
                                    "1800",
                                    "spacing",
                                    "input_again",
                                    "document",
                                ] {
                                    match stage {
                                        "seed" => {}
                                        "document" => {
                                            f.doc.reflow_linesegs();
                                        }
                                        "input_again" => edit(&mut f, "input", text),
                                        action => edit(&mut f, action, text),
                                    }
                                    let id = format!(
                                        "sequence:{form:?}:{by_index}:{geometry}:{}:{stage}",
                                        if text == TEXT { "cjk" } else { "latin" }
                                    );
                                    rows.push(row(&id, &f));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    rows.join("\n") + "\n"
}

pub fn cross(form: Form, by_index: bool) -> String {
    let mut f = fixture(form, Some(TEXT));
    if by_index {
        f.path = None;
    }
    let mut model = f.doc.document().clone();
    let Control::Table(outer) =
        &mut model.sections[0].paragraphs[f.para_idx].controls[f.control_idx]
    else {
        panic!("table")
    };
    let target = if matches!(form, Form::Cell { nested: true, .. }) {
        let Control::Table(inner) = &mut outer.cells[0].paragraphs[0].controls[0] else {
            panic!("nested table")
        };
        inner
    } else {
        outer
    };
    for cell in &mut target.cells {
        cell.width = 0;
    }
    target.dirty = true;
    f.doc.set_document(model);
    edit(&mut f, "900", TEXT);
    row(&format!("cross:{form:?}:{by_index}"), &f)
}

pub fn assert_pre_l1(actual: &str) {
    let id = actual.split('|').next().unwrap();
    let prefix = format!("{id}|");
    let expected = include_str!("../fixtures/vertical-cell-dd1600342.txt")
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap();
    assert_eq!(actual, expected);
}
