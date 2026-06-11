//! `<hp:tbl>` 표 직렬화.
//!
//! Stage 3 (#182): `Control::Table` IR → `<hp:tbl>` + `<hp:tr>` + `<hp:tc>` + `<hp:subList>` + 문단 재귀.
//!
//! 속성·자식 순서는 한컴 OWPML 공식 (hancom-io/hwpx-owpml-model, Apache 2.0)
//! `Class/Para/TableType.cpp` 의 `WriteElement()`, `InitMap()` 기준:
//!
//! ### `<hp:tbl>` 속성 순서 (부모 AbstractShapeObjectType + 자신)
//! id, zOrder, numberingType, textWrap, textFlow, lock, dropcapstyle,
//! pageBreak, repeatHeader, rowCnt, colCnt, cellSpacing, borderFillIDRef, noAdjust
//!
//! ### `<hp:tbl>` 자식 순서
//! sz, pos, outMargin, (caption, shapeComment, parameterset, metaTag — 옵셔널),
//! inMargin, (cellzoneList — 옵셔널), tr (루프), (label — 옵셔널)
//!
//! ### `<hp:tc>` 속성 순서
//! name, header, hasMargin, protect, editable, dirty, borderFillIDRef
//!
//! ### `<hp:tc>` 자식 순서
//! subList, cellAddr, cellSpan, cellSz, cellMargin
//!
//! ## 중요: table.attr 비트 연산 금지
//!
//! HWPX에서 `table.attr` 는 0인 경우가 많으므로 비트 연산으로 `textWrap/textFlow/pageBreak` 등을
//! 추출하면 안 된다. 반드시 `table.common.text_wrap`, `table.page_break` 등 파싱된 IR 필드를 사용.

use std::io::Write;

use quick_xml::Writer;

use crate::model::control::Control;
use crate::model::paragraph::{CharShapeRef, LineSeg};
use crate::model::shape::{
    CommonObjAttr, HorzAlign, HorzRelTo, ShapeObject, TextWrap, VertAlign, VertRelTo,
};
use crate::model::table::{Cell, Table, TablePageBreak, VerticalAlign};

use super::context::SerializeContext;
use super::form::write_form;
use super::picture::write_picture;
use super::shape::{write_container_close, write_container_open, write_line, write_rect};
use super::utils::{
    empty_tag, end_tag, first_char_shape_id, split_char_shape_runs, start_tag, start_tag_attrs,
    trailing_zero_length_refs,
};
use super::SerializeError;

/// `<hp:tbl>` 직렬화.
pub fn write_table<W: Write>(
    w: &mut Writer<W>,
    table: &Table,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    // borderFillIDRef 참조 등록 (assert_all_refs_resolved 검증 대상)
    ctx.border_fill_ids.reference(table.border_fill_id);
    for zone in &table.zones {
        ctx.border_fill_ids.reference(zone.border_fill_id);
    }
    for cell in &table.cells {
        ctx.border_fill_ids.reference(cell.border_fill_id);
    }

    // --- <hp:tbl> 시작 태그 + 속성 ---
    let id_str = table.common.instance_id.to_string();
    let z_order = table.common.z_order.to_string();
    let text_wrap = text_wrap_str(table.common.text_wrap);
    let text_flow = text_flow_str(table.common.text_wrap);
    let lock = bool01(false);
    let page_break = table_page_break_str(table.page_break);
    let repeat_header = bool01(table.repeat_header);
    let row_cnt = table.row_count.to_string();
    let col_cnt = table.col_count.to_string();
    let cell_spacing = table.cell_spacing.to_string();
    let border_fill_id_ref = table.border_fill_id.to_string();

    start_tag_attrs(
        w,
        "hp:tbl",
        &[
            ("id", &id_str),
            ("zOrder", &z_order),
            ("numberingType", "TABLE"),
            ("textWrap", text_wrap),
            ("textFlow", text_flow),
            ("lock", lock),
            ("dropcapstyle", "None"),
            ("pageBreak", page_break),
            ("repeatHeader", repeat_header),
            ("rowCnt", &row_cnt),
            ("colCnt", &col_cnt),
            ("cellSpacing", &cell_spacing),
            ("borderFillIDRef", &border_fill_id_ref),
            ("noAdjust", "0"),
        ],
    )?;

    // --- 자식: sz, pos, outMargin, inMargin, tr[] ---
    write_sz(w, &table.common)?;
    write_pos(w, &table.common)?;
    write_out_margin(w, table)?;
    write_in_margin(w, table)?;

    // tr[]: 행 단위 반복. 각 행에 속한 셀 (cell.row == r) 을 col 오름차순으로 출력.
    for row_idx in 0..table.row_count {
        start_tag(w, "hp:tr")?;
        let mut row_cells: Vec<&Cell> = table.cells.iter().filter(|c| c.row == row_idx).collect();
        row_cells.sort_by_key(|c| c.col);
        for cell in row_cells {
            write_cell(w, cell, ctx)?;
        }
        end_tag(w, "hp:tr")?;
    }

    end_tag(w, "hp:tbl")?;
    Ok(())
}

