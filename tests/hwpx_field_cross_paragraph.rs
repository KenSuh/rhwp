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

/// 그림 컨트롤마다 bin_data_id 를 넘긴다(본문·표 셀).
fn for_each_picture_bin_id(
    paras: &mut [rhwp::model::paragraph::Paragraph],
    f: &mut dyn FnMut(&mut u16),
) {
    for p in paras {
        for c in &mut p.controls {
            match c {
                Control::Picture(pic) => f(&mut pic.image_attr.bin_data_id),
                Control::Table(t) => {
                    for cell in &mut t.cells {
                        for_each_picture_bin_id(&mut cell.paragraphs, f);
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn t5_opens_through_parser_and_document_core() {
    let doc = parse_hwpx(T5).expect("T5 must open: its fieldBegin/fieldEnd pair spans paragraphs");
    assert_eq!(field_count(&doc), 1);
    rhwp::DocumentCore::from_bytes(T5).expect("T5 must open through DocumentCore");
}

/// T5 의 두 번째 그림은 `binaryItemIDRef=""`(그림 없는 그림 개체)라 rhwp 저장이 필드와 무관하게 거부한다.
/// 필드 왕복만 재려고 그 그림 하나를 같은 문서의 image1 로 가리킨 뒤 저장→재열기→재저장한다.
#[test]
fn t5_cross_paragraph_field_round_trips() {
    let mut doc = parse_hwpx(T5).expect("T5 open");
    let mut values = Vec::new();
    for_each_picture_bin_id(&mut doc.sections[0].paragraphs, &mut |id| values.push(*id));
    assert_eq!(
        values.iter().filter(|&&v| v == 0).count(),
        1,
        "T5 has one image-less picture"
    );
    let image1 = *values.iter().find(|&&v| v != 0).unwrap();
    for_each_picture_bin_id(&mut doc.sections[0].paragraphs, &mut |id| {
        if *id == 0 {
            *id = image1;
        }
    });

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
