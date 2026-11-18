//! Contents/header.xml — DocInfo 리소스 테이블 동적 직렬화.
//!
//! Stage 1 (#182): IR의 `doc_info` 에 담긴 리소스를 역방향으로 HWPX XML로 출력한다.
//! IR이 비어있으면 해당 섹션도 비어있게 출력한다 (IR에 없는 리소스를 자동 생성하지 않음).
//!
//! 속성·자식 순서는 한컴 OWPML 공식 구현(hancom-io/hwpx-owpml-model, Apache 2.0)의
//! `Class/Head/*.cpp` 파일 `WriteElement()`, `InitMap()` 을 기준으로 맞춘다.
//!
//! ## 범위
//!
//! - 1단계 목표: 기존 HWPX 문서를 parse→serialize 했을 때 한컴2020이 온전히 다시 연다
//! - 완전히 새 빈 문서 생성은 1단계 범위 밖 (기본값 채우기 로직 없음)

use std::io::Write;

use quick_xml::Writer;

use crate::model::document::{DocInfo, DocProperties, Document};
use crate::model::style::{
    Alignment, BorderFill, BorderLine, BorderLineType, CharShape, DiagonalLine, Fill, FillType,
    Font, HeadType, LineSpacingType, Numbering, ParaShape, Style, TabDef,
};
use crate::model::ColorRef;

use super::canonical_defaults::FONTFACE_LANG_NAMES;
use super::context::SerializeContext;
use super::utils::{empty_tag, end_tag, start_tag_attrs, write_xml_decl};
use super::SerializeError;

/// 한컴이 `hp:switch` 의 case 로 쓰는 HwpUnitChar 이름공간. 그 case 값은 IR(HWP5 척도)의 절반이다.
const HWP_UNIT_CHAR_NS: &str = "http://www.hancom.co.kr/hwpml/2016/HwpUnitChar";

/// `header.xml` 바이트 생성. Stage 1 진입점.
pub fn write_header(doc: &Document, ctx: &SerializeContext) -> Result<Vec<u8>, SerializeError> {
    let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
    write_xml_decl(&mut w)?;

    // <hh:head> 루트 + 전체 네임스페이스 (parser가 기대하는 접두어 모두 선언)
    let sec_cnt = doc.doc_properties.section_count.max(1).to_string();
    start_tag_attrs(
        &mut w,
        "hh:head",
        &[
            ("xmlns:ha", "http://www.hancom.co.kr/hwpml/2011/app"),
            ("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"),
            ("xmlns:hp10", "http://www.hancom.co.kr/hwpml/2016/paragraph"),
            ("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"),
            ("xmlns:hc", "http://www.hancom.co.kr/hwpml/2011/core"),
            ("xmlns:hh", "http://www.hancom.co.kr/hwpml/2011/head"),
            ("xmlns:hhs", "http://www.hancom.co.kr/hwpml/2011/history"),
            ("xmlns:hm", "http://www.hancom.co.kr/hwpml/2011/master-page"),
            ("xmlns:dc", "http://purl.org/dc/elements/1.1/"),
            ("xmlns:opf", "http://www.idpf.org/2007/opf/"),
            ("xmlns:epub", "http://www.idpf.org/2007/ops"),
            (
                "xmlns:ooxmlchart",
                "http://www.hancom.co.kr/hwpml/2016/ooxmlchart",
            ),
            ("xmlns:hwpunitchar", HWP_UNIT_CHAR_NS),
            ("xmlns:hpf", "http://www.hancom.co.kr/schema/2011/hpf"),
            (
                "xmlns:config",
                "urn:oasis:names:tc:opendocument:xmlns:config:1.0",
            ),
            ("version", "1.2"),
            ("secCnt", &sec_cnt),
        ],
    )?;

    write_begin_num(&mut w, &doc.doc_properties)?;

    // <hh:refList>: 모든 리소스 테이블을 감싸는 컨테이너
    super::utils::start_tag(&mut w, "hh:refList")?;
    write_fontfaces(&mut w, &doc.doc_info)?;
    write_border_fills(&mut w, &doc.doc_info, ctx)?;
    write_char_properties(&mut w, &doc.doc_info, ctx)?;
    write_tab_properties(&mut w, &doc.doc_info)?;
    write_numberings(&mut w, &doc.doc_info)?;
    write_para_properties(&mut w, &doc.doc_info, ctx)?;
    write_styles(&mut w, &doc.doc_info, ctx)?;
    end_tag(&mut w, "hh:refList")?;

    write_compatible_document(&mut w)?;
    write_doc_option(&mut w)?;
    write_track_change_config(&mut w, &doc.doc_info)?;

    end_tag(&mut w, "hh:head")?;
    Ok(w.into_inner())
}

// =====================================================================
// <hh:beginNum>
// =====================================================================
fn write_begin_num<W: Write>(
    w: &mut Writer<W>,
    props: &DocProperties,
) -> Result<(), SerializeError> {
    empty_tag(
        w,
        "hh:beginNum",
        &[
            ("page", &props.page_start_num.max(1).to_string()),
            ("footnote", &props.footnote_start_num.max(1).to_string()),
            ("endnote", &props.endnote_start_num.max(1).to_string()),
            ("pic", &props.picture_start_num.max(1).to_string()),
            ("tbl", &props.table_start_num.max(1).to_string()),
            ("equation", &props.equation_start_num.max(1).to_string()),
        ],
    )
}

