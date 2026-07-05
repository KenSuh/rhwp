use rhwp::model::control::Control;
use rhwp::model::page::{ColumnDef, PageDef};
use rhwp::model::paragraph::{LineSeg, Paragraph};
use rhwp::model::shape::{CommonObjAttr, TextWrap};
use rhwp::model::table::{Cell, Table};
use rhwp::renderer::pagination::{PageItem, PaginationResult, Paginator};
use rhwp::renderer::style_resolver::ResolvedStyleSet;

fn a4_page_def() -> PageDef {
    PageDef {
        width: 59_528,
        height: 84_188,
        margin_left: 8_504,
        margin_right: 8_504,
        margin_top: 5_669,
        margin_bottom: 4_252,
        margin_header: 4_252,
        margin_footer: 4_252,
        margin_gutter: 0,
        ..Default::default()
    }
}

fn make_text_paragraph(text: &str, line_height: i32) -> Paragraph {
    Paragraph {
        text: text.to_string(),
        line_segs: vec![LineSeg {
            line_height,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn make_square_table_paragraph(table_height: u32) -> Paragraph {
    Paragraph {
        line_segs: vec![LineSeg {
            segment_width: 42_518,
            ..Default::default()
        }],
        controls: vec![Control::Table(Box::new(Table {
            row_count: 1,
            col_count: 1,
            cells: vec![Cell {
                row: 0,
                col: 0,
                row_span: 1,
                col_span: 1,
                height: table_height,
                width: 30_000,
                ..Default::default()
            }],
            common: CommonObjAttr {
                text_wrap: TextWrap::Square,
                height: table_height,
                width: 30_000,
                ..Default::default()
            },
            ..Default::default()
        }))],
        ..Default::default()
    }
}

fn make_mixed_heading_and_square_table_paragraph() -> Paragraph {
    Paragraph {
        line_segs: vec![LineSeg {
            line_height: 3_000,
            line_spacing: 600,
            segment_width: 42_518,
            ..Default::default()
        }],
        controls: vec![
            Control::Table(Box::new(Table {
                row_count: 1,
                col_count: 1,
                cells: vec![Cell {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    height: 2_714,
                    width: 30_000,
                    ..Default::default()
                }],
                common: CommonObjAttr {
                    treat_as_char: true,
                    text_wrap: TextWrap::TopAndBottom,
                    height: 2_714,
                    width: 30_000,
                    ..Default::default()
                },
                ..Default::default()
            })),
            Control::Table(Box::new(Table {
                row_count: 1,
                col_count: 1,
                cells: vec![Cell {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    height: 58_000,
                    width: 41_900,
                    ..Default::default()
                }],
                common: CommonObjAttr {
                    text_wrap: TextWrap::Square,
                    height: 58_000,
                    width: 41_900,
                    ..Default::default()
                },
                ..Default::default()
            })),
        ],
        ..Default::default()
    }
}

fn make_tac_heading_and_in_front_table_paragraph() -> Paragraph {
    Paragraph {
        line_segs: vec![LineSeg {
            line_height: 3_000,
            segment_width: 42_518,
            ..Default::default()
        }],
        controls: vec![
            Control::Table(Box::new(Table {
                row_count: 1,
                col_count: 1,
                cells: vec![Cell {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    height: 58_000,
                    width: 30_000,
                    ..Default::default()
                }],
                common: CommonObjAttr {
                    treat_as_char: true,
                    text_wrap: TextWrap::TopAndBottom,
                    height: 58_000,
                    width: 30_000,
                    ..Default::default()
                },
                ..Default::default()
            })),
            Control::Table(Box::new(Table {
                row_count: 1,
                col_count: 1,
                cells: vec![Cell {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    height: 58_000,
                    width: 30_000,
                    ..Default::default()
                }],
                common: CommonObjAttr {
                    text_wrap: TextWrap::InFrontOfText,
                    height: 58_000,
                    width: 30_000,
                    ..Default::default()
                },
                ..Default::default()
            })),
        ],
        ..Default::default()
    }
}

fn page_index_for_paragraph(result: &PaginationResult, para_index: usize) -> Option<usize> {
    result.pages.iter().position(|page| {
        page.column_contents.iter().any(|column| {
            column.items.iter().any(|item| match item {
                PageItem::FullParagraph { para_index: pi }
                | PageItem::PartialParagraph { para_index: pi, .. }
                | PageItem::Table { para_index: pi, .. }
                | PageItem::PartialTable { para_index: pi, .. }
                | PageItem::Shape { para_index: pi, .. } => *pi == para_index,
            })
        })
    })
}

#[test]
fn in_front_table_does_not_disable_tac_height_correction() {
    let paginator = Paginator::with_default_dpi();
    let styles = ResolvedStyleSet::default();
    let paragraphs = vec![
        make_tac_heading_and_in_front_table_paragraph(),
        make_text_paragraph("다음 문단", 20_000),
    ];
    let composed = Vec::new();

    let (result, _measured) = paginator.paginate(
        &paragraphs,
        &composed,
        &styles,
        &a4_page_def(),
        &ColumnDef::default(),
        0,
    );

    let table_page =
        page_index_for_paragraph(&result, 0).expect("the mixed table paragraph should be placed");
    let following_page = page_index_for_paragraph(&result, 1)
        .expect("the paragraph after the mixed table paragraph should be placed");
    assert_eq!(
        following_page, table_page,
        "non-flow tables must not disable the TAC paragraph height correction"
    );
}

#[test]
fn mixed_tac_heading_and_square_table_reserves_square_table_height() {
    let paginator = Paginator::with_default_dpi();
    let styles = ResolvedStyleSet::default();
    let paragraphs = vec![
        make_mixed_heading_and_square_table_paragraph(),
        make_text_paragraph("주업종 구분 방법", 20_000),
    ];
    let composed = Vec::new();

    let (result, _measured) = paginator.paginate(
        &paragraphs,
        &composed,
        &styles,
        &a4_page_def(),
        &ColumnDef::default(),
        0,
    );

    let table_page =
        page_index_for_paragraph(&result, 0).expect("the mixed table paragraph should be placed");
    let following_page = page_index_for_paragraph(&result, 1)
        .expect("the paragraph after the mixed table paragraph should be placed");
    assert!(
        following_page > table_page,
        "the TAC height correction must not discard a following non-TAC Square table height"
    );
}

#[test]
fn square_wrap_table_that_fills_page_paginates_following_paragraph() {
    let paginator = Paginator::with_default_dpi();
    let styles = ResolvedStyleSet::default();
    let paragraphs = vec![
        make_square_table_paragraph(58_000),
        make_text_paragraph("업종 구분 방법", 20_000),
    ];
    let composed = Vec::new();

    let (result, _measured) = paginator.paginate(
        &paragraphs,
        &composed,
        &styles,
        &a4_page_def(),
        &ColumnDef::default(),
        0,
    );

    assert!(
        !result
            .wrap_around_paras
            .iter()
            .any(|wrap| wrap.para_index == 1),
        "the following paragraph must leave the Square-wrap region when the table leaves no vertical room"
    );

    let table_page =
        page_index_for_paragraph(&result, 0).expect("the Square-wrap table should be placed");
    let following_page = page_index_for_paragraph(&result, 1)
        .expect("the paragraph after the Square-wrap table should be placed");
    assert!(
        following_page > table_page,
        "the paragraph after a page-filling Square-wrap table must paginate to the next page"
    );
}
