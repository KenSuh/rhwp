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
    assert!(paras[0].field_ranges().is_empty());
    assert_eq!(paras[1].field_ranges().len(), 1);
    assert_eq!(paras[0].text(), "앞😀시작");
    assert_eq!(paras[2].text(), "끝뒤");

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
    let input = fixture(&format!(
        "{}{}",
        paragraph(&format!("<hp:t>ZZ</hp:t>{}<hp:t>12</hp:t>", begin(1))),
        paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
    ));
    for para in [0, 1] {
        let mut core = rhwp::DocumentCore::from_bytes(&input).unwrap();
        core.delete_text_native(0, para, 1, 2).unwrap();
        let err = core.export_hwpx_native().unwrap_err();
        assert!(
            err.to_string().contains("HWPX_FIELD_ENDPOINT"),
            "paragraph {para}: {err}"
        );
    }

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

#[test]
fn c1_formula_replacement_refuses_removed_begin() {
    let content = format!(
        "{}{}",
        paragraph(&format!("<hp:t>Z</hp:t>{}<hp:t>12</hp:t>", begin(1))),
        paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
    );
    let mut doc =
        rhwp::wasm_api::HwpDocument::from_bytes(&fixture(&wrapped_field_list("cell", &content)))
            .unwrap();
    doc.evaluate_table_formula(0, 0, 0, 0, 0, "=12345", true)
        .unwrap();
    let error = doc.export_hwpx_native().unwrap_err();
    assert!(error.to_string().contains("HWPX_FIELD_ENDPOINT"), "{error}");
}

#[test]
fn c1_nested_formula_replacement_refuses_removed_begin() {
    let content = format!(
        "{}{}",
        paragraph(&format!("<hp:t>Z</hp:t>{}<hp:t>12</hp:t>", begin(1))),
        paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
    );
    let nested = wrapped_field_list("cell", &wrapped_field_list("cell", &content));
    let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&fixture(&nested)).unwrap();
    doc.evaluate_table_formula_by_path(0, 0, r#"[{"controlIndex":0,"cellIndex":0,"cellParaIndex":0},{"controlIndex":0,"cellIndex":0,"cellParaIndex":0}]"#, 0, 0, "=12345", true).unwrap();
    let error = doc.export_hwpx_native().unwrap_err();
    assert!(error.to_string().contains("HWPX_FIELD_ENDPOINT"), "{error}");
}

#[test]
fn c1_boundary_insert_undo_redo_keeps_membership() {
    for endpoint in ["begin", "end"] {
        let input = fixture(&format!(
            "{}{}",
            paragraph(&format!("<hp:t>ZZ</hp:t>{}<hp:t>12</hp:t>", begin(1))),
            paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
        ));
        let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&input).unwrap();
        let para = if endpoint == "begin" { 0 } else { 1 };
        for (action, expected) in [("insert", "12ABXY"), ("undo", "12AB"), ("redo", "12ABXY")] {
            if action == "undo" {
                doc.delete_text(0, para, 2, 2).unwrap();
            } else {
                doc.insert_text(0, para, 2, "XY").unwrap();
            }
            let saved = doc
                .export_hwpx_native()
                .unwrap_or_else(|e| panic!("{endpoint}/{action}: {e}"));
            let expected = if endpoint == "begin" && action != "undo" {
                "12AB"
            } else {
                expected
            };
            assert_eq!(
                field_content(&field_tokens(&section_xml(&saved))),
                expected,
                "{endpoint}/{action}"
            );
            assert_eq!(
                serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(),
                saved,
                "{endpoint}/{action}: second save"
            );
        }
    }
}

#[test]
fn c1_delete_ending_at_endpoint_keeps_marker() {
    assert_boundary_delete(0, 2, true);
}

#[test]
fn c1_delete_starting_at_endpoint_keeps_marker() {
    assert_boundary_delete(2, 1, false);
}