// =====================================================================
// <hh:fontfaces> — 7 언어 그룹
// =====================================================================
fn write_fontfaces<W: Write>(w: &mut Writer<W>, doc_info: &DocInfo) -> Result<(), SerializeError> {
    // IR의 font_faces는 항상 7개 언어 그룹을 유지한다고 기대하나,
    // 비어있거나 크기가 다를 수 있으므로 안전하게 처리.
    let groups: Vec<&Vec<Font>> = (0..7)
        .map(|i| doc_info.font_faces.get(i).unwrap_or(&EMPTY_FONT_VEC))
        .collect();

    let item_cnt = groups.iter().filter(|g| !g.is_empty()).count();
    if item_cnt == 0 {
        return Ok(());
    }

    start_tag_attrs(
        w,
        "hh:fontfaces",
        &[(
            "itemCnt",
            &groups.iter().filter(|g| !g.is_empty()).count().to_string(),
        )],
    )?;
    for (lang_idx, fonts) in groups.iter().enumerate() {
        if fonts.is_empty() {
            continue;
        }
        let lang = FONTFACE_LANG_NAMES[lang_idx];
        start_tag_attrs(
            w,
            "hh:fontface",
            &[("lang", lang), ("fontCnt", &fonts.len().to_string())],
        )?;
        for (id, font) in fonts.iter().enumerate() {
            empty_tag(
                w,
                "hh:font",
                &[
                    ("id", &id.to_string()),
                    ("face", &font.name),
                    ("type", font_type_str(font.alt_type)),
                    ("isEmbedded", "0"),
                ],
            )?;
        }
        end_tag(w, "hh:fontface")?;
    }
    end_tag(w, "hh:fontfaces")?;
    Ok(())
}

static EMPTY_FONT_VEC: Vec<Font> = Vec::new();

fn font_type_str(alt_type: u8) -> &'static str {
    match alt_type {
        1 => "TTF",
        2 => "HFT",
        _ => "TTF", // 기본: TTF (한컴 샘플 관찰값)
    }
}

// =====================================================================
// <hh:borderFills>
// =====================================================================
fn write_border_fills<W: Write>(
    w: &mut Writer<W>,
    doc_info: &DocInfo,
    ctx: &SerializeContext,
) -> Result<(), SerializeError> {
    let _ = ctx;
    if doc_info.border_fills.is_empty() {
        return Ok(());
    }
    start_tag_attrs(
        w,
        "hh:borderFills",
        &[("itemCnt", &doc_info.border_fills.len().to_string())],
    )?;
    // HWPX borderFill의 id는 1부터 시작 (관찰값: ref_empty.hwpx).
    // 그러나 rhwp parser는 인덱스 기반으로 저장하므로 id는 배열 인덱스 그대로 사용.
    for (idx, bf) in doc_info.border_fills.iter().enumerate() {
        write_border_fill(w, idx as u16, bf)?;
    }
    end_tag(w, "hh:borderFills")?;
    Ok(())
}

fn write_border_fill<W: Write>(
    w: &mut Writer<W>,
    id: u16,
    bf: &BorderFill,
) -> Result<(), SerializeError> {
    // 속성 순서 (BorderFillType.cpp:64-68): id, threeD, shadow, centerLine, breakCellSeparateLine
    start_tag_attrs(
        w,
        "hh:borderFill",
        &[
            ("id", &(id + 1).to_string()), // HWPX 관찰: id는 1-based
            ("threeD", "0"),
            ("shadow", "0"),
            ("centerLine", "NONE"),
            ("breakCellSeparateLine", "0"),
        ],
    )?;

    // 자식 순서 (BorderFillType.cpp:51-58):
    // slash, backSlash, leftBorder, rightBorder, topBorder, bottomBorder, diagonal, fillBrush
    write_diag_line(w, "hh:slash")?;
    write_diag_line(w, "hh:backSlash")?;
    write_border_line(w, "hh:leftBorder", &bf.borders[0])?;
    write_border_line(w, "hh:rightBorder", &bf.borders[1])?;
    write_border_line(w, "hh:topBorder", &bf.borders[2])?;
    write_border_line(w, "hh:bottomBorder", &bf.borders[3])?;
    write_diagonal(w, &bf.diagonal)?;

    // fillBrush: Fill이 존재할 때만. 단색(winBrush)은 내용까지 쓴다 (업스트림 9ffc9f3 의 위임 자리).
    write_fill_brush(w, &bf.fill)?;

    end_tag(w, "hh:borderFill")?;
    Ok(())
}

/// `<hc:fillBrush>` — 단색은 `<hc:winBrush faceColor hatchColor alpha/>` 로 쓴다.
/// 그러데이션·이미지 채우기는 아직 내용을 쓰지 않고 빈 래퍼만 둔다.
/// (한컴 관찰: ref_empty의 borderFill id=2 에 빈 fillBrush 존재)
fn write_fill_brush<W: Write>(w: &mut Writer<W>, fill: &Fill) -> Result<(), SerializeError> {
    if matches!(fill.fill_type, FillType::None) {
        return Ok(());
    }
    start_tag(w, "hc:fillBrush")?;
    if let (FillType::Solid, Some(solid)) = (fill.fill_type, fill.solid) {
        empty_tag(
            w,
            "hc:winBrush",
            &[
                ("faceColor", &fill_color(solid.background_color)),
                ("hatchColor", &fill_color(solid.pattern_color)),
                ("alpha", &fill_alpha(fill.alpha)),
            ],
        )?;
    }
    end_tag(w, "hc:fillBrush")?;
    Ok(())
}

/// 채우기 색. 파서는 `none` 을 0xFFFFFFFF 로 읽으므로 그 값은 `none` 으로 되쓴다
/// (`#FFFFFF` 로 쓰면 투명이 흰색이 된다).
fn fill_color(c: ColorRef) -> String {
    if c == 0xFFFF_FFFF {
        "none".to_string()
    } else {
        color_hex(c)
    }
}

