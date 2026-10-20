//! 필드 컨트롤의 시작/끝 속성과 형식별 매개변수를 XML 이벤트로 직렬화한다.

#![allow(dead_code)]

use std::io::Write;

use quick_xml::Writer;

use crate::model::control::{
    Bookmark, Field, FieldParameter, FieldParameterKind, FieldType, Hyperlink,
};

use super::utils::{empty_tag, end_tag, start_tag, start_tag_attrs, text};
use super::SerializeError;

// =====================================================================
// <hp:bookmark>
// =====================================================================

pub fn write_bookmark<W: Write>(w: &mut Writer<W>, bm: &Bookmark) -> Result<(), SerializeError> {
    empty_tag(w, "hp:bookmark", &[("name", &bm.name)])
}

// =====================================================================
// <hp:fieldBegin> / <hp:fieldEnd>
// =====================================================================

/// `<hp:fieldBegin>` — 필드 시작 마커.
///
/// HWPX 필드는 텍스트 흐름 안에서 `<hp:fieldBegin>` ~ 텍스트 ~ `<hp:fieldEnd>` 쌍으로 표현된다.
pub fn write_field_begin<W: Write>(w: &mut Writer<W>, field: &Field) -> Result<(), SerializeError> {
    if let Some(hwpx) = &field.hwpx {
        let mut attributes = hwpx.begin_attributes.clone();
        for (key, value) in &mut attributes {
            match key.as_str() {
                "name" => *value = field.ctrl_data_name.as_deref().unwrap_or("").to_string(),
                "editable" => *value = bool01(field.is_editable_in_form()).to_string(),
                _ => {}
            }
        }
        if !attributes.iter().any(|(key, _)| key == "editable") && field.is_editable_in_form() {
            attributes.push(("editable".into(), "1".into()));
        }
        if !attributes.iter().any(|(key, _)| key == "name") {
            if let Some(name) = field
                .ctrl_data_name
                .as_deref()
                .filter(|name| !name.is_empty())
            {
                attributes.push(("name".into(), name.into()));
            }
        }
        let attrs = field_attributes(&attributes, true)?;
        start_tag_attrs(w, "hp:fieldBegin", &attrs)?;
        let items = hwpx
            .parameters
            .as_ref()
            .map(|p| p.items.as_slice())
            .unwrap_or(&[]);
        let has_command = items
            .iter()
            .any(|p| p.name == "Command" && p.kind == FieldParameterKind::String);
        let original_name = hwpx
            .begin_attributes
            .iter()
            .find(|(key, _)| key == "name")
            .map(|(_, value)| value.as_str())
            .unwrap_or("");
        let add_command = !has_command && field.command != original_name;
        if hwpx.parameters.is_some() || add_command {
            let name = hwpx
                .parameters
                .as_ref()
                .map(|p| p.name.as_str())
                .unwrap_or("");
            validate_field_text(name)?;
            start_tag_attrs(
                w,
                "hp:parameters",
                &[
                    ("cnt", &(items.len() + usize::from(add_command)).to_string()),
                    ("name", name),
                ],
            )?;
            for parameter in items {
                write_parameter(w, parameter, Some(&field.command))?;
            }
            if add_command {
                write_command(w, &field.command)?;
            }
            end_tag(w, "hp:parameters")?;
        }
        return end_tag(w, "hp:fieldBegin");
    }
    let id_str = field.field_id.to_string();
    let ft = field_type_str(field.field_type);
    validate_field_text(field.ctrl_data_name.as_deref().unwrap_or(""))?;
    start_tag_attrs(
        w,
        "hp:fieldBegin",
        &[
            ("id", &id_str),
            ("fieldid", &id_str),
            ("type", ft),
            ("name", field.ctrl_data_name.as_deref().unwrap_or("")),
            ("editable", bool01(field.is_editable_in_form())),
        ],
    )?;
    if !field.command.is_empty() {
        start_tag_attrs(w, "hp:parameters", &[("cnt", "1"), ("name", "")])?;
        write_command(w, &field.command)?;
        end_tag(w, "hp:parameters")?;
    }
    end_tag(w, "hp:fieldBegin")
}

fn write_command<W: Write>(w: &mut Writer<W>, command: &str) -> Result<(), SerializeError> {
    validate_field_text(command)?;
    start_tag_attrs(w, "hp:stringParam", &[("name", "Command")])?;
    text(w, command)?;
    end_tag(w, "hp:stringParam")
}

