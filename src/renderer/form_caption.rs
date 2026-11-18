// 양식 컨트롤 caption 표시 helper — 모델 값은 그대로 두고 그릴 때만 `&&` 를 `&` 하나로 접는다.
//! Form control caption display helpers.
//!
//! HWPX stores form captions with UI-caption escaping semantics. In observed
//! Hancom output, `&&` in a form caption displays as one literal `&`, while the
//! stored value and XML roundtrip must remain unchanged.

use std::borrow::Cow;

pub(crate) fn display_form_caption(caption: &str) -> Cow<'_, str> {
    if !caption.contains("&&") {
        return Cow::Borrowed(caption);
    }

    let mut out = String::with_capacity(caption.len());
    let mut chars = caption.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' && chars.peek() == Some(&'&') {
            chars.next();
            out.push('&');
        } else {
            out.push(ch);
        }
    }

    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::display_form_caption;
    use std::borrow::Cow;

    #[test]
    fn collapses_double_ampersand_for_form_caption_display() {
        assert_eq!(display_form_caption("R&&D"), "R&D");
        assert_eq!(display_form_caption("IP R&&D연계"), "IP R&D연계");
        assert_eq!(
            display_form_caption("R&&D 자율성트랙(일반)"),
            "R&D 자율성트랙(일반)"
        );
        assert_eq!(display_form_caption("&&&&"), "&&");
    }

    #[test]
    fn preserves_single_ampersand() {
        assert_eq!(display_form_caption("R&D"), "R&D");
        assert_eq!(display_form_caption("A&B&C"), "A&B&C");
    }

    #[test]
    fn borrows_when_no_display_escape_exists() {
        assert!(matches!(
            display_form_caption("plain caption"),
            Cow::Borrowed("plain caption")
        ));
    }
}

#[cfg(test)]
mod display_cases {
    use super::display_form_caption;
    use std::borrow::Cow;

    /// (모델 caption, 기대 표시) — 기대값은 전부 문자열 리터럴이다 (helper 로 계산하지 않는다).
    const CASES: &[(&str, &str)] = &[
        ("IP R&&D연계", "IP R&D연계"),
        ("R&&D 자율성트랙(일반)", "R&D 자율성트랙(일반)"),
        ("", ""),
        ("평문 캡션", "평문 캡션"),
        ("A&&B&&C", "A&B&C"),
        ("&&&&", "&&"),
        ("R&amp;&amp;D", "R&amp;&amp;D"),
        ("<&&>", "<&>"),
        ("&&&", "&&"),
        ("A&B", "A&B"),
        ("A&B&C", "A&B&C"),
        ("R&&D", "R&D"),
    ];

    #[test]
    fn display_cases_match_literal_expectations() {
        for (caption, expected) in CASES {
            assert_eq!(display_form_caption(caption), *expected, "{:?}", caption);
        }
        assert!(matches!(
            display_form_caption("평문 캡션"),
            Cow::Borrowed("평문 캡션")
        ));
        println!(
            "FORM_CAPTION_CASES n={} literal={}",
            CASES.len(),
            CASES.len()
        );
    }
}