/// 채우기 alpha(0~255) → HWPX 실수 문자열. 파서의 `(f * 255) as u8` 가 같은 값을 돌려주며 0 은 "0" 이다.
fn fill_alpha(alpha: u8) -> String {
    (f64::from(alpha) / 255.0).to_string()
}

fn write_diag_line<W: Write>(w: &mut Writer<W>, name: &str) -> Result<(), SerializeError> {
    empty_tag(
        w,
        name,
        &[("type", "NONE"), ("Crooked", "0"), ("isCounter", "0")],
    )
}

fn write_border_line<W: Write>(
    w: &mut Writer<W>,
    name: &str,
    line: &BorderLine,
) -> Result<(), SerializeError> {
    let type_str = border_line_type_str(line.line_type);
    let width_mm = format!("{} mm", border_width_mm(line.width));
    let color = color_hex(line.color);
    empty_tag(
        w,
        name,
        &[("type", type_str), ("width", &width_mm), ("color", &color)],
    )
}

fn write_diagonal<W: Write>(w: &mut Writer<W>, d: &DiagonalLine) -> Result<(), SerializeError> {
    let type_str = if d.width == 0 { "NONE" } else { "SOLID" };
    let width_mm = format!("{} mm", border_width_mm(d.width));
    let color = color_hex(d.color);
    empty_tag(
        w,
        "hh:diagonal",
        &[("type", type_str), ("width", &width_mm), ("color", &color)],
    )
}

fn border_line_type_str(t: BorderLineType) -> &'static str {
    use BorderLineType::*;
    match t {
        None => "NONE",
        Solid => "SOLID",
        Dash => "DASH",
        Dot => "DOT",
        DashDot => "DASH_DOT",
        DashDotDot => "DASH_DOT_DOT",
        LongDash => "LONG_DASH",
        Circle => "CIRCLE",
        Double => "DOUBLE_SLIM",
        ThinThickDouble => "SLIM_THICK",
        ThickThinDouble => "THICK_SLIM",
        ThinThickThinTriple => "SLIM_THICK_SLIM",
        Wave => "WAVE",
        DoubleWave => "DOUBLE_WAVE",
        Thick3D => "THICK3D",
        Thick3DReverse => "THICKREV3D",
        Thin3D => "3D",
        Thin3DReverse => "REV3D",
    }
}

fn border_width_mm(w: u8) -> &'static str {
    // HWP 선 굵기 인덱스(0~) → mm (한컴 매핑)
    // 0=0.1mm, 1=0.12mm, 2=0.15mm, 3=0.2mm, 4=0.25mm, 5=0.3mm, 6=0.4mm, 7=0.5mm,
    // 8=0.6mm, 9=0.7mm, 10=1.0mm, 11=1.5mm, 12=2.0mm, 13=3.0mm, 14=4.0mm, 15=5.0mm
    // ref_empty.hwpx에서 기본값은 "0.1 mm" 관찰
    match w {
        0 => "0.1",
        1 => "0.12",
        2 => "0.15",
        3 => "0.2",
        4 => "0.25",
        5 => "0.3",
        6 => "0.4",
        7 => "0.5",
        8 => "0.6",
        9 => "0.7",
        10 => "1.0",
        11 => "1.5",
        12 => "2.0",
        13 => "3.0",
        14 => "4.0",
        15 => "5.0",
        _ => "0.1",
    }
}

