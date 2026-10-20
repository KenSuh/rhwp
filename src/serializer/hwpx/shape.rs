//! 지원 도형의 형상·변환·스타일과 글상자 문단을 직렬화한다.

#![allow(dead_code)]

use std::io::Write;

use quick_xml::Writer;

use crate::model::shape::{
    Caption, CaptionDirection, CommonObjAttr, DrawingObjAttr, GroupShape, HorzAlign, HorzRelTo,
    LineShape, RectangleShape, ShapeComponentAttr, TextWrap, VertAlign, VertRelTo,
};

use super::context::SerializeContext;
use super::utils::{empty_tag, end_tag, start_tag, start_tag_attrs};
use super::SerializeError;
use crate::model::style::{FillType, ShapeBorderLine};

// =====================================================================
// <hp:rect>
// =====================================================================

/// `<hp:rect>` 직렬화 진입점. Rectangle IR → XML.
pub fn write_rect<W: Write>(
    w: &mut Writer<W>,
    rect: &RectangleShape,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    let c = &rect.common;
    // 속성 (부모 AbstractShapeObjectType + 자신):
    // id, zOrder, numberingType, textWrap, textFlow, lock, dropcapstyle,
    // href, groupLevel, instid, ratio
    let id_str = c.instance_id.to_string();
    let z_order = c.z_order.to_string();
    let tw = text_wrap_str(c.text_wrap);

    start_tag_attrs(
        w,
        "hp:rect",
        &[
            ("id", &id_str),
            ("zOrder", &z_order),
            ("numberingType", "NONE"),
            ("textWrap", tw),
            ("textFlow", "BOTH_SIDES"),
            ("lock", "0"),
            ("dropcapstyle", "None"),
            ("href", ""),
            (
                "groupLevel",
                &rect.drawing.shape_attr.group_level.to_string(),
            ),
            ("instid", &id_str),
            ("ratio", &rect.round_rate.to_string()),
        ],
    )?;

    write_shape_component(w, &rect.drawing.shape_attr)?;
    write_drawing(w, &rect.drawing, ctx)?;
    for i in 0..4 {
        empty_tag(
            w,
            &format!("hc:pt{i}"),
            &[
                ("x", &rect.x_coords[i].to_string()),
                ("y", &rect.y_coords[i].to_string()),
            ],
        )?;
    }
    write_sz(w, c)?;
    write_pos(w, c)?;
    write_out_margin(w, c)?;

    write_shape_comment(w, c)?;
    end_tag(w, "hp:rect")?;
    Ok(())
}

// =====================================================================
// <hp:line>
// =====================================================================

/// `<hp:line>` 직렬화 진입점. LineShape IR → XML.
pub fn write_line<W: Write>(
    w: &mut Writer<W>,
    line: &LineShape,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    let c = &line.common;
    let id_str = c.instance_id.to_string();
    let z_order = c.z_order.to_string();
    let tw = text_wrap_str(c.text_wrap);
    let sx = line.start.x.to_string();
    let sy = line.start.y.to_string();
    let ex = line.end.x.to_string();
    let ey = line.end.y.to_string();
    let srb = bool01(line.started_right_or_bottom);

    start_tag_attrs(
        w,
        "hp:line",
        &[
            ("id", &id_str),
            ("zOrder", &z_order),
            ("numberingType", "NONE"),
            ("textWrap", tw),
            ("textFlow", "BOTH_SIDES"),
            ("lock", "0"),
            ("dropcapstyle", "None"),
            ("href", ""),
            (
                "groupLevel",
                &line.drawing.shape_attr.group_level.to_string(),
            ),
            ("instid", &id_str),
            ("startX", &sx),
            ("startY", &sy),
            ("endX", &ex),
            ("endY", &ey),
            ("isReverseHV", srb),
        ],
    )?;

    write_shape_component(w, &line.drawing.shape_attr)?;
    write_drawing(w, &line.drawing, ctx)?;
    write_sz(w, c)?;
    write_pos(w, c)?;
    write_out_margin(w, c)?;

    write_shape_comment(w, c)?;
    end_tag(w, "hp:line")?;
    Ok(())
}

// =====================================================================
// <hp:container> — 묶음 개체 (GroupShape). Stage 5 뼈대만.
// =====================================================================

