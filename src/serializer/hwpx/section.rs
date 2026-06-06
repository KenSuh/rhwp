//! Contents/section{N}.xml — Section 본문 직렬화
//!
//! Stage 2 (#182): 기존 템플릿 기반 구조를 유지하되, `<hp:p>` 와 `<hp:run>` 의 속성을
//! IR에서 가져와 동적으로 생성한다. `secPr`/`pagePr`/`grid` 등 섹션 정의는 템플릿 보존
//! (IR에 대응 필드가 더 담길 때까지 점진적으로 동적화 예정).
//!
//! Stage #177 (2026-04-18): `<hp:lineseg>` 직렬화를 IR 기반으로 전환.
//! `Paragraph.line_segs` 의 6개 필드(line_height, text_height, baseline_distance,
//! line_spacing, column_start/segment_width, tag)를 그대로 출력하여 **원본 lineseg 값
//! 보존**. rhwp 는 자신의 문서에서 새로 부정확한 값을 생산하지 않는다.
//!
//! IR 매핑 관행:
//!   - `section.paragraphs` 여러 개 = 하드 문단 경계 (`<hp:p>` 여러 개)
//!   - `paragraph.text` 내 `\n` = 소프트 라인브레이크 (`<hp:lineBreak/>`, 같은 문단 내)
//!   - `paragraph.text` 내 `\t` = 탭 (`<hp:tab width=... leader="0" type="1"/>`)
//!   - `paragraph.para_shape_id` → `<hp:p paraPrIDRef>`
//!   - `paragraph.style_id` → `<hp:p styleIDRef>`
//!   - `paragraph.column_type` → `<hp:p pageBreak/columnBreak>`
//!   - `paragraph.char_shapes[0].char_shape_id` → 첫 `<hp:run charPrIDRef>`
//!   - `paragraph.line_segs[i]` → 각 `<hp:lineseg>` 속성 (6개 필드 그대로 출력)

use quick_xml::Writer;

use crate::model::control::{Control, PageHide, PageNumberPos};
use crate::model::document::{Document, Section};
use crate::model::page::PageDef;
use crate::model::paragraph::{ColumnBreakType, LineSeg, Paragraph};
use crate::model::shape::ShapeObject;

use super::context::SerializeContext;
use super::form::write_form;
use super::picture::write_picture;
use super::shape::{write_container_close, write_container_open, write_line, write_rect};
use super::table::write_table;
use super::utils::{empty_tag, end_tag, start_tag, xml_escape};
use super::SerializeError;

const EMPTY_SECTION_XML: &str = include_str!("templates/empty_section0.xml");
const TEXT_SLOT: &str = "<hp:t/>";
const LINESEG_SLOT_OPEN: &str = "<hp:linesegarray>";
const LINESEG_SLOT_CLOSE: &str = "</hp:linesegarray>";
const PARA_CLOSE: &str = "</hp:p></hs:sec>";

// 템플릿 내 첫 <hp:p> 태그의 실제 문자열 (id="3121190098" 랜덤 해시 포함).
// 템플릿은 정적이므로 이 문자열이 고정 위치에 있음이 보장됨.
const TEMPLATE_FIRST_P_TAG: &str = r#"<hp:p id="3121190098" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0">"#;
// 템플릿 내 고정 pagePr(용지 크기/여백) 문자열. write_section 에서 IR PageDef 기반으로 교체한다.
const TEMPLATE_PAGE_PR: &str = r#"<hp:pagePr landscape="WIDELY" width="59528" height="84186" gutterType="LEFT_ONLY"><hp:margin header="4252" footer="4252" gutter="0" left="8504" right="8504" top="5668" bottom="4252"/></hp:pagePr>"#;
// 템플릿 내 <hp:run charPrIDRef="0"> 직후에 TEXT_SLOT 이 오는 패턴.
const TEMPLATE_RUN_BEFORE_TEXT: &str = r#"<hp:run charPrIDRef="0"><hp:t/>"#;

/// 레퍼런스 기준 줄 레이아웃 파라미터.
const VERT_STEP: u32 = 1600; // vertsize(1000) + spacing(600)
const LINE_FLAGS: u32 = 393216;
const HORZ_SIZE: u32 = 42520;
/// 탭 기본 폭 (한컴이 열면서 재계산하지만 초기값으로 필요).
const TAB_DEFAULT_WIDTH: u32 = 4000;