fn assert_boundary_delete(start: u32, count: u32, removes_inside: bool) {
    for endpoint in ["begin", "end"] {
        let input = fixture(&format!(
            "{}{}",
            paragraph(&format!("<hp:t>ZZ</hp:t>{}<hp:t>12</hp:t>", begin(1))),
            paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
        ));
        let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&input).unwrap();
        let para = if endpoint == "begin" { 0 } else { 1 };
        doc.delete_text(0, para, start, count).unwrap();
        let saved = doc
            .export_hwpx_native()
            .unwrap_or_else(|e| panic!("{endpoint}/{start}+{count}: {e}"));
        let expected = match (endpoint, removes_inside) {
            ("begin", true) | ("end", false) => "12AB",
            ("begin", false) => "2AB",
            ("end", true) => "12",
            _ => unreachable!(),
        };
        assert_eq!(
            field_content(&field_tokens(&section_xml(&saved))),
            expected,
            "{endpoint}"
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
fn c1_c3_adjacent_replacement_preserves_order() {
    c3_adjacent_replacement("A", "B");
}

#[test]
fn c1_c3_empty_field_replacement_preserves_neighbor() {
    c3_adjacent_replacement("", "B");
}

#[test]
fn c1_c3_replacement_preserves_empty_neighbor() {
    c3_adjacent_replacement("A", "");
}

#[test]
fn c1_c3_two_empty_fields_replace_independently() {
    c3_adjacent_replacement("", "");
}

fn c3_adjacent_replacement(first: &str, second: &str) {
    let body = |a: &str| {
        paragraph(&format!(
            "{}<hp:t>{a}</hp:t>{}{}<hp:t>{second}</hp:t>{}",
            begin(1),
            end(1),
            begin(2),
            end(2)
        ))
    };
    let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&body(first))).unwrap();
    doc.set_field_value_by_id(501, "XY").unwrap();
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(
        field_tokens(&section_xml(&saved)),
        field_tokens(&section_xml(&fixture(&body("XY")))),
        "first={first:?} second={second:?}"
    );
    let reopened = parse_hwpx(&saved).unwrap();
    let ranges = reopened.sections[0].paragraphs[0].field_ranges();
    assert_eq!(
        ranges
            .iter()
            .map(|r| (r.start_char_idx, r.end_char_idx))
            .collect::<Vec<_>>(),
        vec![(0, 2), (2, 2 + second.chars().count())]
    );
    assert_eq!(serialize_hwpx(&reopened).unwrap(), saved);
}

#[test]
fn c1_c3_typing_at_adjacent_and_empty_markers_uses_one_slot() {
    for (first, second) in [("A", "B"), ("", "")] {
        let input = fixture(&paragraph(&format!(
            "{}<hp:t>{first}</hp:t>{}{}<hp:t>{second}</hp:t>{}",
            begin(1),
            end(1),
            begin(2),
            end(2)
        )));
        let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&input).unwrap();
        let position = u32::try_from(first.chars().count()).unwrap();
        doc.insert_text(0, 0, position, "Q").unwrap();
        let saved = doc.export_hwpx_native().unwrap();
        let expected = if first.is_empty() {
            paragraph(&format!(
                "<hp:t>Q</hp:t>{}{}{}{}",
                begin(1),
                end(1),
                begin(2),
                end(2)
            ))
        } else {
            paragraph(&format!(
                "{}<hp:t>AQ</hp:t>{}{}<hp:t>B</hp:t>{}",
                begin(1),
                end(1),
                begin(2),
                end(2)
            ))
        };
        assert_eq!(
            field_tokens(&section_xml(&saved)),
            field_tokens(&section_xml(&fixture(&expected)))
        );
        assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
    }
}

fn c3_boundary_fixture() -> Vec<u8> {
    fixture(&format!(
        "{}{}",
        paragraph(&format!("<hp:t>ZZ</hp:t>{}<hp:t>12</hp:t>", begin(1))),
        paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1)))
    ))
}

#[test]
fn c1_c3_delete_inverse_at_begin() {
    c3_delete_inverse(0, 0, "ZZ");
}

#[test]
fn c1_c3_delete_inverse_at_end() {
    c3_delete_inverse(1, 2, "CD");
}

