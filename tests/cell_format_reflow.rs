// 셀 서식 적용(색인·cellPath 의 글자/문단) 뒤 셀 문단 줄이 셀 폭에서 나뉘는지 본다 — 편집기 WASM 과 생성기가 함께 쓰는 셀 편집 경로의 회귀 시험.

use rhwp::model::control::Control;
use rhwp::model::table::Table;
use rhwp::wasm_api::HwpDocument;

/// 본문 170mm 를 9열로 균등 분할한 생성기 표의 열 폭(HWPUNIT).
const COL_WIDTH: u32 = 5291;
const LONG_TEXT: &str = "수작업 검사와 육안 판정에 의존하고 있어 편차가 큼";

struct Fixture {
    doc: HwpDocument,
    para_idx: usize,
    control_idx: usize,
}

fn json_number(json: &str, key: &str) -> usize {
    let marker = format!("\"{key}\":");
    let start = json
        .find(&marker)
        .unwrap_or_else(|| panic!("{key} missing in {json}"))
        + marker.len();
    json[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("{key} is not a number in {json}"))
}

fn table_fixture() -> Fixture {
    table_fixture_with_cols(9)
}

fn table_fixture_with_cols(cols: usize) -> Fixture {
    let mut doc = HwpDocument::create_empty();
    doc.create_blank_document().unwrap();
    let widths = vec![COL_WIDTH.to_string(); cols].join(",");
    let created = doc
        .create_table_ex(&format!(
            "{{\"sectionIdx\":0,\"paraIdx\":0,\"charOffset\":0,\"rowCount\":2,\"colCount\":{cols},\
             \"treatAsChar\":true,\"colWidths\":[{widths}]}}"
        ))
        .unwrap();
    Fixture {
        para_idx: json_number(&created, "paraIdx"),
        control_idx: json_number(&created, "controlIdx"),
        doc,
    }
}

fn table<'a>(doc: &'a HwpDocument, fixture: &Fixture) -> &'a Table {
    match &doc.document().sections[0].paragraphs[fixture.para_idx].controls[fixture.control_idx] {
        Control::Table(table) => table,
        _ => panic!("table control missing"),
    }
}

/// 셀 폭 − 좌우 안 여백(셀 값이 0 이면 표 값) — rhwp 셀 리플로우·레이아웃과 같은 규칙.
fn inner_width(table: &Table, cell_idx: usize) -> i32 {
    let cell = &table.cells[cell_idx];
    let pad_left = if cell.padding.left != 0 {
        cell.padding.left
    } else {
        table.padding.left
    };
    let pad_right = if cell.padding.right != 0 {
        cell.padding.right
    } else {
        table.padding.right
    };
    cell.width as i32 - i32::from(pad_left) - i32::from(pad_right)
}

fn insert_long_text(fixture: &mut Fixture, cell_idx: usize) -> usize {
    fixture
        .doc
        .insert_text_in_cell(
            0,
            fixture.para_idx as u32,
            fixture.control_idx as u32,
            cell_idx as u32,
            0,
            0,
            LONG_TEXT,
        )
        .unwrap();
    LONG_TEXT.chars().count()
}

fn cell_path(fixture: &Fixture, cell_idx: usize) -> String {
    format!(
        "[{{\"controlIndex\":{},\"cellIndex\":{cell_idx},\"cellParaIndex\":0}}]",
        fixture.control_idx
    )
}

/// 셀 문단의 모든 줄이 셀 안쪽 폭 안에 있고 긴 글이 여러 줄로 나뉘었는지 확인하고 줄 수를 돌려준다.
fn assert_cell_lines_fit(
    doc: &HwpDocument,
    fixture: &Fixture,
    cell_idx: usize,
    context: &str,
) -> usize {
    let table = table(doc, fixture);
    let inner = inner_width(table, cell_idx);
    let segs = &table.cells[cell_idx].paragraphs[0].line_segs;
    for seg in segs {
        assert!(
            seg.segment_width > 0 && seg.segment_width <= inner,
            "{context}: segment width {} exceeds cell inner width {inner}",
            seg.segment_width
        );
    }
    assert!(
        segs.len() > 1,
        "{context}: long text must wrap inside the cell, got {} line(s)",
        segs.len()
    );
    segs.len()
}