fn validate_field_text(value: &str) -> Result<(), SerializeError> {
    if value.chars().any(|c| {
        (c < ' ' && !matches!(c, '\t' | '\n' | '\r')) || matches!(c, '\u{FFFE}' | '\u{FFFF}')
    }) {
        return Err(SerializeError::XmlError(
            "XML 1.0에서 허용하지 않는 field 문자".into(),
        ));
    }
    Ok(())
}

fn field_attributes(
    attrs: &[(String, String)],
    begin: bool,
) -> Result<Vec<(&str, &str)>, SerializeError> {
    attrs
        .iter()
        .map(|(key, value)| {
            let allowed = if begin {
                matches!(
                    key.as_str(),
                    "id" | "type" | "name" | "editable" | "dirty" | "zorder" | "fieldid"
                )
            } else {
                matches!(key.as_str(), "beginIDRef" | "fieldid")
            };
            if !allowed {
                return Err(SerializeError::XmlError(format!(
                    "알 수 없는 field 속성: {key}"
                )));
            }
            validate_field_text(value)?;
            Ok((key.as_str(), value.as_str()))
        })
        .collect()
}

fn write_parameter<W: Write>(
    w: &mut Writer<W>,
    parameter: &FieldParameter,
    command: Option<&str>,
) -> Result<(), SerializeError> {
    validate_field_text(&parameter.name)?;
    validate_field_text(&parameter.value)?;
    if let Some(command) = command {
        validate_field_text(command)?;
    }
    let tag = match parameter.kind {
        FieldParameterKind::Boolean => "hp:booleanParam",
        FieldParameterKind::Integer => "hp:integerParam",
        FieldParameterKind::Float => "hp:floatParam",
        FieldParameterKind::String => "hp:stringParam",
        FieldParameterKind::List => "hp:listParam",
    };
    let count = parameter.items.len().to_string();
    let mut attrs = vec![("name", parameter.name.as_str())];
    if parameter.kind == FieldParameterKind::List {
        attrs.push(("cnt", &count));
    }
    start_tag_attrs(w, tag, &attrs)?;
    if parameter.kind == FieldParameterKind::List {
        for item in &parameter.items {
            write_parameter(w, item, None)?;
        }
    } else {
        let value = if parameter.kind == FieldParameterKind::String && parameter.name == "Command" {
            command.unwrap_or(&parameter.value)
        } else {
            &parameter.value
        };
        text(w, value)?;
    }
    end_tag(w, tag)
}

pub(super) fn write_field_end_for_field<W: Write>(
    w: &mut Writer<W>,
    field: &Field,
) -> Result<(), SerializeError> {
    if let Some(hwpx) = &field.hwpx {
        let attributes = hwpx
            .end_attributes
            .as_ref()
            .ok_or_else(|| SerializeError::XmlError("fieldEnd가 없는 HWPX fieldBegin".into()))?;
        empty_tag(w, "hp:fieldEnd", &field_attributes(attributes, false)?)
    } else {
        write_field_end(w, field.field_id)
    }
}

/// `<hp:fieldEnd>` — 필드 끝 마커.
pub fn write_field_end<W: Write>(w: &mut Writer<W>, field_id: u32) -> Result<(), SerializeError> {
    let id_str = field_id.to_string();
    empty_tag(w, "hp:fieldEnd", &[("beginIDRef", &id_str)])
}

// =====================================================================
// 하이퍼링크 (필드의 특수형) — <hp:fieldBegin type="HYPERLINK"> 변형
// =====================================================================

pub fn write_hyperlink_begin<W: Write>(
    w: &mut Writer<W>,
    link: &Hyperlink,
    field_id: u32,
) -> Result<(), SerializeError> {
    // command 에 URL 이 들어감. 실제 한컴은 별도 command 파싱 필요.
    let id_str = field_id.to_string();
    let url = &link.url;
    empty_tag(
        w,
        "hp:fieldBegin",
        &[
            ("id", &id_str),
            ("type", "HYPERLINK"),
            ("name", ""),
            ("editable", "0"),
            ("command", url),
        ],
    )
}

// =====================================================================
// 각주 / 미주 뼈대 — <hp:fn> / <hp:en>
// =====================================================================