fn write_sz<W: Write>(w: &mut Writer<W>, c: &CommonObjAttr) -> Result<(), SerializeError> {
    let width = c.width.to_string();
    let height = c.height.to_string();
    empty_tag(
        w,
        "hp:sz",
        &[
            ("width", &width),
            ("widthRelTo", "ABSOLUTE"),
            ("height", &height),
            ("heightRelTo", "ABSOLUTE"),
            ("protect", "0"),
        ],
    )
}

fn write_pos<W: Write>(w: &mut Writer<W>, c: &CommonObjAttr) -> Result<(), SerializeError> {
    let treat = bool01(c.treat_as_char);
    let vert_offset = c.vertical_offset.to_string();
    let horz_offset = c.horizontal_offset.to_string();
    empty_tag(
        w,
        "hp:pos",
        &[
            ("treatAsChar", treat),
            ("affectLSpacing", "0"),
            ("flowWithText", "1"),
            ("allowOverlap", "0"),
            ("holdAnchorAndSO", "0"),
            ("vertRelTo", vert_rel_to_str(c.vert_rel_to)),
            ("horzRelTo", horz_rel_to_str(c.horz_rel_to)),
            ("vertAlign", vert_align_str(c.vert_align)),
            ("horzAlign", horz_align_str(c.horz_align)),
            ("vertOffset", &vert_offset),
            ("horzOffset", &horz_offset),
        ],
    )
}

fn write_out_margin<W: Write>(w: &mut Writer<W>, t: &Table) -> Result<(), SerializeError> {
    let left = t.outer_margin_left.to_string();
    let right = t.outer_margin_right.to_string();
    let top = t.outer_margin_top.to_string();
    let bottom = t.outer_margin_bottom.to_string();
    empty_tag(
        w,
        "hp:outMargin",
        &[
            ("left", &left),
            ("right", &right),
            ("top", &top),
            ("bottom", &bottom),
        ],
    )
}

fn write_in_margin<W: Write>(w: &mut Writer<W>, t: &Table) -> Result<(), SerializeError> {
    let left = t.padding.left.to_string();
    let right = t.padding.right.to_string();
    let top = t.padding.top.to_string();
    let bottom = t.padding.bottom.to_string();
    empty_tag(
        w,
        "hp:inMargin",
        &[
            ("left", &left),
            ("right", &right),
            ("top", &top),
            ("bottom", &bottom),
        ],
    )
}

fn write_cell<W: Write>(
    w: &mut Writer<W>,
    cell: &Cell,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    let name = cell.field_name.as_deref().unwrap_or("");
    let header = bool01(cell.is_header);
    let has_margin = bool01(cell.apply_inner_margin);
    let border_ref = cell.border_fill_id.to_string();

    start_tag_attrs(
        w,
        "hp:tc",
        &[
            ("name", name),
            ("header", header),
            ("hasMargin", has_margin),
            ("protect", "0"),
            ("editable", "0"),
            ("dirty", "0"),
            ("borderFillIDRef", &border_ref),
        ],
    )?;

    // 자식 순서: subList, cellAddr, cellSpan, cellSz, cellMargin
    write_sub_list(w, cell, ctx)?;
    write_cell_addr(w, cell)?;
    write_cell_span(w, cell)?;
    write_cell_sz(w, cell)?;
    write_cell_margin(w, cell)?;

    end_tag(w, "hp:tc")?;
    Ok(())
}

