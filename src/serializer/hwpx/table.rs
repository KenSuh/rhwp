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

use super::field::{write_field_begin, write_field_end_for_field};
use crate::model::control::Control;
use crate::model::paragraph::{CharShapeRef, LineSeg, Paragraph};
use crate::model::shape::{
    CommonObjAttr, HorzAlign, HorzRelTo, ShapeObject, SizeCriterion, TextFlow, TextWrap, VertAlign,
    VertRelTo,
};
use crate::model::table::{
    Cell, Table, TablePageBreak, VerticalAlign, CELL_LINE_WRAP_KEEP, CELL_LINE_WRAP_SQUEEZE,
};

use super::context::SerializeContext;
use super::form::write_form;
use super::picture::write_picture;
use super::section::write_column_def;
use super::shape::{write_container_close, write_container_open, write_line, write_rect};
use super::utils::{empty_tag, end_tag, start_tag, start_tag_attrs};
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
    let text_flow = text_flow_str(table.common.text_flow);
    let lock = bool01(table.common.locked);
    let page_break = table_page_break_str(table.page_break);
    let repeat_header = bool01(table.repeat_header);
    let no_adjust = bool01(table.no_adjust);
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
            ("noAdjust", no_adjust),
        ],
    )?;

    // --- 자식: sz, pos, outMargin, (caption), inMargin, (cellzoneList), tr[] ---
    write_sz(w, &table.common)?;
    write_pos(w, &table.common)?;
    write_out_margin(w, table)?;
    if let Some(caption) = &table.caption {
        super::shape::write_caption(w, caption, ctx)?;
    }
    write_in_margin(w, table)?;
    write_cellzone_list(w, table)?;

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
            ("widthRelTo", width_criterion_str(c.width_criterion)),
            ("height", &height),
            ("heightRelTo", height_criterion_str(c.height_criterion)),
            ("protect", bool01(c.size_protect)),
        ],
    )
}