/// `<hp:container>` 뼈대 — 내부 자식 도형 루프는 dispatcher에서 처리.
pub fn write_container_open<W: Write>(
    w: &mut Writer<W>,
    group: &GroupShape,
) -> Result<(), SerializeError> {
    let common = &group.common;
    let id_str = common.instance_id.to_string();
    let z_order = common.z_order.to_string();
    let tw = text_wrap_str(common.text_wrap);

    start_tag_attrs(
        w,
        "hp:container",
        &[
            ("id", &id_str),
            ("zOrder", &z_order),
            ("numberingType", "NONE"),
            ("textWrap", tw),
            ("textFlow", "BOTH_SIDES"),
            ("lock", "0"),
            ("dropcapstyle", "None"),
            ("href", ""),
            ("groupLevel", &group.shape_attr.group_level.to_string()),
            ("instid", &id_str),
        ],
    )?;

    write_shape_component(w, &group.shape_attr)?;
    Ok(())
}

pub fn write_container_close<W: Write>(
    w: &mut Writer<W>,
    group: &GroupShape,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    write_sz(w, &group.common)?;
    write_pos(w, &group.common)?;
    write_out_margin(w, &group.common)?;
    if let Some(caption) = &group.caption {
        write_caption(w, caption, ctx)?;
    }
    end_tag(w, "hp:container")
}