fn write_sub_list<W: Write>(
    w: &mut Writer<W>,
    cell: &Cell,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    start_tag_attrs(
        w,
        "hp:subList",
        &[
            ("id", ""),
            (
                "textDirection",
                if cell.text_direction == 1 {
                    "VERTICAL"
                } else {
                    "HORIZONTAL"
                },
            ),
            ("lineWrap", "BREAK"),
            ("vertAlign", cell_vert_align_str(cell.vertical_align)),
            ("linkListIDRef", "0"),
            ("linkListNextIDRef", "0"),
            ("textWidth", "0"),
            ("textHeight", "0"),
            ("hasTextRef", "0"),
            ("hasNumRef", "0"),
        ],
    )?;

    // 셀 내부 문단 재귀 — 각 문단은 간단한 <hp:p><hp:run><hp:t>텍스트</hp:t></hp:run></hp:p> 구조
    for (pi, para) in cell.paragraphs.iter().enumerate() {
        ctx.para_shape_ids.reference(para.para_shape_id);
        ctx.style_ids.reference(para.style_id as u16);
        for cs_ref in &para.char_shapes {
            ctx.char_shape_ids.reference(cs_ref.char_shape_id);
        }

        let pi_str = pi.to_string();
        let ppr = para.para_shape_id.to_string();
        let sp = para.style_id.to_string();
        start_tag_attrs(
            w,
            "hp:p",
            &[
                ("id", &pi_str),
                ("paraPrIDRef", &ppr),
                ("styleIDRef", &sp),
                ("pageBreak", "0"),
                ("columnBreak", "0"),
                ("merged", "0"),
            ],
        )?;

        let first_cs = para
            .char_shapes
            .first()
            .map(|r| r.char_shape_id)
            .unwrap_or(0);
        write_cell_text_runs(
            w,
            &para.text,
            &para.char_offsets,
            &para.char_shapes,
            para.controls.is_empty(),
        )?;
        for control in &para.controls {
            write_cell_control_run(w, control, ctx, first_cs)?;
        }

        write_cell_linesegs(w, &para.line_segs)?;

        end_tag(w, "hp:p")?;
    }

    end_tag(w, "hp:subList")?;
    Ok(())
}

fn write_cell_linesegs<W: Write>(
    w: &mut Writer<W>,
    line_segs: &[LineSeg],
) -> Result<(), SerializeError> {
    start_tag(w, "hp:linesegarray")?;
    if line_segs.is_empty() {
        write_cell_fallback_lineseg(w)?;
    } else {
        for seg in line_segs {
            let textpos = seg.text_start.to_string();
            let vertpos = seg.vertical_pos.to_string();
            let vertsize = seg.line_height.to_string();
            let textheight = seg.text_height.to_string();
            let baseline = seg.baseline_distance.to_string();
            let spacing = seg.line_spacing.to_string();
            let horzpos = seg.column_start.to_string();
            let horzsize = seg.segment_width.to_string();
            let flags = seg.tag.to_string();
            empty_tag(
                w,
                "hp:lineseg",
                &[
                    ("textpos", &textpos),
                    ("vertpos", &vertpos),
                    ("vertsize", &vertsize),
                    ("textheight", &textheight),
                    ("baseline", &baseline),
                    ("spacing", &spacing),
                    ("horzpos", &horzpos),
                    ("horzsize", &horzsize),
                    ("flags", &flags),
                ],
            )?;
        }
    }
    end_tag(w, "hp:linesegarray")?;
    Ok(())
}

fn write_cell_fallback_lineseg<W: Write>(w: &mut Writer<W>) -> Result<(), SerializeError> {
    empty_tag(
        w,
        "hp:lineseg",
        &[
            ("textpos", "0"),
            ("vertpos", "0"),
            ("vertsize", "1000"),
            ("textheight", "1000"),
            ("baseline", "850"),
            ("spacing", "600"),
            ("horzpos", "0"),
            ("horzsize", "12964"),
            ("flags", "393216"),
        ],
    )
}