/// `start` 의 `{` 와 짝이 맞는 `}` 의 바이트 위치(문자열 안 괄호는 무시).
fn node_end(json: &str, start: usize) -> usize {
    let bytes = json.as_bytes();
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for (offset, &byte) in bytes[start..].iter().enumerate() {
        if in_string {
            match byte {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return start + offset;
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced render tree json");
}

fn text_values(node: &str) -> String {
    let mut out = String::new();
    let mut rest = node;
    while let Some(position) = rest.find("\"text\":\"") {
        let mut chars = rest[position + 8..].chars();
        while let Some(ch) = chars.next() {
            match ch {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some(other) => out.push(other),
                    None => break,
                },
                other => out.push(other),
            }
        }
        rest = chars.as_str();
    }
    out
}

/// 렌더 트리(편집기 캔버스와 같은 배치)에서 (row, col) 셀의 줄별 글자를 모은다.
fn rendered_cell_lines(doc: &HwpDocument, row: u16, col: u16) -> Vec<String> {
    let json = doc.get_page_render_tree(0).unwrap();
    let address = format!(",\"row\":{row},\"col\":{col}");
    let mut search = 0;
    while let Some(found) = json[search..].find("{\"type\":\"Cell\"") {
        let start = search + found;
        let end = node_end(&json, start);
        let node = &json[start..=end];
        let header_end = node.find(",\"children\"").unwrap_or(node.len());
        if node[..header_end].contains(&address) {
            let mut lines = Vec::new();
            let mut cursor = 0;
            while let Some(line_found) = node[cursor..].find("{\"type\":\"TextLine\"") {
                let line_start = cursor + line_found;
                let line_end = node_end(node, line_start);
                lines.push(text_values(&node[line_start..=line_end]));
                cursor = line_end + 1;
            }
            return lines;
        }
        search = end + 1;
    }
    panic!("rendered cell ({row},{col}) missing");
}

fn assert_rendered_wrap(doc: &HwpDocument, row: u16, col: u16, context: &str) {
    let lines = rendered_cell_lines(doc, row, col);
    assert!(
        lines.len() > 1,
        "{context}: rendered cell must show more than one line, got {lines:?}"
    );
    assert_eq!(
        lines.concat().replace(' ', ""),
        LONG_TEXT.replace(' ', ""),
        "{context}: rendered cell text"
    );
}

#[test]
fn char_format_in_cell_wraps_at_cell_width() {
    let mut fixture = table_fixture();
    let end = insert_long_text(&mut fixture, 0);
    fixture
        .doc
        .apply_char_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            0,
            0,
            0,
            end,
            "{\"fontSize\":900}",
        )
        .unwrap();
    assert_cell_lines_fit(&fixture.doc, &fixture, 0, "char format in cell");
    assert_rendered_wrap(&fixture.doc, 0, 0, "char format in cell");
}

#[test]
fn para_format_in_cell_wraps_at_cell_width() {
    let mut fixture = table_fixture();
    insert_long_text(&mut fixture, 0);
    fixture
        .doc
        .apply_para_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            0,
            0,
            "{\"lineSpacing\":110,\"lineSpacingType\":\"Percent\"}",
        )
        .unwrap();
    assert_cell_lines_fit(&fixture.doc, &fixture, 0, "para format in cell");
    assert_rendered_wrap(&fixture.doc, 0, 0, "para format in cell");
}