fn c3_delete_inverse(para: u32, pos: u32, text: &str) {
    let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&c3_boundary_fixture()).unwrap();
    let token = doc.begin_text_edit(0, para, false).unwrap();
    doc.delete_text(0, para, pos, 2).unwrap();
    let deleted = doc.export_hwpx_native().unwrap();
    assert_eq!(
        serialize_hwpx(&parse_hwpx(&deleted).unwrap()).unwrap(),
        deleted
    );
    doc.finish_text_edit(&token).unwrap();
    let redo = doc.undo_edit(&token).unwrap();
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(field_content(&field_tokens(&section_xml(&saved))), "12AB");
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
    let undo = doc.undo_edit(&redo).unwrap();
    assert_eq!(doc.export_hwpx_native().unwrap(), deleted);
    doc.undo_edit(&undo).unwrap();
    assert_eq!(doc.export_hwpx_native().unwrap(), saved);

    doc.delete_text(0, para, pos, 2).unwrap();
    doc.insert_text(0, para, pos, text).unwrap();
    let normal = doc.export_hwpx_native().unwrap();
    let expected = if para == 0 { "12AB" } else { "12ABCD" };
    assert_eq!(
        field_content(&field_tokens(&section_xml(&normal))),
        expected
    );
    assert_eq!(
        serialize_hwpx(&parse_hwpx(&normal).unwrap()).unwrap(),
        normal
    );
}

#[test]
fn c1_c3_insert_inverse_and_redo_at_each_boundary() {
    for para in [0, 1] {
        let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&c3_boundary_fixture()).unwrap();
        let original = doc.export_hwpx_native().unwrap();
        let original_markers = doc.document().sections[0].paragraphs
            [usize::try_from(para).unwrap()]
        .field_markers()
        .to_vec();
        let original_offsets = doc.document().sections[0].paragraphs
            [usize::try_from(para).unwrap()]
        .char_offsets()
        .to_vec();
        let token = doc.begin_text_edit(0, para, false).unwrap();
        doc.insert_text(0, para, 2, "XY").unwrap();
        let inserted = doc.export_hwpx_native().unwrap();
        let expected = if para == 0 { "12AB" } else { "12ABXY" };
        assert_eq!(
            field_content(&field_tokens(&section_xml(&inserted))),
            expected
        );
        assert_eq!(
            serialize_hwpx(&parse_hwpx(&inserted).unwrap()).unwrap(),
            inserted
        );
        doc.finish_text_edit(&token).unwrap();
        let redo = doc.undo_edit(&token).unwrap();
        let restored = doc.export_hwpx_native().unwrap();
        assert_eq!(
            field_tokens(&section_xml(&restored)),
            field_tokens(&section_xml(&original))
        );
        let owner = &doc.document().sections[0].paragraphs[usize::try_from(para).unwrap()];
        assert_eq!(owner.field_markers(), original_markers);
        assert_eq!(owner.char_offsets(), original_offsets);
        assert_eq!(
            serialize_hwpx(&parse_hwpx(&restored).unwrap()).unwrap(),
            restored
        );
        doc.undo_edit(&redo).unwrap();
        assert_eq!(doc.export_hwpx_native().unwrap(), inserted);
    }
}

#[test]
fn c1_c3_collapsed_adjacent_markers_have_exact_inverse() {
    let body = paragraph(&format!(
        "{}<hp:t>A</hp:t>{}{}<hp:t>B</hp:t>{}",
        begin(1),
        end(1),
        begin(2),
        end(2)
    ));
    for (start, count) in [(0, 1), (1, 1), (0, 2)] {
        let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&fixture(&body)).unwrap();
        let original = doc.export_hwpx_native().unwrap();
        let token = doc.begin_text_edit(0, 0, false).unwrap();
        doc.delete_text(0, 0, start, count).unwrap();
        if count == 2 {
            assert!(doc.export_hwpx_native().is_err());
        } else {
            let deleted = doc.export_hwpx_native().unwrap();
            assert_eq!(
                serialize_hwpx(&parse_hwpx(&deleted).unwrap()).unwrap(),
                deleted
            );
        }
        doc.finish_text_edit(&token).unwrap();
        let redo = doc.undo_edit(&token).unwrap();
        assert_eq!(doc.export_hwpx_native().unwrap(), original);
        let saved = doc.export_hwpx_native().unwrap();
        assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
        let undo = doc.undo_edit(&redo).unwrap();
        doc.undo_edit(&undo).unwrap();
        assert_eq!(doc.export_hwpx_native().unwrap(), original);
    }
}