fn write_cell_text_runs<W: Write>(
    w: &mut Writer<W>,
    text: &str,
    char_offsets: &[u32],
    char_shapes: &[CharShapeRef],
    preserve_trailing_runs: bool,
) -> Result<(), SerializeError> {
    if text.is_empty() {
        let cs = first_char_shape_id(char_shapes).to_string();
        start_tag_attrs(w, "hp:run", &[("charPrIDRef", &cs)])?;
        write_cell_text(w, "")?;
        end_tag(w, "hp:run")?;
        if preserve_trailing_runs {
            write_zero_length_runs(w, &trailing_zero_length_refs(char_shapes, 0, true))?;
        }
        return Ok(());
    }

    let (runs, text_end) = split_char_shape_runs(text, char_offsets, char_shapes);
    for (shape_id, piece) in &runs {
        write_cell_text_run(w, *shape_id, piece)?;
    }
    // 텍스트 끝 이후 위치에 걸린 zero-length charPr run(문단말 캐럿 스타일 — 실제 한컴
    // 파일의 trailing <hp:run charPrIDRef="N"/> 패턴)은 글자 루프가 도달하지 못해
    // 이전에는 저장 시 유실됐다. 원본 순서대로 빈 run 으로 재출력해 보존한다.
    //
    // 단 컨트롤이 있는 문단(preserve_trailing_runs=false)에서는 보존하지 않는다 —
    // export 가 컨트롤 run 을 텍스트 뒤에 배치하며 그 charPr 가 reparse 때 text-end
    // 위치의 ref 로 기록되는데(아티팩트), 이를 다시 echo 하면 저장 사이클마다 ref 가
    // 증식한다. 문단말 캐럿 스타일의 전형 케이스(컨트롤 없는 문단)만 보존한다.
    if preserve_trailing_runs {
        write_zero_length_runs(w, &trailing_zero_length_refs(char_shapes, text_end, false))?;
    }
    Ok(())
}

/// zero-length charPr run 목록을 빈 `<hp:run/>` 태그로 출력한다.
fn write_zero_length_runs<W: Write>(
    w: &mut Writer<W>,
    char_shape_ids: &[u32],
) -> Result<(), SerializeError> {
    for id in char_shape_ids {
        let cs = id.to_string();
        empty_tag(w, "hp:run", &[("charPrIDRef", &cs)])?;
    }
    Ok(())
}

fn write_cell_text_run<W: Write>(
    w: &mut Writer<W>,
    char_shape_id: u32,
    text: &str,
) -> Result<(), SerializeError> {
    let cs = char_shape_id.to_string();
    start_tag_attrs(w, "hp:run", &[("charPrIDRef", &cs)])?;
    write_cell_text(w, text)?;
    end_tag(w, "hp:run")
}

fn write_cell_text<W: Write>(w: &mut Writer<W>, text: &str) -> Result<(), SerializeError> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    // <hp:t>text</hp:t>
    // 파서가 셀 문단 텍스트에 남긴 필드 마커(U+0003/U+0004, hp:fieldBegin/End) 등
    // 탭·줄바꿈 외 제어문자는 XML 1.0 Char 범위 밖이라 escape 로도 표현이 불가능해,
    // 그대로 쓰면 well-formed 하지 않은 section XML 이 되어 한컴오피스 등 conforming
    // 파서가 저장 파일을 열지 못한다. body 경로(render_hp_t_content)와 동일하게 제거한다.
    let sanitized: String = text
        .chars()
        .filter(|c| (*c as u32) >= 0x20 || *c == '\t' || *c == '\n')
        .collect();
    w.write_event(Event::Start(BytesStart::new("hp:t")))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    if !sanitized.is_empty() {
        w.write_event(Event::Text(BytesText::new(&sanitized)))
            .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    }
    w.write_event(Event::End(BytesEnd::new("hp:t")))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

fn write_cell_control_run<W: Write>(
    w: &mut Writer<W>,
    control: &Control,
    ctx: &mut SerializeContext,
    char_shape_id: u32,
) -> Result<(), SerializeError> {
    if !control_supported(control) {
        return Ok(());
    }

    let cs_str = char_shape_id.to_string();
    start_tag_attrs(w, "hp:run", &[("charPrIDRef", &cs_str)])?;
    write_cell_control(w, control, ctx)?;
    end_tag(w, "hp:run")?;
    Ok(())
}

fn control_supported(control: &Control) -> bool {
    match control {
        Control::Table(_) | Control::Picture(_) | Control::Form(_) => true,
        Control::Shape(shape) => shape_supported(shape),
        _ => false,
    }
}

fn write_cell_control<W: Write>(
    w: &mut Writer<W>,
    control: &Control,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    match control {
        Control::Table(table) => write_table(w, table, ctx),
        Control::Picture(pic) => write_picture(w, pic, ctx),
        Control::Shape(shape) => {
            let _ = write_cell_shape(w, shape, ctx)?;
            Ok(())
        }
        Control::Form(form) => write_form(w, form),
        _ => Ok(()),
    }
}