#[test]
fn composer_cell_sequence_survives_hwpx_roundtrip() {
    let mut fixture = table_fixture();
    let end = insert_long_text(&mut fixture, 1);
    fixture
        .doc
        .apply_char_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            1,
            0,
            0,
            end,
            "{\"fontSize\":900,\"bold\":false}",
        )
        .unwrap();
    fixture
        .doc
        .apply_para_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            1,
            0,
            "{\"alignment\":\"left\",\"lineSpacing\":110,\"lineSpacingType\":\"Percent\",\"spacingAfter\":0}",
        )
        .unwrap();
    let lines = assert_cell_lines_fit(&fixture.doc, &fixture, 1, "composer sequence");

    let bytes = fixture.doc.export_hwpx().unwrap();
    let doc = HwpDocument::from_bytes(&bytes).unwrap();
    // 다시 읽으면 구역·단 정의 컨트롤 표현이 달라 표의 컨트롤 순번이 바뀔 수 있다 — 종류로 찾는다.
    let tables = doc.document().sections[0].paragraphs[fixture.para_idx]
        .controls
        .iter()
        .enumerate()
        .filter(|(_, control)| matches!(control, Control::Table(_)))
        .map(|(control_idx, _)| control_idx)
        .collect::<Vec<_>>();
    assert_eq!(tables.len(), 1, "reloaded paragraph must hold one table");
    let reloaded = Fixture {
        doc,
        para_idx: fixture.para_idx,
        control_idx: tables[0],
    };
    assert_eq!(
        assert_cell_lines_fit(&reloaded.doc, &reloaded, 1, "reloaded hwpx"),
        lines
    );
    assert_rendered_wrap(&reloaded.doc, 0, 1, "reloaded hwpx");
}

#[test]
fn cell_path_formats_wrap_at_cell_width() {
    let mut fixture = table_fixture();
    let end = insert_long_text(&mut fixture, 2);
    let path = cell_path(&fixture, 2);
    fixture
        .doc
        .apply_char_format_in_cell_by_path(0, fixture.para_idx, &path, 0, end, "{\"fontSize\":900}")
        .unwrap();
    assert_cell_lines_fit(&fixture.doc, &fixture, 2, "char format by cell path");
    fixture
        .doc
        .apply_para_format_in_cell_by_path(
            0,
            fixture.para_idx,
            &path,
            "{\"lineSpacing\":110,\"lineSpacingType\":\"Percent\"}",
        )
        .unwrap();
    assert_cell_lines_fit(&fixture.doc, &fixture, 2, "para format by cell path");
    assert_rendered_wrap(&fixture.doc, 0, 2, "formats by cell path");
}

#[test]
fn font_size_change_in_cell_sets_line_height_from_new_size() {
    let mut fixture = table_fixture();
    let end = insert_long_text(&mut fixture, 3);
    fixture
        .doc
        .apply_char_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            3,
            0,
            0,
            end,
            "{\"fontSize\":2000}",
        )
        .unwrap();
    assert_cell_lines_fit(&fixture.doc, &fixture, 3, "20pt char format in cell");
    let table = table(&fixture.doc, &fixture);
    for seg in &table.cells[3].paragraphs[0].line_segs {
        assert!(
            seg.line_height >= 1900,
            "20pt line height must follow the new size, got {}",
            seg.line_height
        );
    }
}

#[test]
fn cell_path_font_size_change_sets_line_height_from_new_size() {
    let mut fixture = table_fixture();
    let end = insert_long_text(&mut fixture, 5);
    let path = cell_path(&fixture, 5);
    fixture
        .doc
        .apply_char_format_in_cell_by_path(
            0,
            fixture.para_idx,
            &path,
            0,
            end,
            "{\"fontSize\":2000}",
        )
        .unwrap();
    assert_cell_lines_fit(&fixture.doc, &fixture, 5, "20pt char format by cell path");
    let table = table(&fixture.doc, &fixture);
    for seg in &table.cells[5].paragraphs[0].line_segs {
        assert!(
            seg.line_height >= 1900,
            "20pt line height by cell path must follow the new size, got {}",
            seg.line_height
        );
    }
}

/// 300% 줄 간격이면 줄마다 line_spacing = 2 × line_height 여야 한다(Percent 규칙).
fn assert_triple_line_spacing(
    doc: &HwpDocument,
    fixture: &Fixture,
    cell_idx: usize,
    context: &str,
) {
    let table = table(doc, fixture);
    for seg in &table.cells[cell_idx].paragraphs[0].line_segs {
        assert_eq!(
            seg.line_spacing,
            seg.line_height * 2,
            "{context}: 300% line spacing must follow the new paragraph shape"
        );
    }
}