#[test]
fn c1_c3_cell_merge_preserves_whole_paragraphs() {
    let content = format!(
        "{}{}{}",
        paragraph(&begin(1)),
        paragraph("<hp:t>HEADTAIL</hp:t>"),
        paragraph(&end(1))
    );
    let cell = |col: u16, inner: &str| {
        format!(
            r#"<hp:tc borderFillIDRef="1"><hp:subList>{inner}</hp:subList><hp:cellAddr colAddr="{col}" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="1000"/></hp:tc>"#
        )
    };
    let body = paragraph(&format!(
        r#"<hp:tbl rowCnt="1" colCnt="2" borderFillIDRef="1"><hp:tr>{}{}</hp:tr></hp:tbl>"#,
        cell(0, &paragraph("<hp:t>PRIMARY</hp:t>")),
        cell(1, &content)
    ));
    let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&body)).unwrap();
    doc.merge_table_cells_native(0, 0, 0, 0, 0, 0, 1).unwrap();
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(
        field_content(&field_tokens(&section_xml(&saved))),
        "HEADTAIL"
    );
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_c3_split_at_adjacent_empty_fields_preserves_sequence() {
    let input = fixture(&paragraph(&format!(
        "{}{}{}{}",
        begin(1),
        end(1),
        begin(2),
        end(2)
    )));
    let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&input).unwrap();
    let original = field_tokens(&section_xml(&doc.export_hwpx_native().unwrap()));
    let token = doc.begin_text_edit(0, 0, true).unwrap();
    doc.split_paragraph(0, 0, 0).unwrap();
    let split = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&split).unwrap()).unwrap(), split);
    doc.merge_paragraph(0, 1).unwrap();
    let merged = doc.export_hwpx_native().unwrap();
    assert_eq!(field_tokens(&section_xml(&merged)), original);
    assert_eq!(
        serialize_hwpx(&parse_hwpx(&merged).unwrap()).unwrap(),
        merged
    );
    doc.finish_text_edit(&token).unwrap();
    doc.undo_edit(&token).unwrap();
    let restored = doc.export_hwpx_native().unwrap();
    assert_eq!(field_tokens(&section_xml(&restored)), original);
    assert_eq!(
        serialize_hwpx(&parse_hwpx(&restored).unwrap()).unwrap(),
        restored
    );
}