/// 개체의 내부 변환은 페이지의 pos와 별도로 보존한다.
pub(super) fn write_shape_component<W: Write>(
    w: &mut Writer<W>,
    shape: &ShapeComponentAttr,
) -> Result<(), SerializeError> {
    empty_tag(
        w,
        "hp:offset",
        &[
            ("x", &shape.offset_x.to_string()),
            ("y", &shape.offset_y.to_string()),
        ],
    )?;
    empty_tag(
        w,
        "hp:orgSz",
        &[
            ("width", &shape.original_width.to_string()),
            ("height", &shape.original_height.to_string()),
        ],
    )?;
    empty_tag(
        w,
        "hp:curSz",
        &[
            ("width", &shape.current_width.to_string()),
            ("height", &shape.current_height.to_string()),
        ],
    )?;
    empty_tag(
        w,
        "hp:flip",
        &[
            ("horizontal", bool01(shape.horz_flip)),
            ("vertical", bool01(shape.vert_flip)),
        ],
    )?;
    empty_tag(
        w,
        "hp:rotationInfo",
        &[
            ("angle", &shape.rotation_angle.to_string()),
            ("centerX", &shape.rotation_center.x.to_string()),
            ("centerY", &shape.rotation_center.y.to_string()),
            ("rotateimage", "1"),
        ],
    )?;
    start_tag(w, "hp:renderingInfo")?;
    // IR은 행렬의 합성 결과를 저장하므로 같은 아핀 변환과 두 identity를 쓴다.
    for (tag, matrix) in [
        (
            "hc:transMatrix",
            [
                shape.render_sx,
                shape.render_b,
                shape.render_tx,
                shape.render_c,
                shape.render_sy,
                shape.render_ty,
            ],
        ),
        ("hc:scaMatrix", [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        ("hc:rotMatrix", [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
    ] {
        if matrix.iter().any(|v| !v.is_finite()) {
            return Err(SerializeError::XmlError("유한하지 않은 shape 행렬".into()));
        }
        let values = matrix.map(|v| v.to_string());
        empty_tag(
            w,
            tag,
            &[
                ("e1", &values[0]),
                ("e2", &values[1]),
                ("e3", &values[2]),
                ("e4", &values[3]),
                ("e5", &values[4]),
                ("e6", &values[5]),
            ],
        )?;
    }
    end_tag(w, "hp:renderingInfo")
}

fn write_drawing<W: Write>(
    w: &mut Writer<W>,
    drawing: &DrawingObjAttr,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    write_line_shape(w, &drawing.border_line)?;
    match drawing.fill.fill_type {
        FillType::None => {}
        FillType::Solid => {
            let solid = drawing
                .fill
                .solid
                .as_ref()
                .ok_or_else(|| SerializeError::XmlError("단색 shape fill 누락".into()))?;

            start_tag(w, "hc:fillBrush")?;
            empty_tag(
                w,
                "hc:winBrush",
                &[
                    ("faceColor", &color_ref_hex(solid.background_color)),
                    ("hatchColor", &color_ref_hex(solid.pattern_color)),
                    (
                        "alpha",
                        &(f64::from(drawing.fill.alpha) / 255.0).to_string(),
                    ),
                ],
            )?;
            end_tag(w, "hc:fillBrush")?;
        }
        FillType::Gradient | FillType::Image => {
            return Err(SerializeError::XmlError(
                "지원하지 않는 shape 채우기 직렬화".into(),
            ))
        }
    }
    empty_tag(
        w,
        "hp:shadow",
        &[
            (
                "type",
                match drawing.shadow_type {
                    1 => "DROP",
                    2 => "PERSPECTIVE",
                    _ => "NONE",
                },
            ),
            ("color", &color_ref_hex(drawing.shadow_color)),
            ("offsetX", &drawing.shadow_offset_x.to_string()),
            ("offsetY", &drawing.shadow_offset_y.to_string()),
            ("alpha", &drawing.shadow_alpha.to_string()),
        ],
    )?;
    if let Some(text_box) = &drawing.text_box {
        start_tag_attrs(
            w,
            "hp:drawText",
            &[
                ("lastWidth", &text_box.max_width.to_string()),
                ("name", ""),
                ("editable", "0"),
            ],
        )?;
        let align = match text_box.vertical_align {
            crate::model::table::VerticalAlign::Top => "TOP",
            crate::model::table::VerticalAlign::Center => "CENTER",
            crate::model::table::VerticalAlign::Bottom => "BOTTOM",
        };
        write_text_sublist(w, &text_box.paragraphs, align, ctx)?;
        empty_tag(
            w,
            "hp:textMargin",
            &[
                ("left", &text_box.margin_left.to_string()),
                ("right", &text_box.margin_right.to_string()),
                ("top", &text_box.margin_top.to_string()),
                ("bottom", &text_box.margin_bottom.to_string()),
            ],
        )?;
        end_tag(w, "hp:drawText")?;
    }
    if let Some(caption) = &drawing.caption {
        write_caption(w, caption, ctx)?;
    }
    Ok(())
}

fn color_ref_hex(value: u32) -> String {
    if value == 0xFFFF_FFFF {
        "none".to_string()
    } else {
        format!(
            "#{:02X}{:02X}{:02X}",
            value & 0xFF,
            (value >> 8) & 0xFF,
            (value >> 16) & 0xFF
        )
    }
}

fn write_shape_comment<W: Write>(
    w: &mut Writer<W>,
    common: &CommonObjAttr,
) -> Result<(), SerializeError> {
    if !common.description.is_empty() {
        start_tag(w, "hp:shapeComment")?;
        super::utils::text(w, &common.description)?;
        end_tag(w, "hp:shapeComment")?;
    }
    Ok(())
}

fn write_line_shape<W: Write>(
    w: &mut Writer<W>,
    line: &ShapeBorderLine,
) -> Result<(), SerializeError> {
    let styles = [
        "NONE",
        "SOLID",
        "DASH",
        "DOT",
        "DASH_DOT",
        "DASH_DOT_DOT",
        "LONG_DASH",
        "CIRCLE",
        "DOUBLE_SLIM",
        "SLIM_THICK",
        "THICK_SLIM",
        "SLIM_THICK_SLIM",
    ];
    let style = styles
        .get(
            usize::try_from(line.attr & 0xFF)
                .map_err(|e| SerializeError::XmlError(e.to_string()))?,
        )
        .ok_or_else(|| SerializeError::XmlError("지원하지 않는 shape 선 종류".into()))?;
    let outline = match line.outline_style {
        1 => "OUTER",
        2 => "INNER",
        _ => "NORMAL",
    };
    empty_tag(
        w,
        "hp:lineShape",
        &[
            ("color", &format!("#{:06X}", line.color)),
            ("width", &line.width.to_string()),
            ("style", style),
            ("endCap", "FLAT"),
            ("headStyle", "NORMAL"),
            ("tailStyle", "NORMAL"),
            ("headfill", "1"),
            ("tailfill", "1"),
            ("headSz", "MEDIUM_MEDIUM"),
            ("tailSz", "MEDIUM_MEDIUM"),
            ("outlineStyle", outline),
            ("alpha", "0"),
        ],
    )
}

fn write_text_sublist<W: Write>(
    w: &mut Writer<W>,
    paragraphs: &[crate::model::paragraph::Paragraph],
    align: &str,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    start_tag_attrs(
        w,
        "hp:subList",
        &[
            ("id", ""),
            ("textDirection", "HORIZONTAL"),
            ("lineWrap", "BREAK"),
            ("vertAlign", align),
            ("linkListIDRef", "0"),
            ("linkListNextIDRef", "0"),
            ("textWidth", "0"),
            ("textHeight", "0"),
            ("hasTextRef", "0"),
            ("hasNumRef", "0"),
        ],
    )?;
    for (id, para) in paragraphs.iter().enumerate() {
        super::table::write_paragraph(w, para, id, ctx)?;
    }
    end_tag(w, "hp:subList")
}

pub(super) fn write_caption<W: Write>(
    w: &mut Writer<W>,
    caption: &Caption,
    ctx: &mut SerializeContext,
) -> Result<(), SerializeError> {
    let side = match caption.direction {
        CaptionDirection::Left => "LEFT",
        CaptionDirection::Right => "RIGHT",
        CaptionDirection::Top => "TOP",
        CaptionDirection::Bottom => "BOTTOM",
    };
    start_tag_attrs(
        w,
        "hp:caption",
        &[
            ("side", side),
            ("fullSz", bool01(caption.include_margin)),
            ("width", &caption.width.to_string()),
            ("gap", &caption.spacing.to_string()),
            ("lastWidth", &caption.max_width.to_string()),
        ],
    )?;
    let align = match caption.vert_align {
        crate::model::shape::CaptionVertAlign::Top => "TOP",
        crate::model::shape::CaptionVertAlign::Center => "CENTER",
        crate::model::shape::CaptionVertAlign::Bottom => "BOTTOM",
    };
    write_text_sublist(w, &caption.paragraphs, align, ctx)?;
    end_tag(w, "hp:caption")
}

// =====================================================================
// 공통 자식 요소 (sz / pos / outMargin)
// =====================================================================

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

fn write_out_margin<W: Write>(w: &mut Writer<W>, c: &CommonObjAttr) -> Result<(), SerializeError> {
    let l = c.margin.left.to_string();
    let r = c.margin.right.to_string();
    let t = c.margin.top.to_string();
    let b = c.margin.bottom.to_string();
    empty_tag(
        w,
        "hp:outMargin",
        &[("left", &l), ("right", &r), ("top", &t), ("bottom", &b)],
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::shape::{LineShape, RectangleShape};
    use crate::model::Point;

    fn serialize_rect(rect: &RectangleShape) -> String {
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        write_rect(&mut w, rect, &mut SerializeContext::default()).expect("write_rect");
        String::from_utf8(w.into_inner()).unwrap()
    }

    fn serialize_line(line: &LineShape) -> String {
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        write_line(&mut w, line, &mut SerializeContext::default()).expect("write_line");
        String::from_utf8(w.into_inner()).unwrap()
    }

    #[test]
    fn rect_emits_root_tag() {
        let mut rect = RectangleShape::default();
        rect.common.width = 1000;
        rect.common.height = 500;
        let xml = serialize_rect(&rect);
        assert!(xml.contains("<hp:rect "));
        assert!(xml.contains("</hp:rect>"));
    }

    #[test]
    fn rect_has_canonical_attrs() {
        let rect = RectangleShape::default();
        let xml = serialize_rect(&rect);
        assert!(xml.contains(r#"id=""#));
        assert!(xml.contains(r#"zOrder=""#));
        assert!(xml.contains(r#"textWrap=""#));
        assert!(xml.contains(r#"textFlow="BOTH_SIDES""#));
    }

    #[test]
    fn line_emits_start_end_attrs() {
        let mut line = LineShape::default();
        line.start = Point { x: 100, y: 200 };
        line.end = Point { x: 300, y: 400 };
        let xml = serialize_line(&line);
        assert!(xml.contains(r#"startX="100""#));
        assert!(xml.contains(r#"startY="200""#));
        assert!(xml.contains(r#"endX="300""#));
        assert!(xml.contains(r#"endY="400""#));
    }

    #[test]
    fn rect_has_sz_pos_out_margin() {
        let rect = RectangleShape::default();
        let xml = serialize_rect(&rect);
        assert!(xml.contains("<hp:sz "));
        assert!(xml.contains("<hp:pos "));
        assert!(xml.contains("<hp:outMargin "));
    }
}