#[test]
fn para_format_in_cell_applies_new_line_spacing() {
    let mut fixture = table_fixture();
    insert_long_text(&mut fixture, 6);
    fixture
        .doc
        .apply_para_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            6,
            0,
            "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
        )
        .unwrap();
    assert_triple_line_spacing(&fixture.doc, &fixture, 6, "para format in cell");
}

#[test]
fn cell_path_para_format_applies_new_line_spacing() {
    let mut fixture = table_fixture();
    insert_long_text(&mut fixture, 7);
    let path = cell_path(&fixture, 7);
    fixture
        .doc
        .apply_para_format_in_cell_by_path(
            0,
            fixture.para_idx,
            &path,
            "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
        )
        .unwrap();
    assert_triple_line_spacing(&fixture.doc, &fixture, 7, "para format by cell path");
}

/// 표 캡션(셀 순번 65534) 문단의 줄이 캡션 폭(표 폭) 안에서 여러 줄로 나뉘었는지 확인한다.
fn assert_caption_lines_fit(doc: &HwpDocument, fixture: &Fixture, context: &str) {
    let caption = table(doc, fixture).caption.as_ref().expect("table caption");
    let segs = &caption.paragraphs[0].line_segs;
    assert!(
        segs.len() > 1,
        "{context}: caption must wrap, got {} line(s)",
        segs.len()
    );
    for seg in segs {
        assert!(
            seg.segment_width > 0 && seg.segment_width <= caption.max_width as i32,
            "{context}: segment width {} exceeds caption width {}",
            seg.segment_width,
            caption.max_width
        );
    }
}

#[test]
fn table_caption_formats_reflow_at_caption_width() {
    let mut fixture = table_fixture_with_cols(2);
    fixture
        .doc
        .set_table_properties(
            0,
            fixture.para_idx as u32,
            fixture.control_idx as u32,
            "{\"hasCaption\":true}",
        )
        .unwrap();
    let prefix = table(&fixture.doc, &fixture)
        .caption
        .as_ref()
        .unwrap()
        .paragraphs[0]
        .text
        .chars()
        .count();
    fixture
        .doc
        .insert_text_in_cell(
            0,
            fixture.para_idx as u32,
            fixture.control_idx as u32,
            65534,
            0,
            prefix as u32,
            LONG_TEXT,
        )
        .unwrap();
    let end = prefix + LONG_TEXT.chars().count();
    fixture
        .doc
        .apply_char_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            65534,
            0,
            0,
            end,
            "{\"fontSize\":900}",
        )
        .unwrap();
    assert_caption_lines_fit(&fixture.doc, &fixture, "char format in table caption");
    fixture
        .doc
        .apply_para_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            65534,
            0,
            "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
        )
        .unwrap();
    assert_caption_lines_fit(&fixture.doc, &fixture, "para format in table caption");
    let caption = table(&fixture.doc, &fixture).caption.as_ref().unwrap();
    for seg in &caption.paragraphs[0].line_segs {
        assert_eq!(
            seg.line_spacing,
            seg.line_height * 2,
            "table caption: 300% line spacing must follow the new paragraph shape"
        );
    }
}

#[test]
fn textbox_char_format_reflows_at_textbox_width() {
    let mut doc = HwpDocument::create_empty();
    doc.create_blank_document().unwrap();
    let created = doc
        .create_shape_control(
            "{\"sectionIdx\":0,\"paraIdx\":0,\"charOffset\":0,\"width\":10000,\"height\":20000,\
             \"shapeType\":\"textbox\"}",
        )
        .unwrap();
    let para_idx = json_number(&created, "paraIdx");
    let control_idx = json_number(&created, "controlIdx");
    doc.insert_text_in_cell(0, para_idx as u32, control_idx as u32, 0, 0, 0, LONG_TEXT)
        .unwrap();
    doc.apply_char_format_in_cell(
        0,
        para_idx,
        control_idx,
        0,
        0,
        0,
        LONG_TEXT.chars().count(),
        "{\"fontSize\":900}",
    )
    .unwrap();
    let Control::Shape(shape) =
        &doc.document().sections[0].paragraphs[para_idx].controls[control_idx]
    else {
        panic!("textbox control missing");
    };
    let text_box = shape
        .drawing()
        .and_then(|drawing| drawing.text_box.as_ref())
        .expect("text box");
    let inner = shape.common().width as i32
        - i32::from(text_box.margin_left)
        - i32::from(text_box.margin_right);
    let segs = &text_box.paragraphs[0].line_segs;
    assert!(
        segs.len() > 1,
        "textbox text must wrap, got {} line(s)",
        segs.len()
    );
    for seg in segs {
        assert!(
            seg.segment_width > 0 && seg.segment_width <= inner,
            "textbox: segment width {} exceeds textbox inner width {inner}",
            seg.segment_width
        );
    }
}

