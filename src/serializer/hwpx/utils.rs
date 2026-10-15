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
    append_attributes(&mut el, attrs);
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
    append_attributes(&mut el, attrs);
    w.write_event(Event::Empty(el))
        .map_err(|e| SerializeError::XmlError(e.to_string()))?;
    Ok(())
}

fn append_attributes(el: &mut BytesStart<'_>, attrs: &[(&str, &str)]) {
    for (key, value) in attrs {
        let mut escaped_value = quick_xml::escape::escape(*value);
        if escaped_value.contains(['\t', '\n', '\r']) {
            escaped_value = escaped_value
                .replace('\t', "&#9;")
                .replace('\n', "&#10;")
                .replace('\r', "&#13;")
                .into();
        }
        el.push_attribute((key.as_bytes(), escaped_value.as_bytes()));
    }
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