/// `<hp:fn>` 각주 뼈대 (내부 문단 직렬화는 #186 에서 연결).
pub fn write_footnote_open<W: Write>(w: &mut Writer<W>, number: u16) -> Result<(), SerializeError> {
    let n = number.to_string();
    start_tag(w, "hp:fn")?;
    empty_tag(w, "hp:autoNum", &[("num", &n)])?;
    Ok(())
}

pub fn write_footnote_close<W: Write>(w: &mut Writer<W>) -> Result<(), SerializeError> {
    end_tag(w, "hp:fn")
}

pub fn write_endnote_open<W: Write>(w: &mut Writer<W>, number: u16) -> Result<(), SerializeError> {
    let n = number.to_string();
    start_tag(w, "hp:en")?;
    empty_tag(w, "hp:autoNum", &[("num", &n)])?;
    Ok(())
}

pub fn write_endnote_close<W: Write>(w: &mut Writer<W>) -> Result<(), SerializeError> {
    end_tag(w, "hp:en")
}

// =====================================================================
// 헬퍼
// =====================================================================

fn bool01(b: bool) -> &'static str {
    if b {
        "1"
    } else {
        "0"
    }
}

fn field_type_str(t: FieldType) -> &'static str {
    use FieldType::*;
    match t {
        Unknown => "UNKNOWN",
        Date => "DATE",
        DocDate => "DOCDATE",
        Path => "PATH",
        Bookmark => "BOOKMARK",
        MailMerge => "MAILMERGE",
        CrossRef => "CROSSREF",
        Formula => "FORMULA",
        ClickHere => "CLICKHERE",
        Summary => "SUMMARY",
        UserInfo => "USERINFO",
        Hyperlink => "HYPERLINK",
        Memo => "MEMO",
        PrivateInfoSecurity => "PRIVATE_INFO",
        TableOfContents => "TOC",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::control::{Bookmark, Field, FieldType, Hyperlink};

    fn to_string<F: FnOnce(&mut Writer<Vec<u8>>) -> Result<(), SerializeError>>(f: F) -> String {
        let mut w: Writer<Vec<u8>> = Writer::new(Vec::new());
        f(&mut w).expect("write");
        String::from_utf8(w.into_inner()).unwrap()
    }

    #[test]
    fn bookmark_emits_name() {
        let bm = Bookmark {
            name: "chapter1".to_string(),
        };
        let xml = to_string(|w| write_bookmark(w, &bm));
        assert!(xml.contains(r#"<hp:bookmark name="chapter1"/>"#), "{}", xml);
    }

    #[test]
    fn field_begin_emits_type_attr() {
        let mut f = Field::default();
        f.field_type = FieldType::ClickHere;
        f.field_id = 42;
        let xml = to_string(|w| write_field_begin(w, &f));
        assert!(xml.contains(r#"id="42""#));
        assert!(xml.contains(r#"type="CLICKHERE""#));
    }

    #[test]
    fn field_end_references_begin_id() {
        let xml = to_string(|w| write_field_end(w, 42));
        assert!(xml.contains(r#"<hp:fieldEnd beginIDRef="42"/>"#));
    }

    #[test]
    fn hyperlink_begin_uses_url_command() {
        let link = Hyperlink {
            url: "https://example.com".to_string(),
            text: "".to_string(),
        };
        let xml = to_string(|w| write_hyperlink_begin(w, &link, 7));
        assert!(xml.contains(r#"type="HYPERLINK""#));
        assert!(xml.contains(r#"command="https://example.com""#));
    }

    #[test]
    fn footnote_emits_autoNum() {
        let xml = to_string(|w| {
            write_footnote_open(w, 3)?;
            write_footnote_close(w)
        });
        assert!(xml.contains("<hp:fn>"));
        assert!(xml.contains(r#"<hp:autoNum num="3"/>"#));
        assert!(xml.contains("</hp:fn>"));
    }

    #[test]
    fn field_type_str_covers_main_variants() {
        assert_eq!(field_type_str(FieldType::Hyperlink), "HYPERLINK");
        assert_eq!(field_type_str(FieldType::Bookmark), "BOOKMARK");
        assert_eq!(field_type_str(FieldType::Date), "DATE");
        assert_eq!(field_type_str(FieldType::TableOfContents), "TOC");
    }
}