#[test]
fn text_insert_in_cell_still_reflows_at_cell_width() {
    let mut fixture = table_fixture();
    insert_long_text(&mut fixture, 4);
    let lines = assert_cell_lines_fit(&fixture.doc, &fixture, 4, "insert text in cell");
    let table = table(&fixture.doc, &fixture);
    let inner = inner_width(table, 4);
    for seg in &table.cells[4].paragraphs[0].line_segs {
        assert!(
            (inner - 1..=inner).contains(&seg.segment_width),
            "insert text in cell: segment width {} vs cell inner width {inner}",
            seg.segment_width
        );
    }
    fixture
        .doc
        .delete_text_in_cell(
            0,
            fixture.para_idx as u32,
            fixture.control_idx as u32,
            4,
            0,
            0,
            3,
        )
        .unwrap();
    assert!(assert_cell_lines_fit(&fixture.doc, &fixture, 4, "delete text in cell") <= lines);
}

fn vertical_fixture(direction: u8) -> Fixture {
    let mut fixture = table_fixture_with_cols(2);
    fixture
        .doc
        .set_cell_properties(
            0,
            fixture.para_idx.try_into().unwrap(),
            fixture.control_idx.try_into().unwrap(),
            0,
            &format!(
                "{{\"textDirection\":{direction},\"height\":32000,\"paddingTop\":1200,\"paddingBottom\":2300}}"
            ),
        )
        .unwrap();
    fixture
}

fn assert_vertical_capacity(fixture: &Fixture, capacity: i32, context: &str) {
    let segs = &table(&fixture.doc, fixture).cells[0].paragraphs[0].line_segs;
    if context == "vertical input" {
        assert!(segs.len() > 1, "pre-L1 input uses the cell width");
    } else {
        assert_eq!(
            segs.len(),
            1,
            "pre-L1 indexed formatting uses the section width"
        );
    }
    assert!(
        (capacity - 1..=capacity).contains(&segs[0].segment_width),
        "{context}: pre-L1 capacity is {capacity}, got {}",
        segs[0].segment_width
    );
    let tree = fixture.doc.get_page_render_tree(0).unwrap();
    assert_eq!(
        text_values(&tree).replace(' ', ""),
        LONG_TEXT.replace(' ', ""),
        "{context}: native render must preserve all vertical text"
    );
}

#[test]
fn vertical_cell_font_size_keeps_pre_l1_section_width() {
    for direction in [1, 2] {
        let mut fixture = vertical_fixture(direction);
        let end = insert_long_text(&mut fixture, 0);
        let page = &fixture.doc.document().sections[0].section_def.page_def;
        let capacity = i32::try_from(page.width - page.margin_left - page.margin_right).unwrap();
        fixture
            .doc
            .apply_char_format_in_cell(
                0,
                fixture.para_idx,
                fixture.control_idx,
                0,
                0,
                0,
                end,
                "{\"fontSize\":900}",
            )
            .unwrap();
        assert_vertical_capacity(&fixture, capacity, "vertical font size");
    }
}

#[test]
fn vertical_cell_line_spacing_keeps_pre_l1_section_width() {
    for direction in [1, 2] {
        let mut fixture = vertical_fixture(direction);
        insert_long_text(&mut fixture, 0);
        let page = &fixture.doc.document().sections[0].section_def.page_def;
        let capacity = i32::try_from(page.width - page.margin_left - page.margin_right).unwrap();
        fixture
            .doc
            .apply_para_format_in_cell(
                0,
                fixture.para_idx,
                fixture.control_idx,
                0,
                0,
                "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
            )
            .unwrap();
        assert_vertical_capacity(&fixture, capacity, "vertical line spacing");
        assert_triple_line_spacing(&fixture.doc, &fixture, 0, "vertical line spacing");
    }
}