fn color_hex(c: ColorRef) -> String {
    // ColorRef = u32. HWP는 BGR 저장. HWPX는 RGB "#RRGGBB".
    let r = (c & 0xFF) as u8;
    let g = ((c >> 8) & 0xFF) as u8;
    let b = ((c >> 16) & 0xFF) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

// =====================================================================
// <hh:charProperties>
// =====================================================================
fn write_char_properties<W: Write>(
    w: &mut Writer<W>,
    doc_info: &DocInfo,
    ctx: &SerializeContext,
) -> Result<(), SerializeError> {
    let _ = ctx;
    if doc_info.char_shapes.is_empty() {
        return Ok(());
    }
    start_tag_attrs(
        w,
        "hh:charProperties",
        &[("itemCnt", &doc_info.char_shapes.len().to_string())],
    )?;
    for (idx, cs) in doc_info.char_shapes.iter().enumerate() {
        write_char_pr(w, idx as u32, cs)?;
    }
    end_tag(w, "hh:charProperties")?;
    Ok(())
}

fn write_char_pr<W: Write>(
    w: &mut Writer<W>,
    id: u32,
    cs: &CharShape,
) -> Result<(), SerializeError> {
    // 속성 순서 (CharShapeType.cpp:79-86): id, height, textColor, shadeColor,
    // useFontSpace, useKerning, symMark, borderFillIDRef
    let shade = if cs.shade_color == 0 {
        "none".to_string()
    } else {
        fill_color(cs.shade_color)
    };
    start_tag_attrs(
        w,
        "hh:charPr",
        &[
            ("id", &id.to_string()),
            ("height", &cs.base_size.to_string()),
            ("textColor", &color_hex(cs.text_color)),
            ("shadeColor", &shade),
            ("useFontSpace", bool01(false)),
            ("useKerning", bool01(cs.kerning)),
            ("symMark", sym_mark_str(cs.emphasis_dot)),
            ("borderFillIDRef", &cs.border_fill_id.to_string()),
        ],
    )?;

    // 자식 순서 (CharShapeType.cpp:59-73):
    // fontRef, ratio, spacing, relSz, offset, italic, bold, underline, strikeout, outline,
    // shadow, emboss, engrave, supscript, subscript
    write_lang_attrs(w, "hh:fontRef", &cs.font_ids.map(|v| v as i32))?;
    write_lang_attrs(w, "hh:ratio", &cs.ratios.map(|v| v as i32))?;
    write_lang_attrs(w, "hh:spacing", &cs.spacings.map(|v| v as i32))?;
    write_lang_attrs(w, "hh:relSz", &cs.relative_sizes.map(|v| v as i32))?;
    write_lang_attrs(w, "hh:offset", &cs.char_offsets.map(|v| v as i32))?;
    if cs.italic {
        empty_tag(w, "hh:italic", &[])?;
    }
    if cs.bold {
        empty_tag(w, "hh:bold", &[])?;
    }
    if !matches!(cs.underline_type, crate::model::style::UnderlineType::None) {
        empty_tag(
            w,
            "hh:underline",
            &[
                ("type", underline_type_str(cs.underline_type)),
                ("shape", line_shape_str(cs.underline_shape)),
                ("color", &color_hex(cs.underline_color)),
            ],
        )?;
    }
    if cs.strikethrough {
        empty_tag(
            w,
            "hh:strikeout",
            &[
                ("shape", line_shape_str(cs.strike_shape)),
                ("color", &color_hex(cs.strike_color)),
            ],
        )?;
    }
    if cs.outline_type != 0 {
        empty_tag(
            w,
            "hh:outline",
            &[("type", outline_type_str(cs.outline_type))],
        )?;
    }
    if cs.shadow_type != 0 {
        empty_tag(
            w,
            "hh:shadow",
            &[
                ("type", "CONTINUOUS"),
                ("color", &color_hex(cs.shadow_color)),
                ("offsetX", &cs.shadow_offset_x.to_string()),
                ("offsetY", &cs.shadow_offset_y.to_string()),
            ],
        )?;
    }
    if cs.emboss {
        empty_tag(w, "hh:emboss", &[])?;
    }
    if cs.engrave {
        empty_tag(w, "hh:engrave", &[])?;
    }
    if cs.superscript {
        empty_tag(w, "hh:supscript", &[])?;
    }
    if cs.subscript {
        empty_tag(w, "hh:subscript", &[])?;
    }

    end_tag(w, "hh:charPr")?;
    Ok(())
}

fn write_lang_attrs<W: Write>(
    w: &mut Writer<W>,
    name: &str,
    vals: &[i32; 7],
) -> Result<(), SerializeError> {
    let s0 = vals[0].to_string();
    let s1 = vals[1].to_string();
    let s2 = vals[2].to_string();
    let s3 = vals[3].to_string();
    let s4 = vals[4].to_string();
    let s5 = vals[5].to_string();
    let s6 = vals[6].to_string();
    empty_tag(
        w,
        name,
        &[
            ("hangul", &s0),
            ("latin", &s1),
            ("hanja", &s2),
            ("japanese", &s3),
            ("other", &s4),
            ("symbol", &s5),
            ("user", &s6),
        ],
    )
}

fn bool01(b: bool) -> &'static str {
    if b {
        "1"
    } else {
        "0"
    }
}

fn sym_mark_str(em: u8) -> &'static str {
    match em {
        0 => "NONE",
        1 => "DOT_ABOVE",
        2 => "RING_ABOVE",
        3 => "TILDE",
        4 => "CARON",
        5 => "SIDE",
        6 => "COLON",
        _ => "NONE",
    }
}

fn underline_type_str(t: crate::model::style::UnderlineType) -> &'static str {
    use crate::model::style::UnderlineType::*;
    match t {
        None => "NONE",
        Bottom => "BOTTOM",
        Top => "TOP",
    }
}

fn line_shape_str(s: u8) -> &'static str {
    match s {
        0 => "SOLID",
        1 => "DASH",
        2 => "DOT",
        3 => "DASH_DOT",
        4 => "DASH_DOT_DOT",
        5 => "LONG_DASH",
        6 => "CIRCLE",
        7 => "DOUBLE_SLIM",
        8 => "SLIM_THICK",
        9 => "THICK_SLIM",
        10 => "SLIM_THICK_SLIM",
        11 => "WAVE",
        12 => "DOUBLE_WAVE",
        _ => "SOLID",
    }
}

fn outline_type_str(t: u8) -> &'static str {
    match t {
        0 => "NONE",
        1 => "SOLID",
        2 => "DASH",
        3 => "DOT",
        _ => "NONE",
    }
}

// =====================================================================
// <hh:tabProperties>
// =====================================================================
fn write_tab_properties<W: Write>(
    w: &mut Writer<W>,
    doc_info: &DocInfo,
) -> Result<(), SerializeError> {
    if doc_info.tab_defs.is_empty() {
        return Ok(());
    }
    start_tag_attrs(
        w,
        "hh:tabProperties",
        &[("itemCnt", &doc_info.tab_defs.len().to_string())],
    )?;
    for (idx, td) in doc_info.tab_defs.iter().enumerate() {
        write_tab_pr(w, idx as u16, td)?;
    }
    end_tag(w, "hh:tabProperties")?;
    Ok(())
}

