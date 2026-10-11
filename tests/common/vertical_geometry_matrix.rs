#[path = "vertical_geometry_fixture.rs"]
mod fixture;
pub use fixture::{cases, Case};
use fixture::{nested, prepared, root, TEXT};
use rhwp::model::control::Control;
use rhwp::model::paragraph::Paragraph;
use rhwp::renderer::composer::compose_paragraph;
use rhwp::renderer::height_measurer::HeightMeasurer;
use rhwp::renderer::style_resolver::resolve_styles;
use rhwp::wasm_api::HwpDocument;

fn edit(doc: &mut HwpDocument, case: Case, location: (usize, usize, &str)) -> String {
    let (para, control, action) = location;
    if action == "document" {
        doc.reflow_linesegs();
        return "ok".to_owned();
    }
    let (ctrl, cell) = match case.api {
        "target-path" | "ancestor-index" => (control, 0),
        "caption-index" => (
            if case.container == "textbox" {
                doc.document().sections[0].paragraphs[para]
                    .controls
                    .iter()
                    .position(|c| matches!(c, Control::Table(_)))
                    .unwrap()
            } else {
                control
            },
            65534,
        ),
        "textbox-index" => (
            doc.document().sections[0].paragraphs[para]
                .controls
                .iter()
                .rposition(|c| matches!(c, Control::Shape(_)))
                .unwrap(),
            0,
        ),
        _ => panic!("matrix API"),
    };
    let path_api = case.api == "target-path";
    let mut native_path = vec![(
        control,
        if case.container == "caption" {
            65534
        } else {
            0
        },
        0,
    )];
    if case.container == "textbox" {
        native_path.push((0, 0, 0));
    }
    native_path.extend(std::iter::repeat_n((0, 0, 0), case.depth));
    let end = if path_api {
        nested(root(doc, para, control), case.depth).cells[0].paragraphs[0]
            .text
            .chars()
            .count()
    } else {
        0
    };
    match action {
        "input" | "growth" => {
            let seed = if path_api { TEXT } else { "ANCESTOR" };
            let text = if action == "growth" {
                seed.repeat(8)
            } else {
                seed.to_owned()
            };
            if path_api {
                let result = doc.insert_text_in_cell_by_path(0, para, &native_path, 0, &text);
                if case.container != "table" {
                    return format!("unsupported:{:?}", result.unwrap_err());
                }
                result.unwrap();
            } else {
                doc.insert_text_in_cell(
                    0,
                    para.try_into().unwrap(),
                    ctrl.try_into().unwrap(),
                    cell,
                    0,
                    0,
                    &text,
                )
                .unwrap();
            }
        }
        "900" | "600" => {
            for size in ["1800", action] {
                let props = format!("{{\"fontSize\":{size}}}");
                if path_api {
                    let result = doc.apply_char_format_in_cell_by_path_native(
                        0,
                        para,
                        &native_path,
                        0,
                        end,
                        &props,
                    );
                    if case.container != "table" {
                        return format!("unsupported:{:?}", result.unwrap_err());
                    }
                    result.unwrap();
                } else {
                    doc.apply_char_format_in_cell(
                        0,
                        para,
                        ctrl,
                        usize::try_from(cell).unwrap(),
                        0,
                        0,
                        end,
                        &props,
                    )
                    .unwrap();
                }
            }
        }
        _ => panic!("matrix action"),
    }
    "ok".to_owned()
}

pub fn snapshot(case: Case, action: &str) -> String {
    let (mut doc, para, control) = prepared(case);
    let outcome = edit(&mut doc, case, (para, control, action));
    let leaf = nested(root(&doc, para, control), case.depth);
    let p = &leaf.cells[0].paragraphs[0];
    let mut measurement_para = Paragraph::new_empty();
    measurement_para
        .controls
        .push(Control::Table(Box::new(leaf.clone())));
    let composed = compose_paragraph(&measurement_para);
    let styles = resolve_styles(&doc.document().doc_info, doc.get_dpi());
    let measured = HeightMeasurer::new(doc.get_dpi()).measure_section(
        &[measurement_para],
        &[composed],
        &styles,
    );
    let content_height = measured.tables[0].cells[0].total_content_height.to_bits();
    let row_heights: Vec<_> = measured.tables[0]
        .row_heights
        .iter()
        .map(|height| height.to_bits())
        .collect();
    // Renderer vertical content height is max segment_width (400 HU when unavailable).
    let max_segment = p
        .line_segs
        .iter()
        .map(|s| s.segment_width)
        .max()
        .unwrap_or(0);
    let vertical_height = if max_segment > 0 { max_segment } else { 400 };
    let rendered = (0..doc.page_count())
        .map(|page| doc.get_page_render_tree(page).unwrap())
        .flat_map(|tree| {
            tree.split("\"text\":\"")
                .skip(1)
                .map(|part| part.split('"').next().unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<String>()
        .chars()
        .filter(|ch| !ch.is_ascii())
        .collect::<String>();
    format!(
        "{case:?}:{action}|{outcome}|{:?}|{content_height:016x}|{vertical_height}|{row_heights:?}|{rendered}",
        p.line_segs
    )
}
