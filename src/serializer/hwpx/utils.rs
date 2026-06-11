//! HWPX 직렬화 공용 헬퍼 — XML escape / 공통 이벤트 쓰기

use std::io::Write;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use super::SerializeError;

/// `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` 선언을 쓴다.
pub fn write_xml_decl<W: Write>(w: &mut Writer<W>) -> Result<(), SerializeError> {
    w.write_event(Event::Decl(BytesDecl::new(
        "1.0",
        Some("UTF-8"),
        Some("yes"),
    )))
    .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

/// 속성 없는 시작 태그
pub fn start_tag<W: Write>(w: &mut Writer<W>, name: &str) -> Result<(), SerializeError> {
    w.write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

/// 속성 있는 시작 태그
pub fn start_tag_attrs<W: Write>(
    w: &mut Writer<W>,
    name: &str,
    attrs: &[(&str, &str)],
) -> Result<(), SerializeError> {
    let mut el = BytesStart::new(name);
    for (k, v) in attrs {
        el.push_attribute((*k, *v));
    }
    w.write_event(Event::Start(el))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

/// 종료 태그
pub fn end_tag<W: Write>(w: &mut Writer<W>, name: &str) -> Result<(), SerializeError> {
    w.write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

/// 자기 닫힘 태그 (`<name a="..."/>`)
pub fn empty_tag<W: Write>(
    w: &mut Writer<W>,
    name: &str,
    attrs: &[(&str, &str)],
) -> Result<(), SerializeError> {
    let mut el = BytesStart::new(name);
    for (k, v) in attrs {
        el.push_attribute((*k, *v));
    }
    w.write_event(Event::Empty(el))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

/// 텍스트 노드 (자동 이스케이프)
pub fn text<W: Write>(w: &mut Writer<W>, content: &str) -> Result<(), SerializeError> {
    w.write_event(Event::Text(BytesText::new(content)))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

/// XML 속성·텍스트 이스케이프 (&, <, >, ", ')
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

// ---- charPr run 분할 공용 로직 (셀 write_cell_text_runs · 본문 render_body_runs 공유) ----

use crate::model::paragraph::CharShapeRef;

/// 문단 텍스트를 charPr run 경계로 분할해 `(char_shape_id, 조각 텍스트)` 목록과
/// 텍스트 끝 UTF-16 위치를 돌려준다. 오프셋 규약은 파서와 동일하다:
/// `char_offsets` 가 있으면 그대로 쓰고, 없으면 탭=8 유닛(HWP LINE_SEG 규약)으로 누적한다.
pub(crate) fn split_char_shape_runs(
    text: &str,
    char_offsets: &[u32],
    char_shapes: &[CharShapeRef],
) -> (Vec<(u32, String)>, u32) {
    let mut runs: Vec<(u32, String)> = Vec::new();
    let mut current_shape: Option<u32> = None;
    let mut current_text = String::new();
    let mut fallback_utf16_pos = 0u32;

    for (idx, ch) in text.chars().enumerate() {
        let utf16_pos = char_offsets.get(idx).copied().unwrap_or(fallback_utf16_pos);
        let shape_id = active_char_shape_id(char_shapes, utf16_pos);
        fallback_utf16_pos = utf16_pos + char_utf16_len(ch);

        if current_shape.is_some_and(|existing| existing != shape_id) {
            runs.push((
                current_shape.unwrap_or(0),
                std::mem::take(&mut current_text),
            ));
        }
        current_shape = Some(shape_id);
        current_text.push(ch);
    }
    if let Some(shape) = current_shape {
        runs.push((shape, current_text));
    }
    (runs, fallback_utf16_pos)
}

/// 텍스트 끝(`text_end_utf16`) 이후에 시작하는 zero-length charPr run id 목록.
/// 실제 한컴 파일의 trailing `<hp:run charPrIDRef="N"/>`(문단말 캐럿 스타일) 보존용.
/// `skip_first` 는 빈 문단 경로용 — 첫 ref 는 본문 run 으로 이미 출력된 경우 제외한다.
pub(crate) fn trailing_zero_length_refs(
    char_shapes: &[CharShapeRef],
    text_end_utf16: u32,
    skip_first: bool,
) -> Vec<u32> {
    char_shapes
        .iter()
        .enumerate()
        .filter(|(i, shape)| !(skip_first && *i == 0) && shape.start_pos >= text_end_utf16)
        .map(|(_, shape)| shape.char_shape_id)
        .collect()
}

/// `utf16_pos` 시점에 활성인 charPr id — `start_pos <= pos` 인 마지막 ref (정렬 가정).
pub(crate) fn active_char_shape_id(char_shapes: &[CharShapeRef], utf16_pos: u32) -> u32 {
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

pub(crate) fn first_char_shape_id(char_shapes: &[CharShapeRef]) -> u32 {
    char_shapes.first().map(|r| r.char_shape_id).unwrap_or(0)
}

/// HWP LINE_SEG / char_offsets 규약의 문자 폭 — 탭은 확장 데이터 포함 8 유닛.
pub(crate) fn char_utf16_len(ch: char) -> u32 {
    if ch == '\t' {
        8
    } else {
        ch.len_utf16() as u32
    }
}