#[test]
fn c1_c3_undo_tokens_are_bounded_consumed_and_dropped_on_reload() {
    let mut doc = rhwp::DocumentCore::from_bytes(&c3_boundary_fixture()).unwrap();
    let mut tokens = Vec::new();
    for _ in 0..1024 {
        let token = doc.begin_text_edit_native(0, 0, false).unwrap();
        doc.finish_text_edit_native(&token).unwrap();
        tokens.push(token);
    }
    assert!(doc.begin_text_edit_native(0, 0, false).is_err());
    let latest = tokens.last().unwrap();
    let redo = doc.undo_edit_native(latest).unwrap();
    assert!(doc.undo_edit_native(latest).is_err());
    doc.discard_edit_native(&redo);
    let released = doc.begin_text_edit_native(0, 0, false).unwrap();
    doc.create_blank_document_native().unwrap();
    assert!(doc.undo_edit_native(&released).is_err());
    doc.set_document(parse_hwpx(&c3_boundary_fixture()).unwrap());
    assert!(doc.undo_edit_native(&released).is_err());
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_history_refuses_out_of_order_handles_without_changing_text() {
    let mut doc = rhwp::DocumentCore::from_bytes(&c3_boundary_fixture()).unwrap();
    let old = doc.begin_text_edit_native(0, 1, false).unwrap();
    doc.delete_text_native(0, 1, 2, 2).unwrap();
    doc.finish_text_edit_native(&old).unwrap();
    let later = doc.begin_text_edit_native(0, 1, false).unwrap();
    doc.insert_text_native(0, 1, 2, "LATER").unwrap();
    doc.finish_text_edit_native(&later).unwrap();
    assert!(
        doc.undo_edit_native(&old).is_err(),
        "out-of-order inverse accepted"
    );
    assert_eq!(doc.document().sections[0].paragraphs[1].text(), "ABLATER");
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
    doc.undo_edit_native(&later).unwrap();
}

#[test]
fn c1_history_refuses_redo_after_new_edit() {
    for captured in [false, true] {
        let mut doc = rhwp::DocumentCore::from_bytes(&c3_boundary_fixture()).unwrap();
        let token = doc.begin_text_edit_native(0, 1, false).unwrap();
        doc.insert_text_native(0, 1, 2, "OLD").unwrap();
        doc.finish_text_edit_native(&token).unwrap();
        let redo = doc.undo_edit_native(&token).unwrap();
        if captured {
            doc.begin_text_edit_native(0, 1, false).unwrap();
        }
        doc.insert_text_native(0, 1, 2, "NEW").unwrap();
        assert!(
            doc.undo_edit_native(&redo).is_err(),
            "abandoned redo accepted"
        );
        assert_eq!(doc.document().sections[0].paragraphs[1].text(), "ABNEWCD");
        let saved = doc.export_hwpx_native().unwrap();
        assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
    }
}

#[test]
fn c1_clipboard_owns_complete_fields_and_remaps_each_paste() {
    for prefix in ["", "ZZ"] {
        let body = format!(
            "{}{}",
            paragraph(&format!(
                "<hp:t>{prefix}</hp:t>{}<hp:t>A</hp:t>{}<hp:t>CD</hp:t>",
                begin(1),
                end(1)
            )),
            paragraph("<hp:t>TARGET</hp:t>")
        );
        let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&body)).unwrap();
        doc.copy_selection_native(0, 0, prefix.len(), 0, prefix.len() + 1)
            .unwrap();
        for expected in [2, 3] {
            doc.paste_internal_native(0, 1, 0).unwrap();
            let saved = doc.export_hwpx_native().unwrap();
            let parsed = parse_hwpx(&saved).unwrap();
            let fields: Vec<_> = parsed.sections[0]
                .paragraphs
                .iter()
                .flat_map(|p| p.controls.iter())
                .filter_map(|c| match c {
                    Control::Field(f) => f.endpoint_id.clone(),
                    _ => None,
                })
                .collect();
            assert_eq!(fields.len(), expected);
            assert_eq!(
                fields
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len(),
                expected
            );
            assert_eq!(serialize_hwpx(&parsed).unwrap(), saved);
        }
    }
}

#[test]
fn c1_revision_refuses_redo_after_replace_all() {
    revision_after_public_edit(false);
}

#[test]
fn c1_revision_refuses_redo_after_field_replacement() {
    revision_after_public_edit(true);
}