fn write_tab_pr<W: Write>(w: &mut Writer<W>, id: u16, td: &TabDef) -> Result<(), SerializeError> {
    let attrs = [
        ("id", id.to_string()),
        ("autoTabLeft", bool01(td.auto_tab_left).to_string()),
        ("autoTabRight", bool01(td.auto_tab_right).to_string()),
    ];
    let attrs_ref: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();

    if td.tabs.is_empty() {
        empty_tag(w, "hh:tabPr", &attrs_ref)?;
    } else {
        start_tag_attrs(w, "hh:tabPr", &attrs_ref)?;
        for tab in &td.tabs {
            write_tab_item_switch(w, tab)?;
        }
        end_tag(w, "hh:tabPr")?;
    }
    Ok(())
}

/// tabItem 하나를 한컴 원본처럼 `<hp:switch>` 로 쓴다 — HwpUnitChar case 는 위치의 절반(홀수는 CHAR),
/// default 는 IR 위치. 한컴은 switch 없는 평문 위치를 case 척도로 읽어 탭이 두 배 자리로 간다.
fn write_tab_item_switch<W: Write>(
    w: &mut Writer<W>,
    tab: &crate::model::style::TabItem,
) -> Result<(), SerializeError> {
    super::utils::start_tag(w, "hp:switch")?;
    start_tag_attrs(w, "hp:case", &[("hp:required-namespace", HWP_UNIT_CHAR_NS)])?;
    empty_tag(
        w,
        "hh:tabItem",
        &[
            ("pos", &(tab.position / 2).to_string()),
            ("type", tab_type_str(tab.tab_type)),
            ("leader", tab_leader_str(tab.fill_type)),
            (
                "unit",
                if tab.position.is_multiple_of(2) { "HWPUNIT" } else { "CHAR" },
            ),
        ],
    )?;
    end_tag(w, "hp:case")?;
    super::utils::start_tag(w, "hp:default")?;
    empty_tag(
        w,
        "hh:tabItem",
        &[
            ("pos", &tab.position.to_string()),
            ("type", tab_type_str(tab.tab_type)),
            ("leader", tab_leader_str(tab.fill_type)),
        ],
    )?;
    end_tag(w, "hp:default")?;
    end_tag(w, "hp:switch")
}

fn tab_type_str(t: u8) -> &'static str {
    match t {
        0 => "LEFT",
        1 => "RIGHT",
        2 => "CENTER",
        3 => "DECIMAL",
        _ => "LEFT",
    }
}

fn tab_leader_str(f: u8) -> &'static str {
    match f {
        0 => "NONE",
        1 => "SOLID",
        2 => "DOT",
        3 => "DASH",
        4 => "DASH_DOT",
        5 => "DASH_DOT_DOT",
        6 => "LONG_DASH",
        7 => "CIRCLE",
        8 => "DOUBLE_SLIM",
        _ => "NONE",
    }
}

// =====================================================================
// <hh:numberings>
// =====================================================================
fn write_numberings<W: Write>(w: &mut Writer<W>, doc_info: &DocInfo) -> Result<(), SerializeError> {
    if doc_info.numberings.is_empty() {
        return Ok(());
    }
    start_tag_attrs(
        w,
        "hh:numberings",
        &[("itemCnt", &doc_info.numberings.len().to_string())],
    )?;
    for (idx, n) in doc_info.numberings.iter().enumerate() {
        write_numbering(w, idx as u16, n)?;
    }
    end_tag(w, "hh:numberings")?;
    Ok(())
}

fn write_numbering<W: Write>(
    w: &mut Writer<W>,
    id: u16,
    n: &Numbering,
) -> Result<(), SerializeError> {
    start_tag_attrs(
        w,
        "hh:numbering",
        &[
            ("id", &(id + 1).to_string()), // 관찰: 1-based
            ("start", &n.start_number.to_string()),
        ],
    )?;
    // Stage 1: 10 레벨 paraHead 뼈대 출력. 실제 값은 NumberingHead 참조해 생성.
    for level in 0..10usize {
        let idx = level.min(6);
        let h = &n.heads[idx];
        let start = n.level_start_numbers.get(idx).copied().unwrap_or(1);
        let level_s = (level + 1).to_string();
        let start_s = start.to_string();
        let wa = h.width_adjust.to_string();
        empty_tag(
            w,
            "hh:paraHead",
            &[
                ("start", &start_s),
                ("level", &level_s),
                ("align", "LEFT"),
                ("useInstWidth", "1"),
                ("autoIndent", "1"),
                ("widthAdjust", &wa),
                ("textOffsetType", "PERCENT"),
                ("textOffset", "50"),
                ("numFormat", "DIGIT"),
                ("charPrIDRef", &u32::MAX.to_string()),
                ("checkable", "0"),
            ],
        )?;
    }
    end_tag(w, "hh:numbering")?;
    Ok(())
}

// =====================================================================
// <hh:paraProperties>
// =====================================================================
fn write_para_properties<W: Write>(
    w: &mut Writer<W>,
    doc_info: &DocInfo,
    ctx: &SerializeContext,
) -> Result<(), SerializeError> {
    let _ = ctx;
    if doc_info.para_shapes.is_empty() {
        return Ok(());
    }
    start_tag_attrs(
        w,
        "hh:paraProperties",
        &[("itemCnt", &doc_info.para_shapes.len().to_string())],
    )?;
    for (idx, ps) in doc_info.para_shapes.iter().enumerate() {
        write_para_pr(w, idx as u16, ps)?;
    }
    end_tag(w, "hh:paraProperties")?;
    Ok(())
}