fn shape_supported(shape: &ShapeObject) -> bool {
    match shape {
        ShapeObject::Line(_) | ShapeObject::Rectangle(_) | ShapeObject::Picture(_) => true,
        ShapeObject::Group(group) => group.children.iter().any(shape_supported),
        _ => false,
    }
}

fn write_cell_shape<W: Write>(
    w: &mut Writer<W>,
    shape: &ShapeObject,
    ctx: &SerializeContext,
) -> Result<bool, SerializeError> {
    match shape {
        ShapeObject::Line(line) => {
            write_line(w, line)?;
            Ok(true)
        }
        ShapeObject::Rectangle(rect) => {
            write_rect(w, rect)?;
            Ok(true)
        }
        ShapeObject::Group(group) => {
            write_container_open(w, &group.common)?;
            for child in &group.children {
                let _ = write_cell_shape(w, child, ctx)?;
            }
            write_container_close(w)?;
            Ok(true)
        }
        ShapeObject::Picture(pic) => {
            write_picture(w, pic, ctx)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn write_cell_addr<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let col = cell.col.to_string();
    let row = cell.row.to_string();
    empty_tag(w, "hp:cellAddr", &[("colAddr", &col), ("rowAddr", &row)])
}

fn write_cell_span<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let cs = cell.col_span.max(1).to_string();
    let rs = cell.row_span.max(1).to_string();
    empty_tag(w, "hp:cellSpan", &[("colSpan", &cs), ("rowSpan", &rs)])
}

fn write_cell_sz<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let w_s = cell.width.to_string();
    let h_s = cell.height.to_string();
    empty_tag(w, "hp:cellSz", &[("width", &w_s), ("height", &h_s)])
}

fn write_cell_margin<W: Write>(w: &mut Writer<W>, cell: &Cell) -> Result<(), SerializeError> {
    let l = cell.padding.left.to_string();
    let r = cell.padding.right.to_string();
    let t = cell.padding.top.to_string();
    let b = cell.padding.bottom.to_string();
    empty_tag(
        w,
        "hp:cellMargin",
        &[("left", &l), ("right", &r), ("top", &t), ("bottom", &b)],
    )
}

// ---------- enum 변환 헬퍼 ----------

fn bool01(b: bool) -> &'static str {
    if b {
        "1"
    } else {
        "0"
    }
}

fn text_wrap_str(w: TextWrap) -> &'static str {
    use TextWrap::*;
    match w {
        Square => "SQUARE",
        Tight => "TIGHT",
        Through => "THROUGH",
        TopAndBottom => "TOP_AND_BOTTOM",
        BehindText => "BEHIND_TEXT",
        InFrontOfText => "IN_FRONT_OF_TEXT",
    }
}

/// textFlow: TextWrap 에 따라 결정 (한컴 관찰값 기준).
fn text_flow_str(w: TextWrap) -> &'static str {
    use TextWrap::*;
    match w {
        Square | Tight | Through => "BOTH_SIDES",
        _ => "BOTH_SIDES",
    }
}

fn table_page_break_str(pb: TablePageBreak) -> &'static str {
    use TablePageBreak::*;
    match pb {
        None => "NONE",
        CellBreak => "CELL",
        RowBreak => "TABLE",
    }
}

fn vert_rel_to_str(v: VertRelTo) -> &'static str {
    use VertRelTo::*;
    match v {
        Paper => "PAPER",
        Page => "PAGE",
        Para => "PARA",
    }
}

fn horz_rel_to_str(h: HorzRelTo) -> &'static str {
    use HorzRelTo::*;
    match h {
        Paper => "PAPER",
        Page => "PAGE",
        Column => "COLUMN",
        Para => "PARA",
    }
}

fn vert_align_str(v: VertAlign) -> &'static str {
    use VertAlign::*;
    match v {
        Top => "TOP",
        Center => "CENTER",
        Bottom => "BOTTOM",
        Inside => "INSIDE",
        Outside => "OUTSIDE",
    }
}

fn horz_align_str(h: HorzAlign) -> &'static str {
    use HorzAlign::*;
    match h {
        Left => "LEFT",
        Center => "CENTER",
        Right => "RIGHT",
        Inside => "INSIDE",
        Outside => "OUTSIDE",
    }
}

