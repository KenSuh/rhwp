//! HWPX 파서 공통 유틸리티 함수
//!
//! header.rs, section.rs 등에서 공통으로 사용하는 XML 파싱 헬퍼.

use quick_xml::events::Event;
use quick_xml::Reader;

use super::HwpxError;

/// XML 네임스페이스 접두사를 제거하고 로컬 이름만 반환
/// 예: b"hp:p" → b"p", b"tbl" → b"tbl"
pub fn local_name(name: &[u8]) -> &[u8] {
    if let Some(pos) = name.iter().position(|&b| b == b':') {
        &name[pos + 1..]
    } else {
        name
    }
}

/// 속성 값을 String으로 변환 (XML 엔티티 unescape 포함).
///
/// quick-xml 은 속성값을 **원문 그대로**(`&amp;` 등 이스케이프된 형태) 보관한다.
/// IR 은 시맨틱 값(예: `R&D`)을 저장해야 하고, 직렬화 측 writer 가 유일한 escape
/// 권위가 되어야 대칭이 성립한다. unescape 를 빠뜨리면 폼 caption 등 문자열 속성이
/// 저장할 때마다 한 겹씩 이중 이스케이프된다(`R&&D`→`R&amp;&amp;D`→…, Task #1534).
///
/// 숫자/열거형 속성은 엔티티가 없어 unescape 가 no-op 이라 무영향. 미정의 엔티티/
/// malformed 입력은 원문 lossy 로 안전 폴백한다.
pub fn attr_str(attr: &quick_xml::events::attributes::Attribute) -> String {
    let raw = String::from_utf8_lossy(&attr.value);
    match quick_xml::escape::unescape(&raw) {
        Ok(value) => value.into_owned(),
        Err(_) => raw.into_owned(),
    }
}

/// 속성 값이 특정 문자열과 일치하는지 확인 (비교용)
pub fn attr_eq(attr: &quick_xml::events::attributes::Attribute, val: &str) -> bool {
    attr.value.as_ref() == val.as_bytes()
}

pub fn parse_u8(attr: &quick_xml::events::attributes::Attribute) -> u8 {
    attr_str(attr).parse().unwrap_or(0)
}

pub fn parse_i8(attr: &quick_xml::events::attributes::Attribute) -> i8 {
    attr_str(attr).parse().unwrap_or(0)
}

pub fn parse_u16(attr: &quick_xml::events::attributes::Attribute) -> u16 {
    attr_str(attr).parse().unwrap_or(0)
}

pub fn parse_i16(attr: &quick_xml::events::attributes::Attribute) -> i16 {
    attr_str(attr).parse().unwrap_or(0)
}

pub fn parse_u32(attr: &quick_xml::events::attributes::Attribute) -> u32 {
    attr_str(attr).parse().unwrap_or(0)
}

pub fn parse_i32(attr: &quick_xml::events::attributes::Attribute) -> i32 {
    attr_str(attr).parse().unwrap_or(0)
}

/// "#RRGGBB" 또는 "#AARRGGBB" 형식의 색상을 HWP ColorRef(0x00BBGGRR)로 변환
pub fn parse_color(attr: &quick_xml::events::attributes::Attribute) -> u32 {
    let s = attr_str(attr);
    parse_color_str(&s)
}

/// 색상 문자열을 HWP ColorRef로 변환
pub fn parse_color_str(s: &str) -> u32 {
    if s == "none" || s.is_empty() {
        return 0xFFFFFFFF; // 투명/없음
    }
    let hex = s.trim_start_matches('#');
    if hex.len() == 6 {
        // RRGGBB → 0x00BBGGRR
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            let r = (v >> 16) & 0xFF;
            let g = (v >> 8) & 0xFF;
            let b = v & 0xFF;
            return b << 16 | g << 8 | r;
        }
    } else if hex.len() == 8 {
        // AARRGGBB → 0x00BBGGRR (alpha 무시)
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            let r = (v >> 16) & 0xFF;
            let g = (v >> 8) & 0xFF;
            let b = v & 0xFF;
            return b << 16 | g << 8 | r;
        }
    }
    0x00000000 // 검정
}

/// 속성 값을 bool로 파싱 ("true", "1" → true)
pub fn parse_bool(attr: &quick_xml::events::attributes::Attribute) -> bool {
    let s = attr_str(attr);
    s == "true" || s == "1"
}