fn write_para_pr<W: Write>(
    w: &mut Writer<W>,
    id: u16,
    ps: &ParaShape,
) -> Result<(), SerializeError> {
    // 속성 순서 (ParaShapeType.cpp:62-68): id, tabPrIDRef, condense,
    // fontLineHeight, snapToGrid, suppressLineNumbers, checked
    start_tag_attrs(
        w,
        "hh:paraPr",
        &[
            ("id", &id.to_string()),
            ("tabPrIDRef", &ps.tab_def_id.to_string()),
            ("condense", "0"),
            ("fontLineHeight", bool01(ps.attr1 & (1 << 22) != 0)),
            ("snapToGrid", "1"),
            ("suppressLineNumbers", "0"),
            ("checked", "0"),
        ],
    )?;

    // 자식 순서 (한컴 원본 관찰):
    // align, heading, breakSetting, autoSpacing, switch(margin, lineSpacing), border
    empty_tag(
        w,
        "hh:align",
        &[
            ("horizontal", alignment_str(ps.alignment)),
            (
                "vertical",
                match (ps.attr1 >> 20) & 3 {
                    1 => "TOP",
                    2 => "CENTER",
                    3 => "BOTTOM",
                    _ => "BASELINE",
                },
            ),
        ],
    )?;
    empty_tag(
        w,
        "hh:heading",
        &[
            ("type", head_type_str(ps.head_type)),
            ("idRef", &ps.numbering_id.to_string()),
            ("level", &ps.para_level.to_string()),
        ],
    )?;
    empty_tag(
        w,
        "hh:breakSetting",
        &[
            (
                "breakLatinWord",
                match (ps.attr1 >> 5) & 3 {
                    1 => "HYPHENATION",
                    2 => "BREAK_WORD",
                    _ => "KEEP_WORD",
                },
            ),
            (
                "breakNonLatinWord",
                if ps.attr1 & (1 << 7) != 0 {
                    "BREAK_WORD"
                } else {
                    "KEEP_WORD"
                },
            ),
            ("widowOrphan", bool01(ps.attr1 & (1 << 16) != 0)),
            ("keepWithNext", bool01(ps.attr1 & (1 << 17) != 0)),
            ("keepLines", bool01(ps.attr1 & (1 << 18) != 0)),
            ("pageBreakBefore", bool01(ps.attr1 & (1 << 19) != 0)),
            (
                "lineWrap",
                if ps.attr2 & 3 == 1 {
                    "SQUEEZE"
                } else {
                    "BREAK"
                },
            ),
        ],
    )?;

    empty_tag(
        w,
        "hh:autoSpacing",
        &[
            ("eAsianEng", bool01(ps.attr2 & (1 << 4) != 0)),
            ("eAsianNum", bool01(ps.attr2 & (1 << 5) != 0)),
        ],
    )?;

    write_para_margin_switch(w, ps)?;

    empty_tag(
        w,
        "hh:border",
        &[
            ("borderFillIDRef", &ps.border_fill_id.to_string()),
            ("offsetLeft", &ps.border_spacing[0].to_string()),
            ("offsetRight", &ps.border_spacing[1].to_string()),
            ("offsetTop", &ps.border_spacing[2].to_string()),
            ("offsetBottom", &ps.border_spacing[3].to_string()),
            ("connect", "0"),
            ("ignoreMargin", "0"),
        ],
    )?;

    end_tag(w, "hh:paraPr")?;
    Ok(())
}

/// margin·lineSpacing 을 한컴 원본처럼 `<hp:switch>` 로 쓴다 — HwpUnitChar case 는 IR 값의 절반,
/// default 는 IR 값. 한컴은 switch 없는 평문 값을 case 척도로 읽어 들여쓰기·여백이 두 배가 된다.
/// 홀수는 CHAR 단위로 나머지 1을 보존한다.
fn write_para_margin_switch<W: Write>(
    w: &mut Writer<W>,
    ps: &ParaShape,
) -> Result<(), SerializeError> {
    super::utils::start_tag(w, "hp:switch")?;
    start_tag_attrs(w, "hp:case", &[("hp:required-namespace", HWP_UNIT_CHAR_NS)])?;
    write_para_margin(w, ps, true)?;
    write_para_line_spacing(w, ps, true)?;
    end_tag(w, "hp:case")?;
    super::utils::start_tag(w, "hp:default")?;
    write_para_margin(w, ps, false)?;
    write_para_line_spacing(w, ps, false)?;
    end_tag(w, "hp:default")?;
    end_tag(w, "hp:switch")
}

/// `<hh:margin>` 자식 다섯(intent, left, right, prev, next). `case` 면 HwpUnitChar 값을 쓴다.
fn write_para_margin<W: Write>(
    w: &mut Writer<W>,
    ps: &ParaShape,
    case: bool,
) -> Result<(), SerializeError> {
    super::utils::start_tag(w, "hh:margin")?;
    write_margin_child(w, "hc:intent", ps.indent, case)?;
    write_margin_child(w, "hc:left", ps.margin_left, case)?;
    write_margin_child(w, "hc:right", ps.margin_right, case)?;
    write_margin_child(w, "hc:prev", ps.spacing_before, case)?;
    write_margin_child(w, "hc:next", ps.spacing_after, case)?;
    end_tag(w, "hh:margin")
}

/// `<hh:lineSpacing>`. case 에서 PERCENT 는 그대로, 다른 종류는 HWPUNIT 이라 절반이다(파서가 두 배로 읽는다).
fn write_para_line_spacing<W: Write>(
    w: &mut Writer<W>,
    ps: &ParaShape,
    case: bool,
) -> Result<(), SerializeError> {
    let (value, unit) = hwpunitchar_value_unit(
        ps.line_spacing,
        case && !matches!(ps.line_spacing_type, LineSpacingType::Percent),
    );
    empty_tag(
        w,
        "hh:lineSpacing",
        &[
            ("type", line_spacing_type_str(ps.line_spacing_type)),
            ("value", &value.to_string()),
            ("unit", unit),
        ],
    )
}