#[test]
fn vertical_cell_input_keeps_pre_l1_cell_width() {
    let mut fixture = vertical_fixture(1);
    insert_long_text(&mut fixture, 0);
    let capacity = inner_width(table(&fixture.doc, &fixture), 0);
    assert_vertical_capacity(&fixture, capacity, "vertical input");
}

fn resized_caption_fixture() -> Fixture {
    let mut fixture = table_fixture_with_cols(2);
    fixture
        .doc
        .set_table_properties(
            0,
            fixture.para_idx.try_into().unwrap(),
            fixture.control_idx.try_into().unwrap(),
            "{\"hasCaption\":true}",
        )
        .unwrap();
    assert_eq!(
        table(&fixture.doc, &fixture)
            .caption
            .as_ref()
            .unwrap()
            .max_width,
        10582
    );
    let page = &fixture.doc.document().sections[0].section_def.page_def;
    let width = page.width - page.margin_left - page.margin_right;
    let target = width / 2;
    let delta = i64::from(target) - i64::from(COL_WIDTH);
    let updates = (0..4)
        .map(|i| format!("{{\"cellIdx\":{i},\"widthDelta\":{delta}}}"))
        .collect::<Vec<_>>()
        .join(",");
    fixture
        .doc
        .resize_table_cells(
            0,
            fixture.para_idx.try_into().unwrap(),
            fixture.control_idx.try_into().unwrap(),
            &format!("[{updates}]"),
        )
        .unwrap();
    fixture
}

fn insert_caption_text(fixture: &mut Fixture) -> usize {
    let prefix = table(&fixture.doc, fixture)
        .caption
        .as_ref()
        .unwrap()
        .paragraphs[0]
        .text
        .chars()
        .count();
    fixture
        .doc
        .insert_text_in_cell(
            0,
            fixture.para_idx.try_into().unwrap(),
            fixture.control_idx.try_into().unwrap(),
            65534,
            0,
            prefix.try_into().unwrap(),
            LONG_TEXT,
        )
        .unwrap();
    prefix + LONG_TEXT.chars().count()
}

fn assert_resized_caption(fixture: &Fixture, context: &str) {
    let current_table = table(&fixture.doc, fixture);
    let width: u32 = current_table.get_column_widths().iter().sum();
    let segs = &current_table.caption.as_ref().unwrap().paragraphs[0].line_segs;
    assert_eq!(
        segs.len(),
        1,
        "{context}: enlarged caption must fit on one line"
    );
    let width = i32::try_from(width).unwrap();
    assert!(
        (width - 1..=width).contains(&segs[0].segment_width),
        "{context}: current table width {width}, got {}",
        segs[0].segment_width
    );
}

#[test]
fn resized_table_caption_font_size_uses_current_geometry() {
    let mut fixture = resized_caption_fixture();
    let end = insert_caption_text(&mut fixture);
    fixture
        .doc
        .apply_char_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            65534,
            0,
            0,
            end,
            "{\"fontSize\":900}",
        )
        .unwrap();
    assert_resized_caption(&fixture, "resized caption font size");
}

#[test]
fn resized_table_caption_line_spacing_uses_current_geometry() {
    let mut fixture = resized_caption_fixture();
    insert_caption_text(&mut fixture);
    fixture
        .doc
        .apply_para_format_in_cell(
            0,
            fixture.para_idx,
            fixture.control_idx,
            65534,
            0,
            "{\"lineSpacing\":300,\"lineSpacingType\":\"Percent\"}",
        )
        .unwrap();
    assert_resized_caption(&fixture, "resized caption line spacing");
}

#[test]
fn resized_table_caption_input_uses_current_geometry() {
    let mut fixture = resized_caption_fixture();
    insert_caption_text(&mut fixture);
    assert_resized_caption(&fixture, "resized caption input");
}