fn cell_vert_align_str(v: VerticalAlign) -> &'static str {
    use VerticalAlign::*;
    match v {
        Top => "TOP",
        Center => "CENTER",
        Bottom => "BOTTOM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Document;
    use crate::model::paragraph::{CharShapeRef, LineSeg, Paragraph};
    use crate::model::table::{Cell, Table};
    use crate::serializer::hwpx::context::SerializeContext;

    fn empty_table(rows: u16, cols: u16) -> Table {
        let mut t = Table::default();
        t.row_count = rows;
        t.col_count = cols;
        for r in 0..rows {
            for c in 0..cols {
                let mut cell = Cell::default();
                cell.col = c;
                cell.row = r;
                cell.col_span = 1;
                cell.row_span = 1;
                cell.width = 1000;
                cell.height = 300;
                cell.paragraphs.push(Paragraph::default());
                t.cells.push(cell);
            }
        }
        t.rebuild_grid();
        t
    }

    fn serialize(table: &Table) -> String {
        let doc = Document::default();
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        write_table(&mut w, table, &mut ctx).expect("write_table");
        String::from_utf8(w.into_inner()).unwrap()
    }

    #[test]
    fn tbl_root_attrs_in_canonical_order() {
        let t = empty_table(2, 3);
        let xml = serialize(&t);
        assert!(xml.contains("<hp:tbl "), "should emit <hp:tbl>: {}", xml);
        // id → zOrder → numberingType → textWrap → textFlow → lock → dropcapstyle →
        // pageBreak → repeatHeader → rowCnt → colCnt → cellSpacing → borderFillIDRef → noAdjust
        let ip = xml.find("id=").unwrap();
        let zp = xml.find("zOrder=").unwrap();
        let nt = xml.find("numberingType=").unwrap();
        let tw = xml.find("textWrap=").unwrap();
        let tf = xml.find("textFlow=").unwrap();
        let rc = xml.find("rowCnt=").unwrap();
        let cc = xml.find("colCnt=").unwrap();
        let bf = xml.find("borderFillIDRef=").unwrap();
        let na = xml.find("noAdjust=").unwrap();
        assert!(
            ip < zp && zp < nt && nt < tw && tw < tf && tf < rc && rc < cc && cc < bf && bf < na
        );
    }

    #[test]
    fn tr_count_matches_row_count() {
        let t = empty_table(4, 2);
        let xml = serialize(&t);
        assert_eq!(xml.matches("<hp:tr>").count(), 4);
    }

    #[test]
    fn tc_count_matches_cell_count() {
        let t = empty_table(2, 3);
        let xml = serialize(&t);
        assert_eq!(xml.matches("<hp:tc ").count(), 6);
    }

    #[test]
    fn cells_have_canonical_child_order() {
        let t = empty_table(1, 1);
        let xml = serialize(&t);
        // subList → cellAddr → cellSpan → cellSz → cellMargin
        let sl = xml.find("<hp:subList ").unwrap();
        let ca = xml.find("<hp:cellAddr ").unwrap();
        let cs = xml.find("<hp:cellSpan ").unwrap();
        let cz = xml.find("<hp:cellSz ").unwrap();
        let cm = xml.find("<hp:cellMargin ").unwrap();
        assert!(sl < ca && ca < cs && cs < cz && cz < cm);
    }

    #[test]
    fn cell_addr_reflects_coordinates() {
        let t = empty_table(2, 2);
        let xml = serialize(&t);
        assert!(xml.contains(r#"<hp:cellAddr colAddr="0" rowAddr="0"/>"#));
        assert!(xml.contains(r#"<hp:cellAddr colAddr="1" rowAddr="0"/>"#));
        assert!(xml.contains(r#"<hp:cellAddr colAddr="0" rowAddr="1"/>"#));
        assert!(xml.contains(r#"<hp:cellAddr colAddr="1" rowAddr="1"/>"#));
    }

    #[test]
    fn cell_span_defaults_to_one() {
        let t = empty_table(1, 1);
        let xml = serialize(&t);
        assert!(xml.contains(r#"<hp:cellSpan colSpan="1" rowSpan="1"/>"#));
    }

    #[test]
    fn border_fill_id_ref_registered_in_ctx() {
        let doc = Document::default();
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let mut t = empty_table(1, 1);
        t.border_fill_id = 99;
        t.cells[0].border_fill_id = 99;
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        write_table(&mut w, &t, &mut ctx).unwrap();
        // 99 는 등록되지 않은 borderFill → unresolved
        assert!(ctx.border_fill_ids.unresolved().contains(&99u16));
    }

    #[test]
    fn cell_paragraph_linesegs_are_preserved() {
        let mut t = empty_table(1, 1);
        t.cells[0].paragraphs[0].text = "긴 셀 문단".to_string();
        t.cells[0].paragraphs[0].line_segs = vec![
            LineSeg {
                text_start: 0,
                vertical_pos: 123,
                line_height: 1400,
                text_height: 1100,
                baseline_distance: 900,
                line_spacing: 300,
                column_start: 77,
                segment_width: 8888,
                tag: 111,
            },
            LineSeg {
                text_start: 6,
                vertical_pos: 1523,
                line_height: 1300,
                text_height: 1000,
                baseline_distance: 820,
                line_spacing: 250,
                column_start: 99,
                segment_width: 7777,
                tag: 222,
            },
        ];

        let xml = serialize(&t);
        assert_eq!(xml.matches("<hp:lineseg ").count(), 2, "{}", xml);
        assert!(
            xml.contains(
                r#"<hp:lineseg textpos="0" vertpos="123" vertsize="1400" textheight="1100" baseline="900" spacing="300" horzpos="77" horzsize="8888" flags="111"/>"#
            ),
            "{}",
            xml,
        );
        assert!(
            xml.contains(
                r#"<hp:lineseg textpos="6" vertpos="1523" vertsize="1300" textheight="1000" baseline="820" spacing="250" horzpos="99" horzsize="7777" flags="222"/>"#
            ),
            "{}",
            xml,
        );
    }

    #[test]
    fn cell_paragraph_char_shape_runs_are_preserved() {
        let mut t = empty_table(1, 1);
        t.cells[0].paragraphs[0].text = "가나다ABC".to_string();
        t.cells[0].paragraphs[0].char_offsets = vec![0, 1, 2, 3, 4, 5];
        t.cells[0].paragraphs[0].char_shapes = vec![
            CharShapeRef {
                start_pos: 0,
                char_shape_id: 28,
            },
            CharShapeRef {
                start_pos: 3,
                char_shape_id: 39,
            },
        ];

        let xml = serialize(&t);
        assert!(
            xml.contains(
                r#"<hp:run charPrIDRef="28"><hp:t>가나다</hp:t></hp:run><hp:run charPrIDRef="39"><hp:t>ABC</hp:t></hp:run>"#
            ),
            "{}",
            xml,
        );
    }

    #[test]
    fn cell_text_control_chars_are_sanitized_for_xml() {
        // 셀 내 누름틀 필드 마커(U+0003/U+0004)는 XML 1.0 에서 표현 불가능 — 그대로 쓰면
        // well-formed 가 깨져 한컴오피스가 저장 파일을 못 연다. 탭/줄바꿈은 보존한다.
        let mut t = empty_table(1, 1);
        t.cells[0].paragraphs[0].text = "(\u{3}정부24\u{4})\t끝".to_string();

        let xml = serialize(&t);
        assert!(!xml.contains('\u{3}'), "U+0003 제거: {}", xml);
        assert!(!xml.contains('\u{4}'), "U+0004 제거: {}", xml);
        assert!(xml.contains("(정부24)\t끝"), "본문/탭 보존: {}", xml);
    }

    #[test]
    fn cell_trailing_zero_length_char_run_is_preserved() {
        // 텍스트 끝 이후 위치의 zero-length charPr run(문단말 캐럿 스타일)은 빈
        // <hp:run/> 으로 재출력된다 (컨트롤 없는 문단 한정).
        let mut t = empty_table(1, 1);
        t.cells[0].paragraphs[0].text = "ab".to_string();
        t.cells[0].paragraphs[0].char_offsets = vec![0, 1];
        t.cells[0].paragraphs[0].char_shapes = vec![
            CharShapeRef {
                start_pos: 0,
                char_shape_id: 5,
            },
            CharShapeRef {
                start_pos: 2,
                char_shape_id: 9,
            },
        ];

        let xml = serialize(&t);
        assert!(
            xml.contains(
                r#"<hp:run charPrIDRef="5"><hp:t>ab</hp:t></hp:run><hp:run charPrIDRef="9"/>"#
            ),
            "trailing zero-length run 보존: {}",
            xml,
        );
    }
}