/// margin 자식 하나(`<hc:intent value="…" unit="…"/>`, 한컴 속성 순서). case 에서 홀수 IR 값은 절반이
/// 정수로 떨어지지 않으므로 한컴 HWP→HWPX 변환본처럼 `(값 - 1) / 2` 와 `unit="CHAR"` 로 쓴다.
fn write_margin_child<W: Write>(
    w: &mut Writer<W>,
    name: &str,
    stored: i32,
    case: bool,
) -> Result<(), SerializeError> {
    let (value, unit) = hwpunitchar_value_unit(stored, case);
    empty_tag(w, name, &[("value", &value.to_string()), ("unit", unit)])
}

fn hwpunitchar_value_unit(stored: i32, case: bool) -> (i32, &'static str) {
    match (case, stored % 2 != 0) {
        (false, _) => (stored, "HWPUNIT"),
        (true, false) => (stored / 2, "HWPUNIT"),
        (true, true) => ((stored - 1) / 2, "CHAR"),
    }
}

fn alignment_str(a: Alignment) -> &'static str {
    use Alignment::*;
    match a {
        Justify => "JUSTIFY",
        Left => "LEFT",
        Right => "RIGHT",
        Center => "CENTER",
        Distribute => "DISTRIBUTE",
        Split => "DISTRIBUTE_SPACE",
    }
}

fn head_type_str(h: HeadType) -> &'static str {
    use HeadType::*;
    match h {
        None => "NONE",
        Outline => "OUTLINE",
        Number => "NUMBER",
        Bullet => "BULLET",
    }
}

fn line_spacing_type_str(t: LineSpacingType) -> &'static str {
    use LineSpacingType::*;
    match t {
        Percent => "PERCENT",
        Fixed => "FIXED",
        SpaceOnly => "BETWEEN_LINES",
        Minimum => "AT_LEAST",
    }
}

// =====================================================================
// <hh:styles>
// =====================================================================
fn write_styles<W: Write>(
    w: &mut Writer<W>,
    doc_info: &DocInfo,
    ctx: &SerializeContext,
) -> Result<(), SerializeError> {
    let _ = ctx;
    if doc_info.styles.is_empty() {
        return Ok(());
    }
    start_tag_attrs(
        w,
        "hh:styles",
        &[("itemCnt", &doc_info.styles.len().to_string())],
    )?;
    for (idx, st) in doc_info.styles.iter().enumerate() {
        write_style(w, idx as u16, st)?;
    }
    end_tag(w, "hh:styles")?;
    Ok(())
}

fn write_style<W: Write>(w: &mut Writer<W>, id: u16, st: &Style) -> Result<(), SerializeError> {
    let type_str = if st.style_type == 1 { "CHAR" } else { "PARA" };
    empty_tag(
        w,
        "hh:style",
        &[
            ("id", &id.to_string()),
            ("type", type_str),
            ("name", &st.local_name),
            ("engName", &st.english_name),
            ("paraPrIDRef", &st.para_shape_id.to_string()),
            ("charPrIDRef", &st.char_shape_id.to_string()),
            ("nextStyleIDRef", &st.next_style_id.to_string()),
            ("langID", "1042"),
            ("lockForm", "0"),
        ],
    )
}

// =====================================================================
// <hh:compatibleDocument>, <hh:docOption>, <hh:trackchageConfig>
// =====================================================================
fn write_compatible_document<W: Write>(w: &mut Writer<W>) -> Result<(), SerializeError> {
    start_tag_attrs(w, "hh:compatibleDocument", &[("targetProgram", "HWP201X")])?;
    super::utils::start_tag(w, "hh:layoutCompatibility")?;
    empty_tag(w, "hh:char", &[])?;
    empty_tag(w, "hh:paragraph", &[])?;
    empty_tag(w, "hh:section", &[])?;
    empty_tag(w, "hh:object", &[])?;
    empty_tag(w, "hh:field", &[])?;
    end_tag(w, "hh:layoutCompatibility")?;
    end_tag(w, "hh:compatibleDocument")?;
    Ok(())
}

fn write_doc_option<W: Write>(w: &mut Writer<W>) -> Result<(), SerializeError> {
    super::utils::start_tag(w, "hh:docOption")?;
    empty_tag(
        w,
        "hh:linkinfo",
        &[("path", ""), ("pageInherit", "0"), ("footnoteInherit", "0")],
    )?;
    end_tag(w, "hh:docOption")?;
    Ok(())
}

fn write_track_change_config<W: Write>(
    w: &mut Writer<W>,
    doc_info: &DocInfo,
) -> Result<(), SerializeError> {
    empty_tag(
        w,
        "hh:trackchageConfig",
        &[("flags", &doc_info.hwpx_track_change_flags.to_string())],
    )
}

