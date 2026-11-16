//! 문단을 건너 끝나는 HWPX 필드의 열기·저장→재열기 왕복과 짝 없는 fieldEnd 거부를 검증한다.
//!
//! 필드 짝은 같은 문단 목록(본문·셀·글상자) 범위의 스택으로 판정한다. T5 는 한컴이 만든 결재 양식으로
//! `fieldBegin`(누름틀 «본문») 이 앞 문단에, `fieldEnd` 가 구역 마지막 문단에 있다.

use quick_xml::events::Event;
use quick_xml::Reader;
use rhwp::model::control::Control;
use rhwp::parser::hwpx::parse_hwpx;
use rhwp::serializer::hwpx::serialize_hwpx;

const T5: &[u8] = include_bytes!("fixtures/table-fidelity/T5_36395325_gyeoljae_consulting.hwpx");

fn fixture(body: &str) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let template = include_bytes!("../samples/hwpx/ref/ref_empty.hwpx");
    let mut original = zip::ZipArchive::new(Cursor::new(template)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let section = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core">{body}</hs:sec>"#
    );
    for i in 0..original.len() {
        let mut entry = original.by_index(i).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name == "Contents/section0.xml" {
            bytes = section.as_bytes().to_vec();
        }
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
    output.finish().unwrap().into_inner()
}