fn revision_after_public_edit(field: bool) {
    let input = fixture(&paragraph(&format!(
        "{}<hp:t>ABC</hp:t>{}",
        begin(1),
        end(1)
    )));
    let mut doc = rhwp::DocumentCore::from_bytes(&input).unwrap();
    let token = doc.begin_text_edit_native(0, 0, false).unwrap();
    doc.insert_text_native(0, 0, 1, "OLD").unwrap();
    doc.finish_text_edit_native(&token).unwrap();
    let redo = doc.undo_edit_native(&token).unwrap();
    if field {
        doc.set_field_value_by_id(501, "NEW").unwrap();
    } else {
        doc.replace_all_native("ABC", "NEW", true).unwrap();
    }
    assert!(
        doc.undo_edit_native(&redo).is_err(),
        "redo accepted a different revision"
    );
    assert_eq!(doc.document().sections[0].paragraphs[0].text(), "NEW");
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_fragment_omits_unselected_control_subtrees() {
    let table = wrapped_field_list(
        "cell",
        &paragraph(&format!("{}<hp:t>SECRET</hp:t>{}", begin(1), end(1))),
    );
    let body = table.replacen("</hp:tbl>", "</hp:tbl><hp:t>ZZ</hp:t>", 1);
    let input = fixture(&format!("{body}{}", paragraph("<hp:t>TARGET</hp:t>")));
    let mut doc = rhwp::DocumentCore::from_bytes(&input).unwrap();
    doc.copy_selection_native(0, 0, 1, 0, 2).unwrap();
    doc.paste_internal_native(0, 1, 0).unwrap();
    assert_eq!(doc.document().sections[0].paragraphs[1].text(), "ZTARGET");
    assert!(doc.document().sections[0].paragraphs[1].controls.is_empty());
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(section_xml(&saved).matches("<hp:fieldBegin ").count(), 1);
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_selected_table_keeps_subtree_with_fresh_ids_on_each_paste() {
    let table = wrapped_field_list(
        "cell",
        &paragraph(&format!("{}<hp:t>SECRET</hp:t>{}", begin(1), end(1))),
    );
    let body = table.replacen("</hp:tbl>", "</hp:tbl><hp:t>ZZ</hp:t>", 1);
    let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&format!(
        "{body}{}",
        paragraph("<hp:t>TARGET</hp:t>")
    )))
    .unwrap();
    doc.copy_selection_native(0, 0, 0, 0, 2).unwrap();
    for count in [2, 3] {
        doc.paste_internal_native(0, 1, 0).unwrap();
        let saved = doc.export_hwpx_native().unwrap();
        assert_eq!(
            doc.document().sections[0].paragraphs[1].controls.len(),
            count - 1
        );
        let fields = doc.collect_all_fields();
        assert_eq!(fields.len(), count);
        assert_eq!(
            fields
                .iter()
                .map(|field| field.field.field_id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            count
        );
        assert!(fields.iter().all(|field| field.value == "SECRET"));
        let resaved = serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap();
        assert_eq!(section_xml(&resaved), section_xml(&saved));
    }
}

#[test]
fn c1_cell_selection_copies_text_without_the_ancestor_table() {
    let table = wrapped_field_list(
        "cell",
        &paragraph(&format!("{}<hp:t>SECRET</hp:t>{}", begin(1), end(1))),
    );
    let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&format!(
        "{table}{}",
        paragraph("<hp:t>TARGET</hp:t>")
    )))
    .unwrap();
    doc.copy_selection_in_cell_native(0, 0, 0, 0, 0, 0, 0, 3)
        .unwrap();
    doc.paste_internal_native(0, 1, 0).unwrap();
    let target = &doc.document().sections[0].paragraphs[1];
    assert_eq!(target.text(), "SECTARGET");
    assert!(target.controls.is_empty());
    assert_eq!(doc.collect_all_fields().len(), 1);
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_revision_refuses_redo_after_cell_edit() {
    let table = wrapped_field_list(
        "cell",
        &paragraph(&format!("{}<hp:t>ABC</hp:t>{}", begin(1), end(1))),
    );
    let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&table)).unwrap();
    let token = doc.begin_text_edit_native(0, 0, false).unwrap();
    doc.insert_text_in_cell_native(0, 0, 0, 0, 0, 1, "OLD")
        .unwrap();
    doc.finish_text_edit_native(&token).unwrap();
    let redo = doc.undo_edit_native(&token).unwrap();
    doc.insert_text_in_cell_native(0, 0, 0, 0, 0, 0, "NEW")
        .unwrap();
    assert!(doc.undo_edit_native(&redo).is_err());
    assert_eq!(doc.collect_all_fields()[0].value, "ABC");
    let Control::Table(table) = &doc.document().sections[0].paragraphs[0].controls[0] else {
        panic!("table")
    };
    assert_eq!(table.cells[0].paragraphs[0].text(), "NEWABC");
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_revision_refuses_uncaptured_edits_and_resealing() {
    let input = fixture(&paragraph(&format!(
        "{}<hp:t>ABC</hp:t>{}",
        begin(1),
        end(1)
    )));
    let mut doc = rhwp::DocumentCore::from_bytes(&input).unwrap();
    let token = doc.begin_text_edit_native(0, 0, false).unwrap();
    doc.insert_text_native(0, 0, 1, "OLD").unwrap();
    assert!(doc.undo_edit_native(&token).is_err());
    doc.finish_text_edit_native(&token).unwrap();
    doc.set_field_value_by_id(501, "NEW").unwrap();
    assert!(doc.finish_text_edit_native(&token).is_err());
    assert!(doc.undo_edit_native(&token).is_err());
    assert_eq!(doc.document().sections[0].paragraphs[0].text(), "NEW");
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_clipboard_partial_fields_contribute_text_without_endpoints() {
    let mut doc = rhwp::DocumentCore::from_bytes(&c3_boundary_fixture()).unwrap();
    doc.copy_selection_native(0, 0, 3, 1, 1).unwrap();
    doc.paste_internal_native(0, 1, 4).unwrap();
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(
        doc.document().sections[0]
            .paragraphs
            .iter()
            .flat_map(|p| &p.controls)
            .filter(|c| matches!(c, Control::Field(_)))
            .count(),
        1
    );
    assert_eq!(doc.document().sections[0].paragraphs[1].text(), "ABCD2");
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_clipboard_spanning_fields_keep_one_owner_in_body_and_cell() {
    for in_cell in [false, true] {
        let body = format!(
            "{}{}{}",
            paragraph(&format!("<hp:t>ZZ</hp:t>{}<hp:t>12</hp:t>", begin(1))),
            paragraph(&format!("<hp:t>AB</hp:t>{}<hp:t>CD</hp:t>", end(1))),
            paragraph("<hp:t>TARGET</hp:t>")
        );
        let input = if in_cell {
            wrapped_field_list("cell", &body)
        } else {
            body
        };
        let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&input)).unwrap();
        if in_cell {
            doc.copy_selection_in_cell_native(0, 0, 0, 0, 0, 2, 1, 2)
                .unwrap();
            doc.paste_internal_in_cell_native(0, 0, 0, 0, 2, 0).unwrap();
        } else {
            doc.copy_selection_native(0, 0, 2, 1, 2).unwrap();
            doc.paste_internal_native(0, 2, 0).unwrap();
        }
        let saved = doc.export_hwpx_native().unwrap();
        assert_eq!(section_xml(&saved).matches("<hp:fieldBegin ").count(), 2);
        assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
    }
}