// 내부에서 쓰는 start_tag 별명
use super::utils::start_tag;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hwpx::parse_hwpx;

    #[test]
    fn write_header_runs_on_empty_document() {
        let doc = Document::default();
        let ctx = SerializeContext::collect_from_document(&doc);
        let bytes = write_header(&doc, &ctx).expect("write_header");
        let xml = std::str::from_utf8(&bytes).unwrap();
        assert!(xml.contains("<hh:head"));
        assert!(xml.contains("</hh:head>"));
    }

    #[test]
    fn write_header_preserves_char_shape_count() {
        let bytes = include_bytes!("../../../samples/hwpx/ref/ref_empty.hwpx");
        let doc = parse_hwpx(bytes).expect("parse ref_empty");
        let ctx = SerializeContext::collect_from_document(&doc);
        let header_bytes = write_header(&doc, &ctx).expect("write header");
        let xml = std::str::from_utf8(&header_bytes).unwrap();
        // ref_empty.hwpx 의 charPr 개수는 관찰 결과 7개
        let expected = doc.doc_info.char_shapes.len();
        let actual = xml.matches("<hh:charPr ").count();
        assert_eq!(actual, expected, "charPr count mismatch");
    }

    #[test]
    fn write_header_emits_seven_fontfaces_when_populated() {
        let bytes = include_bytes!("../../../samples/hwpx/ref/ref_empty.hwpx");
        let doc = parse_hwpx(bytes).expect("parse");
        let ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_header(&doc, &ctx).unwrap()).unwrap();
        assert_eq!(xml.matches("<hh:fontface ").count(), 7);
    }

    #[test]
    fn canonical_attr_order_charpr() {
        let bytes = include_bytes!("../../../samples/hwpx/ref/ref_empty.hwpx");
        let doc = parse_hwpx(bytes).expect("parse");
        let ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_header(&doc, &ctx).unwrap()).unwrap();
        let snippet = xml
            .find("<hh:charPr ")
            .and_then(|i| {
                let end = xml[i..].find('>').map(|e| i + e)?;
                Some(&xml[i..=end])
            })
            .expect("charPr tag");
        // 속성이 id → height → textColor → shadeColor → useFontSpace → useKerning → symMark → borderFillIDRef 순서여야 함
        let ip = snippet.find("id=").unwrap();
        let hp = snippet.find("height=").unwrap();
        let tc = snippet.find("textColor=").unwrap();
        let sc = snippet.find("shadeColor=").unwrap();
        let uf = snippet.find("useFontSpace=").unwrap();
        let uk = snippet.find("useKerning=").unwrap();
        let sm = snippet.find("symMark=").unwrap();
        let bf = snippet.find("borderFillIDRef=").unwrap();
        assert!(ip < hp && hp < tc && tc < sc && sc < uf && uf < uk && uk < sm && sm < bf);
    }

    #[test]
    fn c12_shade_preserves_real_colors_legacy_zero_and_no_shade_sentinel() {
        for (shade_color, expected) in [
            (0, "none"),
            (0x00FF_FFFF, "#FFFFFF"),
            (0x0056_3412, "#123456"),
            (0xFFFF_FFFF, "none"),
        ] {
            let mut doc = Document::default();
            doc.doc_info.char_shapes = vec![CharShape {
                text_color: 0x00FF_FFFF,
                shade_color,
                ..CharShape::default()
            }];
            let ctx = SerializeContext::collect_from_document(&doc);
            let xml = String::from_utf8(write_header(&doc, &ctx).unwrap()).unwrap();
            assert!(xml.contains(r##"textColor="#FFFFFF""##));
            assert!(
                xml.contains(&format!(r#"shadeColor="{expected}""#)),
                "shade {shade_color:#010x} must emit {expected}: {xml}"
            );
        }
    }

    #[test]
    fn winbrush_alpha_and_none_roundtrip_through_parser() {
        use crate::model::style::SolidFill;
        use crate::parser::hwpx::header::parse_hwpx_header;

        let solid = |face: ColorRef, alpha: u8| {
            let mut bf = BorderFill::default();
            bf.fill.fill_type = FillType::Solid;
            bf.fill.solid = Some(SolidFill {
                background_color: face,
                pattern_color: 0x0099_9999,
                pattern_type: 0,
            });
            bf.fill.alpha = alpha;
            bf
        };
        let mut doc = Document::default();
        // id 1..=256: alpha 0..=255 · id 257: faceColor none · id 258: 그러데이션(빈 래퍼 유지)
        doc.doc_info.border_fills = (0..=255u8).map(|a| solid(0x00D8_E7FF, a)).collect();
        doc.doc_info.border_fills.push(solid(0xFFFF_FFFF, 0));
        let mut gradient = BorderFill::default();
        gradient.fill.fill_type = FillType::Gradient;
        doc.doc_info.border_fills.push(gradient);

        let ctx = SerializeContext::collect_from_document(&doc);
        let xml = String::from_utf8(write_header(&doc, &ctx).unwrap()).unwrap();
        let (info, _) = parse_hwpx_header(&xml).expect("reparse header.xml");
        assert_eq!(info.border_fills.len(), 258);

        let alpha_roundtrip = (0..=255u8)
            .filter(|&a| {
                let fill = &info.border_fills[usize::from(a)].fill;
                fill.fill_type == FillType::Solid && fill.alpha == a
            })
            .count();
        let none_bf = &info.border_fills[256].fill;
        let none_roundtrip = xml
            .contains(r##"<hc:winBrush faceColor="none" hatchColor="#999999" alpha="0"/>"##)
            && none_bf.fill_type == FillType::Solid
            && none_bf.solid.map(|s| s.background_color) == Some(0xFFFF_FFFF);
        println!(
            "WINBRUSH_UNIT alpha_roundtrip={}/256 none_roundtrip={} ",
            alpha_roundtrip, none_roundtrip
        );

        assert_eq!(alpha_roundtrip, 256, "alpha 0..=255 reparse");
        assert!(none_roundtrip, "faceColor none is written and read back");
        assert!(
            xml.contains(r##"<hc:winBrush faceColor="#FFE7D8" hatchColor="#999999" alpha="0"/>"##),
            "colored winBrush keeps #RRGGBB"
        );
        assert_eq!(
            xml.matches("<hc:fillBrush></hc:fillBrush>").count(),
            1,
            "gradient keeps the empty wrapper"
        );
    }
}