fn section_xml(hwpx: &[u8]) -> String {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("Contents/section0.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

/// 필드 경계와 문단 경계·글자를 문서 순서로 모은다.
/// `P(` / `)P` 는 문단 열림·닫힘, `B{id}` / `E{id}` 는 필드 시작·끝, 그 밖은 글자다.
/// 각 경계는 원문 속성 전부(`B`/`E` 뒤 괄호)를 함께 적어 속성 보존도 같이 본다.
fn field_tokens(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut out = Vec::new();
    let mut in_text = 0usize;
    let mut buf = Vec::new();
    let attrs = |e: &quick_xml::events::BytesStart| {
        let mut v: Vec<String> = e
            .attributes()
            .flatten()
            .map(|a| {
                format!(
                    "{}={}",
                    String::from_utf8_lossy(a.key.as_ref()),
                    String::from_utf8_lossy(&a.value)
                )
            })
            .collect();
        v.sort();
        v.join(",")
    };
    loop {
        match reader.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => match e.local_name().as_ref() {
                b"p" => out.push("P(".into()),
                b"t" => in_text += 1,
                b"fieldBegin" => out.push(format!("B[{}]", attrs(&e))),
                _ => {}
            },
            Event::Empty(e) => match e.local_name().as_ref() {
                b"fieldBegin" => out.push(format!("B[{}]", attrs(&e))),
                b"fieldEnd" => out.push(format!("E[{}]", attrs(&e))),
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                b"p" => out.push(")P".into()),
                b"t" => in_text -= 1,
                _ => {}
            },
            ev @ (Event::Text(_) | Event::GeneralRef(_)) if in_text > 0 => {
                // quick-xml 은 `&apos;` 같은 엔티티 참조를 Text 와 따로(GeneralRef) 준다 — 글자로 풀어 잇는다.
                let s = match ev {
                    Event::Text(t) => t.decode().unwrap().into_owned(),
                    Event::GeneralRef(r) => match r.decode().unwrap().as_ref() {
                        "apos" => "'".into(),
                        "quot" => "\"".into(),
                        "amp" => "&".into(),
                        "lt" => "<".into(),
                        "gt" => ">".into(),
                        other => format!("&{other};"),
                    },
                    _ => unreachable!(),
                };
                if !s.is_empty() {
                    match out.last_mut() {
                        Some(last) if last.starts_with('"') => {
                            last.pop();
                            last.push_str(&s);
                            last.push('"');
                        }
                        _ => out.push(format!("\"{s}\"")),
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

/// 필드 경계·문단 경계만 남긴다(글자 제외).
fn boundaries(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| !t.starts_with('"'))
        .cloned()
        .collect()
}

/// `B...` 부터 짝 `E...` 까지 사이의 글자를 이어 붙인다(필드 내용).
/// 공백 문자는 뺀다 — 특수 공백 요소(`hp:fwSpace` 등)가 저장 뒤 같은 공백 글자로 바뀌는 것은 필드와 무관한
/// 기존 표현 정규화이고, v374 조건 1 판정도 공백 표현 차이를 내용 손실로 세지 않는다.
fn field_content(tokens: &[String]) -> String {
    let start = tokens.iter().position(|t| t.starts_with('B')).unwrap();
    let end = tokens.iter().position(|t| t.starts_with('E')).unwrap();
    tokens[start..end]
        .iter()
        .filter(|t| t.starts_with('"'))
        .flat_map(|t| t.trim_matches('"').chars())
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn field_count(doc: &rhwp::model::document::Document) -> usize {
    doc.sections
        .iter()
        .flat_map(|s| s.paragraphs.iter())
        .flat_map(|p| p.controls.iter())
        .filter(|c| matches!(c, Control::Field(_)))
        .count()
}

#[test]
fn t5_opens_through_parser_and_document_core() {
    let doc = parse_hwpx(T5).expect("T5 must open: its fieldBegin/fieldEnd pair spans paragraphs");
    assert_eq!(field_count(&doc), 1);
    rhwp::DocumentCore::from_bytes(T5).expect("T5 must open through DocumentCore");
}

/// 원본 T5 그대로 저장→재열기→재저장한다(두 번째 그림은 `binaryItemIDRef=""` 그림 없는 그림 개체).
#[test]
fn t5_cross_paragraph_field_round_trips() {
    let doc = parse_hwpx(T5).expect("T5 open");
    let original = field_tokens(&section_xml(T5));
    let saved = serialize_hwpx(&doc).expect("T5 save");
    let reopened = parse_hwpx(&saved).expect("T5 reopen");
    assert_eq!(field_count(&reopened), 1);
    let saved_tokens = field_tokens(&section_xml(&saved));

    // 필드 경계는 한 쌍이고, 같은 문단 위치·같은 속성으로 남는다.
    let begins = |t: &[String]| t.iter().filter(|x| x.starts_with('B')).count();
    let ends = |t: &[String]| t.iter().filter(|x| x.starts_with('E')).count();
    assert_eq!((begins(&original), ends(&original)), (1, 1));
    assert_eq!((begins(&saved_tokens), ends(&saved_tokens)), (1, 1));
    assert_eq!(boundaries(&saved_tokens), boundaries(&original));
    // 필드 내용(시작과 끝 사이 글자)이 같다.
    assert_eq!(field_content(&saved_tokens), field_content(&original));

    // 두 번째 저장도 같은 바이트의 section XML 을 낸다.
    let saved2 = serialize_hwpx(&reopened).expect("T5 second save");
    assert_eq!(section_xml(&saved2), section_xml(&saved));
}

const P: &str = r#"<hp:p paraPrIDRef="0" styleIDRef="0">"#;

fn begin(id: u32) -> String {
    format!(
        r#"<hp:ctrl><hp:fieldBegin id="{id}" type="CLICK_HERE" name="본문{id}" editable="1" dirty="0" zorder="-1" fieldid="{}"><hp:parameters cnt="1" name=""><hp:stringParam name="Command">안내;</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl>"#,
        id + 500
    )
}

fn end(id: u32) -> String {
    format!(
        r#"<hp:ctrl><hp:fieldEnd beginIDRef="{id}" fieldid="{}"/></hp:ctrl>"#,
        id + 500
    )
}

/// 본문 세 문단에 걸친 필드 하나 + 같은 문단 안 필드 하나.
fn body_spanning_fixture() -> String {
    format!(
        r#"{P}<hp:run charPrIDRef="0"><hp:t>앞😀</hp:t>{b1}<hp:t>시작</hp:t></hp:run></hp:p>{P}<hp:run charPrIDRef="0"><hp:t>가운데</hp:t>{b2}<hp:t>안</hp:t>{e2}</hp:run></hp:p>{P}<hp:run charPrIDRef="0"><hp:t>끝</hp:t>{e1}<hp:t>뒤</hp:t></hp:run></hp:p>"#,
        b1 = begin(1),
        b2 = begin(2),
        e1 = end(1),
        e2 = end(2),
    )
}

#[test]
fn body_field_spanning_paragraphs_round_trips_two_saves() {
    let input = fixture(&body_spanning_fixture());
    let doc = parse_hwpx(&input).expect("field ending in a later body paragraph must open");
    let original = field_tokens(&section_xml(&input));

    // 시작 문단은 이 필드의 field_ranges 를 갖지 않는다(같은 문단 안 필드만 범위를 갖는다).
    let paras = &doc.sections[0].paragraphs;
    assert!(paras[0].field_ranges.is_empty());
    assert_eq!(paras[1].field_ranges.len(), 1);
    assert_eq!(paras[0].text, "앞😀시작");
    assert_eq!(paras[2].text, "끝뒤");

    let saved = serialize_hwpx(&doc).expect("save");
    let saved_tokens = field_tokens(&section_xml(&saved));
    assert_eq!(saved_tokens, original);
    let reopened = parse_hwpx(&saved).expect("reopen");
    let saved2 = serialize_hwpx(&reopened).expect("second save");
    assert_eq!(section_xml(&saved2), section_xml(&saved));
}

#[test]
fn cell_field_spanning_paragraphs_round_trips() {
    let cell = format!(
        r#"<hp:tbl id="7" rowCnt="1" colCnt="1" cellSpacing="0" borderFillIDRef="1"><hp:sz width="1000" widthRelTo="ABSOLUTE" height="1000" heightRelTo="ABSOLUTE" protect="0"/><hp:pos treatAsChar="1" affectLSpacing="0" flowWithText="1" allowOverlap="0" holdAnchorAndSO="0" vertRelTo="PARA" horzRelTo="PARA" vertAlign="TOP" horzAlign="LEFT" vertOffset="0" horzOffset="0"/><hp:tr><hp:tc name="" header="0" hasMargin="0" protect="0" editable="0" dirty="0" borderFillIDRef="1"><hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="TOP"><hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>셀앞</hp:t>{b}</hp:run></hp:p><hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>셀끝</hp:t>{e}</hp:run></hp:p></hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="1000"/><hp:cellMargin left="0" right="0" top="0" bottom="0"/></hp:tc></hp:tr></hp:tbl>"#,
        b = begin(3),
        e = end(3),
    );
    let body = format!(r#"{P}<hp:run charPrIDRef="0">{cell}<hp:t>본문</hp:t></hp:run></hp:p>"#);
    let input = fixture(&body);
    let doc = parse_hwpx(&input).expect("field ending in a later cell paragraph must open");
    let original = boundaries(&field_tokens(&section_xml(&input)));
    let saved = serialize_hwpx(&doc).expect("save");
    assert_eq!(boundaries(&field_tokens(&section_xml(&saved))), original);
    let reopened = parse_hwpx(&saved).expect("reopen");
    let saved2 = serialize_hwpx(&reopened).expect("second save");
    assert_eq!(section_xml(&saved2), section_xml(&saved));
}

#[test]
fn unmatched_field_end_is_refused_within_and_across_paragraphs() {
    let cases = [
        // 어느 문단에도 대응 begin 이 없다.
        format!(
            r#"{P}<hp:run charPrIDRef="0"><hp:t>가</hp:t>{b}</hp:run></hp:p>{P}<hp:run charPrIDRef="0"><hp:t>나</hp:t>{e}</hp:run></hp:p>"#,
            b = begin(1),
            e = end(9),
        ),
        // end 가 begin 보다 앞 문단에 있다.
        format!(
            r#"{P}<hp:run charPrIDRef="0"><hp:t>가</hp:t>{e}</hp:run></hp:p>{P}<hp:run charPrIDRef="0"><hp:t>나</hp:t>{b}</hp:run></hp:p>"#,
            b = begin(1),
            e = end(1),
        ),
        // 같은 문단에서 end 가 begin 보다 앞에 있다.
        format!(
            r#"{P}<hp:run charPrIDRef="0">{e}<hp:t>가</hp:t>{b}</hp:run></hp:p>"#,
            b = begin(1),
            e = end(1),
        ),
        // 이미 닫힌 begin 에 end 가 한 번 더 온다.
        format!(
            r#"{P}<hp:run charPrIDRef="0">{b}<hp:t>가</hp:t>{e}</hp:run></hp:p>{P}<hp:run charPrIDRef="0"><hp:t>나</hp:t>{e}</hp:run></hp:p>"#,
            b = begin(1),
            e = end(1),
        ),
    ];
    for (i, body) in cases.iter().enumerate() {
        let err = parse_hwpx(&fixture(body))
            .err()
            .unwrap_or_else(|| panic!("case {i}: unmatched fieldEnd must be refused"));
        assert!(
            err.to_string()
                .contains("fieldEnd beginIDRef에 대응하는 fieldBegin 없음"),
            "case {i}: {err}"
        );
    }
}

#[test]
fn unclosed_field_begin_opens_but_save_is_refused() {
    let body = format!(
        r#"{P}<hp:run charPrIDRef="0"><hp:t>가</hp:t>{b}</hp:run></hp:p>{P}<hp:run charPrIDRef="0"><hp:t>나</hp:t></hp:run></hp:p>"#,
        b = begin(1),
    );
    let doc = parse_hwpx(&fixture(&body)).expect("unclosed fieldBegin still opens");
    assert!(
        serialize_hwpx(&doc).is_err(),
        "unclosed fieldBegin must not be saved as a pair"
    );
}

fn paragraph(run: &str) -> String {
    format!(r#"{P}<hp:run charPrIDRef="0">{run}</hp:run></hp:p>"#)
}

fn editable_cross_paragraph_fixture() -> Vec<u8> {
    fixture(&format!(
        "{}{}",
        paragraph(&begin(1)),
        paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
    ))
}

#[test]
fn c1_field_end_tracks_text_edits_and_paragraph_structure() {
    for (case, expected, paragraph_count) in [
        ("insert_inside", "XYAB", 2),
        ("insert_after", "AB", 2),
        ("insert_at_end", "ABXY", 2),
        ("delete_inside", "B", 2),
        ("split_inside", "AB", 3),
        ("split_begin", "AB", 3),
        ("merge_begin", "AB", 2),
        ("merge_empty_end", "AB", 1),
        ("merge", "AB", 1),
    ] {
        let input = match case {
            "merge_begin" => fixture(&format!(
                "{}{}{}",
                paragraph(""),
                paragraph(&begin(1)),
                paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
            )),
            "merge_empty_end" => fixture(&format!(
                "{}{}",
                paragraph(&format!("{}<hp:t>AB</hp:t>", begin(1))),
                paragraph(&end(1))
            )),
            _ => editable_cross_paragraph_fixture(),
        };
        let mut core = rhwp::DocumentCore::from_bytes(&input).unwrap();
        match case {
            "insert_inside" => core.insert_text_native(0, 1, 0, "XY").unwrap(),
            "insert_after" => core.insert_text_native(0, 1, 4, "XY").unwrap(),
            "insert_at_end" => core.insert_text_native(0, 1, 2, "XY").unwrap(),
            "delete_inside" => core.delete_text_native(0, 1, 0, 1).unwrap(),
            "split_inside" => core.split_paragraph_native(0, 1, 1).unwrap(),
            "split_begin" => core.split_paragraph_native(0, 0, 0).unwrap(),
            "merge" | "merge_begin" | "merge_empty_end" => {
                core.merge_paragraph_native(0, 1).unwrap()
            }
            _ => unreachable!(),
        };
        let saved = core
            .export_hwpx_native()
            .unwrap_or_else(|e| panic!("{case}: {e}"));
        let tokens = field_tokens(&section_xml(&saved));
        assert_eq!(field_content(&tokens), expected, "{case}: saved membership");
        let expected_paragraphs = match case {
            "split_inside" => (0, 2),
            "split_begin" => (1, 2),
            "merge" | "merge_empty_end" => (0, 0),
            _ => (0, 1),
        };
        let mut pi = 0;
        let mut actual_paragraphs = (None, None);
        for token in &tokens {
            if token.starts_with('B') {
                actual_paragraphs.0 = Some(pi);
            }
            if token.starts_with('E') {
                actual_paragraphs.1 = Some(pi);
            }
            if token == ")P" {
                pi += 1;
            }
        }
        assert_eq!(
            actual_paragraphs,
            (Some(expected_paragraphs.0), Some(expected_paragraphs.1)),
            "{case}: marker paragraphs"
        );
        assert_eq!(
            tokens.iter().filter(|t| t.starts_with('B')).count(),
            1,
            "{case}"
        );
        assert_eq!(
            tokens.iter().filter(|t| t.starts_with('E')).count(),
            1,
            "{case}"
        );
        let reopened = parse_hwpx(&saved).unwrap();
        assert_eq!(
            reopened.sections[0].paragraphs.len(),
            paragraph_count,
            "{case}"
        );
        assert_eq!(
            serialize_hwpx(&reopened).unwrap(),
            saved,
            "{case}: second save bytes"
        );
    }
}

#[test]
fn c1_deleted_or_reversed_endpoints_refuse_save() {
    let mut core = rhwp::DocumentCore::from_bytes(&editable_cross_paragraph_fixture()).unwrap();
    core.delete_text_native(0, 1, 1, 1).unwrap();
    let err = core
        .export_hwpx_native()
        .err()
        .unwrap_or_else(|| panic!("deleting the end's anchor text must refuse"));
    assert!(err.to_string().contains("HWPX_FIELD_ENDPOINT"), "{err}");

    let mut doc = parse_hwpx(&editable_cross_paragraph_fixture()).unwrap();
    doc.sections[0].paragraphs.swap(0, 1);
    let err = serialize_hwpx(&doc)
        .err()
        .unwrap_or_else(|| panic!("begin after end must refuse"));
    assert!(err.to_string().contains("HWPX_FIELD_ENDPOINT"), "{err}");
}

#[test]
fn c1_split_and_merge_keep_fields_nested_inside_a_span() {
    let inner = format!(
        "{}<hp:t>AX</hp:t>{}<hp:t>B</hp:t>{}<hp:t>CD</hp:t>",
        begin(2),
        end(2),
        end(1)
    );
    let input = fixture(&format!("{}{}", paragraph(&begin(1)), paragraph(&inner)));
    for (action, expected) in [
        ("merge", paragraph(&format!("{}{inner}", begin(1)))),
        (
            "split_before",
            format!(
                "{}{}{}",
                paragraph(&begin(1)),
                paragraph(""),
                paragraph(&inner)
            ),
        ),
        (
            "split_inside",
            format!(
                "{}{}{}",
                paragraph(&begin(1)),
                paragraph(&format!("{}<hp:t>A</hp:t>", begin(2))),
                paragraph(&format!(
                    "<hp:t>X</hp:t>{}<hp:t>B</hp:t>{}<hp:t>CD</hp:t>",
                    end(2),
                    end(1)
                ))
            ),
        ),
    ] {
        let mut core = rhwp::DocumentCore::from_bytes(&input).unwrap();
        match action {
            "merge" => core.merge_paragraph_native(0, 1).unwrap(),
            "split_before" => core.split_paragraph_native(0, 1, 0).unwrap(),
            "split_inside" => core.split_paragraph_native(0, 1, 1).unwrap(),
            _ => unreachable!(),
        };
        let saved = core
            .export_hwpx_native()
            .unwrap_or_else(|e| panic!("{action}: {e}"));
        assert_eq!(
            field_tokens(&section_xml(&saved)),
            field_tokens(&section_xml(&fixture(&expected))),
            "{action}"
        );
        assert_eq!(
            serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(),
            saved,
            "{action}: second save"
        );
    }
}

#[test]
fn c1_field_replacements_and_guide_cleanup_shift_enclosing_endpoint() {
    let body = |value| {
        format!(
            "{}{}",
            paragraph(&begin(1)),
            paragraph(&format!(
                "{}<hp:t>{value}</hp:t>{}<hp:t>B</hp:t>{}<hp:t>CD</hp:t>",
                begin(2),
                end(2),
                end(1)
            ))
        )
    };
    for (guide, expected) in [(false, "XY"), (true, "")] {
        let input = if guide {
            body("A").replace("안내;", "Direction:wstring:1:A")
        } else {
            body("A")
        };
        let mut core = rhwp::DocumentCore::from_bytes(&fixture(&input)).unwrap();
        if !guide {
            core.set_field_value_by_id(502, "XY").unwrap();
        }
        let saved = core.export_hwpx_native().unwrap();
        assert_eq!(
            field_tokens(&section_xml(&saved)),
            field_tokens(&section_xml(&fixture(&body(expected)))),
            "guide={guide}"
        );
        assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
    }
}

fn wrapped_field_list(kind: &str, content: &str) -> String {
    let sublist = format!("<hp:subList>{content}</hp:subList>");
    let object = match kind {
        "body" => return content.to_string(),
        "cell" => format!(
            r#"<hp:tbl rowCnt="1" colCnt="1" borderFillIDRef="1"><hp:tr><hp:tc borderFillIDRef="1">{sublist}<hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="1000"/></hp:tc></hp:tr></hp:tbl>"#
        ),
        "caption" => format!(
            r#"<hp:tbl rowCnt="0" colCnt="0" borderFillIDRef="1"><hp:caption>{sublist}</hp:caption></hp:tbl>"#
        ),
        "drawText" => format!("<hp:rect><hp:drawText>{sublist}</hp:drawText></hp:rect>"),
        "ellipseCaption" => format!("<hp:ellipse><hp:caption>{sublist}</hp:caption></hp:ellipse>"),
        "ellipseText" => format!("<hp:ellipse><hp:drawText>{sublist}</hp:drawText></hp:ellipse>"),
        _ => format!("<hp:ctrl><hp:{kind}>{sublist}</hp:{kind}></hp:ctrl>"),
    };
    paragraph(&object)
}

#[test]
fn c2_cross_paragraph_acceptance_matches_serialized_lists() {
    let content = format!(
        "{}{}",
        paragraph(&format!("{}<hp:t>HEAD</hp:t>", begin(1))),
        paragraph(&format!("<hp:t>TAIL</hp:t>{}", end(1)))
    );
    for kind in [
        "header",
        "footer",
        "footNote",
        "endNote",
        "hiddenComment",
        "ellipseCaption",
        "ellipseText",
    ] {
        let err = parse_hwpx(&fixture(&wrapped_field_list(kind, &content)))
            .err()
            .unwrap_or_else(|| panic!("{kind}: omitted list must refuse at open"));
        assert!(
            err.to_string().contains("HWPX_FIELD_LIST_UNSUPPORTED"),
            "{kind}: {err}"
        );
    }
    let nested = wrapped_field_list("header", &wrapped_field_list("cell", &content));
    assert!(parse_hwpx(&fixture(&nested)).is_err());
    for kind in ["body", "cell", "caption", "drawText"] {
        let input = fixture(&wrapped_field_list(kind, &content));
        let doc = parse_hwpx(&input).unwrap();
        let saved = serialize_hwpx(&doc).unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert_eq!(
            field_tokens(&section_xml(&saved)),
            field_tokens(&section_xml(&input)),
            "{kind}"
        );
        assert_eq!(
            serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(),
            saved,
            "{kind}: second save"
        );
    }
}

#[test]
fn c2_unclosed_begins_are_refused_in_every_list_kind() {
    let content = paragraph(&format!("{}<hp:t>HEAD</hp:t>", begin(1)));
    for kind in [
        "body",
        "cell",
        "caption",
        "drawText",
        "header",
        "footer",
        "footNote",
        "endNote",
        "hiddenComment",
        "ellipseCaption",
        "ellipseText",
    ] {
        match parse_hwpx(&fixture(&wrapped_field_list(kind, &content))) {
            Err(err) => assert!(err.to_string().contains("fieldBegin"), "{kind}: {err}"),
            Ok(doc) => assert!(
                serialize_hwpx(&doc).is_err(),
                "{kind}: unclosed begin was saved"
            ),
        }
    }
}