#[test]
fn c1_clipboard_repeated_fragment_assigns_four_new_field_identities() {
    let fields = format!(
        "{}<hp:t>A</hp:t>{}{}<hp:t>B</hp:t>{}",
        begin(1),
        end(1),
        begin(2),
        end(2)
    );
    let body = format!("{}{}", paragraph(&fields), paragraph("<hp:t>TARGET</hp:t>"));
    let mut doc = rhwp::DocumentCore::from_bytes(&fixture(&body)).unwrap();
    doc.copy_selection_native(0, 0, 0, 0, 2).unwrap();
    doc.paste_internal_native(0, 1, 0).unwrap();
    doc.paste_internal_native(0, 1, 0).unwrap();
    let pasted: Vec<_> = doc.document().sections[0].paragraphs[1]
        .controls
        .iter()
        .filter_map(|c| match c {
            Control::Field(f) => f.endpoint_id.as_ref(),
            _ => None,
        })
        .collect();
    assert_eq!(pasted.len(), 4);
    assert_eq!(
        pasted
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
}

#[test]
fn c1_cut_and_paste_within_field_keeps_the_sequence() {
    let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(&c3_boundary_fixture()).unwrap();
    doc.copy_selection(0, 1, 0, 1, 1).unwrap();
    doc.delete_range(0, 1, 0, 1, 1).unwrap();
    doc.paste_internal(0, 1, 1).unwrap();
    assert_eq!(doc.document().sections[0].paragraphs[1].text(), "BACD");
    let saved = doc.export_hwpx_native().unwrap();
    assert_eq!(field_content(&field_tokens(&section_xml(&saved))), "12BA");
    assert_eq!(serialize_hwpx(&parse_hwpx(&saved).unwrap()).unwrap(), saved);
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