/// Stage 2 진입점. `ctx` 는 Stage 3+ 에서 파라미터 검증에 사용.
pub fn write_section(
    section: &Section,
    _doc: &Document,
    _index: usize,
    ctx: &mut SerializeContext,
) -> Result<Vec<u8>, SerializeError> {
    let mut vert_cursor: u32 = 0;

    let first_para = section.paragraphs.first();
    let (first_t, first_linesegs, first_advance) = match first_para {
        Some(p) => render_paragraph_parts(p, vert_cursor),
        None => render_paragraph_parts_for_text("", vert_cursor),
    };
    vert_cursor = first_advance;

    let mut out = EMPTY_SECTION_XML.replacen(TEXT_SLOT, &first_t, 1);
    out = replace_first_linesegs(&out, &first_linesegs);

    // secPr 의 pagePr(용지 크기/여백)를 IR PageDef 에서 동적 생성한다. 기존엔 empty_section0
    // 템플릿의 고정 여백(top=5668 등)이 그대로 나가, 저장→재로드 시 원본 여백을 잃고 본문
    // 영역이 바뀌어 페이지가 재배치(reflow)되는 회귀가 있었다. width/height 가 0(미파싱
    // PageDef)이면 템플릿 기본값을 유지한다.
    //
    // fail-closed: 템플릿 pagePr 앵커가 (whitespace/속성순서/기본값 변경 등으로) 안 맞으면
    // replacen 이 조용히 no-op 해 IR 여백을 잃고도 Ok 를 반환하는 silent corruption 이 된다.
    // 그래서 PageDef 를 써야 하는데 앵커가 없으면 에러로 실패시킨다(테스트/런타임에서 즉시 발각).
    let page_def = &section.section_def.page_def;
    if page_def.width > 0 && page_def.height > 0 {
        if !out.contains(TEMPLATE_PAGE_PR) {
            return Err(SerializeError::XmlError(
                "secPr 템플릿 pagePr 앵커를 찾지 못해 IR PageDef 용지 크기/여백을 직렬화할 수 없음 \
                 (empty_section0.xml 템플릿 또는 TEMPLATE_PAGE_PR 상수가 어긋남)"
                    .to_string(),
            ));
        }
        out = out.replacen(TEMPLATE_PAGE_PR, &render_page_pr(page_def), 1);
    }

    // 첫 문단 `<hp:p>` 태그를 IR 기반 속성으로 교체
    if let Some(p) = first_para {
        let new_p_tag = render_hp_p_open(p, 0);
        out = out.replacen(TEMPLATE_FIRST_P_TAG, &new_p_tag, 1);

        // 첫 문단의 텍스트용 <hp:run> 의 charPrIDRef 를 IR 기반으로 교체
        // 템플릿에서 TEXT_SLOT 이 있던 자리 바로 앞의 <hp:run charPrIDRef="0"> 패턴.
        let first_run_cs = first_run_char_shape_id(p);
        let new_run = format!(r#"<hp:run charPrIDRef="{}">"#, first_run_cs);
        let replacement = format!("{}{}", new_run, &first_t);
        // 이미 first_t 는 out 에 들어갔으므로 그 직전의 <hp:run charPrIDRef="0"> 만 변경
        let anchor = format!("{}{}", r#"<hp:run charPrIDRef="0">"#, &first_t);
        if out.contains(&anchor) {
            out = out.replacen(&anchor, &replacement, 1);
        }

        let controls_xml = render_controls_xml(p, ctx)?;
        if !controls_xml.is_empty() {
            out = out.replacen(
                "</hp:run><hp:linesegarray>",
                &format!("</hp:run>{}<hp:linesegarray>", controls_xml),
                1,
            );
        }
    }

    // 추가 문단: `</hp:p></hs:sec>` 직전에 `<hp:p>` 요소를 삽입.
    if section.paragraphs.len() > 1 {
        let mut extra = String::new();
        for (idx, p) in section.paragraphs.iter().enumerate().skip(1) {
            let (t, linesegs, advance) = render_paragraph_parts(p, vert_cursor);
            vert_cursor = advance;
            let cs = first_run_char_shape_id(p);
            extra.push_str(&render_hp_p_open(p, idx as u32));
            extra.push_str(&format!(r#"<hp:run charPrIDRef="{}">"#, cs));
            extra.push_str(&t);
            extra.push_str("</hp:run>");
            extra.push_str(&render_controls_xml(p, ctx)?);
            extra.push_str(r#"<hp:linesegarray>"#);
            extra.push_str(&linesegs);
            extra.push_str(r#"</hp:linesegarray></hp:p>"#);
        }
        out = out.replacen(PARA_CLOSE, &format!("</hp:p>{}</hs:sec>", extra), 1);
    }

    Ok(out.into_bytes())
}

/// 섹션 `pagePr`(용지 크기 + 여백)를 IR(PageDef)에서 생성한다. 파서가 다시 읽는 값은
/// width/height 와 margin(left/right/top/bottom/header/footer/gutter) 뿐이므로 그 값을
/// 출력해 저장→재로드 시 페이지 크기/여백/본문 영역이 보존되게 한다.
///
/// HWPX 의 width/height 는 **실제(렌더) 방향**으로 저장하는 규약이고, 파서(parse_page_pr)는
/// landscape 속성을 읽지 않고 swap 도 하지 않는다. 따라서 PageDef.landscape=true(HWP 바이너리
/// 임포트·setPageDef 편집 등 — 렌더러가 width/height 를 교환해 그림)인 경우, 그 교환된 실제
/// 치수를 써야 재로드 후에도 같은 가로 방향으로 렌더된다. landscape=false(HWPX 파싱 결과)면
/// width/height 가 이미 실제 방향이라 그대로 쓴다. (PageAreas::from_page_def 의 swap 규칙과 정합.)
///
/// gutterType 은 LEFT_ONLY 로 고정한다(추적되는 제약). HWPX 파서(parse_page_pr)가 gutterType 을
/// PageDef.binding 으로 되읽지 않아 binding(DuplexSided/TopFlip)은 HWPX↔HWPX 경로에서
/// round-trip 되지 않고 SingleSided 로 정규화된다(landscape 플래그와 동일). 단 제본 여백 '값'
/// (margin_gutter)은 그대로 직렬화돼 본문 영역(레이아웃)에는 영향이 없고, 제본 '변(side)' 표기만
/// 고정된다. binding↔gutterType 매핑(LEFT_RIGHT/TOP_BOTTOM 등)은 코퍼스/파서에 근거가 없어
/// 추측하지 않으며 파서 보완을 별도 작업으로 추적한다
/// (계약 검증: tests/hwpx_roundtrip_integration.rs::binding_gutter_value_preserved_side_normalized).
fn render_page_pr(pd: &PageDef) -> String {
    // landscape 이면 렌더러가 교환하는 실제 치수(height, width)를 HWPX 규약대로 기록한다.
    let (eff_w, eff_h) = if pd.landscape {
        (pd.height, pd.width)
    } else {
        (pd.width, pd.height)
    };
    format!(
        r#"<hp:pagePr landscape="WIDELY" width="{w}" height="{h}" gutterType="LEFT_ONLY"><hp:margin header="{header}" footer="{footer}" gutter="{gutter}" left="{left}" right="{right}" top="{top}" bottom="{bottom}"/></hp:pagePr>"#,
        w = eff_w,
        h = eff_h,
        header = pd.margin_header,
        footer = pd.margin_footer,
        gutter = pd.margin_gutter,
        left = pd.margin_left,
        right = pd.margin_right,
        top = pd.margin_top,
        bottom = pd.margin_bottom,
    )
}

#[cfg(test)]
mod page_pr_tests {
    use super::render_page_pr;
    use crate::model::page::PageDef;

    #[test]
    fn render_page_pr_portrait_writes_raw_dims() {
        let pd = PageDef {
            width: 59528,
            height: 84186,
            margin_top: 2835,
            margin_bottom: 2835,
            margin_header: 4252,
            margin_footer: 4252,
            margin_left: 8504,
            margin_right: 8504,
            margin_gutter: 0,
            landscape: false,
            ..Default::default()
        };
        let xml = render_page_pr(&pd);
        assert!(xml.contains(r#"width="59528""#), "portrait width: {xml}");
        assert!(xml.contains(r#"height="84186""#), "portrait height: {xml}");
        assert!(xml.contains(r#"top="2835""#) && xml.contains(r#"gutter="0""#));
    }

    #[test]
    fn render_page_pr_landscape_writes_effective_swapped_dims() {
        // landscape=true: PageDef.width=짧은변, height=긴변 (렌더러가 교환). HWPX 에는 실제
        // 방향(가로>세로)인 width=긴변, height=짧은변 으로 나가야 재로드 후에도 가로로 렌더된다.
        let pd = PageDef {
            width: 59528,  // 짧은변
            height: 84186, // 긴변
            landscape: true,
            ..Default::default()
        };
        let xml = render_page_pr(&pd);
        assert!(
            xml.contains(r#"width="84186""#),
            "landscape effective width(긴변): {xml}"
        );
        assert!(
            xml.contains(r#"height="59528""#),
            "landscape effective height(짧은변): {xml}"
        );
    }
}

fn render_controls_xml(
    p: &Paragraph,
    ctx: &mut SerializeContext,
) -> Result<String, SerializeError> {
    let mut out = String::new();
    for ctrl in &p.controls {
        let mut writer = Writer::new(Vec::new());
        if write_control_xml(&mut writer, ctrl, ctx)? {
            let bytes = writer.into_inner();
            let xml = String::from_utf8(bytes).map_err(|e| {
                SerializeError::XmlError(format!("control XML UTF-8 변환 실패: {}", e))
            })?;
            out.push_str(&format!(
                r#"<hp:run charPrIDRef="{}">"#,
                first_run_char_shape_id(p)
            ));
            out.push_str(&xml);
            out.push_str("</hp:run>");
        }
    }
    Ok(out)
}

fn write_control_xml(
    writer: &mut Writer<Vec<u8>>,
    ctrl: &Control,
    ctx: &mut SerializeContext,
) -> Result<bool, SerializeError> {
    match ctrl {
        Control::Table(table) => {
            write_table(writer, table, ctx)?;
            Ok(true)
        }
        Control::Picture(pic) => {
            write_picture(writer, pic, ctx)?;
            Ok(true)
        }
        Control::Shape(shape) => write_shape_xml(writer, shape, ctx),
        Control::PageHide(page_hide) => {
            write_page_hide(writer, page_hide)?;
            Ok(true)
        }
        Control::PageNumberPos(page_number_pos) => {
            write_page_number_pos(writer, page_number_pos)?;
            Ok(true)
        }
        Control::Form(form) => {
            write_form(writer, form)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn write_page_hide(
    writer: &mut Writer<Vec<u8>>,
    page_hide: &PageHide,
) -> Result<(), SerializeError> {
    start_tag(writer, "hp:ctrl")?;
    empty_tag(
        writer,
        "hp:pageHiding",
        &[
            ("hideHeader", bool01(page_hide.hide_header)),
            ("hideFooter", bool01(page_hide.hide_footer)),
            ("hideMasterPage", bool01(page_hide.hide_master_page)),
            ("hideBorder", bool01(page_hide.hide_border)),
            ("hideFill", bool01(page_hide.hide_fill)),
            ("hidePageNum", bool01(page_hide.hide_page_num)),
        ],
    )?;
    end_tag(writer, "hp:ctrl")?;
    Ok(())
}

fn write_page_number_pos(
    writer: &mut Writer<Vec<u8>>,
    page_number_pos: &PageNumberPos,
) -> Result<(), SerializeError> {
    let side_char = if page_number_pos.dash_char == '\0' {
        "-".to_string()
    } else {
        page_number_pos.dash_char.to_string()
    };
    start_tag(writer, "hp:ctrl")?;
    empty_tag(
        writer,
        "hp:pageNum",
        &[
            ("pos", page_number_position_str(page_number_pos.position)),
            ("formatType", page_number_format_str(page_number_pos.format)),
            ("sideChar", &side_char),
        ],
    )?;
    end_tag(writer, "hp:ctrl")?;
    Ok(())
}

fn bool01(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn page_number_position_str(position: u8) -> &'static str {
    match position {
        0 => "NONE",
        1 => "TOP_LEFT",
        2 => "TOP_CENTER",
        3 => "TOP_RIGHT",
        4 => "BOTTOM_LEFT",
        5 => "BOTTOM_CENTER",
        6 => "BOTTOM_RIGHT",
        7 => "OUTSIDE_TOP",
        8 => "OUTSIDE_BOTTOM",
        9 => "INSIDE_TOP",
        10 => "INSIDE_BOTTOM",
        _ => "BOTTOM_CENTER",
    }
}

fn page_number_format_str(format: u8) -> &'static str {
    match format {
        0 => "DIGIT",
        1 => "CIRCLE_DIGIT",
        2 => "ROMAN_CAPITAL",
        3 => "ROMAN_SMALL",
        4 => "LATIN_CAPITAL",
        5 => "LATIN_SMALL",
        6 => "HANGUL",
        7 => "HANJA",
        _ => "DIGIT",
    }
}

fn write_shape_xml(
    writer: &mut Writer<Vec<u8>>,
    shape: &ShapeObject,
    ctx: &SerializeContext,
) -> Result<bool, SerializeError> {
    match shape {
        ShapeObject::Line(line) => {
            write_line(writer, line)?;
            Ok(true)
        }
        ShapeObject::Rectangle(rect) => {
            write_rect(writer, rect)?;
            Ok(true)
        }
        ShapeObject::Group(group) => {
            write_container_open(writer, &group.common)?;
            for child in &group.children {
                let _ = write_shape_xml(writer, child, ctx)?;
            }
            write_container_close(writer)?;
            Ok(true)
        }
        ShapeObject::Picture(pic) => {
            write_picture(writer, pic, ctx)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// IR의 Paragraph를 기반으로 `<hp:p>` 시작 태그를 생성.
///
/// `id` 는 문단 순서 기반(0, 1, 2, ...)로 할당한다. 한컴 샘플은 랜덤 해시도 쓰지만
/// 파서는 id 를 무시하므로 순차값으로 충분.
fn render_hp_p_open(p: &Paragraph, id: u32) -> String {
    let page_break = if matches!(p.column_type, ColumnBreakType::Page) {
        1
    } else {
        0
    };
    let column_break = if matches!(p.column_type, ColumnBreakType::Column) {
        1
    } else {
        0
    };
    format!(
        r#"<hp:p id="{}" paraPrIDRef="{}" styleIDRef="{}" pageBreak="{}" columnBreak="{}" merged="0">"#,
        id, p.para_shape_id, p.style_id, page_break, column_break,
    )
}

/// 문단 첫 run 의 charPrIDRef. IR의 `char_shapes[0].char_shape_id` 사용.
/// 비어있으면 0 (기본 글자모양) 반환.
fn first_run_char_shape_id(p: &Paragraph) -> u32 {
    p.char_shapes.first().map(|r| r.char_shape_id).unwrap_or(0)
}

/// Paragraph 하나를 (`<hp:t>` XML, lineseg XML, 다음 vert_cursor)로 변환.
///
/// `<hp:lineseg>` 출력 원칙 (#177):
/// - `para.line_segs` 가 비어있지 않으면 **IR 값 그대로 출력**
/// - 비어있을 때만 텍스트 내 `\n` 기반으로 fallback 생성 (빈 문단·`Document::default()` 호환)
fn render_paragraph_parts(para: &Paragraph, vert_start: u32) -> (String, String, u32) {
    let t_xml = render_hp_t_content(&para.text);

    if !para.line_segs.is_empty() {
        // IR 기반 출력 — 원본 lineseg 값 보존 (#177)
        let linesegs = render_lineseg_array_from_ir(&para.line_segs);
        let vert_end = next_vert_cursor_from_ir(&para.line_segs, vert_start);
        (t_xml, linesegs, vert_end)
    } else {
        // Fallback — IR에 line_segs 가 없으면 기존 생성 로직 유지
        let (linesegs, vert_end) = render_lineseg_array_fallback(&para.text, vert_start);
        (t_xml, linesegs, vert_end)
    }
}

/// IR 없이 텍스트만 있을 때 `<hp:t>` 와 fallback lineseg 생성.
/// `write_section` 이 `first_para == None` 인 경우를 위해 유지.
fn render_paragraph_parts_for_text(text: &str, vert_start: u32) -> (String, String, u32) {
    let t_xml = render_hp_t_content(text);
    let (linesegs, vert_end) = render_lineseg_array_fallback(text, vert_start);
    (t_xml, linesegs, vert_end)
}

/// `<hp:t>...</hp:t>` 본문 생성 — 탭/소프트브레이크/XML escape 포함.
fn render_hp_t_content(text: &str) -> String {
    let mut t_xml = String::from("<hp:t>");
    let mut buf = String::new();
    for c in text.chars() {
        match c {
            '\t' => {
                flush_buf(&mut t_xml, &mut buf);
                t_xml.push_str(&format!(
                    r#"<hp:tab width="{}" leader="0" type="1"/>"#,
                    TAB_DEFAULT_WIDTH
                ));
            }
            '\n' => {
                flush_buf(&mut t_xml, &mut buf);
                t_xml.push_str("<hp:lineBreak/>");
            }
            c if (c as u32) < 0x20 => { /* 기타 제어문자 무시 */ }
            c => buf.push(c),
        }
    }
    flush_buf(&mut t_xml, &mut buf);
    t_xml.push_str("</hp:t>");
    t_xml
}

/// IR의 `line_segs` 를 그대로 XML로 직렬화 (6개 필드 전부 IR 값 사용).
///
/// rhwp 는 자신의 문서에서 비표준 lineseg 를 **새로 생산하지 않는다**.
/// 원본 한컴 파일의 lineseg 값이 파서에 의해 `Paragraph.line_segs` 에 담겼다면,
/// 저장 시 그 값을 훼손 없이 보존한다.
fn render_lineseg_array_from_ir(segs: &[LineSeg]) -> String {
    let mut out = String::new();
    for seg in segs {
        out.push_str(&format!(
            r#"<hp:lineseg textpos="{}" vertpos="{}" vertsize="{}" textheight="{}" baseline="{}" spacing="{}" horzpos="{}" horzsize="{}" flags="{}"/>"#,
            seg.text_start,
            seg.vertical_pos,
            seg.line_height,
            seg.text_height,
            seg.baseline_distance,
            seg.line_spacing,
            seg.column_start,
            seg.segment_width,
            seg.tag,
        ));
    }
    out
}

/// IR 기반 다음 문단의 vert_start 계산 — 마지막 lineseg 의 vpos + lh 사용.
fn next_vert_cursor_from_ir(segs: &[LineSeg], vert_start: u32) -> u32 {
    if let Some(last) = segs.last() {
        // vertical_pos 는 섹션 시작 기준 절대값일 수도, 문단 기준 상대값일 수도 있음.
        // 현재 rhwp 는 섹션 절대값이므로 그대로 + lh 로 다음 커서 산출.
        let next = (last.vertical_pos as i64) + (last.line_height.max(0) as i64);
        if next > vert_start as i64 {
            next as u32
        } else {
            vert_start + VERT_STEP
        }
    } else {
        vert_start + VERT_STEP
    }
}

/// Fallback — IR 에 line_segs 가 없는 경우에만 사용 (예: `Document::default()`).
/// 과거 동작을 보존하기 위해 기존 정적값으로 lineseg 생성.
fn render_lineseg_array_fallback(text: &str, vert_start: u32) -> (String, u32) {
    let mut linesegs = String::new();
    push_lineseg_static(&mut linesegs, 0, vert_start);
    let mut utf16_pos: u32 = 0;
    let mut lines_in_para: u32 = 0;
    for c in text.chars() {
        let u16_len = c.len_utf16() as u32;
        match c {
            '\t' | '\n' => {
                utf16_pos += u16_len;
                if c == '\n' {
                    lines_in_para += 1;
                    push_lineseg_static(
                        &mut linesegs,
                        utf16_pos,
                        vert_start + lines_in_para * VERT_STEP,
                    );
                }
            }
            c if (c as u32) < 0x20 => {}
            _ => utf16_pos += u16_len,
        }
    }
    let vert_end = vert_start + (lines_in_para + 1) * VERT_STEP;
    (linesegs, vert_end)
}

fn flush_buf(t_xml: &mut String, buf: &mut String) {
    if !buf.is_empty() {
        t_xml.push_str(&xml_escape(buf));
        buf.clear();
    }
}

/// Fallback 전용 static lineseg 생성기 — IR에 값이 없을 때만 사용.
/// 주: 이 함수의 출력은 "명세 상 정확한 값" 이 아닌 정적 자리표이므로,
/// 호출 후 문서는 `DocumentCore::from_bytes` 의 `reflow_zero_height_paragraphs`
/// 또는 사용자의 `reflow_linesegs_on_demand` 로 재계산되어야 한다.
fn push_lineseg_static(out: &mut String, textpos: u32, vertpos: u32) {
    out.push_str(&format!(
        r#"<hp:lineseg textpos="{}" vertpos="{}" vertsize="1000" textheight="1000" baseline="850" spacing="600" horzpos="0" horzsize="{}" flags="{}"/>"#,
        textpos, vertpos, HORZ_SIZE, LINE_FLAGS,
    ));
}

fn replace_first_linesegs(xml: &str, new_inner: &str) -> String {
    let open = xml
        .find(LINESEG_SLOT_OPEN)
        .expect("template has linesegarray");
    let inner_start = open + LINESEG_SLOT_OPEN.len();
    let close_rel = xml[inner_start..]
        .find(LINESEG_SLOT_CLOSE)
        .expect("template has closing linesegarray");
    let inner_end = inner_start + close_rel;
    let mut out = String::with_capacity(xml.len() + new_inner.len());
    out.push_str(&xml[..inner_start]);
    out.push_str(new_inner);
    out.push_str(&xml[inner_end..]);
    out
}

// `TEMPLATE_RUN_BEFORE_TEXT` 는 패턴 인식용 상수로만 쓰이므로 명시 참조.
#[allow(dead_code)]
fn _template_anchor_hint() {
    let _ = TEMPLATE_RUN_BEFORE_TEXT;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::paragraph::{CharShapeRef, Paragraph};

    fn make_doc_with_paragraph(para: Paragraph) -> (Document, Section) {
        let mut section = Section::default();
        section.paragraphs.push(para);
        let mut doc = Document::default();
        doc.sections.push(section.clone());
        (doc, section)
    }

    #[test]
    fn hp_p_attrs_reflect_para_shape_id_and_style_id() {
        let mut para = Paragraph::default();
        para.para_shape_id = 7;
        para.style_id = 3;
        para.text = "hi".to_string();
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &mut ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(
            xml.contains(r#"paraPrIDRef="7""#),
            "<hp:p> must reflect para_shape_id=7: {}",
            &xml[..200.min(xml.len())]
        );
        assert!(
            xml.contains(r#"styleIDRef="3""#),
            "<hp:p> must reflect style_id=3"
        );
    }

    #[test]
    fn hp_run_reflects_first_char_shape_id() {
        let mut para = Paragraph::default();
        para.text = "hello".to_string();
        para.char_shapes.push(CharShapeRef {
            start_pos: 0,
            char_shape_id: 42,
        });
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &mut ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(
            xml.contains(r#"<hp:run charPrIDRef="42"><hp:t>hello</hp:t>"#),
            "first run must use char_shape_id 42, xml excerpt around <hp:t>: {:?}",
            xml.find("<hp:t>")
                .map(|i| &xml[i.saturating_sub(50)..(i + 50).min(xml.len())])
        );
    }

    #[test]
    fn page_break_paragraph_emits_attr() {
        let mut para = Paragraph::default();
        para.text = "p1".to_string();
        para.column_type = crate::model::paragraph::ColumnBreakType::Page;
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &mut ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(
            xml.contains(r#"pageBreak="1""#),
            "pageBreak must be 1 for Page column_type"
        );
        assert!(xml.contains(r#"columnBreak="0""#));
    }

    #[test]
    fn default_paragraph_keeps_zero_attrs() {
        let mut para = Paragraph::default();
        para.text = "x".to_string();
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_section(&section, &doc, 0, &mut ctx).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(xml.contains(r#"paraPrIDRef="0""#));
        assert!(xml.contains(r#"styleIDRef="0""#));
        // char_shapes 가 비어있으면 fallback 0
        assert!(xml.contains(r#"<hp:run charPrIDRef="0">"#));
    }

    #[test]
    fn additional_paragraphs_use_their_own_char_shape() {
        let mut p1 = Paragraph::default();
        p1.text = "first".to_string();
        p1.char_shapes.push(CharShapeRef {
            start_pos: 0,
            char_shape_id: 5,
        });
        let mut p2 = Paragraph::default();
        p2.text = "second".to_string();
        p2.para_shape_id = 2;
        p2.char_shapes.push(CharShapeRef {
            start_pos: 0,
            char_shape_id: 6,
        });
        let mut section = Section::default();
        section.paragraphs.push(p1);
        section.paragraphs.push(p2);
        let mut doc = Document::default();
        doc.sections.push(section.clone());
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_section(&section, &doc, 0, &mut ctx).unwrap()).unwrap();
        // 두 번째 문단: paraPrIDRef=2, charPrIDRef=6
        assert!(xml.contains(r#"paraPrIDRef="2""#));
        assert!(
            xml.matches(r#"charPrIDRef="6""#).count() >= 1,
            "second paragraph must emit charPrIDRef=6"
        );
    }

    // ---------- #177 Stage 2: IR 기반 lineseg 출력 ----------

    use crate::model::paragraph::LineSeg;

    #[test]
    fn task177_lineseg_reflects_ir_values() {
        // IR에 담긴 lineseg 값이 XML 속성에 그대로 반영되는지 확인.
        let mut para = Paragraph::default();
        para.text = "hello".to_string();
        para.line_segs.push(LineSeg {
            text_start: 0,
            vertical_pos: 5000,
            line_height: 1200,
            text_height: 1100,
            baseline_distance: 900,
            line_spacing: 700,
            column_start: 100,
            segment_width: 50000,
            tag: 999,
        });
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_section(&section, &doc, 0, &mut ctx).unwrap()).unwrap();
        assert!(xml.contains(r#"<hp:lineseg textpos="0" vertpos="5000" vertsize="1200" textheight="1100" baseline="900" spacing="700" horzpos="100" horzsize="50000" flags="999"/>"#),
            "lineseg must reflect IR values exactly, got XML: {}",
            &xml[xml.find("<hp:lineseg").unwrap_or(0)..(xml.find("<hp:lineseg").unwrap_or(0) + 200).min(xml.len())]);
    }

    #[test]
    fn task177_multiple_linesegs_preserved_in_order() {
        let mut para = Paragraph::default();
        para.text = "three\nlines\nhere".to_string();
        for (i, (tp, vp, lh)) in [(0u32, 0i32, 1000), (6, 1500, 1200), (12, 3100, 1100)]
            .iter()
            .enumerate()
        {
            let _ = i;
            para.line_segs.push(LineSeg {
                text_start: *tp,
                vertical_pos: *vp,
                line_height: *lh,
                text_height: *lh,
                baseline_distance: 850,
                line_spacing: 600,
                column_start: 0,
                segment_width: 42520,
                tag: 393216,
            });
        }
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_section(&section, &doc, 0, &mut ctx).unwrap()).unwrap();
        // 3개 lineseg 모두 출력되고 각각의 vertsize 값이 IR 값과 일치
        assert_eq!(xml.matches("<hp:lineseg ").count(), 3);
        assert!(xml.contains(r#"textpos="0" vertpos="0" vertsize="1000""#));
        assert!(xml.contains(r#"textpos="6" vertpos="1500" vertsize="1200""#));
        assert!(xml.contains(r#"textpos="12" vertpos="3100" vertsize="1100""#));
    }

    #[test]
    fn task177_fallback_used_when_ir_empty() {
        // IR 의 line_segs 가 비어있으면 fallback 경로로 정적 값 출력.
        let mut para = Paragraph::default();
        para.text = "a\nb".to_string(); // 소프트브레이크 1개 → fallback 은 lineseg 2개 생성
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_section(&section, &doc, 0, &mut ctx).unwrap()).unwrap();
        // 정적 fallback: vertsize=1000, textheight=1000, baseline=850, spacing=600
        assert!(xml.contains(r#"vertsize="1000""#));
        assert!(xml.contains(r#"baseline="850""#));
    }

    #[test]
    fn task177_ir_lineseg_takes_precedence_over_text() {
        // text 의 \n 개수가 2개(lineseg 3개 기대)이지만 IR의 line_segs 는 1개만 있음.
        // IR 기반 출력이 우선 — 1개만 출력돼야 함.
        let mut para = Paragraph::default();
        para.text = "a\nb\nc".to_string(); // 3줄
        para.line_segs.push(LineSeg {
            text_start: 0,
            vertical_pos: 0,
            line_height: 2000, // IR 값
            text_height: 2000,
            baseline_distance: 1700,
            line_spacing: 300,
            column_start: 0,
            segment_width: 40000,
            tag: 0,
        });
        let (doc, section) = make_doc_with_paragraph(para);
        let mut ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_section(&section, &doc, 0, &mut ctx).unwrap()).unwrap();
        // IR 에 1개만 있으므로 lineseg 도 1개만 출력 (rhwp 는 원본 보존)
        assert_eq!(xml.matches("<hp:lineseg ").count(), 1);
        assert!(
            xml.contains(r#"vertsize="2000""#),
            "IR value 2000 must be used, not fallback 1000"
        );
    }
}