/// XML 요소를 자식 포함하여 건너뛰기 (깊이 추적)
pub fn skip_element(reader: &mut Reader<&[u8]>, _end_tag: &[u8]) -> Result<(), HwpxError> {
    let mut buf = Vec::new();
    let mut depth = 1u32;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(_)) => {
                depth += 1;
            }
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(HwpxError::XmlError(format!("skip: {}", e))),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_name() {
        assert_eq!(local_name(b"hp:p"), b"p");
        assert_eq!(local_name(b"tbl"), b"tbl");
        assert_eq!(local_name(b"hh:charPr"), b"charPr");
    }

    #[test]
    fn test_parse_color_str() {
        assert_eq!(parse_color_str("#FF0000"), 0x000000FF); // 빨강 → R=FF → BGR=0000FF
        assert_eq!(parse_color_str("#00FF00"), 0x0000FF00); // 초록
        assert_eq!(parse_color_str("#0000FF"), 0x00FF0000); // 파랑
        assert_eq!(parse_color_str("#000000"), 0x00000000); // 검정
        assert_eq!(parse_color_str("none"), 0xFFFFFFFF); // 투명
    }

    #[test]
    fn test_parse_color_str_with_alpha() {
        // AARRGGBB — alpha 무시
        assert_eq!(parse_color_str("#80FF0000"), 0x000000FF);
    }

    /// `<x a="…"/>` 의 첫 속성을 실제 XML 파서로 읽어 `f` 에 넘긴다.
    fn with_first_attr<T>(
        xml: &str,
        f: impl Fn(&quick_xml::events::attributes::Attribute) -> T,
    ) -> T {
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event().expect("well-formed test XML") {
                Event::Empty(e) | Event::Start(e) => {
                    let attr = e
                        .attributes()
                        .next()
                        .expect("test element has an attribute")
                        .expect("well-formed attribute");
                    return f(&attr);
                }
                Event::Eof => panic!("test XML has no element"),
                _ => {}
            }
        }
    }

    #[test]
    fn attr_str_decodes_xml_entities_exactly_once() {
        let cases = [
            // form-002.hwpx 의 실제 checkBtn caption
            (r#"<x caption="IP R&amp;&amp;D연계"/>"#, "IP R&&D연계"),
            (
                r#"<x caption="&lt;Say &quot;hi&quot;&gt; &apos;now&apos;"/>"#,
                "<Say \"hi\"> 'now'",
            ),
            (r#"<x caption="&#38;&#x26;&#xAC00;&#45208;"/>"#, "&&가나"),
            // 원문에 글자 그대로 들어 있는 "&amp;" 는 한 번만 풀려 "&amp;" 로 남는다
            (
                r#"<x caption="literal &amp;amp; stays"/>"#,
                "literal &amp; stays",
            ),
            (r#"<x name="CheckBox14"/>"#, "CheckBox14"),
        ];
        for (xml, expected) in cases {
            assert_eq!(with_first_attr(xml, attr_str), expected, "{}", xml);
        }
    }

    #[test]
    fn attr_str_keeps_raw_text_for_malformed_or_unknown_entities() {
        // 기존 계약: attr_str 은 실패하지 않는다. 풀 수 없는 값은 받은 그대로 둔다
        // (본문 `decode_text` 의 실패 시 원문 유지와 같은 규칙).
        for raw in ["A &bogus; B", "R&D", "50 &#xZZ; off", "tail &amp"] {
            let xml = format!(r#"<x caption="{}"/>"#, raw);
            assert_eq!(with_first_attr(&xml, attr_str), raw, "{}", xml);
        }
    }

    #[test]
    fn typed_attr_parsers_read_the_decoded_value() {
        assert_eq!(with_first_attr(r#"<x width="1&#50;"/>"#, parse_i32), 12);
        assert_eq!(with_first_attr(r#"<x width="&#52;&#50;"/>"#, parse_u16), 42);
        assert_eq!(
            with_first_attr(r#"<x color="&#35;FF0000"/>"#, parse_color),
            0x000000FF
        );
        assert!(with_first_attr(r#"<x enabled="&#49;"/>"#, parse_bool));
    }
}
