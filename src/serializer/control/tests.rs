use super::*;
use crate::model::document::{Section, SectionDef};
use crate::model::page::PageDef;
use crate::model::paragraph::{CharShapeRef, LineSeg, Paragraph};
use crate::parser::body_text::parse_body_text_section;
use crate::serializer::body_text::serialize_section;

/// SectionDef 라운드트립
#[test]
fn test_roundtrip_section_def() {
    let sd = SectionDef {
        flags: 0,
        default_tab_spacing: 800,
        page_num: 1,
        page_def: PageDef {
            width: 59528,
            height: 84188,
            margin_left: 8504,
            margin_right: 8504,
            margin_top: 5669,
            margin_bottom: 4252,
            margin_header: 4252,
            margin_footer: 4252,
            ..Default::default()
        },
        ..Default::default()
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("A".to_string(), vec![8], Vec::new());

        paragraph_value.char_count = 3;
        // 0~7 = secd 컨트롤
        paragraph_value.char_shapes = vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }];

        paragraph_value.line_segs = vec![LineSeg {
            text_start: 0,
            ..Default::default()
        }];

        paragraph_value.controls = vec![Control::SectionDef(Box::new(sd))];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    assert_eq!(parsed.section_def.default_tab_spacing, 800);
    assert_eq!(parsed.section_def.page_num, 1);
    assert_eq!(parsed.section_def.page_def.width, 59528);
    assert_eq!(parsed.section_def.page_def.height, 84188);
}

/// ColumnDef 라운드트립
#[test]
fn test_roundtrip_column_def() {
    let cd = ColumnDef {
        column_type: ColumnType::Normal,
        column_count: 2,
        same_width: true,
        spacing: 1000,
        ..Default::default()
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::ColumnDef(cd)];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    let has_cold = parsed.paragraphs[0]
        .controls
        .iter()
        .any(|c| matches!(c, Control::ColumnDef(_)));
    assert!(has_cold);

    if let Some(Control::ColumnDef(cd)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::ColumnDef(_)))
    {
        assert_eq!(cd.column_count, 2);
        assert!(cd.same_width);
        assert_eq!(cd.spacing, 1000);
    }
}

/// Table 라운드트립
#[test]
fn test_roundtrip_table() {
    let cell = Cell {
        col: 0,
        row: 0,
        col_span: 1,
        row_span: 1,
        width: 10000,
        height: 5000,
        border_fill_id: 1,
        paragraphs: vec![{
            let mut paragraph_value = Paragraph::with_text("test".to_string(), vec![0, 1, 2, 3], Vec::new());

            paragraph_value.char_count = 5;

            paragraph_value.char_shapes = vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }];

            paragraph_value.line_segs = vec![LineSeg {
                text_start: 0,
                ..Default::default()
            }];
            paragraph_value
        }],
        ..Default::default()
    };

    let table = Table {
        row_count: 1,
        col_count: 1,
        cell_spacing: 0,
        row_sizes: vec![1], // 행별 셀 수
        border_fill_id: 1,
        cells: vec![cell],
        ..Default::default()
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::Table(Box::new(table))];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    let has_table = parsed.paragraphs[0]
        .controls
        .iter()
        .any(|c| matches!(c, Control::Table(_)));
    assert!(has_table);

    if let Some(Control::Table(t)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Table(_)))
    {
        assert_eq!(t.row_count, 1);
        assert_eq!(t.col_count, 1);
        assert_eq!(t.cells.len(), 1);
        assert_eq!(t.cells[0].width, 10000);
        assert_eq!(t.cells[0].paragraphs[0].text(), "test");
    }
}

/// AutoNumber 라운드트립
#[test]
fn test_roundtrip_auto_number() {
    let an = AutoNumber {
        number_type: AutoNumberType::Table,
        format: 0,
        superscript: false,
        ..Default::default()
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::AutoNumber(an)];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    if let Some(Control::AutoNumber(an)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::AutoNumber(_)))
    {
        assert_eq!(an.number_type, AutoNumberType::Table);
    } else {
        panic!("Expected AutoNumber control");
    }
}

/// Bookmark 라운드트립
#[test]
fn test_roundtrip_bookmark() {
    let bm = Bookmark {
        name: "테스트".to_string(),
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::Bookmark(bm)];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    if let Some(Control::Bookmark(bm)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Bookmark(_)))
    {
        assert_eq!(bm.name, "테스트");
    } else {
        panic!("Expected Bookmark control");
    }
}

/// PageHide 라운드트립
#[test]
fn test_roundtrip_page_hide() {
    let ph = PageHide {
        hide_header: true,
        hide_footer: true,
        hide_master_page: false,
        hide_border: false,
        hide_fill: false,
        hide_page_num: true,
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::PageHide(ph)];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    if let Some(Control::PageHide(ph)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::PageHide(_)))
    {
        assert!(ph.hide_header);
        assert!(ph.hide_footer);
        assert!(!ph.hide_master_page);
        assert!(ph.hide_page_num);
    } else {
        panic!("Expected PageHide control");
    }
}

/// Footnote 라운드트립
#[test]
fn test_roundtrip_footnote() {
    use crate::model::footnote::Footnote;

    let fn_ = Footnote {
        number: 3,
        paragraphs: vec![{
            let mut paragraph_value = Paragraph::with_text("각주".to_string(), vec![0, 1], Vec::new());

            paragraph_value.char_count = 3;

            paragraph_value.char_shapes = vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }];

            paragraph_value.line_segs = vec![LineSeg {
                text_start: 0,
                ..Default::default()
            }];
            paragraph_value
        }],
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::Footnote(Box::new(fn_))];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    if let Some(Control::Footnote(fn_)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Footnote(_)))
    {
        assert_eq!(fn_.number, 3);
        assert_eq!(fn_.paragraphs.len(), 1);
        assert_eq!(fn_.paragraphs[0].text(), "각주");
    } else {
        panic!("Expected Footnote control");
    }
}

/// Header 라운드트립
#[test]
fn test_roundtrip_header() {
    use crate::model::header_footer::Header;

    let header = Header {
        apply_to: HeaderFooterApply::Both,
        paragraphs: vec![{
            let mut paragraph_value = Paragraph::with_text("머리말".to_string(), vec![0, 1, 2], Vec::new());

            paragraph_value.char_count = 4;

            paragraph_value.char_shapes = vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }];

            paragraph_value.line_segs = vec![LineSeg {
                text_start: 0,
                ..Default::default()
            }];
            paragraph_value
        }],
        ..Default::default()
    };

    let para = {
        let mut paragraph_value = Paragraph::with_text("".to_string(), vec![], Vec::new());

        paragraph_value.char_count = 2;

        paragraph_value.controls = vec![Control::Header(Box::new(header))];
        paragraph_value
    };

    let section = Section {
        paragraphs: vec![para],
        raw_stream: None,
        ..Default::default()
    };

    let bytes = serialize_section(&section);
    let parsed = parse_body_text_section(&bytes).unwrap();

    if let Some(Control::Header(h)) = parsed.paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Header(_)))
    {
        assert_eq!(h.apply_to, HeaderFooterApply::Both);
        assert_eq!(h.paragraphs.len(), 1);
        assert_eq!(h.paragraphs[0].text(), "머리말");
    } else {
        panic!("Expected Header control");
    }
}