fn write_pos<W: Write>(w: &mut Writer<W>, c: &CommonObjAttr) -> Result<(), SerializeError> {
    let treat = bool01(c.treat_as_char);
    // 오프셋은 부호 있는 HWPUNIT 이다. IR 은 u32 로 담으므로 음수가 4294967xxx 로 새지 않게 되돌린다.
    let vert_offset = (c.vertical_offset as i32).to_string();
    let horz_offset = (c.horizontal_offset as i32).to_string();
    empty_tag(
        w,
        "hp:pos",
        &[
            ("treatAsChar", treat),
            ("affectLSpacing", bool01(c.affect_line_spacing)),
            ("flowWithText", bool01(c.flow_with_text)),
            ("allowOverlap", bool01(c.allow_overlap)),
            ("holdAnchorAndSO", bool01(c.prevent_page_break != 0)),
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

/// `<hp:cellzoneList>` — 자식 순서상 inMargin 다음, tr 앞. 영역이 없으면 내지 않는다.
fn write_cellzone_list<W: Write>(w: &mut Writer<W>, t: &Table) -> Result<(), SerializeError> {
    if t.zones.is_empty() {
        return Ok(());
    }
    start_tag(w, "hp:cellzoneList")?;
    for zone in &t.zones {
        let start_row = zone.start_row.to_string();
        let start_col = zone.start_col.to_string();
        let end_row = zone.end_row.to_string();
        let end_col = zone.end_col.to_string();
        let border_fill = zone.border_fill_id.to_string();
        empty_tag(
            w,
            "hp:cellzone",
            &[
                ("startRowAddr", &start_row),
                ("startColAddr", &start_col),
                ("endRowAddr", &end_row),
                ("endColAddr", &end_col),
                ("borderFillIDRef", &border_fill),
            ],
        )?;
    }
    end_tag(w, "hp:cellzoneList")
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
            ("protect", bool01(cell.cell_protect())),
            ("editable", bool01(cell.editable_in_form())),
            ("dirty", bool01(cell.dirty_flag)),
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
            ("lineWrap", cell_line_wrap_str(cell.line_wrap)),
            ("vertAlign", cell_vert_align_str(cell.vertical_align)),
            ("linkListIDRef", "0"),
            ("linkListNextIDRef", "0"),
            ("textWidth", "0"),
            ("textHeight", "0"),
            ("hasTextRef", "0"),
            ("hasNumRef", "0"),
        ],
    )?;

    for (pi, para) in cell.paragraphs.iter().enumerate() {
        write_paragraph(w, para, pi, ctx)?;
    }
    end_tag(w, "hp:subList")?;
    Ok(())
}

pub(super) fn write_paragraph<W: Write>(
    w: &mut Writer<W>,
    para: &Paragraph,
    id: usize,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    ctx.para_shape_ids.reference(para.para_shape_id);
    ctx.style_ids.reference(para.style_id as u16);
    for cs_ref in &para.char_shapes {
        ctx.char_shape_ids.reference(cs_ref.char_shape_id);
    }

    let pi_str = id.to_string();
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
    write_paragraph_runs(w, para, None, |w, control| {
        write_cell_control_run(w, control, ctx, first_cs)
    })?;

    write_cell_linesegs(w, &para.line_segs)?;

    end_tag(w, "hp:p")?;
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

/// 문단의 run 을 글자 위치 순서로 쓴다 — 글자·필드 표지·인라인 컨트롤을 char_offsets 에 맞춰 섞는다.
///
/// `write_control` 은 컨트롤 하나를 run 째로 쓴다(쓰지 않는 컨트롤이면 아무것도 쓰지 않는다).
/// `skip` 컨트롤은 호출자가 다른 자리에 이미 썼다(구역 첫 문단의 단 정의).
/// 글자 위치를 모르는 합성 문단은 옛 순서(글자 → 컨트롤)를 쓴다.
pub(super) fn write_paragraph_runs<W, F>(
    w: &mut Writer<W>,
    para: &Paragraph,
    skip: Option<usize>,
    mut write_control: F,
) -> Result<(), SerializeError>
where
    W: Write,
    F: FnMut(&mut Writer<W>, &Control) -> Result<(), SerializeError>,
{
    match inline_control_positions(para) {
        Some(positions) => {
            let placed: Vec<(usize, usize)> = positions
                .into_iter()
                .filter(|&(_, ci)| Some(ci) != skip)
                .collect();
            write_text_runs(w, para, &placed, &mut write_control)
        }
        None => {
            write_text_runs(w, para, &[], &mut write_control)?;
            for (ci, control) in para.controls.iter().enumerate() {
                if Some(ci) != skip && !matches!(control, Control::Field(_)) {
                    write_control(w, control)?;
                }
            }
            Ok(())
        }
    }
}

/// 필드가 아닌 컨트롤마다 (들어갈 글자 인덱스, 컨트롤 인덱스) — 컨트롤은 그 글자 앞에 놓인다.
///
/// char_offsets 의 8 단위 빈칸이 컨트롤 자리다. 같은 빈칸의 필드 시작·끝 표지는 field_ranges 로 빼고
/// 남은 자리를 컨트롤 순서대로 채운다. 자리가 모자라면 남은 컨트롤은 문단 끝에 둔다.
/// HWPX 파서는 필드가 아닌 컨트롤을 모두 문단 앞머리 빈칸에 두므로 그런 문단은 «컨트롤 → 글자» 가 된다.
/// 글자가 없으면 모든 컨트롤이 0 이고, 글자 수와 char_offsets 수가 다르면(합성 문단) `None` 이다.
fn inline_control_positions(para: &Paragraph) -> Option<Vec<(usize, usize)>> {
    let chars: Vec<char> = para.text.chars().collect();
    let mut pending = para
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| !matches!(c, Control::Field(_)))
        .map(|(ci, _)| ci);
    if chars.is_empty() {
        return Some(pending.map(|ci| (0, ci)).collect());
    }
    if para.char_offsets.len() != chars.len() {
        return None;
    }
    let mut placed = Vec::new();
    let mut previous_end = 0u32;
    for (idx, (&ch, &offset)) in chars.iter().zip(&para.char_offsets).enumerate() {
        let slots = (offset.saturating_sub(previous_end) / 8) as usize;
        let field_marks = para
            .field_ranges
            .iter()
            .map(|r| usize::from(r.start_char_idx == idx) + usize::from(r.end_char_idx == idx))
            .sum::<usize>();
        for ci in pending.by_ref().take(slots.saturating_sub(field_marks)) {
            placed.push((idx, ci));
        }
        previous_end = offset + char_utf16_len(ch);
    }
    placed.extend(pending.map(|ci| (chars.len(), ci)));
    Some(placed)
}

/// 글자 인덱스로 run 을 나누고, 같은 위치의 필드·컨트롤은 원래 순서로 합친다.
fn write_text_runs<W, F>(
    w: &mut Writer<W>,
    para: &Paragraph,
    placed: &[(usize, usize)],
    write_control: &mut F,
) -> Result<(), SerializeError>
where
    W: Write,
    F: FnMut(&mut Writer<W>, &Control) -> Result<(), SerializeError>,
{
    let chars: Vec<char> = para.text.chars().collect();
    for range in &para.field_ranges {
        if range.start_char_idx > range.end_char_idx
            || range.end_char_idx > chars.len()
            || !matches!(
                para.controls.get(range.control_idx),
                Some(Control::Field(_))
            )
        {
            return Err(SerializeError::XmlError("잘못된 field range".into()));
        }
    }
    for (ci, ctrl) in para.controls.iter().enumerate() {
        if matches!(ctrl, Control::Field(_))
            && !para.field_ranges.iter().any(|r| r.control_idx == ci)
        {
            return Err(SerializeError::XmlError("위치 없는 field 컨트롤".into()));
        }
    }
    let mut events = Vec::with_capacity(para.field_ranges.len() * 2);
    let mut binary_ranges = Vec::new();
    let mut hwpx_end_orders = Vec::new();
    for range in &para.field_ranges {
        let Some(Control::Field(field)) = para.controls.get(range.control_idx) else {
            return Err(SerializeError::XmlError("field 컨트롤 누락".into()));
        };
        if let Some(h) = &field.hwpx {
            events.push((
                range.start_char_idx, h.begin_order, range.control_idx, Some(true),
            ));
            events.push((
                range.end_char_idx, h.end_order, range.control_idx, Some(false),
            ));
            hwpx_end_orders.push(h.end_order);
        } else {
            binary_ranges.push(range);
        }
    }
    let mut boundaries: Vec<_> = binary_ranges
        .iter()
        .flat_map(|r| [r.start_char_idx, r.end_char_idx])
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();
    for &pos in &boundaries {
        let mut starts: Vec<_> = binary_ranges
            .iter()
            .filter(|r| r.start_char_idx == pos)
            .map(|r| (r.control_idx, Some(true)))
            .chain(
                placed.iter().filter(|&&(p, _)| p == pos).map(|&(_, ci)| (ci, None)),
            )
            .collect();
        // Binary controls retain begin order; field_ranges retain end order.
        starts.sort_by_key(|&(ci, _)| ci);
        let mut starts = starts.into_iter().peekable();
        let mut order = 0;
        for end in binary_ranges.iter().filter(|r| r.end_char_idx == pos) {
            while starts
                .peek()
                .is_some_and(|&(ci, _)| ci <= end.control_idx)
            {
                if let Some((ci, begin)) = starts.next() {
                    events.push((pos, order, ci, begin));
                }
                order += 1;
            }
            events.push((pos, order, end.control_idx, Some(false)));
            order += 1;
        }
        for (ci, begin) in starts {
            events.push((pos, order, ci, begin));
            order += 1;
        }
    }
    hwpx_end_orders.sort_unstable();
    for &(pos, ci) in placed.iter().filter(|(pos, _)| !boundaries.contains(pos)) {
        let mut order = ci;
        for &end in &hwpx_end_orders {
            if end <= order {
                order += 1;
            }
        }
        events.push((pos, order, ci, None));
    }
    events.sort_by_key(|(pos, order, _, _)| (*pos, *order));
    let mut current_shape = None;
    let mut current_text = String::new();
    let mut fallback_utf16_pos = 0;
    for idx in 0..=chars.len() {
        if events.iter().any(|(pos, _, _, _)| *pos == idx) {
            if !current_text.is_empty() {
                write_cell_text_run(w, current_shape.unwrap_or(0), &current_text)?;
                current_text.clear();
            }
            for &(_, _, ci, begin) in events.iter().filter(|(pos, _, _, _)| *pos == idx) {
                match begin {
                    Some(begin) => write_field_run(w, para, ci, begin, fallback_utf16_pos)?,
                    None => write_control(w, &para.controls[ci])?,
                }
            }
        }
        let Some(&ch) = chars.get(idx) else {
            break;
        };
        let utf16_pos = para
            .char_offsets
            .get(idx)
            .copied()
            .unwrap_or(fallback_utf16_pos);
        let shape_id = active_char_shape_id(&para.char_shapes, utf16_pos);
        fallback_utf16_pos = utf16_pos + char_utf16_len(ch);
        if current_shape.is_some_and(|id| id != shape_id) && !current_text.is_empty() {
            write_cell_text_run(w, current_shape.unwrap_or(0), &current_text)?;
            current_text.clear();
        }
        current_shape = Some(shape_id);
        current_text.push(ch);
    }
    if !current_text.is_empty() || (chars.is_empty() && para.field_ranges.is_empty()) {
        write_cell_text_run(
            w,
            current_shape.unwrap_or_else(|| first_char_shape_id(&para.char_shapes)),
            &current_text,
        )?;
    }
    Ok(())
}

fn write_field_run<W: Write>(
    w: &mut Writer<W>,
    para: &Paragraph,
    control_idx: usize,
    begin: bool,
    utf16_pos: u32,
) -> Result<(), SerializeError> {
    let Some(Control::Field(field)) = para.controls.get(control_idx) else {
        return Err(SerializeError::XmlError(
            "field range의 컨트롤이 Field가 아님".into(),
        ));
    };
    let shape = field
        .hwpx
        .as_ref()
        .map(|h| {
            if begin {
                h.begin_char_shape_id
            } else {
                h.end_char_shape_id
            }
        })
        .unwrap_or_else(|| active_char_shape_id(&para.char_shapes, utf16_pos))
        .to_string();
    start_tag_attrs(w, "hp:run", &[("charPrIDRef", &shape)])?;
    start_tag(w, "hp:ctrl")?;
    if begin {
        write_field_begin(w, field)?;
    } else {
        write_field_end_for_field(w, field)?;
    }
    end_tag(w, "hp:ctrl")?;
    end_tag(w, "hp:run")
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

fn first_char_shape_id(char_shapes: &[CharShapeRef]) -> u32 {
    char_shapes.first().map(|r| r.char_shape_id).unwrap_or(0)
}

fn active_char_shape_id(char_shapes: &[CharShapeRef], utf16_pos: u32) -> u32 {
    let mut active_id = first_char_shape_id(char_shapes);
    for shape in char_shapes {
        if shape.start_pos <= utf16_pos {
            active_id = shape.char_shape_id;
        } else {
            break;
        }
    }
    active_id
}

fn char_utf16_len(ch: char) -> u32 {
    if ch == '\t' {
        8
    } else {
        ch.len_utf16() as u32
    }
}

fn write_cell_text<W: Write>(w: &mut Writer<W>, text: &str) -> Result<(), SerializeError> {
    use super::utils::text as write_text;
    start_tag(w, "hp:t")?;
    let mut pending = String::new();
    for ch in text.chars() {
        match ch {
            '\t' | '\n' => {
                if !pending.is_empty() {
                    write_text(w, &pending)?;
                    pending.clear();
                }
                if ch == '\n' {
                    empty_tag(w, "hp:lineBreak", &[])?;
                } else {
                    empty_tag(
                        w,
                        "hp:tab",
                        &[("width", "4000"), ("leader", "0"), ("type", "1")],
                    )?;
                }
            }
            c if c < ' ' => {
                return Err(SerializeError::XmlError(format!(
                    "XML 1.0에서 허용하지 않는 문자: U+{:04X}",
                    u32::from(c)
                )))
            }
            c => pending.push(c),
        }
    }
    if !pending.is_empty() {
        write_text(w, &pending)?;
    }
    end_tag(w, "hp:t")
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

    let mut char_shape_id = char_shape_id;
    if let Control::Table(table) = control {
        if let Some(id) = table.hwpx_owner_char_shape_id {
            ctx.char_shape_ids.reference(id);
            char_shape_id = id;
        }
    }
    let cs_str = char_shape_id.to_string();
    start_tag_attrs(w, "hp:run", &[("charPrIDRef", &cs_str)])?;
    write_cell_control(w, control, ctx)?;
    end_tag(w, "hp:run")?;
    Ok(())
}

fn control_supported(control: &Control) -> bool {
    match control {
        Control::Table(_) | Control::Picture(_) | Control::Form(_) | Control::ColumnDef(_) => true,
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
            let _ = write_cell_shape(w, shape, false, ctx)?;
            Ok(())
        }
        Control::Form(form) => write_form(w, form),
        Control::ColumnDef(column) => write_column_def(w, column),
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

/// `in_group` 이면 묶음 안 자식이다.
fn write_cell_shape<W: Write>(
    w: &mut Writer<W>,
    shape: &ShapeObject,
    in_group: bool,
    ctx: &mut SerializeContext,
) -> Result<bool, SerializeError> {
    match shape {
        ShapeObject::Line(line) => {
            write_line(w, line, in_group, ctx)?;
            Ok(true)
        }
        ShapeObject::Rectangle(rect) => {
            write_rect(w, rect, in_group, ctx)?;
            Ok(true)
        }
        ShapeObject::Group(group) => {
            write_container_open(w, group)?;
            for child in &group.children {
                let _ = write_cell_shape(w, child, true, ctx)?;
            }
            write_container_close(w, group, in_group, ctx)?;
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

fn text_flow_str(f: TextFlow) -> &'static str {
    match f {
        TextFlow::BothSides => "BOTH_SIDES",
        TextFlow::LeftOnly => "LEFT_ONLY",
        TextFlow::RightOnly => "RIGHT_ONLY",
        TextFlow::LargestOnly => "LARGEST_ONLY",
    }
}

/// 파서 `parse_table_page_break` 의 역. 한컴은 `CellBreak`(HWP5 1)를 `TABLE`,
/// `RowBreak`(HWP5 2)를 `CELL` 로 저장한다. `ROW` 별칭은 `RowBreak` 로 읽혀 `CELL` 로 나간다.
fn table_page_break_str(pb: TablePageBreak) -> &'static str {
    match pb {
        TablePageBreak::None => "NONE",
        TablePageBreak::CellBreak => "TABLE",
        TablePageBreak::RowBreak => "CELL",
    }
}

/// 너비 기준 → `widthRelTo`. 파서 `parse_size_criterion(_, true)` 의 역.
fn width_criterion_str(c: SizeCriterion) -> &'static str {
    match c {
        SizeCriterion::Paper => "PAPER",
        SizeCriterion::Page => "PAGE",
        SizeCriterion::Column => "COLUMN",
        SizeCriterion::Para => "PARA",
        SizeCriterion::Absolute => "ABSOLUTE",
    }
}

/// 높이 기준 → `heightRelTo`. 높이 기준은 종이·쪽·절대 세 값뿐이라
/// (파서 `parse_size_criterion(_, false)`, HWP5 2비트) 나머지는 `ABSOLUTE` 로 접는다.
fn height_criterion_str(c: SizeCriterion) -> &'static str {
    match c {
        SizeCriterion::Paper => "PAPER",
        SizeCriterion::Page => "PAGE",
        SizeCriterion::Column | SizeCriterion::Para | SizeCriterion::Absolute => "ABSOLUTE",
    }
}

fn cell_line_wrap_str(line_wrap: u8) -> &'static str {
    match line_wrap {
        CELL_LINE_WRAP_SQUEEZE => "SQUEEZE",
        CELL_LINE_WRAP_KEEP => "KEEP",
        _ => "BREAK",
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

    // ---------- v353: 파싱한 표 속성이 저장에서 살아남는지 (HWPX → IR → HWPX) ----------
    //
    // 기대값은 아래 입력 XML 리터럴이다. 직렬화기가 만든 값으로 기대값을 만들지 않는다.

    /// 속성을 바꿔 끼울 기준 표. 1행 2열이고 모든 플래그가 꺼져 있다.
    const BASE_TBL: &str = concat!(
        r#"<hp:tbl id="7" zOrder="0" numberingType="TABLE" textWrap="TOP_AND_BOTTOM" textFlow="BOTH_SIDES" lock="0" dropcapstyle="None" pageBreak="NONE" repeatHeader="1" rowCnt="1" colCnt="2" cellSpacing="0" borderFillIDRef="1" noAdjust="0">"#,
        r#"<hp:sz width="2000" widthRelTo="ABSOLUTE" height="1000" heightRelTo="ABSOLUTE" protect="0"/>"#,
        r#"<hp:pos treatAsChar="1" affectLSpacing="0" flowWithText="1" allowOverlap="0" holdAnchorAndSO="0" vertRelTo="PARA" horzRelTo="PARA" vertAlign="TOP" horzAlign="LEFT" vertOffset="0" horzOffset="0"/>"#,
        r#"<hp:outMargin left="0" right="0" top="0" bottom="0"/>"#,
        r#"<hp:inMargin left="0" right="0" top="0" bottom="0"/>"#,
        r#"<hp:tr>"#,
        r#"<hp:tc name="" header="0" hasMargin="0" protect="0" editable="0" dirty="0" borderFillIDRef="1">"#,
        r#"<hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="CENTER" linkListIDRef="0" linkListNextIDRef="0" textWidth="0" textHeight="0" hasTextRef="0" hasNumRef="0">"#,
        r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0"><hp:t>A</hp:t></hp:run></hp:p>"#,
        r#"</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="1000"/><hp:cellMargin left="0" right="0" top="0" bottom="0"/></hp:tc>"#,
        r#"<hp:tc name="" header="0" hasMargin="0" protect="0" editable="0" dirty="0" borderFillIDRef="1">"#,
        r#"<hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="CENTER" linkListIDRef="0" linkListNextIDRef="0" textWidth="0" textHeight="0" hasTextRef="0" hasNumRef="0">"#,
        r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0"><hp:t>B</hp:t></hp:run></hp:p>"#,
        r#"</hp:subList><hp:cellAddr colAddr="1" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="1000"/><hp:cellMargin left="0" right="0" top="0" bottom="0"/></hp:tc>"#,
        r#"</hp:tr></hp:tbl>"#,
    );

    /// `BASE_TBL` 의 `from` 들을 `to` 로 바꾼 표를 HWPX 구역으로 파싱한 뒤 다시 직렬화한다.
    fn roundtrip_with(edits: &[(&str, &str)]) -> String {
        let mut tbl = BASE_TBL.to_string();
        for (from, to) in edits {
            assert!(tbl.contains(from), "기준 표에 {from} 이 없다");
            tbl = tbl.replacen(from, to, 1);
        }
        let section_xml = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
                r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">"#,
                r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0">{}</hp:run></hp:p>"#,
                r#"</hs:sec>"#
            ),
            tbl
        );
        let section = crate::parser::hwpx::section::parse_hwpx_section(&section_xml)
            .expect("구역 파싱");
        let table = section
            .paragraphs
            .iter()
            .flat_map(|p| p.controls.iter())
            .find_map(|c| match c {
                Control::Table(t) => Some(t.as_ref()),
                _ => None,
            })
            .expect("파싱한 구역에 표가 있어야 한다");
        serialize(table)
    }

    /// 직렬화 결과에서 n번째 `<hp:{element} ...>` 의 속성 값을 읽는다.
    fn attr_value(xml: &str, element: &str, nth: usize, name: &str) -> String {
        let open = format!("<hp:{element} ");
        let start = xml
            .match_indices(&open)
            .nth(nth)
            .unwrap_or_else(|| panic!("{open} #{nth} 없음: {xml}"))
            .0;
        let tag = &xml[start..start + xml[start..].find('>').expect("태그 끝")];
        let key = format!(" {name}=\"");
        let at = tag
            .find(&key)
            .unwrap_or_else(|| panic!("{name} 없음: {tag}"))
            + key.len();
        tag[at..at + tag[at..].find('"').expect("값 끝")].to_string()
    }

    #[test]
    fn hwpx_table_no_adjust_survives_save() {
        let xml = roundtrip_with(&[(r#"noAdjust="0""#, r#"noAdjust="1""#)]);
        assert_eq!(attr_value(&xml, "tbl", 0, "noAdjust"), "1");
        assert_eq!(attr_value(&roundtrip_with(&[]), "tbl", 0, "noAdjust"), "0");
    }

    #[test]
    fn hwpx_table_no_adjust_keeps_page_break_and_repeat_header() {
        // noAdjust 가 HWPTAG_TABLE raw 속성을 채워도 쪽 나눔·제목 줄 반복이 그대로 나가야 한다.
        let xml = roundtrip_with(&[
            (r#"noAdjust="0""#, r#"noAdjust="1""#),
            (r#"pageBreak="NONE""#, r#"pageBreak="TABLE""#),
        ]);
        assert_eq!(attr_value(&xml, "tbl", 0, "pageBreak"), "TABLE");
        assert_eq!(attr_value(&xml, "tbl", 0, "repeatHeader"), "1");
        assert_eq!(attr_value(&xml, "tbl", 0, "noAdjust"), "1");
    }

    #[test]
    fn hwpx_table_lock_survives_save() {
        let xml = roundtrip_with(&[(r#"lock="0""#, r#"lock="1""#)]);
        assert_eq!(attr_value(&xml, "tbl", 0, "lock"), "1");
        assert_eq!(attr_value(&roundtrip_with(&[]), "tbl", 0, "lock"), "0");
    }

    #[test]
    fn hwpx_table_text_flow_survives_save() {
        for value in ["BOTH_SIDES", "LEFT_ONLY", "RIGHT_ONLY", "LARGEST_ONLY"] {
            let to = format!(r#"textFlow="{value}""#);
            let xml = roundtrip_with(&[(r#"textFlow="BOTH_SIDES""#, &to)]);
            assert_eq!(attr_value(&xml, "tbl", 0, "textFlow"), value);
        }
    }

    #[test]
    fn hwpx_table_text_wrap_tight_and_through_survive_save() {
        for value in ["TIGHT", "THROUGH", "SQUARE", "BEHIND_TEXT", "IN_FRONT_OF_TEXT"] {
            let to = format!(r#"textWrap="{value}""#);
            let xml = roundtrip_with(&[(r#"textWrap="TOP_AND_BOTTOM""#, &to)]);
            assert_eq!(attr_value(&xml, "tbl", 0, "textWrap"), value);
        }
    }

    #[test]
    fn hwpx_table_page_break_strings_survive_save() {
        for value in ["TABLE", "CELL", "NONE"] {
            let to = format!(r#"pageBreak="{value}""#);
            let xml = roundtrip_with(&[(r#"pageBreak="NONE""#, &to)]);
            assert_eq!(attr_value(&xml, "tbl", 0, "pageBreak"), value);
        }
        // ROW 는 한컴 값이 아닌 별칭이다. RowBreak 로 읽혀 CELL 로 정규화된다 (Parent 처분 2).
        let xml = roundtrip_with(&[(r#"pageBreak="NONE""#, r#"pageBreak="ROW""#)]);
        assert_eq!(attr_value(&xml, "tbl", 0, "pageBreak"), "CELL");
    }

    #[test]
    fn table_page_break_str_matches_hancom_mapping() {
        assert_eq!(table_page_break_str(TablePageBreak::CellBreak), "TABLE");
        assert_eq!(table_page_break_str(TablePageBreak::RowBreak), "CELL");
        assert_eq!(table_page_break_str(TablePageBreak::None), "NONE");
    }

    #[test]
    fn hwpx_table_size_protect_survives_save() {
        let xml = roundtrip_with(&[(
            r#"heightRelTo="ABSOLUTE" protect="0""#,
            r#"heightRelTo="ABSOLUTE" protect="1""#,
        )]);
        assert_eq!(attr_value(&xml, "sz", 0, "protect"), "1");
        assert_eq!(attr_value(&roundtrip_with(&[]), "sz", 0, "protect"), "0");
    }

    #[test]
    fn hwpx_table_size_criteria_survive_save() {
        let xml = roundtrip_with(&[
            (r#"widthRelTo="ABSOLUTE""#, r#"widthRelTo="PARA""#),
            (r#"heightRelTo="ABSOLUTE""#, r#"heightRelTo="PAGE""#),
        ]);
        assert_eq!(attr_value(&xml, "sz", 0, "widthRelTo"), "PARA");
        assert_eq!(attr_value(&xml, "sz", 0, "heightRelTo"), "PAGE");
        let xml = roundtrip_with(&[(r#"widthRelTo="ABSOLUTE""#, r#"widthRelTo="COLUMN""#)]);
        assert_eq!(attr_value(&xml, "sz", 0, "widthRelTo"), "COLUMN");
    }

    #[test]
    fn hwpx_table_allow_overlap_survives_save() {
        let xml = roundtrip_with(&[(r#"allowOverlap="0""#, r#"allowOverlap="1""#)]);
        assert_eq!(attr_value(&xml, "pos", 0, "allowOverlap"), "1");
        assert_eq!(attr_value(&roundtrip_with(&[]), "pos", 0, "allowOverlap"), "0");
    }

    #[test]
    fn hwpx_table_flow_with_text_survives_save() {
        let xml = roundtrip_with(&[(r#"flowWithText="1""#, r#"flowWithText="0""#)]);
        assert_eq!(attr_value(&xml, "pos", 0, "flowWithText"), "0");
        assert_eq!(attr_value(&roundtrip_with(&[]), "pos", 0, "flowWithText"), "1");
    }

    #[test]
    fn hwpx_table_affect_line_spacing_survives_save() {
        let xml = roundtrip_with(&[(r#"affectLSpacing="0""#, r#"affectLSpacing="1""#)]);
        assert_eq!(attr_value(&xml, "pos", 0, "affectLSpacing"), "1");
    }

    #[test]
    fn hwpx_table_hold_anchor_survives_save() {
        let xml = roundtrip_with(&[(r#"holdAnchorAndSO="0""#, r#"holdAnchorAndSO="1""#)]);
        assert_eq!(attr_value(&xml, "pos", 0, "holdAnchorAndSO"), "1");
        assert_eq!(attr_value(&roundtrip_with(&[]), "pos", 0, "holdAnchorAndSO"), "0");
    }

    #[test]
    fn hwpx_table_negative_offsets_survive_save() {
        let xml = roundtrip_with(&[(
            r#"vertOffset="0" horzOffset="0""#,
            r#"vertOffset="-120" horzOffset="-340""#,
        )]);
        assert_eq!(attr_value(&xml, "pos", 0, "vertOffset"), "-120");
        assert_eq!(attr_value(&xml, "pos", 0, "horzOffset"), "-340");
    }

    #[test]
    fn hwpx_cell_line_wrap_squeeze_survives_save() {
        let xml = roundtrip_with(&[(r#"lineWrap="BREAK""#, r#"lineWrap="SQUEEZE""#)]);
        assert_eq!(attr_value(&xml, "subList", 0, "lineWrap"), "SQUEEZE");
        // 두 번째 셀은 손대지 않았으므로 BREAK 그대로여야 한다 (셀 단위 보존).
        assert_eq!(attr_value(&xml, "subList", 1, "lineWrap"), "BREAK");
        let xml = roundtrip_with(&[(r#"lineWrap="BREAK""#, r#"lineWrap="KEEP""#)]);
        assert_eq!(attr_value(&xml, "subList", 0, "lineWrap"), "KEEP");
    }

    #[test]
    fn hwpx_cell_protect_survives_save() {
        let xml = roundtrip_with(&[(r#"protect="0" editable"#, r#"protect="1" editable"#)]);
        assert_eq!(attr_value(&xml, "tc", 0, "protect"), "1");
        assert_eq!(attr_value(&xml, "tc", 1, "protect"), "0");
    }

    #[test]
    fn hwpx_cell_editable_and_dirty_survive_save() {
        let xml = roundtrip_with(&[(
            r#"editable="0" dirty="0""#,
            r#"editable="1" dirty="1""#,
        )]);
        assert_eq!(attr_value(&xml, "tc", 0, "editable"), "1");
        assert_eq!(attr_value(&xml, "tc", 0, "dirty"), "1");
        assert_eq!(attr_value(&xml, "tc", 1, "editable"), "0");
        assert_eq!(attr_value(&xml, "tc", 1, "dirty"), "0");
    }

    #[test]
    fn hwpx_table_caption_survives_save() {
        let caption = concat!(
            r#"<hp:caption side="BOTTOM" fullSz="0" width="8504" gap="425" lastWidth="2000">"#,
            r#"<hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="TOP" linkListIDRef="0" linkListNextIDRef="0" textWidth="0" textHeight="0" hasTextRef="0" hasNumRef="0">"#,
            r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0"><hp:t>캡션</hp:t></hp:run></hp:p>"#,
            r#"</hp:subList></hp:caption>"#,
        );
        let with_caption = format!(r#"<hp:outMargin left="0" right="0" top="0" bottom="0"/>{caption}"#);
        let xml = roundtrip_with(&[(
            r#"<hp:outMargin left="0" right="0" top="0" bottom="0"/>"#,
            &with_caption,
        )]);
        assert_eq!(attr_value(&xml, "caption", 0, "side"), "BOTTOM");
        assert_eq!(attr_value(&xml, "caption", 0, "width"), "8504");
        assert_eq!(attr_value(&xml, "caption", 0, "gap"), "425");
        assert_eq!(attr_value(&xml, "caption", 0, "lastWidth"), "2000");
        assert!(xml.contains("<hp:t>캡션</hp:t>"), "{xml}");
        // 자식 순서: outMargin → caption → inMargin
        let out = xml.find("<hp:outMargin ").unwrap();
        let cap = xml.find("<hp:caption ").unwrap();
        let inm = xml.find("<hp:inMargin ").unwrap();
        assert!(out < cap && cap < inm, "{xml}");
    }

    #[test]
    fn hwpx_table_cellzone_survives_save() {
        let zone = r#"<hp:cellzoneList><hp:cellzone startRowAddr="0" startColAddr="1" endRowAddr="0" endColAddr="1" borderFillIDRef="1"/></hp:cellzoneList>"#;
        let with_zone = format!(r#"<hp:inMargin left="0" right="0" top="0" bottom="0"/>{zone}"#);
        let xml = roundtrip_with(&[(
            r#"<hp:inMargin left="0" right="0" top="0" bottom="0"/>"#,
            &with_zone,
        )]);
        assert!(xml.contains(zone), "{xml}");
        // 자식 순서: inMargin → cellzoneList → tr
        let inm = xml.find("<hp:inMargin ").unwrap();
        let zl = xml.find("<hp:cellzoneList>").unwrap();
        let tr = xml.find("<hp:tr>").unwrap();
        assert!(inm < zl && zl < tr, "{xml}");
    }

    #[test]
    fn table_without_caption_or_zones_emits_neither() {
        let xml = roundtrip_with(&[]);
        assert!(!xml.contains("<hp:caption"), "{xml}");
        assert!(!xml.contains("<hp:cellzoneList"), "{xml}");
    }
}
