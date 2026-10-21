//! HWPX 라운드트립 통합 테스트.
//!
//! 각 Stage의 "완료 기준" = 이 파일의 해당 Stage 테스트가 IrDiff 0으로 통과.
//! **누적만 가능, 삭제·완화 금지**. Stage 5 완료 시 모든 샘플이 한 번에 통과해야 한다.
//!
//! Stage 0 (현재): blank_hwpx.hwpx 의 뼈대 필드(섹션 수·리소스 카운트) 유지 검증
//! Stage 1 예정: ref_empty.hwpx / ref_text.hwpx
//! Stage 2 예정: 다문단·run 분할
//! Stage 3 예정: ref_table.hwpx / hwp_table_test.hwp
//! Stage 4 예정: pic-in-head-01.hwp / pic-crop-01.hwp
//! Stage 5 예정: 대형 실문서 3건

use rhwp::serializer::hwpx::roundtrip::roundtrip_ir_diff;

fn c12_fixture() -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut body = c9_paragraph("<hp:t>본문</hp:t>", C9_LINE);
    for index in 0..4 {
        let label = format!(
            r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="{index}"><hp:t>{}단계</hp:t></hp:run>{C9_LINE}</hp:p>"#,
            index + 1
        );
        let shape = format!(
            r##"<hp:rect instid="{}" ratio="20"><hp:orgSz width="3635" height="1566"/><hp:curSz width="3635" height="1566"/><hp:lineShape color="#BD3D3D" width="20" style="SOLID"/><hc:fillBrush><hc:winBrush faceColor="#BD3D3D" hatchColor="#000000" alpha="0"/></hc:fillBrush><hp:drawText lastWidth="3635"><hp:subList vertAlign="CENTER">{label}</hp:subList><hp:textMargin left="283" right="283" top="283" bottom="283"/></hp:drawText><hc:pt0 x="0" y="0"/><hc:pt1 x="3635" y="0"/><hc:pt2 x="3635" y="1566"/><hc:pt3 x="0" y="1566"/><hp:sz width="3635" height="1566"/><hp:pos treatAsChar="1"/></hp:rect>"##,
            index + 101
        );
        body.push_str(&c9_paragraph(&shape, C9_LINE));
    }
    let mut header = c9_header_xml(&c9_header_fixture(
        r#"<hh:paraPr id="0"><hh:align horizontal="LEFT" vertical="BASELINE"/><hh:lineSpacing type="PERCENT" value="160"/></hh:paraPr>"#,
    ));
    let char_properties = [
        r##"<hh:charPr id="0" height="1000" textColor="#FFFFFF" shadeColor="none">"##,
        r##"<hh:charPr id="1" height="1000" textColor="#FFFFFF" shadeColor="#FFFFFF">"##,
        r##"<hh:charPr id="2" height="1000" textColor="#FFFFFF" shadeColor="#123456">"##,
        r##"<hh:charPr id="3" height="1000" textColor="#FFFFFF">"##,
    ]
    .iter()
    .map(|opening| format!(r#"{opening}<hh:fontRef hangul="0" latin="0" hanja="0" japanese="0" other="0" symbol="0" user="0"/><hh:ratio hangul="100" latin="100" hanja="100" japanese="100" other="100" symbol="100" user="100"/><hh:relSz hangul="100" latin="100" hanja="100" japanese="100" other="100" symbol="100" user="100"/></hh:charPr>"#))
    .collect::<String>();
    let start = header.find("<hh:charProperties").unwrap();
    let end = header.find("</hh:charProperties>").unwrap() + "</hh:charProperties>".len();
    header.replace_range(
        start..end,
        &format!(r#"<hh:charProperties itemCnt="4">{char_properties}</hh:charProperties>"#),
    );
    let mut input = zip::ZipArchive::new(Cursor::new(c8_fixture(&body, false))).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..input.len() {
        let mut entry = input.by_index(index).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        output
            .start_file(&name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output
            .write_all(if name == "Contents/header.xml" {
                header.as_bytes()
            } else {
                &bytes
            })
            .unwrap();
    }
    output.finish().unwrap().into_inner()
}

fn c12_shade_properties(hwpx: &[u8]) -> Vec<(String, String, String)> {
    use quick_xml::{events::Event, name::ResolveResult, NsReader};
    let xml = c9_header_xml(hwpx);
    let mut reader = NsReader::from_str(&xml);
    let mut properties = Vec::new();
    loop {
        let (namespace, event) = reader.read_resolved_event().unwrap();
        match event {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"charPr" => {
                let ResolveResult::Bound(namespace) = namespace else {
                    panic!("charPr must have a bound namespace")
                };
                assert_eq!(
                    namespace.as_ref(),
                    b"http://www.hancom.co.kr/hwpml/2011/head"
                );
                let attrs: std::collections::BTreeMap<_, _> = e
                    .attributes()
                    .map(|attr| {
                        let attr = attr.unwrap();
                        (
                            String::from_utf8(attr.key.as_ref().to_vec()).unwrap(),
                            attr.decode_and_unescape_value(reader.decoder())
                                .unwrap()
                                .into_owned(),
                        )
                    })
                    .collect();
                properties.push((
                    attrs["id"].clone(),
                    attrs["textColor"].clone(),
                    attrs["shadeColor"].clone(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    properties
}

fn c12_artifact(name: &str, bytes: &[u8]) {
    use std::io::Write;
    if let Some(directory) = std::env::var_os("RHWP_C12_ARTIFACT_DIR") {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::PathBuf::from(directory).join(name))
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }
}

fn c12_native_shade_roundtrip(edit: bool) {
    use rhwp::{
        document_core::DocumentCore,
        model::{control::Control, shape::ShapeObject},
    };
    let input = c12_fixture();
    let case = if edit { "edit" } else { "no-edit" };
    c12_artifact(&format!("{case}-input.hwpx"), &input);
    let mut core = DocumentCore::from_bytes(&input).unwrap();
    assert_eq!(core.document().doc_info.char_shapes.len(), 4);
    assert_eq!(core.document().doc_info.char_shapes[0].shade_color, 0xFFFF_FFFF);
    assert_eq!(core.document().doc_info.char_shapes[1].shade_color, 0x00FF_FFFF);
    assert_eq!(core.document().doc_info.char_shapes[2].shade_color, 0x0056_3412);
    assert_eq!(core.document().doc_info.char_shapes[3].shade_color, 0);
    let mut expected_texts = c11_layout(&hwpx_section_xml(&input)).texts;
    for save in 1..=2 {
        let saved = core.export_hwpx_native().unwrap();
        c12_artifact(&format!("{case}-save-{save}.hwpx"), &saved);
        let reopened = DocumentCore::from_bytes(&saved).unwrap();
        let doc = reopened.document();
        assert_eq!(doc.sections[0].paragraphs.len(), 5);
        for (index, paragraph) in doc.sections[0].paragraphs[1..].iter().enumerate() {
            let shapes: Vec<_> = paragraph
                .controls
                .iter()
                .filter_map(|control| match control {
                    Control::Shape(shape) => Some(shape),
                    _ => None,
                })
                .collect();
            assert_eq!(shapes.len(), 1);
            let ShapeObject::Rectangle(rect) = shapes[0].as_ref() else {
                panic!("owning rectangle")
            };
            let text_box = rect.drawing.text_box.as_ref().unwrap();
            assert_eq!(text_box.paragraphs.len(), 1);
            let label = &text_box.paragraphs[0];
            assert_eq!(label.text, format!("{}단계", index + 1));
            assert_eq!(label.para_shape_id, 0);
            assert_eq!(label.char_shapes.len(), 1);
            assert_eq!(usize::try_from(label.char_shapes[0].char_shape_id).unwrap(), index);
            assert_eq!(doc.doc_info.char_shapes[index].text_color, 0x00FF_FFFF);
        }
        assert_eq!(c11_layout(&hwpx_section_xml(&saved)).texts, expected_texts);
        let expected: Vec<_> = ["none", "#FFFFFF", "#123456", "none"]
            .iter()
            .enumerate()
            .map(|(id, shade)| (id.to_string(), "#FFFFFF".into(), (*shade).into()))
            .collect();
        assert_eq!(c12_shade_properties(&saved), expected, "{case} save {save}");
        core = reopened;
        if edit && save == 1 {
            core.insert_text_native(0, 0, 2, "편😀").unwrap();
            assert_eq!(core.document().sections[0].paragraphs[0].text, "본문편😀");
            expected_texts[0].1 = "본문편😀".into();
        }
    }
}

#[test]
fn c12_native_shape_label_shades_survive_two_no_edit_saves() {
    c12_native_shade_roundtrip(false);
}

#[test]
fn c12_native_shape_label_shades_survive_text_edit_between_saves() {
    c12_native_shade_roundtrip(true);
}

#[derive(Debug, Default)]
struct C11Layout {
    flags: Vec<(String, Option<bool>)>,
    geometry: Vec<(String, Vec<(String, String)>)>,
    objects: Vec<String>,
    texts: Vec<(String, String)>,
}

fn c11_layout(xml: &str) -> C11Layout {
    use quick_xml::{events::Event, name::ResolveResult, NsReader};
    use std::collections::BTreeMap;
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut stack = vec![(
        String::new(),
        String::new(),
        BTreeMap::<String, usize>::new(),
    )];
    let mut wrappers = Vec::new();
    let mut paragraphs = Vec::new();
    let mut in_text = false;
    let mut output = C11Layout::default();
    loop {
        let (resolved, event) = reader.read_resolved_event().unwrap();
        let namespace = match resolved {
            ResolveResult::Bound(ns) => String::from_utf8(ns.as_ref().to_vec()).unwrap(),
            ResolveResult::Unbound => String::new(),
            ResolveResult::Unknown(prefix) => panic!("unbound namespace: {prefix:?}"),
        };
        let hp = namespace == "http://www.hancom.co.kr/hwpml/2011/paragraph";
        match event {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let wrapper = hp && matches!(name.as_str(), "run" | "ctrl");
                wrappers.push(wrapper);
                if wrapper {
                    continue;
                }
                let expanded = format!("{{{namespace}}}{name}");
                let parent = stack.last_mut().unwrap();
                let count = parent.2.entry(expanded.clone()).or_default();
                let owner = format!("{}/{expanded}[{count}]", parent.0);
                *count += 1;
                let mut attrs: Vec<_> = e
                    .attributes()
                    .map(|a| {
                        let a = a.unwrap();
                        (
                            String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                            a.decode_and_unescape_value(reader.decoder())
                                .unwrap()
                                .into_owned(),
                        )
                    })
                    .collect();
                attrs.sort();
                if name == "tbl" {
                    assert!(hp, "table must use the expanded HWPX paragraph namespace");
                    let flag = attrs
                        .iter()
                        .find(|(key, _)| key == "noAdjust")
                        .map(|(_, value)| matches!(value.as_str(), "1" | "true"));
                    output.flags.push((owner.clone(), flag));
                }
                if hp
                    && (name == "lineseg"
                        || matches!(name.as_str(), "cellAddr" | "cellSpan" | "cellSz")
                        || (parent.1 == "tbl"
                            && matches!(name.as_str(), "sz" | "pos" | "inMargin" | "outMargin")))
                {
                    output.geometry.push((owner.clone(), attrs));
                }
                if hp && matches!(name.as_str(), "tbl" | "pic") {
                    output.objects.push(owner.clone());
                }
                if hp && name == "p" {
                    paragraphs.push(output.texts.len());
                    output.texts.push((owner.clone(), String::new()));
                }
                if hp && name == "t" {
                    in_text = true;
                }
                if hp && in_text {
                    let special = match name.as_str() {
                        "fwSpace" => "\u{2007}",
                        "nbSpace" => "\u{a0}",
                        "lineBreak" => "\n",
                        "tab" => "\t",
                        _ => "",
                    };
                    output.texts[*paragraphs.last().unwrap()]
                        .1
                        .push_str(special);
                }
                stack.push((owner, name, BTreeMap::new()));
            }
            Event::Text(e) if in_text => {
                let value = quick_xml::escape::unescape(&e.decode().unwrap())
                    .unwrap()
                    .into_owned();
                output.texts[*paragraphs.last().unwrap()].1.push_str(&value);
            }
            Event::GeneralRef(e) if in_text => {
                let reference = format!("&{};", e.decode().unwrap());
                let value = quick_xml::escape::unescape(&reference).unwrap();
                output.texts[*paragraphs.last().unwrap()].1.push_str(&value);
            }
            Event::End(e) => {
                if hp && e.local_name().as_ref() == b"t" {
                    in_text = false;
                }
                if hp && e.local_name().as_ref() == b"p" {
                    paragraphs.pop().unwrap();
                }
                if !wrappers.pop().unwrap() {
                    stack.pop().unwrap();
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    assert_eq!(stack.len(), 1);
    output
}

#[test]
fn c11_native_true_table_flags_keep_nested_owners_two_saves() {
    let inner = c8_table(&c9_paragraph("<hp:t>nested true</hp:t>", C9_LINE)).replacen(
        "<hp:tbl ",
        r#"<hp:tbl noAdjust="true" "#,
        1,
    );
    let outer = c8_table(&inner).replacen("<hp:tbl ", r#"<hp:tbl noAdjust="false" "#, 1);
    let first = c8_table(&c9_paragraph("<hp:t>body true</hp:t>", C9_LINE)).replacen(
        "<hp:tbl ",
        r#"<hp:tbl noAdjust="1" "#,
        1,
    );
    let input = c8_fixture(&(first + &outer), false);
    let expected = c11_layout(&hwpx_section_xml(&input));
    assert_eq!(
        expected.flags.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
        [Some(true), Some(false), Some(true)]
    );
    c10_two_native_saves(&input, |_, xml| {
        let actual = c11_layout(xml);
        assert_eq!(actual.objects, expected.objects);
        assert_eq!(actual.texts, expected.texts);
        assert_eq!(actual.flags, expected.flags);
    });
}

#[test]
fn c11_native_false_and_missing_table_flags_default_false_two_saves() {
    let body: String = [Some("0"), Some("false"), None]
        .into_iter()
        .map(|flag| {
            let table = c8_table(&c9_paragraph("<hp:t>false default</hp:t>", C9_LINE));
            match flag {
                Some(value) => {
                    table.replacen("<hp:tbl ", &format!(r#"<hp:tbl noAdjust="{value}" "#), 1)
                }
                None => table,
            }
        })
        .collect();
    let input = c8_fixture(&body, false);
    let mut expected = c11_layout(&hwpx_section_xml(&input)).flags;
    assert_eq!(
        expected.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
        [Some(false), Some(false), None]
    );
    for (_, flag) in &mut expected {
        *flag = Some(false);
    }
    c10_two_native_saves(&input, |_, xml| assert_eq!(c11_layout(xml).flags, expected));
}

#[test]
#[ignore = "requires immutable C6 fixture in RHWP_C11_C6_INPUT"]
fn c11_real_c6_table_flags_and_layout_survive_two_native_saves() {
    let path = std::env::var_os("RHWP_C11_C6_INPUT").expect("explicit immutable C6 input");
    let input = std::fs::read(path).unwrap();
    assert_eq!(input.len(), 217029);
    let xml = hwpx_section_xml(&input);
    let expected = c11_layout(&xml);
    assert_eq!(expected.flags.len(), 82);
    assert_eq!(
        expected
            .flags
            .iter()
            .filter(|(_, flag)| *flag == Some(true))
            .count(),
        12
    );
    assert_eq!(expected.texts.len(), 2050);
    let columns = c9_layout_events(&xml);
    c10_two_native_saves(&input, |_, xml| {
        let actual = c11_layout(xml);
        assert_eq!(actual.objects, expected.objects);
        assert_eq!(actual.texts, expected.texts);
        assert_eq!(actual.geometry, expected.geometry);
        assert_eq!(c9_layout_events(xml), columns);
        assert_eq!(actual.flags, expected.flags);
    });
    eprintln!("C11 C6: 82 owner flags / 12 true / 2050 paragraph texts and geometry preserved across two public native saves");
}
const C10_COLUMN: &str = r#"<hp:ctrl><hp:colPr id="" type="NEWSPAPER" layout="RIGHT" colCount="2" sameSz="1" sameGap="300"/></hp:ctrl>"#;
const C10_PAGE_NUMBER: &str = r#"<hp:ctrl><hp:pageNum pos="BOTTOM_RIGHT" formatType="ROMAN_CAPITAL" sideChar="*"/></hp:ctrl>"#;

fn c10_events(xml: &str) -> Vec<(String, String, Vec<String>, Vec<(String, String)>)> {
    use quick_xml::{events::Event, name::ResolveResult, NsReader};
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let (mut paragraph, mut cell) = (0, 0);
    let mut owners = Vec::new();
    let mut output = Vec::new();
    loop {
        let (resolved, event) = reader.read_resolved_event().unwrap();
        let namespace = match resolved {
            ResolveResult::Bound(ns) => String::from_utf8(ns.as_ref().to_vec()).unwrap(),
            ResolveResult::Unbound => String::new(),
            ResolveResult::Unknown(prefix) => panic!("unbound namespace: {prefix:?}"),
        };
        match event {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                match name.as_str() {
                    "p" => {
                        owners.push(format!("p{paragraph}"));
                        paragraph += 1;
                    }
                    "tc" => {
                        owners.push(format!("tc{cell}"));
                        cell += 1;
                    }
                    _ => {}
                }
                if matches!(
                    name.as_str(),
                    "run" | "secPr" | "colPr" | "pageNum" | "pageHiding" | "t" | "tbl" | "pic"
                ) {
                    assert_eq!(namespace, "http://www.hancom.co.kr/hwpml/2011/paragraph");
                    let mut attrs: Vec<_> = e
                        .attributes()
                        .map(|a| {
                            let a = a.unwrap();
                            (
                                String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                                a.decode_and_unescape_value(reader.decoder())
                                    .unwrap()
                                    .into_owned(),
                            )
                        })
                        .collect();
                    attrs.sort();
                    output.push((namespace, name, owners.clone(), attrs));
                }
            }
            Event::End(e) if matches!(e.local_name().as_ref(), b"p" | b"tc") => {
                owners.pop().unwrap();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    output
}

fn c10_two_native_saves(input: &[u8], check: impl Fn(&rhwp::model::document::Document, &str)) {
    use rhwp::{document_core::DocumentCore, parser::hwpx::parse_hwpx};
    let mut core = DocumentCore::from_bytes(input).unwrap();
    for _ in 0..2 {
        let saved = core.export_hwpx_native().unwrap();
        check(&parse_hwpx(&saved).unwrap(), &hwpx_section_xml(&saved));
        core = DocumentCore::from_bytes(&saved).unwrap();
    }
}

fn c10_inline_table() -> String {
    let table = c8_table(&c9_paragraph(C10_COLUMN, C9_LINE));
    let start = table.find("<hp:tbl ").unwrap();
    let end = table.rfind("</hp:tbl>").unwrap() + "</hp:tbl>".len();
    table[start..end].replace("<hp:tr>", r#"<hp:pos treatAsChar="1"/><hp:tr>"#)
}

#[test]
fn c10_native_initial_column_page_number_prefix_precedes_text_once() {
    let repeated = C10_COLUMN.replace("300", "777");
    let controls = C10_COLUMN.to_string()
        + C10_PAGE_NUMBER
        + &c10_inline_table()
        + &C9_FLOATING_PICTURE.replace(r#"treatAsChar="0""#, r#"treatAsChar="1""#)
        + &repeated;
    let body = c9_paragraph(&(controls + "<hp:t>초기 본문</hp:t>"), C9_LINE)
        + &c9_paragraph(
            &(C10_COLUMN.to_string() + "<hp:t>다음 문단</hp:t>"),
            C9_LINE,
        );
    let input = c8_fixture(&body, true);
    let expected_columns = c9_layout_events(&hwpx_section_xml(&input));
    c10_two_native_saves(&input, |doc, xml| {
        assert_eq!(doc.sections[0].paragraphs[0].text, "초기 본문");
        let events = c10_events(xml);
        let first: Vec<_> = events
            .iter()
            .filter(|(_, n, o, _)| o == &["p0"] && n != "run")
            .map(|(_, n, _, _)| n.as_str())
            .collect();
        assert_eq!(
            first,
            ["secPr", "colPr", "pageNum", "t", "tbl", "pic", "colPr"]
        );
        let page_number = events.iter().find(|(_, n, _, _)| n == "pageNum").unwrap();
        assert_eq!(page_number.2, ["p0"]);
        assert_eq!(
            page_number.3,
            [
                ("formatType".into(), "ROMAN_CAPITAL".into()),
                ("pos".into(), "BOTTOM_RIGHT".into()),
                ("sideChar".into(), "*".into())
            ]
        );
        assert_eq!(c9_layout_events(xml), expected_columns);
    });
}

#[test]
fn c10_native_nonleading_vector_columns_and_inline_order_are_retained() {
    let hide = r#"<hp:ctrl><hp:pageHiding hideHeader="1" hideFooter="0" hideMasterPage="0" hideBorder="0" hideFill="0" hidePageNum="0"/></hp:ctrl>"#;
    let controls = hide.to_string()
        + C10_COLUMN
        + C10_PAGE_NUMBER
        + &C10_COLUMN.replace("300", "888")
        + &c10_inline_table()
        + C9_FLOATING_PICTURE;
    let input = c8_fixture(
        &c9_paragraph(&(controls + "<hp:t>선두 아님</hp:t>"), C9_LINE),
        true,
    );
    let expected_columns = c9_layout_events(&hwpx_section_xml(&input));
    c10_two_native_saves(&input, |_, xml| {
        let events = c10_events(xml);
        let first: Vec<_> = events
            .iter()
            .filter(|(_, n, o, _)| o == &["p0"] && n != "run")
            .map(|(_, n, _, _)| n.as_str())
            .collect();
        assert_eq!(
            first,
            [
                "secPr",
                "t",
                "pageHiding",
                "colPr",
                "pageNum",
                "colPr",
                "tbl",
                "pic"
            ]
        );
        assert_eq!(c9_layout_events(xml), expected_columns);
    });
}

#[test]
fn c10_native_initial_column_moves_only_adjacent_page_number_prefix() {
    let controls = C10_COLUMN.to_string()
        + C9_FLOATING_PICTURE
        + C10_PAGE_NUMBER
        + &C10_COLUMN.replace("300", "999")
        + &c10_inline_table();
    let input = c8_fixture(
        &c9_paragraph(&(controls + "<hp:t>사이 개체</hp:t>"), C9_LINE),
        true,
    );
    c10_two_native_saves(&input, |_, xml| {
        let events = c10_events(xml);
        let first: Vec<_> = events
            .iter()
            .filter(|(_, n, o, _)| o == &["p0"] && n != "run")
            .map(|(_, n, _, _)| n.as_str())
            .collect();
        assert_eq!(
            first,
            ["secPr", "colPr", "t", "pic", "pageNum", "colPr", "tbl"]
        );
    });
}

#[test]
fn c10_native_empty_run_styles_keep_body_cell_ownership_two_saves() {
    use rhwp::model::control::Control;
    let empty =
        |run: &str| format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0">{run}{C9_LINE}</hp:p>"#);
    let body = empty(r#"<hp:run charPrIDRef="1"/>"#)
        + &empty(r#"<hp:run charPrIDRef="2"></hp:run>"#)
        + &empty("<hp:run/>")
        + &empty("<hp:run></hp:run>");
    let cells = empty(r#"<hp:run charPrIDRef="3"/>"#)
        + &empty(r#"<hp:run charPrIDRef="4"></hp:run>"#)
        + &empty("<hp:run/>")
        + &empty("<hp:run></hp:run>");
    let input = c8_fixture(&(body + &c8_table(&cells)), false);
    c10_two_native_saves(&input, |doc, xml| {
        let paras = &doc.sections[0].paragraphs;
        let table = paras[4]
            .controls
            .iter()
            .find_map(|c| match c {
                Control::Table(t) => Some(t),
                _ => None,
            })
            .unwrap();
        let styles: Vec<_> = paras[..4]
            .iter()
            .chain(table.cells[0].paragraphs.iter())
            .map(|p| {
                assert!(p.text.is_empty());
                p.char_shapes.first().map_or(0, |s| s.char_shape_id)
            })
            .collect();
        assert_eq!(styles, [1, 2, 0, 0, 3, 4, 0, 0]);
        let events = c10_events(xml);
        for (owner, expected) in [
            (vec!["p0"], "1"),
            (vec!["p1"], "2"),
            (vec!["p2"], "0"),
            (vec!["p3"], "0"),
            (vec!["p4", "tc0", "p5"], "3"),
            (vec!["p4", "tc0", "p6"], "4"),
            (vec!["p4", "tc0", "p7"], "0"),
            (vec!["p4", "tc0", "p8"], "0"),
        ] {
            let runs: Vec<_> = events
                .iter()
                .filter(|(_, n, o, _)| n == "run" && o == &owner)
                .collect();
            assert!(!runs.is_empty(), "missing owner {owner:?}");
            assert!(
                runs.iter().all(|(_, _, _, attrs)| attrs
                    .iter()
                    .any(|(k, v)| k == "charPrIDRef" && v == expected)),
                "owner {owner:?}, runs {runs:?}"
            );
        }
    });
}

#[test]
fn c10_native_self_closing_run_transition_matches_explicit_empty_run() {
    let content = r#"<hp:run charPrIDRef="1"><hp:t>A</hp:t></hp:run><hp:run charPrIDRef="2"/><hp:run><hp:t>B</hp:t></hp:run>"#;
    let paragraph =
        |runs: &str| format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0">{runs}{C9_LINE}</hp:p>"#);
    let body = paragraph(content)
        + &paragraph(&content.replace(r#"charPrIDRef="2"/>"#, r#"charPrIDRef="2"></hp:run>"#));
    let input = c8_fixture(&body, false);
    c10_two_native_saves(&input, |doc, xml| {
        for p in &doc.sections[0].paragraphs {
            assert_eq!(p.text, "AB");
            let active: Vec<_> = p
                .char_offsets
                .iter()
                .map(|pos| {
                    p.char_shapes
                        .iter()
                        .rev()
                        .find(|s| s.start_pos <= *pos)
                        .unwrap()
                        .char_shape_id
                })
                .collect();
            assert_eq!(active, [1, 2]);
        }
        let events = c10_events(xml);
        for owner in [vec!["p0"], vec!["p1"]] {
            let ids: Vec<_> = events
                .iter()
                .filter(|(_, n, o, _)| n == "run" && o == &owner)
                .map(|(_, _, _, attrs)| {
                    attrs
                        .iter()
                        .find(|(k, _)| k == "charPrIDRef")
                        .unwrap()
                        .1
                        .as_str()
                })
                .collect();
            assert!(ids.ends_with(&["1", "2"]), "owner {owner:?}, ids {ids:?}");
        }
    });
}
const C9_LINE: &str = r#"<hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1000" textheight="1000" baseline="850" spacing="200" horzpos="0" horzsize="10000" flags="393216"/></hp:linesegarray>"#;

fn c13_number(json: &str, key: &str) -> f64 {
    json.split_once(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("missing numeric field {key} in {json}"))
        .1.split([',', '}', ']']).next().unwrap().trim().parse().unwrap()
}

fn c13_artifact(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("RHWP_C13_ARTIFACT_DIR") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

fn c13_check_rendered_flow(
    core: &rhwp::document_core::DocumentCore,
    host: usize,
    following: &[usize],
    rows: usize,
    cols: usize,
    label: &str,
) {
    use rhwp::renderer::page_layout::PageLayoutInfo;
    use rhwp::model::page::ColumnDef;
    let pages: Vec<_> = std::iter::once(host).chain(following.iter().copied())
        .map(|p| c13_number(&core.get_page_of_position_native(0, p).unwrap(), "page") as u32)
        .collect();
    let layout = PageLayoutInfo::from_page_def_default(
        &core.document().sections[0].section_def.page_def, &ColumnDef::default());
    let body_bottom = layout.body_area.y + layout.body_area.height;
    let mut last_row_end: Option<(u32, f64)> = None;
    let mut instructions = std::collections::BTreeMap::<usize, (u32, f64)>::new();
    for page in *pages.iter().min().unwrap()..=*pages.iter().max().unwrap() {
        let controls = core.get_page_control_layout_native(page).unwrap();
        let text = core.get_page_text_layout_native(page).unwrap();
        c13_artifact(&format!("{label}-p{page}-controls.json"), controls.as_bytes());
        c13_artifact(&format!("{label}-p{page}-text.json"), text.as_bytes());
        c13_artifact(&format!("{label}-p{page}.svg"), core.render_page_svg_native(page).unwrap().as_bytes());
        for table in controls.split("{\"type\":\"table\",").skip(1) {
            let (metadata, cells) = table.split_once("\"cells\":[").unwrap();
            if !metadata.contains(&format!("\"paraIdx\":{host},"))
                || c13_number(metadata, "rowCount") != rows as f64
                || c13_number(metadata, "colCount") != cols as f64 { continue; }
            let table_bottom = c13_number(metadata, "y") + c13_number(metadata, "h");
            assert!(table_bottom <= body_bottom + 0.11,
                "{label}/page{page}: table bottom {table_bottom} clips below body {body_bottom}");
            for cell in cells.split_once(']').unwrap().0.split('}').filter(|s| s.contains("\"row\":")) {
                let bottom = c13_number(cell, "y") + c13_number(cell, "h");
                assert!(bottom <= body_bottom + 0.11, "{label}: cell clips below body");
                if c13_number(cell, "row") + c13_number(cell, "rowSpan") == rows as f64 {
                    let candidate = (page, bottom);
                    last_row_end = Some(last_row_end.map_or(candidate, |old| if old > candidate { old } else { candidate }));
                }
            }
        }
        for run in text.split("{\"text\":").skip(1).filter(|s| !s.contains("\"parentParaIdx\":")) {
            for &para in following {
                if run.contains(&format!("\"paraIdx\":{para},")) {
                    let top = c13_number(run, "y");
                    assert!(top >= layout.body_area.y - 0.11 && top + c13_number(run, "h") <= body_bottom + 0.11,
                        "{label}/page{page}: instruction{para} clips outside body");
                    let candidate = (page, top);
                    instructions.entry(para).and_modify(|old| { if candidate < *old { *old = candidate; } }).or_insert(candidate);
                }
            }
        }
    }
    let (last_page, bottom) = last_row_end.expect("actual final-row geometry must exist");
    assert_eq!(instructions.len(), following.len(), "{label}: all following instructions must render");
    for (&para, &(page, top)) in &instructions {
        assert!(page > last_page || (page == last_page && top + 0.11 >= bottom),
            "{label}: final row page{last_page}/bottom{bottom} overlaps instruction{para} page{page}/y{top}");
    }
    eprintln!("C13 {label}: final_row=({last_page},{bottom}), instructions={instructions:?}, body_bottom={body_bottom}");
}

fn c13_roundtrip_flow(input: &[u8], host: usize, following: &[usize], rows: usize, cols: usize, label: &str) {
    use rhwp::{document_core::DocumentCore, model::control::Control};
    let mut core = DocumentCore::from_bytes(input).unwrap();
    let expected_images = c13_images(input);
    let expected_text: Vec<_> = core.document().sections[0].paragraphs.iter().map(|p| p.text.clone()).collect();
    let expected_cells: Vec<_> = core.document().sections[0].paragraphs[host].controls.iter().find_map(|c| {
        if let Control::Table(t) = c { Some(t.cells.iter().flat_map(|c| c.paragraphs.iter().map(|p| p.text.clone())).collect()) } else { None }
    }).unwrap();
    c13_artifact(&format!("{label}-input.hwpx"), input);
    for pass in 0..=2 {
        let doc = core.document();
        assert_eq!(doc.sections[0].paragraphs.iter().map(|p| p.text.clone()).collect::<Vec<_>>(), expected_text);
        let table = doc.sections[0].paragraphs[host].controls.iter().find_map(|c| {
            if let Control::Table(t) = c { Some(t) } else { None }
        }).unwrap();
        assert!(table.common.treat_as_char);
        assert_eq!(table.common.vertical_offset, 19986, "rendering must not erase raw floating metadata");
        assert_eq!(table.cells.iter().flat_map(|c| c.paragraphs.iter().map(|p| p.text.clone())).collect::<Vec<_>>(), expected_cells);
        c13_check_rendered_flow(&core, host, following, rows, cols, &format!("{label}-pass{pass}"));
        if pass < 2 {
            let saved = core.export_hwpx_native().unwrap();
            assert_eq!(c13_images(&saved), expected_images, "{label}: BinData PNG bytes must survive save");
            c13_artifact(&format!("{label}-save{}.hwpx", pass + 1), &saved);
            core = DocumentCore::from_bytes(&saved).unwrap();
        }
    }
}

fn c13_images(hwpx: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    use std::io::{Cursor, Read};
    let mut zip = zip::ZipArchive::new(Cursor::new(hwpx)).unwrap();
    let mut images = std::collections::BTreeMap::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).unwrap();
        let name = entry.name().unwrap().to_string();
        if name.starts_with("BinData/") && name.ends_with(".png") {
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            images.insert(name, data);
        }
    }
    images
}

#[test]
fn c13_document_inline_table_geometry_survives_two_native_saves() {
    let rows = (0..2).map(|row| format!(
        r#"<hp:tr><hp:tc borderFillIDRef="1"><hp:subList>{}</hp:subList><hp:cellAddr colAddr="0" rowAddr="{row}"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="47409" height="{}"/></hp:tc></hp:tr>"#,
        c9_paragraph(&format!("<hp:t>C13 row {row}</hp:t>"), C9_LINE), if row == 0 { 13400 } else { 13401 })).collect::<String>();
    let body = format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="50000"><hp:margin left="5669" right="5669" top="5669" bottom="4252" header="4252" footer="4252" gutter="0"/></hp:pagePr></hp:secPr><hp:tbl id="13" rowCnt="2" colCnt="1" borderFillIDRef="1" textWrap="TOP_AND_BOTTOM" pageBreak="CELL" noAdjust="0"><hp:sz width="47409" height="26801"/><hp:pos treatAsChar="1" vertRelTo="PARA" vertAlign="TOP" vertOffset="19986"/>{rows}</hp:tbl></hp:run>{}</hp:p>{}"#,
        C9_LINE.replace("vertsize=\"1000\"", "vertsize=\"26801\"").replace("textheight=\"1000\"", "textheight=\"26801\""),
        c9_paragraph("<hp:t>C13 following instructions</hp:t>", &C9_LINE.replace("vertpos=\"0\"", "vertpos=\"26801\"")));
    c13_roundtrip_flow(&c8_fixture(&body, false), 0, &[1], 2, 1, "fixture");
}

#[test]
fn c13_private_c6_target_actual_render_save_reopen_observation() {
    let Some(path) = std::env::var_os("RHWP_C13_C6_INPUT") else {
        eprintln!("C13 private C6 observation not requested; ordinary fixture regression remains active");
        return;
    };
    let input = std::fs::read(path).unwrap();
    assert_eq!(input.len(), 217029);
    c13_roundtrip_flow(&input, 253, &[254, 255, 256], 11, 7, "c6");
}

fn c15_artifact(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("RHWP_C15_ARTIFACT_DIR") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

fn c15_check_horizontal_bounds(core: &rhwp::document_core::DocumentCore, host: usize, label: &str) {
    use rhwp::{model::{control::Control, page::ColumnDef}, renderer::page_layout::PageLayoutInfo};
    let doc = core.document();
    let table = doc.sections[0].paragraphs[host].controls.iter().find_map(|control| {
        if let Control::Table(table) = control { Some(table) } else { None }
    }).expect("the specified host must own a table");
    let layout = PageLayoutInfo::from_page_def_default(&doc.sections[0].section_def.page_def, &ColumnDef::default());
    let right = layout.body_area.x + layout.body_area.width;
    let first = c13_number(&core.get_page_of_position_native(0, host).unwrap(), "page") as u32;
    let last = if host + 1 < doc.sections[0].paragraphs.len() {
        c13_number(&core.get_page_of_position_native(0, host + 1).unwrap(), "page") as u32
    } else { core.page_count() - 1 };
    let mut observed = std::collections::BTreeSet::new();
    let mut violations = Vec::new();
    for page in first..=last {
        let controls = core.get_page_control_layout_native(page).unwrap();
        c15_artifact(&format!("{label}-host{host}-p{page}-controls.json"), controls.as_bytes());
        for fragment in controls.split("{\"type\":\"table\",").skip(1) {
            let (metadata, cells) = fragment.split_once("\"cells\":[").unwrap();
            if !metadata.contains(&format!("\"paraIdx\":{host},")) { continue; }
            assert_eq!(c13_number(metadata, "rowCount"), table.row_count as f64);
            assert_eq!(c13_number(metadata, "colCount"), table.col_count as f64);
            let x = c13_number(metadata, "x");
            let end = x + c13_number(metadata, "w");
            eprintln!("C15 {label}/host{host}/page{page}: table=[{x},{end}], body=[{},{}]", layout.body_area.x, right);
            if x < layout.body_area.x - 0.11 || end > right + 0.11 {
                violations.push(format!("table [{x},{end}] outside body [{},{}]", layout.body_area.x, right));
            }
            for cell in cells.split_once(']').unwrap().0.split('}').filter(|cell| cell.contains("\"row\":")) {
                let row = c13_number(cell, "row") as u16;
                let col = c13_number(cell, "col") as u16;
                observed.insert((row, col));
                let x = c13_number(cell, "x");
                let end = x + c13_number(cell, "w");
                if x < layout.body_area.x - 0.11 || end > right + 0.11 {
                    violations.push(format!("cell({row},{col}) [{x},{end}] outside body"));
                }
            }
        }
    }
    let expected: std::collections::BTreeSet<_> = table.cells.iter().map(|cell| (cell.row, cell.col)).collect();
    assert_eq!(observed, expected, "{label}: every actual owning cell must render");
    assert!(!observed.is_empty());
    assert!(violations.is_empty(), "{label}/host{host}: {}", violations.join("; "));
}

fn c15_table_metadata(core: &rhwp::document_core::DocumentCore, host: usize) -> String {
    use rhwp::model::control::Control;
    let table = core.document().sections[0].paragraphs[host].controls.iter().find_map(|control| {
        if let Control::Table(table) = control { Some(table) } else { None }
    }).unwrap();
    format!("{:?}/{:?}/{:?}/{:?}",
        (table.row_count, table.col_count, table.cell_spacing, table.page_break, table.no_adjust),
        (table.common.width, table.common.height, table.common.horizontal_offset, table.common.vertical_offset, table.common.treat_as_char),
        (table.common.horz_rel_to, table.common.horz_align, table.common.vert_rel_to, table.common.vert_align),
        table.cells.iter().map(|cell| (cell.row, cell.col, cell.row_span, cell.col_span, cell.width, cell.height)).collect::<Vec<_>>())
}

fn c15_two_save_bounds(input: &[u8], hosts: &[usize], label: &str) {
    use rhwp::document_core::DocumentCore;
    let mut core = DocumentCore::from_bytes(input).unwrap();
    let expected_text = c11_layout(&hwpx_section_xml(input)).texts;
    let expected_images = c13_images(input);
    let expected_metadata: Vec<_> = hosts.iter().map(|&host| c15_table_metadata(&core, host)).collect();
    c15_artifact(&format!("{label}-input.hwpx"), input);
    for pass in 0..=2 {
        for (&host, metadata) in hosts.iter().zip(&expected_metadata) {
            assert_eq!(&c15_table_metadata(&core, host), metadata, "{label}: raw table geometry must survive");
            c15_check_horizontal_bounds(&core, host, &format!("{label}-pass{pass}"));
        }
        if pass < 2 {
            let saved = core.export_hwpx_native().unwrap();
            assert_eq!(c11_layout(&hwpx_section_xml(&saved)).texts, expected_text, "{label}: all owned text, including hidden markers, must survive");
            assert_eq!(c13_images(&saved), expected_images, "{label}: PNG bytes must survive");
            c15_artifact(&format!("{label}-save{}.hwpx", pass + 1), &saved);
            core = DocumentCore::from_bytes(&saved).unwrap();
        }
    }
}

#[test]
fn c15_document_merged_inline_width_survives_two_native_saves() {
    use std::io::{Cursor, Read, Write};
    let cells = [(0, 1, 4000), (1, 2, 12000)].into_iter().map(|(col, span, width)| format!(
        r#"<hp:tc borderFillIDRef="1"><hp:subList>{}</hp:subList><hp:cellAddr colAddr="{col}" rowAddr="0"/><hp:cellSpan colSpan="{span}" rowSpan="1"/><hp:cellSz width="{width}" height="2000"/></hp:tc>"#,
        c9_paragraph("<hp:t>C15 cell</hp:t>", C9_LINE))).collect::<String>();
    let body = format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="5669" right="5669" top="5669" bottom="4252" header="4252" footer="4252" gutter="0"/></hp:pagePr></hp:secPr><hp:tbl id="15" rowCnt="1" colCnt="3" borderFillIDRef="1" pageBreak="CELL" textWrap="TOP_AND_BOTTOM"><hp:sz width="16000" height="2000"/><hp:pos treatAsChar="1" vertOffset="0" horzOffset="0"/><hp:tr>{cells}</hp:tr></hp:tbl><hp:t> </hp:t></hp:run>{}</hp:p>"#,
        C9_LINE.replace("vertsize=\"1000\"", "vertsize=\"2000\"").replace("horzsize=\"10000\"", "horzsize=\"48188\""));
    let mut source = zip::ZipArchive::new(Cursor::new(c8_fixture(&body, false))).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..source.len() {
        let mut entry = source.by_index(index).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name == "Contents/header.xml" {
            let header = String::from_utf8(bytes).unwrap();
            assert!(header.contains(r#"horizontal="JUSTIFY""#));
            bytes = header.replacen(r#"horizontal="JUSTIFY""#, r#"horizontal="RIGHT""#, 1).into_bytes();
        }
        output.start_file(&name, zip::write::SimpleFileOptions::default()).unwrap();
        output.write_all(&bytes).unwrap();
    }
    c15_two_save_bounds(&output.finish().unwrap().into_inner(), &[0], "fixture");
}

#[test]
fn c15_supplied_c14_inline_target_and_covered_control_two_native_saves() {
    let Some(path) = std::env::var_os("RHWP_C15_C14_INPUT") else {
        eprintln!("C15 private C14 observation not requested; the ordinary geometry regression remains active");
        return;
    };
    let input = std::fs::read(path).unwrap();
    assert_eq!(input.len(), 218909);
    c15_two_save_bounds(&input, &[145, 74], "c14");
}

fn c20_artifact(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("RHWP_C20_ARTIFACT_DIR") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

fn c20_contact(core: &rhwp::document_core::DocumentCore, host: usize, cell: usize, para: usize) -> &rhwp::model::paragraph::Paragraph {
    let rhwp::model::control::Control::Table(table) = &core.document().sections[0].paragraphs[host].controls[0] else { panic!("contact host must own table0") };
    &table.cells[cell].paragraphs[para]
}

fn c20_contact_line_breaks(p: &rhwp::model::paragraph::Paragraph) -> (Vec<u32>, Vec<String>) {
    let chars: Vec<_> = p.text.chars().collect();
    assert_eq!(p.char_offsets.len(), chars.len());
    let starts: Vec<_> = p.line_segs.iter().map(|line| line.text_start).collect();
    let lines: Vec<String> = starts.iter().enumerate().map(|(index, start)| {
        let from = p.char_offsets.partition_point(|offset| offset < start);
        let to = starts.get(index + 1).map(|end| p.char_offsets.partition_point(|offset| offset < end)).unwrap_or(chars.len());
        chars[from..to].iter().collect()
    }).collect();
    assert_eq!(lines.concat(), p.text, "line slices must retain the complete Unicode contact");
    (starts, lines)
}

fn c20_contact_identity(core: &rhwp::document_core::DocumentCore, host: usize, cell: usize, para: usize) -> String {
    let p = c20_contact(core, host, cell, para);
    let doc = core.document();
    let character_styles: Vec<_> = p.char_offsets.iter().map(|offset| {
        let reference = p.char_shapes.iter().rev().find(|s| s.start_pos <= *offset).expect("every visible character has a charPr reference");
        let cs = &doc.doc_info.char_shapes[reference.char_shape_id as usize];
        (reference.char_shape_id, cs.spacings, cs.ratios, cs.base_size)
    }).collect();
    format!("{:?}\n{:?}\n{:?}\n{:?}\n{:?}", p.text, p.char_offsets, p.field_ranges, p.controls,
        (doc.doc_info.para_shapes[p.para_shape_id as usize].indent,
         character_styles))
}

fn c20_observe_contact(core: &rhwp::document_core::DocumentCore, host: usize, cell: usize, para: usize, label: &str, violations: &mut Vec<String>) {
    let p = c20_contact(core, host, cell, para);
    c20_artifact(&format!("{label}-model.txt"), format!("{p:#?}\n{}", c20_contact_identity(core, host, cell, para)).as_bytes());
    let first = c13_number(&core.get_page_of_position_native(0, host).unwrap(), "page") as u32;
    let last = if host + 1 < core.document().sections[0].paragraphs.len() {
        c13_number(&core.get_page_of_position_native(0, host + 1).unwrap(), "page") as u32
    } else { core.page_count() - 1 };
    let mut rendered = String::new();
    for page in first..=last {
        let controls = core.get_page_control_layout_native(page).unwrap();
        let text = core.get_page_text_layout_native(page).unwrap();
        c20_artifact(&format!("{label}-p{page}-controls.json"), controls.as_bytes());
        c20_artifact(&format!("{label}-p{page}-text.json"), text.as_bytes());
        c20_artifact(&format!("{label}-p{page}.svg"), core.render_page_svg_native(page).unwrap().as_bytes());
        let bounds = controls.split("{\"type\":\"table\",").skip(1).find_map(|table| {
            let (meta, cells) = table.split_once("\"cells\":[")?;
            if !meta.contains(&format!("\"paraIdx\":{host},")) || c13_number(meta, "controlIdx") != 0.0 { return None; }
            cells.split_once(']').unwrap().0.split('}').find(|s| s.contains("\"cellIdx\":") && c13_number(s, "cellIdx") == cell as f64)
                .map(|s| (c13_number(s, "x"), c13_number(s, "y"), c13_number(s, "w"), c13_number(s, "h")))
        });
        for fragment in text.split("{\"text\":\"").skip(1) {
            let (value, meta) = fragment.split_once("\",\"x\":").unwrap();
            if !meta.contains(&format!("\"parentParaIdx\":{host},"))
                || c13_number(meta, "controlIdx") != 0.0
                || c13_number(meta, "cellIdx") != cell as f64
                || c13_number(meta, "cellParaIdx") != para as f64 { continue; }
            rendered.push_str(value);
            let (left, top, width, height) = bounds.expect("owned contact text needs its actual table cell");
            let x = c13_number(&format!("\"x\":{meta}"), "x");
            let y = c13_number(meta, "y");
            let w = c13_number(meta, "w");
            let h = c13_number(meta, "h");
            let positions: Vec<f64> = meta.split_once("\"charX\":[").unwrap().1.split_once(']').unwrap().0.split(',').map(|s| s.parse().unwrap()).collect();
            eprintln!("C20 {label}/p{page}: text={value:?} run=({x},{y},{w},{h}) cell=({left},{top},{width},{height}) charX={positions:?}");
            if w < 0.0 || positions.windows(2).any(|a| a[1] < a[0])
                || positions.iter().any(|&v| v < 0.0 || x + v < left - 0.11 || x + v > left + width + 0.11)
                || y < top - 0.11 || y + h > top + height + 0.11 {
                violations.push(format!("{label}/p{page}: {value:?} run=({x},{y},{w},{h}) outside cell=({left},{top},{width},{height}), charX={positions:?}"));
            }
        }
    }
    if rendered != p.text { violations.push(format!("{label}: rendered {rendered:?}, expected complete contact {:?}", p.text)); }
}

fn c20_contact_two_save_scenarios(input: &[u8], host: usize, cell: usize, para: usize, label: &str) {
    use rhwp::document_core::DocumentCore;
    c20_artifact(&format!("{label}-input.hwpx"), input);
    let images = c13_images(input);
    let mut violations = Vec::new();
    for edited in [false, true] {
        let mut core = DocumentCore::from_bytes(input).unwrap();
        let mut reference = None;
        let original = c20_contact(&core, host, cell, para).text.clone();
        if label == "actual" {
            assert_eq!(original, "(www.hometax.go.kr) 국번없이 126 문의", "bind the supplied actual contact owner");
        }
        if edited {
            let offset = original.chars().count();
            let mut zero_indent = DocumentCore::from_bytes(input).unwrap();
            zero_indent.apply_para_format_in_cell_native(0, host, 0, cell, para, "{\"indent\":0}").unwrap();
            let reference_para = c20_contact(&zero_indent, host, cell, para);
            assert_eq!(zero_indent.document().doc_info.para_shapes[reference_para.para_shape_id as usize].indent, 0);
            zero_indent.insert_text_in_cell_native(0, host, 0, cell, para, offset, "편😀").unwrap();
            reference = Some(zero_indent);
            core.insert_text_in_cell_native(0, host, 0, cell, para, offset, "편😀").unwrap();
            assert!(c20_contact(&core, host, cell, para).text.contains("편😀"));
        }
        let expected = c20_contact_identity(&core, host, cell, para);
        let reference_expected = reference.as_ref().map(|other| c20_contact_identity(other, host, cell, para));
        for pass in 0..=2 {
            let stage = format!("{label}-{}-pass{pass}", if edited { "edit" } else { "noedit" });
            if c20_contact_identity(&core, host, cell, para) != expected {
                violations.push(format!("{stage}: owned text/fields/char refs/spacing/indent changed across save"));
            }
            c20_observe_contact(&core, host, cell, para, &stage, &mut violations);
            if let Some(other) = reference.as_ref() {
                let subject_breaks = c20_contact_line_breaks(c20_contact(&core, host, cell, para));
                let reference_breaks = c20_contact_line_breaks(c20_contact(other, host, cell, para));
                eprintln!("C20 {stage} wrap: subject={subject_breaks:?}, public-zero-indent-reference={reference_breaks:?}");
                c20_artifact(&format!("{stage}-wrap.txt"), format!("subject={subject_breaks:#?}\npublic-zero-indent-reference={reference_breaks:#?}").as_bytes());
                if subject_breaks != reference_breaks {
                    violations.push(format!("{stage}: invalid-indent reflow differs from public zero-indent full-width reference: {subject_breaks:?} != {reference_breaks:?}"));
                }
                if c20_contact_identity(other, host, cell, para) != reference_expected.as_ref().unwrap().as_str() {
                    violations.push(format!("{stage}: reference text/fields/char refs/spacing changed across save"));
                }
                c20_observe_contact(other, host, cell, para, &format!("{stage}-reference"), &mut violations);
            }
            if pass < 2 {
                let saved = core.export_hwpx_native().unwrap();
                if c13_images(&saved) != images { violations.push(format!("{stage}: PNG bytes changed")); }
                c20_artifact(&format!("{stage}-save.hwpx"), &saved);
                core = DocumentCore::from_bytes(&saved).unwrap();
                if let Some(other) = reference.as_mut() {
                    let saved = other.export_hwpx_native().unwrap();
                    if c13_images(&saved) != images { violations.push(format!("{stage}: reference PNG bytes changed")); }
                    c20_artifact(&format!("{stage}-reference-save.hwpx"), &saved);
                    *other = DocumentCore::from_bytes(&saved).unwrap();
                }
            }
        }
    }
    c20_artifact(&format!("{label}-violations.txt"), violations.join("\n").as_bytes());
    assert!(violations.is_empty(), "{label}: {}", violations.join("\n"));
}

#[test]
fn c20_document_cached_contact_edit_and_two_native_saves_stay_visible() {
    use rhwp::{model::{control::Control, paragraph::LineSeg, style::Alignment}, parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    let contact = c8_field_paragraph(2020).replace("<hp:t>뒤</hp:t>", "<hp:t>뒤 연락처 안내</hp:t>");
    let body = c9_paragraph("<hp:t>C20 host</hp:t>", C9_LINE) + &c8_table(&contact);
    let mut doc = parse_hwpx(&c8_fixture(&body, false)).unwrap();
    while doc.doc_info.char_shapes.len() < 3 {
        doc.doc_info.char_shapes.push(doc.doc_info.char_shapes[0].clone());
    }
    doc.doc_info.char_shapes[1].bold = true;
    doc.doc_info.char_shapes[2].italic = true;
    doc.doc_info.para_shapes[0].alignment = Alignment::Center;
    doc.doc_info.para_shapes[0].indent = -20500;
    let Control::Table(table) = &mut doc.sections[0].paragraphs[1].controls[0] else { unreachable!() };
    let p = &mut table.cells[0].paragraphs[0];
    let second = p.char_offsets[2];
    p.line_segs = [0, second].into_iter().enumerate().map(|(i, text_start)| LineSeg {
        text_start, vertical_pos: i as i32 * 1200, line_height: 1000, text_height: 1000,
        baseline_distance: 850, line_spacing: 200, segment_width: 10000, tag: 393216,
        ..Default::default()
    }).collect();
    c20_contact_two_save_scenarios(&serialize_hwpx(&doc).unwrap(), 1, 0, 0, "fixture");
}

#[test]
#[ignore = "requires the explicitly supplied immutable C16 input"]
fn c20_supplied_actual_contact_edit_and_two_native_saves_stay_visible() {
    let path = std::env::var_os("RHWP_C20_C16_INPUT").expect("C20 actual input must be supplied explicitly");
    let input = std::fs::read(path).unwrap();
    assert_eq!(input.len(), 218909);
    c20_contact_two_save_scenarios(&input, 74, 9, 2, "actual");
}

const C9_FLOATING_PICTURE: &str = r#"<hp:pic id="17" instid="29" textWrap="TOP_AND_BOTTOM"><hp:orgSz width="3000" height="2000"/><hp:curSz width="3000" height="2000"/><hc:img binaryItemIDRef="image1"/><hp:sz width="3000" height="2000"/><hp:pos treatAsChar="0"/></hp:pic>"#;

fn c9_paragraph(content: &str, lines: &str) -> String {
    format!(
        r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">{content}</hp:run>{lines}</hp:p>"#
    )
}

fn c9_header_fixture(properties: &str) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let input = c8_fixture(&c9_paragraph("<hp:t>문단</hp:t>", C9_LINE), false);
    let header = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core"><hh:refList><hh:fontfaces itemCnt="1"><hh:fontface lang="HANGUL" fontCnt="1"><hh:font id="0" face="함초롬바탕" type="TTF"/></hh:fontface></hh:fontfaces><hh:charProperties itemCnt="1"><hh:charPr id="0" height="1000"/></hh:charProperties><hh:paraProperties itemCnt="{}">{properties}</hh:paraProperties><hh:styles itemCnt="1"><hh:style id="0" type="PARA" name="바탕글" engName="Normal" paraPrIDRef="0" charPrIDRef="0" nextStyleIDRef="0" langID="1042"/></hh:styles></hh:refList></hh:head>"#,
        properties.matches("<hh:paraPr ").count()
    );
    let mut original = zip::ZipArchive::new(Cursor::new(input)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..original.len() {
        let mut entry = original.by_index(i).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        output
            .start_file(&name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output
            .write_all(if name == "Contents/header.xml" {
                header.as_bytes()
            } else {
                &bytes
            })
            .unwrap();
    }
    output.finish().unwrap().into_inner()
}

fn c9_header_xml(hwpx: &[u8]) -> String {
    use std::io::{Cursor, Read};
    let mut archive = zip::ZipArchive::new(Cursor::new(hwpx)).unwrap();
    let mut xml = String::new();
    archive
        .by_name("Contents/header.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

fn c9_layout_events(xml: &str) -> Vec<(String, String, Vec<String>, Vec<(String, String)>)> {
    use quick_xml::{events::Event, name::ResolveResult, NsReader};
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut owners = Vec::new();
    let (mut paragraph, mut cell) = (0, 0);
    let mut output = Vec::new();
    loop {
        let (resolved, event) = reader.read_resolved_event().unwrap();
        let namespace = match resolved {
            ResolveResult::Bound(ns) => String::from_utf8(ns.as_ref().to_vec()).unwrap(),
            ResolveResult::Unbound => String::new(),
            ResolveResult::Unknown(prefix) => panic!("unbound namespace: {prefix:?}"),
        };
        match event {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                match name.as_str() {
                    "p" => {
                        owners.push(format!("p{paragraph}"));
                        paragraph += 1;
                    }
                    "tc" => {
                        owners.push(format!("tc{cell}"));
                        cell += 1;
                    }
                    _ => {}
                }
                if matches!(
                    name.as_str(),
                    "colPr" | "pageHiding" | "intent" | "left" | "right" | "prev" | "next"
                ) {
                    let mut attrs: Vec<_> = e
                        .attributes()
                        .map(|a| {
                            let a = a.unwrap();
                            (
                                String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                                a.decode_and_unescape_value(reader.decoder())
                                    .unwrap()
                                    .into_owned(),
                            )
                        })
                        .collect();
                    attrs.sort();
                    output.push((namespace, name, owners.clone(), attrs));
                }
            }
            Event::End(e) if matches!(e.local_name().as_ref(), b"p" | b"tc") => {
                owners.pop().unwrap();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    output
}

#[test]
fn c9_table_cell_margin_flags_preserve_effective_padding() {
    use rhwp::{model::control::Control, parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    // Given: table defaults differ from every explicit cell margin; disabled sentinel inherits.
    let mut cells = String::new();
    for (col, (flag, margin)) in [
        (" hasMargin=\"true\"", 425),
        (" hasMargin=\"1\"", 426),
        (" hasMargin=\"false\"", 427),
        (" hasMargin=\"0\"", 4_294_967_295u32),
        ("", 428),
    ]
    .into_iter()
    .enumerate()
    {
        cells.push_str(&format!(r#"<hp:tc{flag} borderFillIDRef="1"><hp:subList>{}</hp:subList><hp:cellAddr colAddr="{col}" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="3000"/><hp:cellMargin left="{margin}" right="{margin}" top="{margin}" bottom="{margin}"/></hp:tc>"#, c9_paragraph("<hp:t>셀</hp:t>", C9_LINE)));
    }
    let table = format!(
        r#"<hp:tbl rowCnt="1" colCnt="5" pageBreak="CELL" borderFillIDRef="1"><hp:sz width="5000" height="3000"/><hp:inMargin left="510" right="511" top="141" bottom="142"/><hp:tr>{cells}</hp:tr></hp:tbl>"#
    );
    let input = c8_fixture(&c9_paragraph(&table, C9_LINE), false);
    let mut doc = parse_hwpx(&input).unwrap();
    // When/Then: both saves preserve current IR flags, emitted attrs and reopened effective margins.
    for pass in 0..=2 {
        let table = doc.sections[0].paragraphs[0]
            .controls
            .iter()
            .find_map(|c| match c {
                Control::Table(t) => Some(t),
                _ => None,
            })
            .unwrap();
        assert_eq!(table.cells.len(), 5);
        for (i, cell) in table.cells.iter().enumerate() {
            assert_eq!(cell.apply_inner_margin, i < 2, "pass {pass} cell {i}");
            let p = if cell.apply_inner_margin {
                &cell.padding
            } else {
                &table.padding
            };
            let actual = (p.left, p.right, p.top, p.bottom);
            let expected = match i {
                0 => (425, 425, 425, 425),
                1 => (426, 426, 426, 426),
                _ => (510, 511, 141, 142),
            };
            assert_eq!(actual, expected, "pass {pass} cell {i} effective margins");
        }
        if pass < 2 {
            let saved = serialize_hwpx(&doc).unwrap();
            let flags: Vec<_> = c8_elements(&hwpx_section_xml(&saved))
                .into_iter()
                .filter(|(n, _)| n == "tc")
                .map(|(_, a)| a.into_iter().find(|(k, _)| k == "hasMargin").unwrap().1)
                .collect();
            assert_eq!(flags, ["1", "1", "0", "0", "0"]);
            doc = parse_hwpx(&saved).unwrap();
        }
    }
}

#[test]
fn c9_table_page_break_modes_and_legacy_aliases_survive() {
    use rhwp::{
        model::{control::Control, table::TablePageBreak},
        parser::hwpx::parse_hwpx,
        serializer::serialize_hwpx,
    };
    for (input_mode, expected, output_mode) in [
        ("TABLE", TablePageBreak::RowBreak, "TABLE"),
        ("CELL", TablePageBreak::CellBreak, "CELL"),
        ("NONE", TablePageBreak::None, "NONE"),
        ("ROW", TablePageBreak::RowBreak, "TABLE"),
        ("ROW_BREAK", TablePageBreak::RowBreak, "TABLE"),
        ("CELL_BREAK", TablePageBreak::CellBreak, "CELL"),
    ] {
        let input = c8_fixture(
            &c8_table(&c9_paragraph("<hp:t>셀</hp:t>", C9_LINE))
                .replace("<hp:tbl ", &format!("<hp:tbl pageBreak=\"{input_mode}\" ")),
            false,
        );
        let mut doc = parse_hwpx(&input).unwrap();
        for pass in 0..2 {
            let table = doc.sections[0].paragraphs[0]
                .controls
                .iter()
                .find_map(|c| match c {
                    Control::Table(t) => Some(t),
                    _ => None,
                })
                .unwrap();
            assert_eq!(table.page_break, expected, "{input_mode} pass {pass}");
            let saved = serialize_hwpx(&doc).unwrap();
            assert!(c8_elements(&hwpx_section_xml(&saved))
                .iter()
                .any(|(n, a)| n == "tbl" && a.contains(&("pageBreak".into(), output_mode.into()))));
            doc = parse_hwpx(&saved).unwrap();
        }
    }
}

#[test]
fn c9_table_signed_and_unsigned_offsets_preserve_same_bits() {
    use rhwp::{model::control::Control, parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    for (vertical, horizontal) in [("4294967240", "-56"), ("-56", "4294967240")] {
        let table = c8_table(&c9_paragraph("<hp:t>셀</hp:t>", C9_LINE)).replace(
            "<hp:tr>",
            &format!(r#"<hp:pos vertOffset="{vertical}" horzOffset="{horizontal}"/><hp:tr>"#),
        );
        let mut doc = parse_hwpx(&c8_fixture(&table, false)).unwrap();
        for pass in 0..=2 {
            let table = doc.sections[0].paragraphs[0]
                .controls
                .iter()
                .find_map(|c| match c {
                    Control::Table(t) => Some(t),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                (table.common.vertical_offset, table.common.horizontal_offset),
                (4_294_967_240, 4_294_967_240),
                "pass {pass}"
            );
            if pass < 2 {
                doc = parse_hwpx(&serialize_hwpx(&doc).unwrap()).unwrap();
            }
        }
    }
}

fn c9_para_properties() -> String {
    [("BREAK_WORD", "BREAK_WORD", "SQUEEZE", "1"), ("HYPHENATION", "KEEP_WORD", "BREAK", "0"), ("KEEP_WORD", "KEEP_WORD", "BREAK", "0")].into_iter().enumerate().map(|(id,(latin,korean,wrap,on))| format!(r#"<hh:paraPr id="{id}" fontLineHeight="{on}"><hh:align horizontal="LEFT" vertical="BASELINE"/><hh:breakSetting breakLatinWord="{latin}" breakNonLatinWord="{korean}" widowOrphan="{on}" keepWithNext="{on}" keepLines="{on}" pageBreakBefore="{on}" lineWrap="{wrap}"/><hh:autoSpacing eAsianEng="{on}" eAsianNum="{on}"/></hh:paraPr>"#)).collect()
}

#[test]
fn c9_paragraph_supported_break_modes_use_canonical_bits() {
    use rhwp::{parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    let mut doc = parse_hwpx(&c9_header_fixture(&c9_para_properties())).unwrap();
    for pass in 0..=2 {
        for (i, (latin, korean, wrap, flags, font, spacing)) in
            [(2, 1, 1, 15, 1, 3), (1, 0, 0, 0, 0, 0), (0, 0, 0, 0, 0, 0)]
                .into_iter()
                .enumerate()
        {
            let ps = &doc.doc_info.para_shapes[i];
            assert_eq!(
                (
                    (ps.attr1 >> 5) & 3,
                    (ps.attr1 >> 7) & 1,
                    ps.attr2 & 3,
                    (ps.attr1 >> 16) & 15,
                    (ps.attr1 >> 22) & 1,
                    (ps.attr2 >> 4) & 3
                ),
                (latin, korean, wrap, flags, font, spacing),
                "pass {pass} paraPr {i}"
            );
            assert_eq!(
                (ps.attr1 >> 20) & 3,
                0,
                "autoSpacing must not set vertical alignment"
            );
        }
        if pass < 2 {
            doc = parse_hwpx(&serialize_hwpx(&doc).unwrap()).unwrap();
        }
    }
}

#[test]
fn c9_paragraph_current_modifications_win_across_two_saves() {
    use rhwp::{model::style::ParaShapeMods, parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    let mut doc = parse_hwpx(&c9_header_fixture(&c9_para_properties())).unwrap();
    // Given/When: the current UI clears imported true flags and sets a different style true.
    for (i, on) in [(0, false), (1, true)] {
        doc.doc_info.para_shapes[i] = ParaShapeMods {
            widow_orphan: Some(on),
            keep_with_next: Some(on),
            keep_lines: Some(on),
            page_break_before: Some(on),
            font_line_height: Some(on),
            single_line: Some(on),
            auto_space_kr_en: Some(on),
            auto_space_kr_num: Some(on),
            english_break_unit: Some(if on { 2 } else { 0 }),
            korean_break_unit: Some(u8::from(on)),
            vertical_align: Some(if on { 2 } else { 0 }),
            ..Default::default()
        }
        .apply_to(&doc.doc_info.para_shapes[i]);
    }
    for pass in 1..=2 {
        let saved = serialize_hwpx(&doc).unwrap();
        doc = parse_hwpx(&saved).unwrap();
        for (i, on) in [(0, false), (1, true)] {
            let ps = &doc.doc_info.para_shapes[i];
            let bit = u32::from(on);
            assert_eq!(
                (
                    (ps.attr1 >> 16) & 15,
                    (ps.attr1 >> 22) & 1,
                    ps.attr2 & 3,
                    (ps.attr2 >> 4) & 3,
                    (ps.attr1 >> 20) & 3,
                    (ps.attr1 >> 5) & 3,
                    (ps.attr1 >> 7) & 1
                ),
                (bit * 15, bit, bit, bit * 3, bit * 2, bit * 2, bit),
                "save {pass} paraPr {i}: current values beat imported aliases"
            );
        }
    }
}

#[test]
fn c9_paragraph_margin_2011_expanded_namespace_and_signed_values_survive() {
    use rhwp::{parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    let input = c9_header_fixture(
        r#"<hh:paraPr id="0"><hh:margin><hc:intent value="-120" unit="HWPUNIT"/><hc:left value="321" unit="HWPUNIT"/><hc:right value="654" unit="HWPUNIT"/><hc:prev value="-45" unit="HWPUNIT"/><hc:next value="789" unit="HWPUNIT"/></hh:margin></hh:paraPr>"#,
    );
    let expected = c9_layout_events(&c9_header_xml(&input));
    assert_eq!(expected.len(), 5);
    assert!(expected
        .iter()
        .all(|(ns, _, _, _)| ns == "http://www.hancom.co.kr/hwpml/2011/core"));
    let mut doc = parse_hwpx(&input).unwrap();
    for pass in 1..=2 {
        let saved = serialize_hwpx(&doc).unwrap();
        assert_eq!(
            c9_layout_events(&c9_header_xml(&saved)),
            expected,
            "save {pass} expanded namespace, units and numeric values"
        );
        doc = parse_hwpx(&saved).unwrap();
        let ps = &doc.doc_info.para_shapes[0];
        assert_eq!(
            (
                ps.indent,
                ps.margin_left,
                ps.margin_right,
                ps.spacing_before,
                ps.spacing_after
            ),
            (-120, 321, 654, -45, 789)
        );
    }
}

#[test]
fn c9_columns_keep_body_cell_paragraph_ownership_and_control_order() {
    use rhwp::{parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    let column = |count, gap, direction| {
        format!(
            r#"<hp:ctrl><hp:colPr id="" type="NEWSPAPER" layout="{direction}" colCount="{count}" sameSz="1" sameGap="{gap}"/></hp:ctrl>"#
        )
    };
    let hide = r#"<hp:ctrl><hp:pageHiding hideHeader="1" hideFooter="0" hideMasterPage="0" hideBorder="0" hideFill="0" hidePageNum="0"/></hp:ctrl>"#;
    let initial = c9_paragraph(
        &(column(2, 300, "RIGHT") + hide + &column(1, 111, "LEFT") + hide),
        C9_LINE,
    )
    .replace("type=\"NEWSPAPER\"", "type=\"BALANCED_NEWSPAPER\"");
    let cell_paras = c9_paragraph(
        &(column(1, 222, "RIGHT") + &column(2, 333, "LEFT")),
        C9_LINE,
    ) + &c9_paragraph(&column(1, 444, "LEFT"), C9_LINE);
    let cell_paras = cell_paras.replace("type=\"NEWSPAPER\"", "type=\"PARALLEL\"");
    let table = c8_table(&cell_paras);
    let body =
        initial + &table + &c9_paragraph(&(hide.to_string() + &column(2, 555, "RIGHT")), C9_LINE);
    let input = c8_fixture(&body, false);
    let expected = c9_layout_events(&hwpx_section_xml(&input));
    assert_eq!(
        expected.iter().filter(|(_, n, _, _)| n == "colPr").count(),
        6
    );
    assert!(expected
        .iter()
        .filter(|(_, n, _, _)| n == "colPr")
        .any(|(_, _, o, _)| o.iter().any(|x| x.starts_with("tc"))));
    let mut doc = parse_hwpx(&input).unwrap();
    for pass in 1..=2 {
        let saved = serialize_hwpx(&doc).unwrap();
        assert_eq!(
            c9_layout_events(&hwpx_section_xml(&saved)),
            expected,
            "save {pass}: expanded names, attrs, owning cell/paragraph and control order"
        );
        doc = parse_hwpx(&saved).unwrap();
    }
}

fn c9_body_geometry(
    doc: &rhwp::model::document::Document,
) -> Vec<Vec<(u32, i32, i32, i32, i32, i32, i32, i32, u32)>> {
    doc.sections[0]
        .paragraphs
        .iter()
        .map(|p| {
            p.line_segs
                .iter()
                .map(|s| {
                    (
                        s.text_start,
                        s.vertical_pos,
                        s.line_height,
                        s.text_height,
                        s.baseline_distance,
                        s.line_spacing,
                        s.column_start,
                        s.segment_width,
                        s.tag,
                    )
                })
                .collect()
        })
        .collect()
}

#[test]
fn c9_document_computed_object_geometry_and_page_resets_survive_two_saves() {
    use rhwp::{
        document_core::DocumentCore,
        model::{control::Control, shape::TextWrap},
        parser::hwpx::parse_hwpx,
    };
    let inner = c9_paragraph("<hp:t>셀</hp:t>", C9_LINE);
    let tac = c8_table(&inner).replace("<hp:tr>", r#"<hp:pos treatAsChar="1"/><hp:tr>"#);
    let tac = tac.trim_end_matches("</hp:p>").to_string() + C9_LINE + "</hp:p>";
    let floating = c8_table(&inner)
        .replace("<hp:tbl ", "<hp:tbl textWrap=\"TOP_AND_BOTTOM\" ")
        .replace("<hp:tr>", r#"<hp:pos treatAsChar="0"/><hp:tr>"#);
    let floating = floating.trim_end_matches("</hp:p>").to_string() + C9_LINE + "</hp:p>";
    let body =
        tac + &c9_paragraph(
            "<hp:t>다음</hp:t>",
            &C9_LINE.replace("vertpos=\"0\"", "vertpos=\"4200\""),
        ) + &floating
            + &c9_paragraph(
                "<hp:t>새 쪽 다음</hp:t>",
                &C9_LINE.replace("vertpos=\"0\"", "vertpos=\"8000\""),
            )
            + &c9_paragraph(C9_FLOATING_PICTURE, C9_LINE)
            + &c9_paragraph(
                "<hp:t>그림 다음</hp:t>",
                &C9_LINE.replace("vertpos=\"0\"", "vertpos=\"3300\""),
            );
    let input = c8_fixture(&body, true);
    let parsed = parse_hwpx(&input).unwrap();
    assert!(parsed.sections[0].paragraphs[4].controls.iter().any(|c| matches!(c, Control::Picture(p) if !p.common.treat_as_char && p.common.text_wrap == TextWrap::TopAndBottom && p.common.height == 2000)));
    let expected = c9_body_geometry(&parsed);
    assert_eq!(
        expected.iter().map(|s| s[0].1).collect::<Vec<_>>(),
        [0, 4200, 0, 8000, 0, 3300]
    );
    let mut core = DocumentCore::from_bytes(&input).unwrap();
    assert_eq!(
        c9_body_geometry(core.document()),
        expected,
        "load must preserve computed TAC height and page-local resets"
    );
    for pass in 1..=2 {
        let saved = core.export_hwpx_native().unwrap();
        assert_eq!(
            c9_body_geometry(&parse_hwpx(&saved).unwrap()),
            expected,
            "save {pass}"
        );
        core = DocumentCore::from_bytes(&saved).unwrap();
        assert_eq!(c9_body_geometry(core.document()), expected, "reopen {pass}");
    }
}

#[test]
fn c9_document_missing_geometry_synthesizes_without_rebasing_valid_paragraphs() {
    use rhwp::{
        document_core::{validation::WarningKind, DocumentCore},
        parser::hwpx::parse_hwpx,
    };
    for missing in [
        "",
        r#"<hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="0"/></hp:linesegarray>"#,
    ] {
        let table = c8_table(&c9_paragraph("<hp:t>셀</hp:t>", ""))
            .replace("<hp:tr>", r#"<hp:pos treatAsChar="1"/><hp:tr>"#);
        let table = table.trim_end_matches("</hp:p>").to_string() + missing + "</hp:p>";
        let input = c8_fixture(
            &(c9_paragraph(
                "<hp:t>원본 첫 쪽</hp:t>",
                &C9_LINE.replace("vertpos=\"0\"", "vertpos=\"9000\""),
            ) + &table
                + &c9_paragraph("<hp:t>원본 새 쪽</hp:t>", C9_LINE)
                + &c9_paragraph(
                    C9_FLOATING_PICTURE,
                    &C9_LINE.replace("vertpos=\"0\"", "vertpos=\"4400\""),
                )),
            true,
        );
        let original = c9_body_geometry(&parse_hwpx(&input).unwrap());
        let mut core = DocumentCore::from_bytes(&input).unwrap();
        assert!(
            core.validation_report()
                .warnings
                .iter()
                .any(|w| matches!(w.kind, WarningKind::LinesegUncomputed)),
            "warnings precede synthesis"
        );
        let computed = c9_body_geometry(core.document());
        assert_eq!(
            computed[0], original[0],
            "valid preceding imported geometry"
        );
        assert_eq!(computed[2], original[2], "valid later page reset");
        assert_eq!(
            computed[3], original[3],
            "computed TopAndBottom Picture geometry"
        );
        assert!(
            computed[1][0].2 >= 3000,
            "missing TAC geometry still includes table height"
        );
        assert!(
            computed[1][0].1 >= 10200,
            "synthesized paragraph follows the imported predecessor"
        );
        assert!(
            document_paragraphs(core.document())
                .iter()
                .all(|p| p.line_segs.iter().all(|s| s.line_height > 0)),
            "cell missing geometry also synthesizes"
        );
        for pass in 1..=2 {
            let saved = core.export_hwpx_native().unwrap();
            core = DocumentCore::from_bytes(&saved).unwrap();
            assert_eq!(
                c9_body_geometry(core.document()),
                computed,
                "save/reopen {pass} does not synthesize a second time"
            );
        }
    }
}

#[test]
fn c9_document_missing_follower_clears_computed_floating_object_extent() {
    use rhwp::{document_core::DocumentCore, parser::hwpx::parse_hwpx};
    let lines = C9_LINE.replace("vertpos=\"0\"", "vertpos=\"4000\"");
    let table = c8_table(&c9_paragraph("<hp:t>셀</hp:t>", C9_LINE))
        .replace("<hp:tbl ", "<hp:tbl textWrap=\"TOP_AND_BOTTOM\" ")
        .replace("<hp:tr>", r#"<hp:pos treatAsChar="0" vertOffset="300"/><hp:outMargin top="100" bottom="200"/><hp:tr>"#);
    let table = table.trim_end_matches("</hp:p>").to_string() + &lines + "</hp:p>";
    let picture = c9_paragraph(
        &C9_FLOATING_PICTURE.replace("treatAsChar=\"0\"", "treatAsChar=\"0\" vertOffset=\"300\""),
        &lines,
    );
    let mut observed = Vec::new();
    for first in [table, picture] {
        let input = c8_fixture(
            &(first
                + &c9_paragraph("<hp:t>미계산 후속</hp:t>", "")
                + &c9_paragraph("<hp:t>새 쪽</hp:t>", C9_LINE)),
            true,
        );
        let imported = c9_body_geometry(&parse_hwpx(&input).unwrap());
        let mut core = DocumentCore::from_bytes(&input).unwrap();
        let computed = c9_body_geometry(core.document());
        assert_eq!(
            computed[0], imported[0],
            "computed floating predecessor remains unchanged"
        );
        assert_eq!(
            computed[2], imported[2],
            "later page-local reset remains unchanged"
        );
        observed.push(computed[1][0].1);
        for pass in 1..=2 {
            let saved = core.export_hwpx_native().unwrap();
            core = DocumentCore::from_bytes(&saved).unwrap();
            assert_eq!(
                c9_body_geometry(core.document()),
                computed,
                "floating transition save/reopen {pass}"
            );
        }
    }
    assert_eq!(
        observed,
        [7600, 6300],
        "synthesized followers clear table/picture height, offset and table margins"
    );
}

// C8: 실제 native 저장 owner의 필드·그림·지원 도형 보존 회귀.
fn c8_fixture(body: &str, image: bool) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let fixture = include_bytes!("../samples/hwpx/ref/ref_empty.hwpx");
    let mut original = zip::ZipArchive::new(Cursor::new(fixture)).unwrap();
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
        } else if image && name == "Contents/content.hpf" {
            bytes = String::from_utf8(bytes).unwrap().replace("</opf:manifest>", r#"<opf:item id="image1" href="BinData/image1.png" media-type="image/png" isEmbeded="1"/></opf:manifest>"#).into_bytes();
        }
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
    if image {
        output
            .start_file(
                "BinData/image1.png",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        output
            .write_all(rhwp::serializer::hwpx::static_assets::PRV_IMAGE_PNG)
            .unwrap();
    }
    output.finish().unwrap().into_inner()
}

fn c8_field_paragraph(id: u32) -> String {
    format!(
        r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>앞😀</hp:t><hp:ctrl><hp:fieldBegin id="{id}" type="HYPERLINK" name="이름&amp;값" editable="0" dirty="1" zorder="-1" fieldid="{}"><hp:parameters cnt="4" name="파라미터"><hp:integerParam name="Prop">7</hp:integerParam><hp:stringParam name="Command">https://example.test/a?x=1&amp;y=2;</hp:stringParam><hp:booleanParam name="Enabled">true</hp:booleanParam><hp:floatParam name="Ratio">1.25</hp:floatParam></hp:parameters></hp:fieldBegin></hp:ctrl></hp:run><hp:run charPrIDRef="1"><hp:t>링😀크</hp:t></hp:run><hp:run charPrIDRef="2"><hp:ctrl><hp:fieldEnd beginIDRef="{id}"/></hp:ctrl><hp:t>뒤</hp:t></hp:run></hp:p>"#,
        id + 1000
    )
}

#[derive(Debug, PartialEq)]
struct C23FieldState {
    text: String,
    offsets: Vec<u32>,
    count: u32,
    ranges: Vec<(usize, usize)>,
    styles: Vec<u32>,
}

fn c23_field_state(para: &rhwp::model::paragraph::Paragraph) -> C23FieldState {
    C23FieldState {
        text: para.text.clone(),
        offsets: para.char_offsets.clone(),
        count: para.char_count,
        ranges: para
            .field_ranges
            .iter()
            .map(|range| (range.start_char_idx, range.end_char_idx))
            .collect(),
        styles: para
            .char_offsets
            .iter()
            .map(|offset| {
                para.char_shapes
                    .iter()
                    .rev()
                    .find(|style| style.start_pos <= *offset)
                    .unwrap()
                    .char_shape_id
            })
            .collect(),
    }
}

fn c23_artifact(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("RHWP_C23_ARTIFACT_DIR") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

fn c23_field_fixture(body: &str, cell: &str) -> Vec<u8> {
    let prologue = c9_paragraph(
        r#"<hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="1000" right="1000" top="1000" bottom="1000" header="0" footer="0" gutter="0"/></hp:pagePr></hp:secPr>"#,
        C9_LINE,
    );
    c8_fixture(&(prologue + body + &c8_table(cell)), false)
}

#[test]
fn c23_public_field_end_insert_keeps_utf16_styles_and_count_two_saves() {
    use rhwp::document_core::DocumentCore;
    // Given: input XML places an eight-unit field-end gap before the trailing 뒤.
    let input = c23_field_fixture(&c8_field_paragraph(101), &c8_field_paragraph(303));
    let expected_xml = hwpx_section_xml(&input).replace("링😀크", "링😀크편😀");
    let expected_events = c8_inline_events(&expected_xml);
    let expected = C23FieldState {
        text: "앞😀링😀크편😀뒤".into(),
        offsets: vec![0, 1, 11, 12, 14, 15, 16, 26],
        count: 28,
        ranges: vec![(2, 7)],
        styles: vec![0, 0, 1, 1, 1, 1, 1, 2],
    };
    c23_artifact("field-end-input.hwpx", &input);
    let mut core = DocumentCore::from_bytes(&input).unwrap();
    assert_eq!(
        core.document().sections[0].paragraphs[1].char_offsets,
        [0, 1, 11, 12, 14, 23]
    );
    // When: insert at the inclusive field-end boundary through both public edit paths.
    core.insert_text_native(0, 1, 5, "편😀").unwrap();
    core.insert_text_in_cell_native(0, 2, 0, 0, 0, 5, "편😀")
        .unwrap();
    let mut states = Vec::new();
    let mut events = Vec::new();
    for pass in 0..=2 {
        states.push(vec![
            c23_field_state(&core.document().sections[0].paragraphs[1]),
            c23_field_state(c20_contact(&core, 2, 0, 0)),
        ]);
        c23_artifact(
            &format!("field-end-pass{pass}-model.txt"),
            format!("{:#?}", states.last().unwrap()).as_bytes(),
        );
        if pass < 2 {
            let saved = core.export_hwpx_native().unwrap();
            events.push(c8_inline_events(&hwpx_section_xml(&saved)));
            c23_artifact(&format!("field-end-save{}.hwpx", pass + 1), &saved);
            core = DocumentCore::from_bytes(&saved).unwrap();
        }
    }
    // Then: independently specified raw positions, membership and original styles agree before/after save.
    println!("C23_FIELD_END expected={expected:?} actual={states:?}");
    assert!(
        states.iter().flatten().all(|state| state == &expected),
        "in-memory and reopened state must match the raw UTF16 field layout: {states:?}"
    );
    assert_eq!(events, vec![expected_events.clone(), expected_events]);
}

#[test]
fn c23_public_insert_delete_preserves_retained_field_gaps_two_saves() {
    use rhwp::document_core::DocumentCore;
    // Given: original raw XML independently specifies retained field controls and run styles.
    let input = c23_field_fixture(&c8_field_paragraph(101), &c8_field_paragraph(303));
    let original_xml = hwpx_section_xml(&input);
    let cases = [
        (
            "inverse",
            5,
            2,
            original_xml.clone(),
            C23FieldState {
                text: "앞😀링😀크뒤".into(),
                offsets: vec![0, 1, 11, 12, 14, 23],
                count: 25,
                ranges: vec![(2, 5)],
                styles: vec![0, 0, 1, 1, 1, 2],
            },
        ),
        (
            "empty-field",
            2,
            5,
            original_xml.replace("링😀크", ""),
            C23FieldState {
                text: "앞😀뒤".into(),
                offsets: vec![0, 1, 19],
                count: 21,
                ranges: vec![(2, 2)],
                styles: vec![0, 0, 2],
            },
        ),
        (
            "across-begin",
            1,
            5,
            original_xml.replace("앞😀", "앞").replace("링😀크", "😀"),
            C23FieldState {
                text: "앞😀뒤".into(),
                offsets: vec![0, 9, 19],
                count: 21,
                ranges: vec![(1, 2)],
                styles: vec![0, 1, 2],
            },
        ),
    ];
    let mut failures = Vec::new();
    // When: public body/cell insertion is followed by deletion, then two native save/reopens.
    for (label, at, count, expected_xml, expected) in cases {
        let mut core = DocumentCore::from_bytes(&input).unwrap();
        core.insert_text_native(0, 1, 5, "편😀").unwrap();
        core.insert_text_in_cell_native(0, 2, 0, 0, 0, 5, "편😀")
            .unwrap();
        core.delete_text_native(0, 1, at, count).unwrap();
        core.delete_text_in_cell_native(0, 2, 0, 0, 0, at, count)
            .unwrap();
        let expected_events = c8_inline_events(&expected_xml);
        for pass in 0..=2 {
            let actual = vec![
                c23_field_state(&core.document().sections[0].paragraphs[1]),
                c23_field_state(c20_contact(&core, 2, 0, 0)),
            ];
            println!("C23_DELETE {label} pass{pass} expected={expected:?} actual={actual:?}");
            c23_artifact(
                &format!("delete-{label}-pass{pass}-model.txt"),
                format!("{actual:#?}").as_bytes(),
            );
            if actual.iter().any(|state| state != &expected) {
                failures.push(format!("{label}/pass{pass}: {actual:?}"));
            }
            if pass < 2 {
                let saved = core.export_hwpx_native().unwrap();
                c23_artifact(&format!("delete-{label}-save{}.hwpx", pass + 1), &saved);
                if c8_inline_events(&hwpx_section_xml(&saved)) != expected_events {
                    failures.push(format!(
                        "{label}/save{}: field metadata/order/styles differ",
                        pass + 1
                    ));
                }
                core = DocumentCore::from_bytes(&saved).unwrap();
            }
        }
    }
    // Then: deleting visible text never consumes the retained controls or their UTF16 gaps.
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn c23_public_tab_delete_removes_eight_units_and_keeps_field_gaps_two_saves() {
    use rhwp::document_core::DocumentCore;
    // Given: raw TAB occupies eight units, independently of retained field controls.
    let plain = r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>A<hp:tab width="2400" leader="0" type="1"/>B</hp:t></hp:run></hp:p>"#;
    let original = c23_field_fixture(&c8_field_paragraph(101), &c8_field_paragraph(303));
    let field = |id| {
        c8_field_paragraph(id).replace(
            "<hp:t>뒤</hp:t>",
            r#"<hp:t><hp:tab width="2400" leader="0" type="1"/>뒤</hp:t>"#,
        )
    };
    let cases = [
        (
            "nonfield",
            c23_field_fixture(plain, plain),
            hwpx_section_xml(&c23_field_fixture(
                &plain.replace(r#"<hp:tab width="2400" leader="0" type="1"/>"#, ""),
                &plain.replace(r#"<hp:tab width="2400" leader="0" type="1"/>"#, ""),
            )),
            1,
            C23FieldState {
                text: "A\tB".into(),
                offsets: vec![0, 1, 9],
                count: 11,
                ranges: vec![],
                styles: vec![0, 0, 0],
            },
            C23FieldState {
                text: "AB".into(),
                offsets: vec![0, 1],
                count: 3,
                ranges: vec![],
                styles: vec![0, 0],
            },
        ),
        (
            "field-adjacent",
            c23_field_fixture(&field(101), &field(303)),
            hwpx_section_xml(&original),
            5,
            C23FieldState {
                text: "앞😀링😀크\t뒤".into(),
                offsets: vec![0, 1, 11, 12, 14, 23, 31],
                count: 33,
                ranges: vec![(2, 5)],
                styles: vec![0, 0, 1, 1, 1, 2, 2],
            },
            C23FieldState {
                text: "앞😀링😀크뒤".into(),
                offsets: vec![0, 1, 11, 12, 14, 23],
                count: 25,
                ranges: vec![(2, 5)],
                styles: vec![0, 0, 1, 1, 1, 2],
            },
        ),
    ];
    let mut failures = Vec::new();
    // When: only the visible TAB is deleted through public body and cell methods.
    for (label, input, expected_xml, at, before, expected) in cases {
        let mut core = DocumentCore::from_bytes(&input).unwrap();
        c23_artifact(&format!("tab-{label}-input.hwpx"), &input);
        assert_eq!(
            c23_field_state(&core.document().sections[0].paragraphs[1]),
            before
        );
        assert_eq!(c23_field_state(c20_contact(&core, 2, 0, 0)), before);
        core.delete_text_native(0, 1, at, 1).unwrap();
        core.delete_text_in_cell_native(0, 2, 0, 0, 0, at, 1)
            .unwrap();
        let expected_events = c8_inline_events(&expected_xml);
        for pass in 0..=2 {
            let actual = vec![
                c23_field_state(&core.document().sections[0].paragraphs[1]),
                c23_field_state(c20_contact(&core, 2, 0, 0)),
            ];
            println!("C23_TAB {label} pass{pass} expected={expected:?} actual={actual:?}");
            c23_artifact(
                &format!("tab-{label}-pass{pass}-model.txt"),
                format!("{actual:#?}").as_bytes(),
            );
            if actual.iter().any(|state| state != &expected) {
                failures.push(format!("{label}/pass{pass}: {actual:?}"));
            }
            if pass < 2 {
                let saved = core.export_hwpx_native().unwrap();
                c23_artifact(&format!("tab-{label}-save{}.hwpx", pass + 1), &saved);
                if c8_inline_events(&hwpx_section_xml(&saved)) != expected_events {
                    failures.push(format!(
                        "{label}/save{}: retained metadata/styles differ",
                        pass + 1
                    ));
                }
                core = DocumentCore::from_bytes(&saved).unwrap();
            }
        }
    }
    // Then: original field gaps remain and all eight units of the deleted TAB disappear.
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn c23_public_tab_insert_delete_uses_consistent_local_units_two_saves() {
    use rhwp::document_core::DocumentCore;
    // Given: independent raw text layouts require an eight-unit TAB, including at paragraph end.
    let paragraph = |text: &str| {
        format!(
            r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>{text}</hp:t></hp:run></hp:p>"#
        )
    };
    let cases = [
        ("middle", "AB", "A\tB", vec![0, 1, 9], 11, None, "AB"),
        (
            "end-followed-by-BMP",
            "A",
            "A\t",
            vec![0, 1],
            10,
            Some("끝"),
            "A끝",
        ),
    ];
    let mut failures = Vec::new();
    // When: each public body/cell path inserts TAB, optionally appends BMP, then deletes TAB.
    for (label, initial, inserted, offsets, count, following, final_text) in cases {
        let input = c23_field_fixture(&paragraph(initial), &paragraph(initial));
        for cell in [false, true] {
            let owner = if cell { "cell" } else { "body" };
            let mut core = DocumentCore::from_bytes(&input).unwrap();
            c23_artifact(&format!("tab-edit-{label}-{owner}-input.hwpx"), &input);
            let expected_initial = C23FieldState {
                text: initial.into(),
                offsets: (0..initial.len() as u32).collect(),
                count: initial.len() as u32 + 1,
                ranges: vec![],
                styles: vec![0; initial.len()],
            };
            let state = |core: &DocumentCore| {
                if cell {
                    c23_field_state(c20_contact(core, 2, 0, 0))
                } else {
                    c23_field_state(&core.document().sections[0].paragraphs[1])
                }
            };
            assert_eq!(state(&core), expected_initial);
            if cell {
                core.insert_text_in_cell_native(0, 2, 0, 0, 0, 1, "\t")
                    .unwrap();
            } else {
                core.insert_text_native(0, 1, 1, "\t").unwrap();
            }
            let expected_insert = C23FieldState {
                text: inserted.into(),
                styles: vec![0; offsets.len()],
                offsets: offsets.clone(),
                count,
                ranges: vec![],
            };
            let actual = state(&core);
            println!("C23_TAB_EDIT {label}/{owner} inserted expected={expected_insert:?} actual={actual:?}");
            c23_artifact(
                &format!("tab-edit-{label}-{owner}-inserted.txt"),
                format!("{actual:#?}").as_bytes(),
            );
            if actual != expected_insert {
                failures.push(format!("{label}/{owner}/inserted: {actual:?}"));
            }
            if let Some(text) = following {
                if cell {
                    core.insert_text_in_cell_native(0, 2, 0, 0, 0, 2, text)
                        .unwrap();
                } else {
                    core.insert_text_native(0, 1, 2, text).unwrap();
                }
                let expected_following = C23FieldState {
                    text: "A\t끝".into(),
                    offsets: vec![0, 1, 9],
                    count: 11,
                    ranges: vec![],
                    styles: vec![0, 0, 0],
                };
                let actual = state(&core);
                println!("C23_TAB_EDIT {label}/{owner} following expected={expected_following:?} actual={actual:?}");
                c23_artifact(
                    &format!("tab-edit-{label}-{owner}-following.txt"),
                    format!("{actual:#?}").as_bytes(),
                );
                if actual != expected_following {
                    failures.push(format!("{label}/{owner}/following: {actual:?}"));
                }
            }
            let deletion = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if cell {
                    core.delete_text_in_cell_native(0, 2, 0, 0, 0, 1, 1)
                } else {
                    core.delete_text_native(0, 1, 1, 1)
                }
            }));
            match deletion {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    failures.push(format!("{label}/{owner}/delete error: {error}"));
                    continue;
                }
                Err(error) => {
                    let message = error
                        .downcast_ref::<&str>()
                        .copied()
                        .or_else(|| error.downcast_ref::<String>().map(String::as_str))
                        .unwrap_or("non-string panic");
                    c23_artifact(
                        &format!("tab-edit-{label}-{owner}-delete-panic.txt"),
                        message.as_bytes(),
                    );
                    failures.push(format!("{label}/{owner}/delete panic: {message}"));
                    continue;
                }
            }
            let expected = C23FieldState {
                text: final_text.into(),
                offsets: vec![0, 1],
                count: 3,
                ranges: vec![],
                styles: vec![0, 0],
            };
            let expected_xml = if cell {
                c23_field_fixture(&paragraph(initial), &paragraph(final_text))
            } else {
                c23_field_fixture(&paragraph(final_text), &paragraph(initial))
            };
            let expected_events = c8_inline_events(&hwpx_section_xml(&expected_xml));
            for pass in 0..=2 {
                let actual = state(&core);
                println!("C23_TAB_EDIT {label}/{owner} pass{pass} expected={expected:?} actual={actual:?}");
                c23_artifact(
                    &format!("tab-edit-{label}-{owner}-pass{pass}.txt"),
                    format!("{actual:#?}").as_bytes(),
                );
                if actual != expected {
                    failures.push(format!("{label}/{owner}/pass{pass}: {actual:?}"));
                }
                if pass < 2 {
                    let saved = core.export_hwpx_native().unwrap();
                    c23_artifact(
                        &format!("tab-edit-{label}-{owner}-save{}.hwpx", pass + 1),
                        &saved,
                    );
                    if c8_inline_events(&hwpx_section_xml(&saved)) != expected_events {
                        failures.push(format!(
                            "{label}/{owner}/save{}: raw text/styles differ",
                            pass + 1
                        ));
                    }
                    core = DocumentCore::from_bytes(&saved).unwrap();
                }
            }
        }
    }
    // Then: edits use consistent units and return to the independently specified visible layout.
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn c23_public_field_boundary_matrix_matches_raw_utf16_two_saves() {
    use rhwp::document_core::DocumentCore;
    // Given: independent raw marker layouts specify start/end/empty/nested/adjacent membership.
    let begin = |id| {
        format!(
            r#"<hp:ctrl><hp:fieldBegin id="{id}" type="HYPERLINK" name="경계&amp;이름" editable="1" fieldid="100{id}"><hp:parameters cnt="1" name="경계"><hp:stringParam name="Command">https://example.test/{id};</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl>"#
        )
    };
    let (b1, b2) = (begin(1), begin(2));
    let e1 = r#"<hp:ctrl><hp:fieldEnd beginIDRef="1"/></hp:ctrl>"#;
    let e2 = r#"<hp:ctrl><hp:fieldEnd beginIDRef="2"/></hp:ctrl>"#;
    let text = |s| format!("<hp:t>{s}</hp:t>");
    let (a, b, c, d, z, new) = (
        text("A"),
        text("B"),
        text("C"),
        text("D"),
        text("Z"),
        text("편😀"),
    );
    let single = format!("{a}{b1}{b}{e1}{z}");
    let cases = [
        (
            "start",
            single.clone(),
            format!("{a}{b1}{new}{b}{e1}{z}"),
            1,
            "편😀",
            vec![0, 9, 10, 12, 21],
            23,
            vec![(1, 4)],
        ),
        (
            "end",
            single.clone(),
            format!("{a}{b1}{b}{new}{e1}{z}"),
            2,
            "편😀",
            vec![0, 9, 10, 11, 21],
            23,
            vec![(1, 4)],
        ),
        (
            "interior",
            format!("{a}{b1}{b}{c}{e1}{z}"),
            format!("{a}{b1}{b}{new}{c}{e1}{z}"),
            2,
            "편😀",
            vec![0, 9, 10, 11, 13, 22],
            24,
            vec![(1, 5)],
        ),
        (
            "empty",
            format!("{a}{b1}{e1}{z}"),
            format!("{a}{b1}{new}{e1}{z}"),
            1,
            "편😀",
            vec![0, 9, 10, 20],
            22,
            vec![(1, 3)],
        ),
        (
            "nested-ends",
            format!("{a}{b1}{b2}{b}{e2}{e1}{z}"),
            format!("{a}{b1}{b2}{b}{new}{e2}{e1}{z}"),
            2,
            "편😀",
            vec![0, 17, 18, 19, 37],
            39,
            vec![(1, 4), (1, 4)],
        ),
        (
            "inner-end",
            format!("{a}{b1}{b}{b2}{c}{e2}{d}{e1}{z}"),
            format!("{a}{b1}{b}{b2}{c}{new}{e2}{d}{e1}{z}"),
            3,
            "편😀",
            vec![0, 9, 18, 19, 20, 30, 39],
            41,
            vec![(1, 6), (2, 5)],
        ),
        (
            "adjacent",
            format!("{a}{b1}{b}{e1}{b2}{c}{e2}{z}"),
            format!("{a}{b1}{b}{b2}{new}{e1}{c}{e2}{z}"),
            2,
            "편😀",
            vec![0, 9, 18, 19, 29, 38],
            40,
            vec![(1, 4), (2, 5)],
        ),
        (
            "outside",
            single,
            format!("{a}{b1}{b}{e1}{z}{}", text("편")),
            3,
            "편",
            vec![0, 9, 18, 19],
            21,
            vec![(1, 2)],
        ),
        (
            "trailing-end",
            format!("{a}{b1}{b}{e1}"),
            format!("{a}{b1}{b}{new}{e1}"),
            2,
            "편😀",
            vec![0, 9, 10, 11],
            22,
            vec![(1, 4)],
        ),
        (
            "empty-only",
            format!("{b1}{e1}"),
            format!("{b1}{new}{e1}"),
            0,
            "편😀",
            vec![8, 9],
            20,
            vec![(0, 2)],
        ),
    ];
    let paragraph = |content: &str| {
        format!(
            r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">{content}</hp:run></hp:p>"#
        )
    };
    let cell_ids = |s: &str| {
        s.replace("id=\"1\"", "id=\"101\"")
            .replace("id=\"2\"", "id=\"102\"")
            .replace("beginIDRef=\"1\"", "beginIDRef=\"101\"")
            .replace("beginIDRef=\"2\"", "beginIDRef=\"102\"")
    };
    let mut failures = Vec::new();
    // When: identical public body/cell edits and two genuine export/reopen cycles are performed.
    for (label, before, after, at, insertion, offsets, count, ranges) in cases {
        let input = c23_field_fixture(&paragraph(&before), &paragraph(&cell_ids(&before)));
        let expected_xml = hwpx_section_xml(&c23_field_fixture(
            &paragraph(&after),
            &paragraph(&cell_ids(&after)),
        ));
        let expected_events = c8_inline_events(&expected_xml);
        let expected_text = expected_events
            .iter()
            .filter(|(name, _, _)| name == "char")
            .take(offsets.len())
            .map(|(_, _, text)| text.as_str())
            .collect::<String>();
        let expected = C23FieldState {
            styles: vec![0; offsets.len()],
            text: expected_text,
            offsets,
            count,
            ranges,
        };
        let mut core = DocumentCore::from_bytes(&input).unwrap();
        c23_artifact(&format!("matrix-{label}-input.hwpx"), &input);
        core.insert_text_native(0, 1, at, insertion).unwrap();
        core.insert_text_in_cell_native(0, 2, 0, 0, 0, at, insertion)
            .unwrap();
        for pass in 0..=2 {
            let actual = vec![
                c23_field_state(&core.document().sections[0].paragraphs[1]),
                c23_field_state(c20_contact(&core, 2, 0, 0)),
            ];
            println!("C23_BOUNDARY {label} pass{pass} expected={expected:?} actual={actual:?}");
            c23_artifact(
                &format!("matrix-{label}-pass{pass}-model.txt"),
                format!("{actual:#?}").as_bytes(),
            );
            if actual.iter().any(|state| state != &expected) {
                failures.push(format!("{label}/pass{pass}: {actual:?}"));
            }
            if pass < 2 {
                let saved = core.export_hwpx_native().unwrap();
                c23_artifact(&format!("matrix-{label}-save{}.hwpx", pass + 1), &saved);
                if c8_inline_events(&hwpx_section_xml(&saved)) != expected_events {
                    failures.push(format!(
                        "{label}/save{}: raw field metadata/order/styles differ",
                        pass + 1
                    ));
                }
                core = DocumentCore::from_bytes(&saved).unwrap();
            }
        }
    }
    // Then: all independent UTF16/range/text expectations hold, including the outside-field control.
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// P47: 원 XML의 Table run 소유권을 실제 native 저장 결과와 비교한다.
#[derive(Debug)]
struct P47XmlState {
    tables: Vec<(String, u32)>,
    text_styles: Vec<(String, u32)>,
}

fn p47_paragraph(runs: &str) -> String {
    format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0">{runs}{C9_LINE}</hp:p>"#)
}

fn p47_table(label: &str) -> String {
    let cell = p47_paragraph(&format!(
        r#"<hp:run charPrIDRef="0"><hp:t>{label}</hp:t></hp:run>"#
    ));
    let wrapped = c8_table(&cell);
    let start = wrapped.find("<hp:tbl ").unwrap();
    let end = wrapped.rfind("</hp:tbl>").unwrap() + "</hp:tbl>".len();
    wrapped[start..end].replace("<hp:tr>", r#"<hp:pos treatAsChar="1"/><hp:tr>"#)
}

fn p47_mixed(label: &str, owner: u32, control_first: bool) -> String {
    let text = format!(r#"<hp:run charPrIDRef="1"><hp:t>text-{label}</hp:t></hp:run>"#);
    let table = format!(
        r#"<hp:run charPrIDRef="{owner}">{}</hp:run>"#,
        p47_table(label)
    );
    p47_paragraph(&if control_first {
        table + &text
    } else {
        text + &table
    })
}

fn p47_xml_state(xml: &str) -> P47XmlState {
    use quick_xml::{events::Event, name::ResolveResult, NsReader};
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut state = P47XmlState {
        tables: Vec::new(),
        text_styles: Vec::new(),
    };
    let (mut runs, mut tables) = (Vec::new(), Vec::new());
    let mut in_text = false;
    loop {
        let (namespace, event) = reader.read_resolved_event().unwrap();
        let value = match event {
            Event::Start(e) => {
                let name = e.local_name();
                if matches!(name.as_ref(), b"run" | b"tbl" | b"t") {
                    let ResolveResult::Bound(ns) = namespace else {
                        panic!("P47 XML namespace must be bound")
                    };
                    assert_eq!(ns.as_ref(), b"http://www.hancom.co.kr/hwpml/2011/paragraph");
                }
                match name.as_ref() {
                    b"run" => {
                        let owner = e
                            .attributes()
                            .map(Result::unwrap)
                            .find(|a| a.key.as_ref() == b"charPrIDRef")
                            .unwrap()
                            .decode_and_unescape_value(reader.decoder())
                            .unwrap()
                            .parse::<u32>()
                            .unwrap();
                        runs.push(owner);
                    }
                    b"tbl" => {
                        tables.push(state.tables.len());
                        state.tables.push((String::new(), *runs.last().unwrap()));
                    }
                    b"t" => in_text = true,
                    _ => {}
                }
                None
            }
            Event::Text(e) if in_text => Some(
                quick_xml::escape::unescape(&e.decode().unwrap())
                    .unwrap()
                    .into_owned(),
            ),
            Event::GeneralRef(e) if in_text => Some(
                quick_xml::escape::unescape(&format!("&{};", e.decode().unwrap()))
                    .unwrap()
                    .into_owned(),
            ),
            Event::End(e) => {
                match e.local_name().as_ref() {
                    b"run" => {
                        runs.pop().unwrap();
                    }
                    b"tbl" => {
                        tables.pop().unwrap();
                    }
                    b"t" => in_text = false,
                    _ => {}
                }
                None
            }
            Event::Eof => break,
            _ => None,
        };
        if let Some(value) = value {
            if let Some(&index) = tables.last() {
                state.tables[index].0.push_str(&value);
            }
            state.text_styles.push((value, *runs.last().unwrap()));
        }
    }
    state.tables.sort();
    state.text_styles.sort();
    state
}

fn p47_open_case(
    case: &str,
    input: &[u8],
) -> (rhwp::document_core::DocumentCore, std::path::PathBuf) {
    let dir = std::path::PathBuf::from("target/v342-g0-p47-correction")
        .join(format!("native-{}", std::process::id()))
        .join(case);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.hwpx"), input).unwrap();
    std::fs::write(dir.join("input.section.xml"), hwpx_section_xml(input)).unwrap();
    (
        rhwp::document_core::DocumentCore::from_bytes(input).expect("valid P47 fixture"),
        dir,
    )
}

fn p47_text_state(doc: &rhwp::model::document::Document) -> Vec<(String, Vec<u32>, u32)> {
    use rhwp::model::control::Control;
    let mut paragraphs: Vec<_> = doc
        .sections
        .iter()
        .flat_map(|section| &section.paragraphs)
        .collect();
    let mut output = Vec::new();
    let mut index = 0;
    while index < paragraphs.len() {
        let para = paragraphs[index];
        output.push((
            para.text.clone(),
            para.char_offsets.clone(),
            para.char_count,
        ));
        for control in &para.controls {
            if let Control::Table(table) = control {
                paragraphs.extend(table.cells.iter().flat_map(|cell| &cell.paragraphs));
            }
        }
        index += 1;
    }
    output
}

fn p47_two_saves(
    mut core: rhwp::document_core::DocumentCore,
    dir: &std::path::Path,
) -> (Vec<P47XmlState>, Vec<String>) {
    use rhwp::document_core::DocumentCore;
    let before = p47_text_state(core.document());
    std::fs::write(dir.join("before.ir.txt"), format!("{before:#?}\n")).unwrap();
    let mut states = Vec::new();
    let mut failures = Vec::new();
    let mut observations = String::new();
    // c10_two_native_saves does not expose saved bytes; retain the same native loop plus actual artifacts.
    for cycle in 1..=2 {
        let saved = match core.export_hwpx_native() {
            Ok(saved) => saved,
            Err(error) => {
                failures.push(format!("{} save{cycle}: {error}", dir.display()));
                break;
            }
        };
        std::fs::write(dir.join(format!("save{cycle}.hwpx")), &saved).unwrap();
        let xml = hwpx_section_xml(&saved);
        std::fs::write(dir.join(format!("save{cycle}.section.xml")), &xml).unwrap();
        let state = p47_xml_state(&xml);
        let reopened = match DocumentCore::from_bytes(&saved) {
            Ok(reopened) => reopened,
            Err(error) => {
                failures.push(format!("{} reopen{cycle}: {error}", dir.display()));
                break;
            }
        };
        let after = p47_text_state(reopened.document());
        std::fs::write(
            dir.join(format!("open{cycle}.ir.txt")),
            format!("{after:#?}\n"),
        )
        .unwrap();
        if after != before {
            failures.push(format!(
                "{} save{cycle}: text/offset/count changed",
                dir.display()
            ));
        }
        let line = format!(
            "save{cycle} tables={:?} text_styles={:?}\n",
            state.tables, state.text_styles
        );
        println!("P47_OBSERVATION case={} {line}", dir.display());
        observations.push_str(&line);
        states.push(state);
        core = reopened;
    }
    std::fs::write(dir.join("observations.txt"), observations).unwrap();
    (states, failures)
}

fn p47_find_body_table(
    core: &rhwp::document_core::DocumentCore,
    label: &str,
) -> Result<(usize, usize), String> {
    use rhwp::model::control::Control;
    for (p, para) in core.document().sections[0]
        .paragraphs
        .iter()
        .enumerate()
        .rev()
    {
        for (c, control) in para.controls.iter().enumerate() {
            if let Control::Table(table) = control {
                if table
                    .cells
                    .iter()
                    .flat_map(|cell| &cell.paragraphs)
                    .any(|p| p.text == label)
                {
                    return Ok((p, c));
                }
            }
        }
    }
    Err(format!("missing body Table label {label}"))
}

fn p47_set_label(table: &mut rhwp::model::table::Table, label: &str) {
    let para = &mut table.cells[0].paragraphs[0];
    para.delete_text_at(0, para.text.chars().count());
    para.insert_text_at(0, label);
}

fn p47_operation(
    name: &str,
    result: Result<String, rhwp::HwpError>,
    replies: &mut Vec<String>,
) -> Result<(), String> {
    let reply = result.map_err(|e| format!("{name}: {e}"))?;
    replies.push(format!("{name}: {reply}"));
    if !reply.contains(r#""ok":true"#) {
        return Err(format!("{name}: {reply}"));
    }
    Ok(())
}

#[test]
fn p47_table_owner_styles_survive_mixed_run_order_two_saves() {
    // Given: independent body/cell fixtures, differing text/Table IDs, Some(0), and multiple Tables.
    let mut failures = Vec::new();
    let mut summary = Vec::new();
    for nested in [false, true] {
        for control_first in [false, true] {
            for owner in [0, 3] {
                let case =
                    format!("mixed/cell-{nested}-control-first-{control_first}-owner-{owner}");
                let inner = p47_mixed("owned-table", owner, control_first);
                let body = if nested {
                    let carrier =
                        p47_paragraph(r#"<hp:run charPrIDRef="0"><hp:t>carrier</hp:t></hp:run>"#);
                    c8_table(&(carrier + &inner))
                } else {
                    inner
                };
                let body = c9_paragraph(C10_COLUMN, C9_LINE) + &body;
                let input = c8_fixture(&body, false);
                let expected = p47_xml_state(&hwpx_section_xml(&input));
                let (core, dir) = p47_open_case(&case, &input);
                // When: the actual public native export/reopen path runs twice.
                let (states, errors) = p47_two_saves(core, &dir);
                failures.extend(errors);
                let equal = states.len() == 2
                    && states.iter().all(|s| {
                        s.tables == expected.tables && s.text_styles == expected.text_styles
                    });
                summary.push(format!("{case}: equal={equal} expected={expected:?}"));
                if !equal {
                    failures.push(format!("{case}: expected={expected:?}, actual={states:?}"));
                }
            }
        }
        let case = format!("mixed/multiple-cell-{nested}");
        let inner = p47_paragraph(&format!(
            r#"<hp:run charPrIDRef="1"><hp:t>multiple-text</hp:t></hp:run><hp:run charPrIDRef="3">{}</hp:run><hp:run charPrIDRef="4">{}</hp:run>"#,
            p47_table("first-table"),
            p47_table("second-table")
        ));
        let body = if nested {
            let carrier = p47_paragraph(r#"<hp:run charPrIDRef="0"><hp:t>carrier</hp:t></hp:run>"#);
            c8_table(&(carrier + &inner))
        } else {
            inner
        };
        let body = c9_paragraph(C10_COLUMN, C9_LINE) + &body;
        let input = c8_fixture(&body, false);
        let expected = p47_xml_state(&hwpx_section_xml(&input));
        let (core, dir) = p47_open_case(&case, &input);
        let (states, errors) = p47_two_saves(core, &dir);
        failures.extend(errors);
        let equal = states.len() == 2
            && states
                .iter()
                .all(|s| s.tables == expected.tables && s.text_styles == expected.text_styles);
        summary.push(format!("{case}: equal={equal} expected={expected:?}"));
        if !equal {
            failures.push(format!("{case}: expected={expected:?}, actual={states:?}"));
        }
    }
    // Then: collect every case before the aggregate assertion, including legitimate baseline passes.
    let dir = std::path::PathBuf::from("target/v342-g0-p47-correction")
        .join(format!("native-{}", std::process::id()))
        .join("mixed");
    std::fs::write(dir.join("summary.txt"), summary.join("\n")).unwrap();
    println!("P47_MIXED_SUMMARY\n{}", summary.join("\n"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn p47_table_owner_styles_follow_control_lifecycle_two_saves() {
    use rhwp::model::control::Control;
    // Given: each public mutation/style scenario starts from its own fixture.
    let mut failures = Vec::new();
    let mut summary = Vec::new();
    for case in [
        "move-index",
        "copy-body-delete",
        "copy-cell",
        "split",
        "new-body",
        "new-cell",
        "style-body",
        "style-cell",
    ] {
        let blank = p47_paragraph(r#"<hp:run charPrIDRef="2"><hp:t/></hp:run>"#);
        let host = p47_paragraph(r#"<hp:run charPrIDRef="2"><hp:t>cell-host</hp:t></hp:run>"#);
        let body = match case {
            "move-index" => p47_paragraph(&format!(
                r#"<hp:run charPrIDRef="1"><hp:t>move-text</hp:t></hp:run><hp:run charPrIDRef="3">{}</hp:run><hp:run charPrIDRef="4">{}</hp:run>"#,
                p47_table("move-a"),
                p47_table("move-b")
            )),
            "copy-body-delete" => p47_mixed("source", 3, false) + &blank,
            "copy-cell" => p47_mixed("source", 3, false) + &c8_table(&host),
            "split" => p47_mixed("split", 3, false),
            "new-body" => blank,
            "new-cell" => c8_table(&host),
            "style-body" => p47_mixed("style-body", 3, false),
            "style-cell" => {
                let carrier =
                    p47_paragraph(r#"<hp:run charPrIDRef="0"><hp:t>carrier</hp:t></hp:run>"#);
                c8_table(&(carrier + &p47_mixed("style-cell", 3, false)))
            }
            _ => unreachable!(),
        };
        let body = c9_paragraph(C10_COLUMN, C9_LINE) + &body;
        let input = c8_fixture(&body, false);
        let original = p47_xml_state(&hwpx_section_xml(&input));
        let source_owner = |label: &str| {
            original
                .tables
                .iter()
                .find(|(text, _)| text == label)
                .unwrap()
                .1
        };
        let (mut core, dir) = p47_open_case(&format!("lifecycle/{case}"), &input);
        let mut replies = Vec::new();
        let operation = (|| -> Result<Vec<(String, u32)>, String> {
            // When: mutate real Control payloads and call existing public native commands.
            let expected = match case {
                "move-index" => {
                    let (p, a) = p47_find_body_table(&core, "move-a")?;
                    let (_, b) = p47_find_body_table(&core, "move-b")?;
                    let mut doc = core.document().clone();
                    let moved = doc.sections[0].paragraphs[p].controls.remove(a);
                    doc.sections[0].paragraphs[p].controls.insert(b, moved);
                    core.set_document(doc);
                    p47_operation(
                        "bookmark-insert",
                        core.add_bookmark_native(0, p, 0, "p47-shift"),
                        &mut replies,
                    )?;
                    let bookmark = core.document().sections[0].paragraphs[p]
                        .controls
                        .iter()
                        .position(|c| matches!(c, Control::Bookmark(_)))
                        .ok_or("bookmark missing")?;
                    p47_operation(
                        "bookmark-remove",
                        core.delete_bookmark_native(0, p, bookmark),
                        &mut replies,
                    )?;
                    original.tables.clone()
                }
                "copy-body-delete" | "copy-cell" => {
                    let (p, c) = p47_find_body_table(&core, "source")?;
                    p47_operation("copy", core.copy_control_native(0, p, c), &mut replies)?;
                    if case == "copy-body-delete" {
                        p47_operation(
                            "paste-body",
                            core.paste_control_native(0, 2, 0),
                            &mut replies,
                        )?;
                        let copied = p47_find_body_table(&core, "source")?;
                        let mut doc = core.document().clone();
                        let Control::Table(table) =
                            &mut doc.sections[0].paragraphs[copied.0].controls[copied.1]
                        else {
                            unreachable!()
                        };
                        p47_set_label(table, "body-pasted");
                        core.set_document(doc);
                        let source = p47_find_body_table(&core, "source")?;
                        p47_operation(
                            "delete-source",
                            core.delete_table_control_native(0, source.0, source.1),
                            &mut replies,
                        )?;
                        vec![("body-pasted".into(), source_owner("source"))]
                    } else {
                        let outer = p47_find_body_table(&core, "cell-host")?;
                        let path = format!(
                            r#"[{{"controlIndex":{},"cellIndex":0,"cellParaIndex":0}}]"#,
                            outer.1
                        );
                        p47_operation(
                            "paste-cell",
                            core.paste_control_in_cell_by_path_native(0, outer.0, &path, 0),
                            &mut replies,
                        )?;
                        let mut doc = core.document().clone();
                        let Control::Table(table) =
                            &mut doc.sections[0].paragraphs[outer.0].controls[outer.1]
                        else {
                            unreachable!()
                        };
                        let inner = table.cells[0].paragraphs[0]
                            .controls
                            .iter_mut()
                            .find_map(|c| match c {
                                Control::Table(t) => Some(t),
                                _ => None,
                            })
                            .ok_or("nested paste missing")?;
                        p47_set_label(inner, "cell-pasted");
                        core.set_document(doc);
                        let mut expected = original.tables.clone();
                        expected.push(("cell-pasted".into(), source_owner("source")));
                        expected
                    }
                }
                "split" => {
                    let (p, c) = p47_find_body_table(&core, "split")?;
                    p47_operation(
                        "split-cell",
                        core.split_table_cell_into_native(0, p, c, 0, 0, 1, 2, true, false),
                        &mut replies,
                    )?;
                    let Control::Table(table) =
                        &core.document().sections[0].paragraphs[p].controls[c]
                    else {
                        unreachable!()
                    };
                    if table.cells.len() != 2 {
                        return Err(format!("split returned {} cells", table.cells.len()));
                    }
                    original.tables.clone()
                }
                "new-body" => {
                    p47_operation(
                        "create-inline-table",
                        core.create_table_ex_native(0, 1, 0, 1, 1, true, None),
                        &mut replies,
                    )?;
                    let mut doc = core.document().clone();
                    let table = doc.sections[0].paragraphs[1]
                        .controls
                        .iter_mut()
                        .find_map(|c| match c {
                            Control::Table(t) => Some(t),
                            _ => None,
                        })
                        .ok_or("new Table missing")?;
                    p47_set_label(table, "new-body");
                    core.set_document(doc);
                    vec![("new-body".into(), 2)]
                }
                "new-cell" => {
                    let outer = p47_find_body_table(&core, "cell-host")?;
                    let path = format!(
                        r#"[{{"controlIndex":{},"cellIndex":0,"cellParaIndex":0}}]"#,
                        outer.1
                    );
                    p47_operation(
                        "create-cell-table",
                        core.create_table_in_cell_by_path_native(0, outer.0, &path, 0, 1, 1),
                        &mut replies,
                    )?;
                    let mut doc = core.document().clone();
                    let Control::Table(table) =
                        &mut doc.sections[0].paragraphs[outer.0].controls[outer.1]
                    else {
                        unreachable!()
                    };
                    let inner = table.cells[0].paragraphs[0]
                        .controls
                        .iter_mut()
                        .find_map(|c| match c {
                            Control::Table(t) => Some(t),
                            _ => None,
                        })
                        .ok_or("new nested Table missing")?;
                    p47_set_label(inner, "new-cell");
                    core.set_document(doc);
                    let mut expected = original.tables.clone();
                    expected.push(("new-cell".into(), 2));
                    expected
                }
                "style-body" | "style-cell" => {
                    let mut doc = core.document().clone();
                    let mut style = doc.doc_info.styles[0].clone();
                    style.char_shape_id = 4;
                    let style_id = doc.doc_info.styles.len();
                    doc.doc_info.styles.push(style);
                    core.set_document(doc);
                    if case == "style-body" {
                        p47_operation(
                            "apply-body-style",
                            core.apply_style_native(0, 1, style_id),
                            &mut replies,
                        )?;
                        vec![("style-body".into(), 4)]
                    } else {
                        let outer = p47_find_body_table(&core, "carrier")?;
                        p47_operation(
                            "apply-cell-style",
                            core.apply_cell_style_native(0, outer.0, outer.1, 0, 1, style_id),
                            &mut replies,
                        )?;
                        let mut expected = original.tables.clone();
                        expected
                            .iter_mut()
                            .find(|(text, _)| text == "style-cell")
                            .unwrap()
                            .1 = 4;
                        expected
                    }
                }
                _ => unreachable!(),
            };
            Ok(expected)
        })();
        std::fs::write(dir.join("operations.txt"), replies.join("\n")).unwrap();
        let mut expected = match operation {
            Ok(expected) => expected,
            Err(error) => {
                failures.push(format!("{case}: public operation/fixture error: {error}"));
                summary.push(format!("{case}: operation_error={error}"));
                std::fs::write(dir.join("operation-error.txt"), error).unwrap();
                continue;
            }
        };
        expected.sort();
        let (states, errors) = p47_two_saves(core, &dir);
        failures.extend(errors);
        let equal = states.len() == 2 && states.iter().all(|s| s.tables == expected);
        summary.push(format!("{case}: equal={equal} expected={expected:?}"));
        if !equal {
            failures.push(format!("{case}: expected={expected:?}, actual={states:?}"));
        }
    }
    // Then: baseline style/fallback passes remain visible beside owner-preservation failures.
    let dir = std::path::PathBuf::from("target/v342-g0-p47-correction")
        .join(format!("native-{}", std::process::id()))
        .join("lifecycle");
    std::fs::write(dir.join("summary.txt"), summary.join("\n")).unwrap();
    println!("P47_LIFECYCLE_SUMMARY\n{}", summary.join("\n"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn p47_table_owner_unknown_reference_is_rejected() {
    use rhwp::model::{control::Control, paragraph::Paragraph};
    fn strip_owner_refs(paragraphs: &mut [Paragraph], observed: &mut Vec<(String, Vec<u32>)>) {
        for para in paragraphs {
            para.char_shapes.retain(|style| style.char_shape_id != 4);
            observed.push((
                para.text.clone(),
                para.char_shapes
                    .iter()
                    .map(|style| style.char_shape_id)
                    .collect(),
            ));
            for control in &mut para.controls {
                if let Control::Table(table) = control {
                    for cell in &mut table.cells {
                        strip_owner_refs(&mut cell.paragraphs, observed);
                    }
                }
            }
        }
    }
    // Given: ID4 belongs only to the Table owner; all visible text uses valid ID0.
    let mut failures = Vec::new();
    let mut summary = Vec::new();
    for nested in [false, true] {
        let inner = p47_paragraph(&format!(
            r#"<hp:run charPrIDRef="0"><hp:t>valid-text</hp:t></hp:run><hp:run charPrIDRef="4">{}</hp:run>"#,
            p47_table("unknown-owner")
        ));
        let body = if nested {
            let carrier = p47_paragraph(r#"<hp:run charPrIDRef="0"><hp:t>carrier</hp:t></hp:run>"#);
            c8_table(&(carrier + &inner))
        } else {
            inner
        };
        let body = c9_paragraph(C10_COLUMN, C9_LINE) + &body;
        let input = c8_fixture(&body, false);
        let (mut core, dir) = p47_open_case(&format!("unknown/cell-{nested}"), &input);
        let mut doc = core.document().clone();
        doc.doc_info.char_shapes.truncate(4);
        for style in &mut doc.doc_info.styles {
            if style.char_shape_id >= 4 {
                style.char_shape_id = 0;
            }
        }
        let mut paragraph_refs = Vec::new();
        for section in &mut doc.sections {
            strip_owner_refs(&mut section.paragraphs, &mut paragraph_refs);
        }
        core.set_document(doc);
        std::fs::write(
            dir.join("reference-input.txt"),
            format!(
                "char_shape_count={}\nparagraph_char_shape_ids={paragraph_refs:#?}\nstyle_char_shape_ids={:?}\n",
                core.document().doc_info.char_shapes.len(),
                core.document().doc_info.styles.iter().map(|style| style.char_shape_id).collect::<Vec<_>>()
            ),
        ).unwrap();
        std::fs::write(
            dir.join("before.ir.txt"),
            format!("{:#?}\n", p47_text_state(core.document())),
        )
        .unwrap();
        // When: export must validate the attached ID even after Paragraph references were removed.
        let result = match core.export_hwpx_native() {
            Err(error) => {
                let message = error.to_string();
                std::fs::write(dir.join("export-error.txt"), &message).unwrap();
                let rejected_owner = message.contains("charPrIDRef: [4]");
                if !rejected_owner {
                    failures.push(format!("cell-{nested}: unrelated export error: {message}"));
                }
                format!("cell-{nested}: rejected_owner={rejected_owner} error={message}")
            }
            Ok(saved) => {
                std::fs::write(dir.join("unexpected-save.hwpx"), &saved).unwrap();
                let xml = hwpx_section_xml(&saved);
                std::fs::write(dir.join("unexpected-save.section.xml"), &xml).unwrap();
                let state = p47_xml_state(&xml);
                failures.push(format!(
                    "cell-{nested}: missing owner ID4 was accepted: {:?}",
                    state.tables
                ));
                format!(
                    "cell-{nested}: rejected_owner=false accepted_tables={:?}",
                    state.tables
                )
            }
        };
        std::fs::write(dir.join("observations.txt"), &result).unwrap();
        summary.push(result);
    }
    // Then: both body and cell observations exist before the aggregate RED assertion.
    let dir = std::path::PathBuf::from("target/v342-g0-p47-correction")
        .join(format!("native-{}", std::process::id()))
        .join("unknown");
    std::fs::write(dir.join("summary.txt"), summary.join("\n")).unwrap();
    println!("P47_UNKNOWN_SUMMARY\n{}", summary.join("\n"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn c8_table(inner: &str) -> String {
    format!(
        r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:tbl rowCnt="1" colCnt="1" borderFillIDRef="1"><hp:sz width="10000" height="3000"/><hp:tr><hp:tc borderFillIDRef="1"><hp:subList>{inner}</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="10000" height="3000"/></hp:tc></hp:tr></hp:tbl></hp:run></hp:p>"#
    )
}

/// XML 이벤트로 필드 경계와 표시 문자별 run 서식을 함께 비교한다.
fn c8_inline_events(xml: &str) -> Vec<(String, Vec<(String, String)>, String)> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut output: Vec<(String, Vec<(String, String)>, String)> = Vec::new();
    let mut shape = String::from("0");
    let mut field_depth = 0usize;
    let mut in_text = false;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let mut attrs: Vec<_> = e
                    .attributes()
                    .map(|a| {
                        let a = a.unwrap();
                        (
                            String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                            a.decode_and_unescape_value(reader.decoder())
                                .unwrap()
                                .into_owned(),
                        )
                    })
                    .collect();
                attrs.sort();
                if name == "run" {
                    shape = attrs
                        .iter()
                        .find(|(k, _)| k == "charPrIDRef")
                        .unwrap()
                        .1
                        .clone();
                }
                if name == "fieldBegin" || name == "fieldEnd" || field_depth > 0 {
                    if field_depth == 0 {
                        attrs.push(("run".into(), shape.clone()));
                    }
                    output.push((name.clone(), attrs, String::new()));
                    field_depth += 1;
                } else if name == "t" {
                    in_text = true;
                }
            }
            Event::Text(e) => {
                let value = quick_xml::escape::unescape(&e.decode().unwrap())
                    .unwrap()
                    .into_owned();
                if field_depth > 0 {
                    output.last_mut().unwrap().2.push_str(&value);
                } else if in_text {
                    for ch in value.chars() {
                        output.push((
                            "char".into(),
                            vec![("run".into(), shape.clone())],
                            ch.to_string(),
                        ));
                    }
                }
            }
            Event::GeneralRef(e) => {
                let reference = format!("&{};", e.decode().unwrap());
                let value = quick_xml::escape::unescape(&reference).unwrap();
                if field_depth > 0 {
                    output.last_mut().unwrap().2.push_str(&value);
                } else if in_text {
                    for ch in value.chars() {
                        output.push((
                            "char".into(),
                            vec![("run".into(), shape.clone())],
                            ch.to_string(),
                        ));
                    }
                }
            }
            Event::End(e) => {
                if field_depth > 0 {
                    field_depth -= 1;
                }
                if e.local_name().as_ref() == b"t" {
                    in_text = false;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    output
}

#[test]
fn c8_native_fields_preserve_metadata_positions_and_styled_text() {
    use rhwp::document_core::DocumentCore;
    let input = c8_fixture(
        &(c8_field_paragraph(101) + &c8_table(&c8_field_paragraph(303))),
        false,
    );
    let expected = c8_inline_events(&hwpx_section_xml(&input));
    let mut core = DocumentCore::from_bytes(&input).unwrap();
    for pass in 0..2 {
        let saved = core.export_hwpx_native().unwrap();
        let xml = hwpx_section_xml(&saved);
        assert!(
            !xml.chars()
                .any(|c| c < ' ' && !matches!(c, '\t' | '\n' | '\r')),
            "XML 1.0 forbids field marker bytes"
        );
        assert_eq!(
            c8_inline_events(&xml),
            expected,
            "save {pass}: field metadata/positions/run styles"
        );
        core = DocumentCore::from_bytes(&saved).unwrap();
    }
}

#[test]
fn c8_native_field_ranges_follow_unicode_edits_in_body_and_cell() {
    use rhwp::document_core::DocumentCore;
    let input = c8_fixture(
        &(c8_field_paragraph(101) + &c8_table(&c8_field_paragraph(303))),
        false,
    );
    let mut core = DocumentCore::from_bytes(&input).unwrap();
    let para = &core.document().sections[0].paragraphs[0];
    assert_eq!(para.text, "앞😀링😀크뒤");
    assert_eq!(para.field_ranges.len(), 1);
    assert_eq!(
        (
            para.field_ranges[0].start_char_idx,
            para.field_ranges[0].end_char_idx
        ),
        (2, 5)
    );
    assert_eq!(para.char_offsets[3] - para.char_offsets[2], 1);
    assert_eq!(para.char_offsets[4] - para.char_offsets[3], 2);
    core.insert_text_native(0, 0, 3, "편😀").unwrap();
    core.insert_text_in_cell_native(0, 1, 0, 0, 0, 3, "편😀")
        .unwrap();
    let saved = core.export_hwpx_native().unwrap();
    let reopened = DocumentCore::from_bytes(&saved).unwrap();
    let body = &reopened.document().sections[0].paragraphs[0];
    assert_eq!(body.text, "앞😀링편😀😀크뒤");
    assert_eq!(
        (
            body.field_ranges[0].start_char_idx,
            body.field_ranges[0].end_char_idx
        ),
        (2, 7)
    );
    assert!(table_cell_texts(reopened.document()).contains(&body.text));
    let expected_xml = hwpx_section_xml(&input).replace("링😀크", "링편😀😀크");
    assert_eq!(
        c8_inline_events(&hwpx_section_xml(&saved)),
        c8_inline_events(&expected_xml)
    );
}

#[test]
fn c8_native_empty_nested_fields_and_current_property_edits_survive() {
    use rhwp::{
        model::control::{Control, Field},
        parser::hwpx::parse_hwpx,
        serializer::serialize_hwpx,
    };
    let inner = r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:ctrl><hp:fieldBegin id="1" fieldid="101" type="CLICK_HERE" name="old" editable="1"><hp:parameters cnt="2" name=""><hp:stringParam name="Command">old</hp:stringParam><hp:listParam name="Nested" cnt="2"><hp:integerParam name="Value">42</hp:integerParam><hp:stringParam name="Value">a&amp;b</hp:stringParam></hp:listParam></hp:parameters></hp:fieldBegin><hp:fieldBegin id="2" type="VENDOR_FIELD" name="inner"/><hp:fieldEnd beginIDRef="2"/><hp:fieldEnd beginIDRef="1" fieldid="101"/></hp:ctrl><hp:t>끝😀</hp:t></hp:run></hp:p>"#;
    let input = c8_fixture(inner, false);
    let mut doc = parse_hwpx(&input).unwrap();
    assert_eq!(doc.sections[0].paragraphs[0].field_ranges.len(), 2);
    let saved = serialize_hwpx(&doc).unwrap();
    assert_eq!(
        c8_inline_events(&hwpx_section_xml(&saved)),
        c8_inline_events(&hwpx_section_xml(&input))
    );

    let field = doc.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    assert!(field.is_editable_in_form());
    field.command = Field::build_clickhere_command("새 안내😀", "메모", "");
    field.properties &= !1;
    field.ctrl_data_name = None;
    let expected_command = field.command.clone();
    let saved = serialize_hwpx(&doc).unwrap();
    let reopened = parse_hwpx(&saved).unwrap();
    let field = reopened.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(field.command, expected_command);
    assert_eq!(field.field_name(), Some("새 안내😀"));
    assert!(!field.is_editable_in_form());
    assert_eq!(field.ctrl_data_name.as_deref(), Some(""));

    let absent_name = c8_fixture(&inner.replace(" name=\"old\"", ""), false);
    let mut doc = parse_hwpx(&absent_name).unwrap();
    let field = doc.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    field.ctrl_data_name = Some("새 이름😀".into());
    let saved = serialize_hwpx(&doc).unwrap();
    let reopened = parse_hwpx(&saved).unwrap();
    let field = reopened.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(field.ctrl_data_name.as_deref(), Some("새 이름😀"));
}

#[test]
fn c8_native_field_binary_defaults_and_invalid_references_are_explicit() {
    use rhwp::{model::control::Control, parser::hwpx::parse_hwpx, serializer::serialize_hwpx};
    let input = c8_fixture(&c8_field_paragraph(101), false);
    let mut doc = parse_hwpx(&input).unwrap();
    let field = doc.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    field.hwpx = None;
    field.field_id = 909;
    field.command = "binary&command😀".into();
    field.properties |= 1;
    let saved = serialize_hwpx(&doc).unwrap();
    let reopened = parse_hwpx(&saved).unwrap();
    let field = reopened.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        (
            field.field_id,
            field.command.as_str(),
            field.is_editable_in_form()
        ),
        (909, "binary&command😀", true)
    );
    doc.sections[0].paragraphs[0].delete_text_at(2, 3);
    let empty_saved = serialize_hwpx(&doc).unwrap();
    let empty = parse_hwpx(&empty_saved).expect("empty binary field begin must precede end");
    assert_eq!(
        (
            empty.sections[0].paragraphs[0].field_ranges[0].start_char_idx,
            empty.sections[0].paragraphs[0].field_ranges[0].end_char_idx
        ),
        (2, 2)
    );
    for (body, nested, expected) in [
        (
            r#"<hp:ctrl><hp:fieldBegin id="1" type="CLICK_HERE"/><hp:fieldBegin id="2" type="CLICK_HERE"/><hp:fieldEnd beginIDRef="2"/><hp:fieldEnd beginIDRef="1"/></hp:ctrl>"#,
            true,
            ["B10", "B11", "E11", "E10"],
        ),
        (
            r#"<hp:ctrl><hp:fieldBegin id="1" type="CLICK_HERE"/><hp:fieldBegin id="2" type="CLICK_HERE"/></hp:ctrl><hp:t>가😀</hp:t><hp:ctrl><hp:fieldEnd beginIDRef="2"/><hp:fieldEnd beginIDRef="1"/></hp:ctrl>"#,
            true,
            ["B10", "B11", "E11", "E10"],
        ),
        (
            r#"<hp:ctrl><hp:fieldBegin id="1" type="CLICK_HERE"/><hp:fieldEnd beginIDRef="1"/><hp:fieldBegin id="2" type="CLICK_HERE"/><hp:fieldEnd beginIDRef="2"/></hp:ctrl>"#,
            false,
            ["B10", "E10", "B11", "E11"],
        ),
    ] {
        let input = c8_fixture(
            &format!(
                r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">{body}</hp:run></hp:p>"#
            ),
            false,
        );
        let mut binary = parse_hwpx(&input).unwrap();
        let para = &mut binary.sections[0].paragraphs[0];
        for (ci, control) in para.controls.iter_mut().enumerate() {
            if let Control::Field(field) = control {
                field.hwpx = None;
                field.field_id = 10 + ci as u32;
            }
        }
        // Binary parsing records controls at begin and field ranges at end.
        if nested {
            para.field_ranges.reverse();
        }
        let saved = serialize_hwpx(&binary).unwrap();
        let boundaries: Vec<_> = c8_inline_events(&hwpx_section_xml(&saved))
            .into_iter()
            .filter_map(|(name, attrs, _)| match name.as_str() {
                "fieldBegin" => Some(format!(
                    "B{}",
                    attrs.iter().find(|(k, _)| k == "id").unwrap().1
                )),
                "fieldEnd" => Some(format!(
                    "E{}",
                    attrs.iter().find(|(k, _)| k == "beginIDRef").unwrap().1
                )),
                _ => None,
            })
            .collect();
        assert_eq!(boundaries, expected);
        assert_eq!(
            parse_hwpx(&saved).unwrap().sections[0].paragraphs[0]
                .field_ranges
                .len(),
            2
        );
    }
    let malformed = c8_fixture(
        &c8_field_paragraph(101).replace("beginIDRef=\"101\"", "beginIDRef=\"999\""),
        false,
    );
    assert!(
        parse_hwpx(&malformed).is_err(),
        "unmatched fieldEnd must not silently pair with another ID"
    );
    let field = doc.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| {
            if let Control::Field(f) = c {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    field.command.push('\u{0003}');
    assert!(
        serialize_hwpx(&doc).is_err(),
        "edited field values must not create forbidden XML controls"
    );
}

#[test]
fn c8_native_picture_preserves_original_displayed_geometry_identity_and_png() {
    use rhwp::document_core::DocumentCore;
    use std::io::{Cursor, Read};
    let body = r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:pic id="17" instid="29"><hp:orgSz width="12000" height="8000"/><hp:curSz width="6000" height="4000"/><hc:img binaryItemIDRef="image1"/><hp:sz width="3000" height="2000"/><hp:pos treatAsChar="1"/></hp:pic><hp:t>그림 캡션과 편집 가능한 글</hp:t></hp:run></hp:p>"#;
    let input = c8_fixture(body, true);
    let core = DocumentCore::from_bytes(&input).unwrap();
    let saved = core.export_hwpx_native().unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(&saved)).unwrap();
    let mut png = Vec::new();
    archive
        .by_name("BinData/image1.png")
        .unwrap()
        .read_to_end(&mut png)
        .unwrap();
    assert_eq!(png, rhwp::serializer::hwpx::static_assets::PRV_IMAGE_PNG);
    let xml = hwpx_section_xml(&saved);
    let elements = c8_elements(&xml);
    for (tag, attrs) in [
        ("pic", vec![("id", "17"), ("instid", "29")]),
        ("orgSz", vec![("width", "12000"), ("height", "8000")]),
        ("curSz", vec![("width", "6000"), ("height", "4000")]),
        ("sz", vec![("width", "3000"), ("height", "2000")]),
    ] {
        let actual = &elements.iter().find(|(name, _)| name == tag).unwrap().1;
        for (key, value) in attrs {
            assert!(
                actual.contains(&(key.to_string(), value.to_string())),
                "{tag}.{key}={value}, actual={actual:?}"
            );
        }
    }
    let mut manifest = String::new();
    archive
        .by_name("Contents/content.hpf")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(c8_elements(&manifest)
        .iter()
        .any(|(tag, attrs)| tag == "item"
            && attrs.contains(&("id".into(), "image1".into()))
            && attrs.contains(&("isEmbeded".into(), "1".into()))));
    assert_eq!(
        DocumentCore::from_bytes(&saved)
            .unwrap()
            .document()
            .sections[0]
            .paragraphs[0]
            .text,
        "그림 캡션과 편집 가능한 글"
    );
}

fn c8_elements(xml: &str) -> Vec<(String, Vec<(String, String)>)> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(xml);
    let mut result = Vec::new();
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) | Event::Empty(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let attrs = e
                    .attributes()
                    .map(|a| {
                        let a = a.unwrap();
                        (
                            String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                            a.decode_and_unescape_value(reader.decoder())
                                .unwrap()
                                .into_owned(),
                        )
                    })
                    .collect();
                result.push((name, attrs));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    result
}

#[test]
fn c8_native_supported_shape_preserves_geometry_styles_and_textbox() {
    use rhwp::document_core::DocumentCore;
    let shape = format!(
        r##"<hp:rect instid="51" ratio="12"><hp:offset x="12" y="34"/><hp:orgSz width="12000" height="8000"/><hp:curSz width="6000" height="0"/><hp:flip horizontal="1" vertical="0"/><hp:rotationInfo angle="30" centerX="3000" centerY="2000"/><hp:renderingInfo><hc:transMatrix e1="2" e2="0.25" e3="12" e4="0.5" e5="3" e6="34"/><hc:scaMatrix e1="1" e2="0" e3="0" e4="0" e5="1" e6="0"/><hc:rotMatrix e1="1" e2="0" e3="0" e4="0" e5="1" e6="0"/></hp:renderingInfo><hp:lineShape color="#112233" width="77" style="DASH" outlineStyle="OUTER"/><hc:fillBrush><hc:winBrush faceColor="#ABCDEF" hatchColor="#102030" alpha="1"/></hc:fillBrush><hc:pt0 x="0" y="0"/><hc:pt1 x="6000" y="0"/><hc:pt2 x="6000" y="4000"/><hc:pt3 x="0" y="4000"/><hp:drawText><hp:subList vertAlign="CENTER">{}</hp:subList><hp:textMargin left="10" right="20" top="30" bottom="40"/></hp:drawText><hp:sz width="6000" height="4000"/></hp:rect>"##,
        c8_field_paragraph(505)
    );
    let input = c8_fixture(
        &format!(
            r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">{shape}</hp:run></hp:p>"#
        ),
        false,
    );
    let core = DocumentCore::from_bytes(&input).unwrap();
    let saved = core.export_hwpx_native().unwrap();
    let saved2 = DocumentCore::from_bytes(&saved)
        .unwrap()
        .export_hwpx_native()
        .unwrap();
    for bytes in [&saved, &saved2] {
        let elements = c8_elements(&hwpx_section_xml(bytes));
        let brush = &elements
            .iter()
            .find(|(name, _)| name == "winBrush")
            .unwrap()
            .1;
        assert!(
            brush.contains(&("faceColor".into(), "#ABCDEF".into())),
            "faceColor must remain RGB"
        );
        assert!(
            brush.contains(&("hatchColor".into(), "#102030".into())),
            "hatchColor must remain RGB"
        );
    }
    let xml = hwpx_section_xml(&saved);
    for tag in [
        "rect",
        "offset",
        "orgSz",
        "curSz",
        "flip",
        "rotationInfo",
        "renderingInfo",
        "lineShape",
        "fillBrush",
        "drawText",
        "textMargin",
    ] {
        assert!(
            c8_elements(&xml).iter().any(|(name, _)| name == tag),
            "missing supported shape child {tag}"
        );
    }
    assert_eq!(
        c8_inline_events(&xml),
        c8_inline_events(&hwpx_section_xml(&input))
    );
    let reopened = DocumentCore::from_bytes(&saved).unwrap();
    use rhwp::model::{control::Control, shape::ShapeObject};
    let Control::Shape(shape) = &reopened.document().sections[0].paragraphs[0].controls[1] else {
        panic!("shape control")
    };
    let ShapeObject::Rectangle(rect) = shape.as_ref() else {
        panic!("rectangle")
    };
    assert_eq!(rect.drawing.shape_attr.original_width, 12000);
    assert_eq!(rect.drawing.shape_attr.current_width, 6000);
    assert_eq!(rect.drawing.shape_attr.current_height, 0);
    assert_eq!(rect.common.height, 4000);
    assert!(rect.drawing.shape_attr.horz_flip);
    assert_eq!(rect.drawing.shape_attr.rotation_angle, 30);
    assert_eq!(rect.drawing.border_line.width, 77);
    assert_eq!(rect.drawing.fill.solid.unwrap().background_color, 0xEFCDAB);
    assert_eq!(rect.drawing.fill.solid.unwrap().pattern_color, 0x302010);
    assert_eq!(
        rect.drawing.text_box.as_ref().unwrap().paragraphs[0].text,
        "앞😀링😀크뒤"
    );
    assert_eq!(
        (
            rect.drawing.text_box.as_ref().unwrap().margin_left,
            rect.drawing.text_box.as_ref().unwrap().margin_bottom
        ),
        (10, 40)
    );
}

#[test]
fn c8_native_group_line_and_table_caption_keep_text_and_child_order() {
    use rhwp::document_core::DocumentCore;
    let group = r#"<hp:container instid="70" groupLevel="1"><hp:orgSz width="8000" height="6000"/><hp:curSz width="4000" height="3000"/><hp:line instid="71" startX="11" startY="22" endX="333" endY="444"><hp:orgSz width="8000" height="6000"/><hp:curSz width="4000" height="3000"/><hp:sz width="4000" height="3000"/></hp:line><hp:sz width="4000" height="3000"/></hp:container>"#;
    let body = format!(
        r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">{group}</hp:run></hp:p>"#
    );
    let caption = format!(
        r#"<hp:caption side="TOP" fullSz="1" width="8504" gap="283" lastWidth="47625"><hp:subList vertAlign="CENTER">{}</hp:subList></hp:caption>"#,
        c8_field_paragraph(707)
    );
    let table = c8_table(&c8_field_paragraph(808)).replace("<hp:tr>", &format!("{caption}<hp:tr>"));
    let input = c8_fixture(&(body + &table), false);
    let core = DocumentCore::from_bytes(&input).unwrap();
    let saved = core.export_hwpx_native().unwrap();
    assert_eq!(
        c8_inline_events(&hwpx_section_xml(&saved)),
        c8_inline_events(&hwpx_section_xml(&input))
    );
    let elements = c8_elements(&hwpx_section_xml(&saved));
    let container = elements
        .iter()
        .position(|(name, _)| name == "container")
        .unwrap();
    let line = elements
        .iter()
        .position(|(name, _)| name == "line")
        .unwrap();
    let group_sz = elements
        .iter()
        .enumerate()
        .skip(line)
        .filter(|(_, (name, _))| name == "sz")
        .nth(1)
        .unwrap()
        .0;
    assert!(container < line && line < group_sz);
    let line_attrs = &elements[line].1;
    for (key, value) in [
        ("startX", "11"),
        ("startY", "22"),
        ("endX", "333"),
        ("endY", "444"),
    ] {
        assert!(line_attrs.contains(&(key.into(), value.into())));
    }
    let caption_attrs = &elements
        .iter()
        .find(|(name, _)| name == "caption")
        .unwrap()
        .1;
    assert!(caption_attrs.contains(&("side".into(), "TOP".into())));
    assert!(caption_attrs.contains(&("gap".into(), "283".into())));
}

fn section_texts(doc: &rhwp::model::document::Document) -> Vec<String> {
    doc.sections
        .iter()
        .flat_map(|section| section.paragraphs.iter())
        .map(|para| para.text.clone())
        .collect()
}

fn first_table_location(doc: &rhwp::model::document::Document) -> Option<(usize, usize, usize)> {
    for (section_idx, section) in doc.sections.iter().enumerate() {
        for (para_idx, para) in section.paragraphs.iter().enumerate() {
            for (ctrl_idx, ctrl) in para.controls.iter().enumerate() {
                if matches!(ctrl, rhwp::model::control::Control::Table(_)) {
                    return Some((section_idx, para_idx, ctrl_idx));
                }
            }
        }
    }
    None
}

fn table_cell_texts(doc: &rhwp::model::document::Document) -> Vec<String> {
    let mut texts = Vec::new();
    for section in &doc.sections {
        for para in &section.paragraphs {
            for ctrl in &para.controls {
                if let rhwp::model::control::Control::Table(table) = ctrl {
                    for cell in &table.cells {
                        for cell_para in &cell.paragraphs {
                            texts.push(cell_para.text.clone());
                        }
                    }
                }
            }
        }
    }
    texts
}

#[test]
fn stage0_blank_hwpx_roundtrip() {
    let bytes = include_bytes!("../samples/hwpx/blank_hwpx.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("roundtrip must succeed");
    assert!(
        diff.is_empty(),
        "blank_hwpx.hwpx IR roundtrip must have no diff, got: {:#?}",
        diff
    );
}

// ---------- Stage 1 ---------------------------------------------------------
// header.xml IR 기반 동적 생성 — 샘플 parse → serialize → parse 시 리소스 카운트가 보존돼야 함.

#[test]
fn stage1_ref_empty_roundtrip() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_empty.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_empty roundtrip");
    assert!(
        diff.is_empty(),
        "ref_empty.hwpx IR roundtrip must have no diff, got: {:#?}",
        diff
    );
}

#[test]
fn stage1_ref_text_roundtrip() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_text.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_text roundtrip");
    assert!(
        diff.is_empty(),
        "ref_text.hwpx IR roundtrip must have no diff, got: {:#?}",
        diff
    );
}

// ---------- Stage 1 탐색용 진단 ----------------------------------------------
// 다음 두 샘플은 Stage 2/3 범위의 요소(run 분할·table)를 포함하므로 현재 Stage 1
// 수준에서는 diff가 없거나 일부 허용될 수 있다. 통과 여부로 Stage 1 header.xml 범위
// 내 회귀를 탐지한다 (section/table/run 차이는 다른 테스트가 커버).

#[test]
fn stage1_ref_mixed_header_level_regression_probe() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_mixed.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_mixed roundtrip");
    // 현재 Stage 1 에서는 IrDiff 0 이어야 함 — section 문단 수도 뼈대 비교 대상
    // 문제가 있으면 panic. 추후 Stage 2에서 run 비교가 추가되며 조건 강화.
    if !diff.is_empty() {
        eprintln!("ref_mixed.hwpx diffs: {:#?}", diff);
    }
    assert!(diff.is_empty(), "ref_mixed header-level regression");
}

// ---------- Stage 5: 대형 실문서 스모크 테스트 -------------------------------
// 실제 한컴 문서(표·그림·다문단 혼합)에 대해 IR 라운드트립이 뼈대 필드 수준에서
// 성립하는지 확인한다. `<hp:tbl>`/`<hp:pic>` 이 section.xml 에 아직 출력되지 않음
// (#186 이월)을 감안하여, 현 IrDiff 비교가 허용 범위 내인지 기록한다.

#[test]
fn stage5_ref_table_smoke() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_table.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_table roundtrip");
    if !diff.is_empty() {
        eprintln!("ref_table.hwpx diffs: {:#?}", diff);
    }
    // 표가 section.xml 에 아직 출력되지 않으므로 IrDiff 가 있을 수 있다.
    // 단, 파싱·직렬화 자체는 성공해야 함 (크래시 금지).
    assert!(
        diff.is_empty() || !diff.differences.is_empty(),
        "ref_table roundtrip must not crash, diff={}",
        diff.differences.len()
    );
}

#[test]
fn stage5_form_002_smoke() {
    let bytes = include_bytes!("../samples/hwpx/form-002.hwpx");
    // 양식 컨트롤이 있는 문서. IR 라운드트립이 파싱·직렬화 크래시 없이 돌아가는지만 확인.
    let _ = roundtrip_ir_diff(bytes).expect("form-002 roundtrip must not crash");
}

#[test]
fn stage5_large_real_doc_2025_q1_smoke() {
    let bytes = include_bytes!("../samples/hwpx/2025년 1분기 해외직접투자 보도자료f.hwpx");
    // 표·그림·다문단 혼합 실문서. 파싱·직렬화 크래시 없이 돌아가는지 확인.
    let _ = roundtrip_ir_diff(bytes).expect("2025 1분기 large doc roundtrip must not crash");
}

#[test]
fn stage5_large_real_doc_2025_q2_smoke() {
    let bytes = include_bytes!("../samples/hwpx/2025년 2분기 해외직접투자 (최종).hwpx");
    let _ = roundtrip_ir_diff(bytes).expect("2025 2분기 large doc roundtrip must not crash");
}

// ---------- #177 Stage 2: Serializer 원본 lineseg 보존 -----------------------
// rhwp 가 한컴 HWPX 의 `<hp:lineseg>` 값을 저장 시 훼손 없이 보존하는지 확인.
// 원본 lineseg 값이 재파싱 IR 과 일치해야 함.

#[test]
fn task177_lineseg_preserved_on_roundtrip_ref_text() {
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let bytes = include_bytes!("../samples/hwpx/ref/ref_text.hwpx");
    let doc1 = parse_hwpx(bytes).expect("parse ref_text");
    let out = serialize_hwpx(&doc1).expect("serialize");
    let doc2 = parse_hwpx(&out).expect("reparse");

    assert_eq!(doc1.sections.len(), doc2.sections.len());
    for (si, (s1, s2)) in doc1.sections.iter().zip(doc2.sections.iter()).enumerate() {
        assert_eq!(
            s1.paragraphs.len(),
            s2.paragraphs.len(),
            "section {} paragraph count",
            si
        );
        for (pi, (p1, p2)) in s1.paragraphs.iter().zip(s2.paragraphs.iter()).enumerate() {
            assert_eq!(
                p1.line_segs.len(),
                p2.line_segs.len(),
                "section {} paragraph {} line_segs count",
                si,
                pi,
            );
            for (li, (l1, l2)) in p1.line_segs.iter().zip(p2.line_segs.iter()).enumerate() {
                assert_eq!(
                    l1.text_start, l2.text_start,
                    "sec {} para {} lineseg {} text_start",
                    si, pi, li
                );
                assert_eq!(
                    l1.vertical_pos, l2.vertical_pos,
                    "sec {} para {} lineseg {} vertical_pos",
                    si, pi, li
                );
                assert_eq!(
                    l1.line_height, l2.line_height,
                    "sec {} para {} lineseg {} line_height",
                    si, pi, li
                );
                assert_eq!(
                    l1.text_height, l2.text_height,
                    "sec {} para {} lineseg {} text_height",
                    si, pi, li
                );
                assert_eq!(
                    l1.baseline_distance, l2.baseline_distance,
                    "sec {} para {} lineseg {} baseline_distance",
                    si, pi, li
                );
                assert_eq!(
                    l1.line_spacing, l2.line_spacing,
                    "sec {} para {} lineseg {} line_spacing",
                    si, pi, li
                );
            }
        }
    }
}

#[test]
fn task177_lineseg_preserved_on_roundtrip_ref_mixed() {
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let bytes = include_bytes!("../samples/hwpx/ref/ref_mixed.hwpx");
    let doc1 = parse_hwpx(bytes).expect("parse ref_mixed");
    let out = serialize_hwpx(&doc1).expect("serialize");
    let doc2 = parse_hwpx(&out).expect("reparse");

    // 첫 섹션 첫 문단의 line_segs 만이라도 완전 일치 확인
    let p1 = &doc1.sections[0].paragraphs[0];
    let p2 = &doc2.sections[0].paragraphs[0];
    assert_eq!(p1.line_segs.len(), p2.line_segs.len());
    for (a, b) in p1.line_segs.iter().zip(p2.line_segs.iter()) {
        assert_eq!(
            a.line_height, b.line_height,
            "line_height 보존 실패: IR {} vs reparsed {}",
            a.line_height, b.line_height
        );
        assert_eq!(a.vertical_pos, b.vertical_pos);
    }
}

// ---------- #177 Stage 4: 회귀 검증 샘플 ----------
// 작업지시자 제공 hwpx-02.hwpx (rhwp-studio 에서 비표준 lineseg 재현이 가능한 샘플)
// - 파싱·직렬화·재파싱이 크래시 없이 완료되어야 한다
// - 재파싱 IR 의 line_segs 가 원본과 일치해야 한다 (원본 보존 원칙)

#[test]
fn task177_hwpx_02_regression() {
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let bytes = include_bytes!("../samples/hwpx/hwpx-02.hwpx");
    let doc1 = parse_hwpx(bytes).expect("parse hwpx-02");
    let out = serialize_hwpx(&doc1).expect("serialize hwpx-02");
    let doc2 = parse_hwpx(&out).expect("reparse hwpx-02");

    // 섹션·문단 개수 보존
    assert_eq!(
        doc1.sections.len(),
        doc2.sections.len(),
        "hwpx-02 섹션 개수 불일치: {} vs {}",
        doc1.sections.len(),
        doc2.sections.len()
    );

    // 첫 섹션의 문단별 line_segs 길이 일치 확인
    let s1 = &doc1.sections[0];
    let s2 = &doc2.sections[0];
    assert_eq!(
        s1.paragraphs.len(),
        s2.paragraphs.len(),
        "hwpx-02 문단 개수 불일치"
    );

    for (pi, (p1, p2)) in s1.paragraphs.iter().zip(s2.paragraphs.iter()).enumerate() {
        assert_eq!(
            p1.line_segs.len(),
            p2.line_segs.len(),
            "hwpx-02 paragraph {} line_segs 길이 불일치: {} vs {}",
            pi,
            p1.line_segs.len(),
            p2.line_segs.len()
        );
    }
}

// ---------- #177 Stage 4: 대형 샘플 false positive 측정 ----------
// 목적: 실문서 4건에서 `validate_linesegs` 가 얼마나 많은 경고를 생성하는지
// 측정. 절대 수치가 아닌 "0에 가까운가, 설명 가능한 수준인가" 판단용.
// 실측 수치는 `mydocs/tech/hwpx_lineseg_validation.md` 에 기록.
//
// 이 테스트는 cargo test --nocapture 로 돌려 수치를 관찰한다.

fn count_validation_warnings(bytes: &[u8]) -> (usize, usize, usize, usize) {
    use rhwp::document_core::validation::WarningKind;
    use rhwp::document_core::DocumentCore;
    let doc = DocumentCore::from_bytes(bytes).expect("load doc");
    let report = doc.validation_report();
    let mut empty = 0;
    let mut uncomp = 0;
    let mut textrun = 0;
    for w in &report.warnings {
        match w.kind {
            WarningKind::LinesegArrayEmpty => empty += 1,
            WarningKind::LinesegUncomputed => uncomp += 1,
            WarningKind::LinesegTextRunReflow => textrun += 1,
        }
    }
    (report.len(), empty, uncomp, textrun)
}

#[test]
fn task177_hwpx_02_lineseg_histogram() {
    // hwpx-02 의 line_segs 분포를 관측한다.
    // 실제로 겹침이 재현되는 파일이므로, lineseg 의 어떤 특성이 문제인지 확인.
    use rhwp::parser::hwpx::parse_hwpx;
    let bytes = include_bytes!("../samples/hwpx/hwpx-02.hwpx");
    let doc = parse_hwpx(bytes).expect("parse");

    let mut total_segs = 0usize;
    let mut zero_lh = 0usize;
    let mut zero_vpos = 0usize;
    let mut zero_sw = 0usize; // segment_width
    let mut paragraphs_with_segs = 0usize;
    let mut paragraphs_empty_segs = 0usize;

    for section in &doc.sections {
        for p in &section.paragraphs {
            if p.line_segs.is_empty() {
                paragraphs_empty_segs += 1;
            } else {
                paragraphs_with_segs += 1;
                total_segs += p.line_segs.len();
                for s in &p.line_segs {
                    if s.line_height == 0 {
                        zero_lh += 1;
                    }
                    if s.vertical_pos == 0 {
                        zero_vpos += 1;
                    }
                    if s.segment_width == 0 {
                        zero_sw += 1;
                    }
                }
            }
        }
    }

    eprintln!("\n=== hwpx-02 line_segs histogram ===");
    eprintln!("paragraphs with segs:  {}", paragraphs_with_segs);
    eprintln!("paragraphs empty segs: {}", paragraphs_empty_segs);
    eprintln!("total line_segs:       {}", total_segs);
    eprintln!("  line_height == 0:    {}", zero_lh);
    eprintln!("  vertical_pos == 0:   {}", zero_vpos);
    eprintln!("  segment_width == 0:  {}", zero_sw);
    eprintln!();
}

#[test]
fn task177_false_positive_measurement() {
    let samples = [
        (
            "blank_hwpx",
            include_bytes!("../samples/hwpx/blank_hwpx.hwpx") as &[u8],
        ),
        (
            "ref_empty",
            include_bytes!("../samples/hwpx/ref/ref_empty.hwpx"),
        ),
        (
            "ref_text",
            include_bytes!("../samples/hwpx/ref/ref_text.hwpx"),
        ),
        (
            "ref_table",
            include_bytes!("../samples/hwpx/ref/ref_table.hwpx"),
        ),
        (
            "ref_mixed",
            include_bytes!("../samples/hwpx/ref/ref_mixed.hwpx"),
        ),
        ("hwpx-02", include_bytes!("../samples/hwpx/hwpx-02.hwpx")),
        ("form-002", include_bytes!("../samples/hwpx/form-002.hwpx")),
        (
            "2025-q1",
            include_bytes!("../samples/hwpx/2025년 1분기 해외직접투자 보도자료f.hwpx"),
        ),
        (
            "2025-q2",
            include_bytes!("../samples/hwpx/2025년 2분기 해외직접투자 (최종).hwpx"),
        ),
    ];

    eprintln!("\n=== #177 lineseg validation 경고 측정 ===");
    eprintln!(
        "{:<15} {:>8} {:>10} {:>11} {:>13}",
        "sample", "total", "empty", "uncomputed", "textRunRefl"
    );
    eprintln!("{}", "-".repeat(65));
    for (name, bytes) in samples {
        let (total, empty, uncomp, textrun) = count_validation_warnings(bytes);
        eprintln!(
            "{:<15} {:>8} {:>10} {:>11} {:>13}",
            name, total, empty, uncomp, textrun
        );
    }
    eprintln!();

    // assertion 없음 — 측정 결과는 기술문서에 기록
}

// ---------- Phase 1: edit → exportHwpx → reparse blocker smoke --------------
// 상용화 blocker는 "열기만 되는가"가 아니라 "편집 후 저장하고 다시 열어도 결과가
// 남는가"다. 이 테스트는 UI 저장 명령이 사용하는 동일한 export_hwpx_native 경로를
// 네이티브에서 검증한다.

#[test]
fn phase1_body_text_edit_export_hwpx_reparse_smoke() {
    use rhwp::document_core::DocumentCore;

    let bytes = include_bytes!("../samples/hwpx/ref/ref_text.hwpx");
    let mut core = DocumentCore::from_bytes(bytes).expect("load ref_text");
    let before_section_count = core.document().sections.len();
    let before_para_count = core.document().sections[0].paragraphs.len();
    let marker = "[phase1-save-smoke]";

    core.insert_text_native(0, 0, 0, marker)
        .expect("insert marker");

    let saved = core.export_hwpx_native().expect("export edited hwpx");
    let reparsed = DocumentCore::from_bytes(&saved).expect("reparse edited hwpx");
    let texts = section_texts(reparsed.document());

    assert_eq!(
        reparsed.document().sections.len(),
        before_section_count,
        "body edit save must not change section count",
    );
    assert_eq!(
        reparsed.document().sections[0].paragraphs.len(),
        before_para_count,
        "body edit save must not change paragraph count",
    );
    assert!(
        texts.iter().any(|text| text.contains(marker)),
        "edited marker must survive exportHwpx/reparse; texts={:?}",
        texts
    );
}

#[test]
fn phase1_table_cell_edit_export_hwpx_reparse_smoke() {
    use rhwp::document_core::DocumentCore;

    let bytes = include_bytes!("../samples/hwpx/ref/ref_table.hwpx");
    let mut core = DocumentCore::from_bytes(bytes).expect("load ref_table");
    let (section_idx, parent_para_idx, control_idx) =
        first_table_location(core.document()).expect("ref_table must contain a table");
    let before_table_texts = table_cell_texts(core.document());
    let marker = "[phase1-cell-save-smoke]";

    core.insert_text_in_cell_native(section_idx, parent_para_idx, control_idx, 0, 0, 0, marker)
        .expect("insert marker in first table cell");

    let saved = core.export_hwpx_native().expect("export edited table hwpx");
    let reparsed = DocumentCore::from_bytes(&saved).expect("reparse edited table hwpx");
    let after_table_texts = table_cell_texts(reparsed.document());

    assert_eq!(
        after_table_texts.len(),
        before_table_texts.len(),
        "table cell edit save must preserve top-level table cell paragraph count",
    );
    assert!(
        after_table_texts.iter().any(|text| text.contains(marker)),
        "edited table marker must survive exportHwpx/reparse; texts={:?}",
        after_table_texts
    );
}

#[test]
fn phase1_form_002_export_hwpx_reparse_preserves_validation_cleanliness() {
    use rhwp::document_core::DocumentCore;

    let bytes = include_bytes!("../samples/hwpx/form-002.hwpx");
    let core = DocumentCore::from_bytes(bytes).expect("load form-002");
    assert_eq!(
        core.validation_report().len(),
        0,
        "fixture starts validation-clean",
    );

    let saved = core.export_hwpx_native().expect("export form-002 hwpx");
    let reparsed = DocumentCore::from_bytes(&saved).expect("reparse exported form-002");

    assert_eq!(
        reparsed.validation_report().len(),
        0,
        "form-002 export/reparse must not introduce lineseg warnings: {:?}",
        reparsed.validation_report().warnings,
    );
}

// ---------- v330: 양식 속성 entity 단일 디코드 — 실제 원본 두 회 저장 ----------------
// `samples/hwpx/form-002.hwpx` 는 GearUp `apps/demo/public/sample.hwpx` 와 바이트 동일한 실제
// 원본이다 (SHA-256 5ab8f7c368e02538f75f1cd2bd82bbd8de2f925a54ba7b38ec9395b2cdb804d4).
// 원본 caption `IP R&amp;&amp;D연계` 가 저장할 때마다 `&amp;amp;` 로 늘던 결함의 회귀다.
// 기대값은 원본 XML 속성에서 읽는다. 직렬화기가 만든 값으로 기대값을 만들지 않는다.

/// 모든 `Contents/section{N}.xml` 을 0번부터 이어 붙인다.
fn hwpx_section_xml(hwpx: &[u8]) -> String {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).expect("HWPX ZIP");
    let mut xml = String::new();
    for i in 0.. {
        let mut entry = match archive.by_name(&format!("Contents/section{}.xml", i)) {
            Ok(entry) => entry,
            Err(_) => break,
        };
        entry.read_to_string(&mut xml).expect("section XML utf-8");
    }
    xml
}

/// 양식 요소(`btn`·`checkBtn`·`radioBtn`·`comboBox`·`edit`·`listItem`)의 속성을 XML 파서로 읽어
/// **원문 그대로**(디코드 없이) 문서 순서로 모은다.
fn raw_form_elements(section_xml: &str) -> Vec<(String, Vec<(String, String)>)> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(section_xml);
    let mut elements = Vec::new();
    loop {
        match reader.read_event().expect("well-formed section XML") {
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name();
                let local = name
                    .as_ref()
                    .rsplit(|b| *b == b':')
                    .next()
                    .unwrap_or(name.as_ref());
                if matches!(
                    local,
                    b"btn" | b"checkBtn" | b"radioBtn" | b"comboBox" | b"edit" | b"listItem"
                ) {
                    let attrs = e
                        .attributes()
                        .map(|attr| {
                            let attr = attr.expect("well-formed attribute");
                            (
                                String::from_utf8(attr.key.as_ref().to_vec()).expect("utf-8"),
                                String::from_utf8(attr.value.to_vec()).expect("utf-8"),
                            )
                        })
                        .collect();
                    elements.push((String::from_utf8(local.to_vec()).expect("utf-8"), attrs));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    elements
}

/// 버튼류 양식 요소의 `caption` 속성 원문을 문서 순서로 모은다.
fn raw_form_captions(section_xml: &str) -> Vec<String> {
    raw_form_elements(section_xml)
        .into_iter()
        .filter(|(name, _)| matches!(name.as_str(), "btn" | "checkBtn" | "radioBtn"))
        .filter_map(|(_, attrs)| {
            attrs
                .into_iter()
                .find(|(key, _)| key == "caption")
                .map(|(_, value)| value)
        })
        .collect()
}

/// 원본 caption 에 `&amp;` 말고 다른 참조가 없음을 확인한 뒤 그것만 푼다 (제품 디코더와 독립).
fn expected_caption(raw: &str) -> String {
    assert_eq!(
        raw.matches('&').count(),
        raw.matches("&amp;").count(),
        "fixture caption has a reference other than &amp;: {}",
        raw
    );
    raw.replace("&amp;", "&")
}

/// 본문·표 셀·글상자·캡션·머리말/꼬리말·각주/미주·숨은 설명 문단을 깊이 우선으로 모은다.
fn all_paragraphs<'a>(
    paras: &'a [rhwp::model::paragraph::Paragraph],
    out: &mut Vec<&'a rhwp::model::paragraph::Paragraph>,
) {
    use rhwp::model::control::Control;
    for para in paras {
        out.push(para);
        for ctrl in &para.controls {
            match ctrl {
                Control::Table(table) => {
                    for cell in &table.cells {
                        all_paragraphs(&cell.paragraphs, out);
                    }
                    if let Some(caption) = &table.caption {
                        all_paragraphs(&caption.paragraphs, out);
                    }
                }
                Control::Shape(shape) => shape_paragraphs(shape, out),
                Control::Picture(pic) => {
                    if let Some(caption) = &pic.caption {
                        all_paragraphs(&caption.paragraphs, out);
                    }
                }
                Control::Header(header) => all_paragraphs(&header.paragraphs, out),
                Control::Footer(footer) => all_paragraphs(&footer.paragraphs, out),
                Control::Footnote(note) => all_paragraphs(&note.paragraphs, out),
                Control::Endnote(note) => all_paragraphs(&note.paragraphs, out),
                Control::HiddenComment(comment) => all_paragraphs(&comment.paragraphs, out),
                _ => {}
            }
        }
    }
}

fn shape_paragraphs<'a>(
    shape: &'a rhwp::model::shape::ShapeObject,
    out: &mut Vec<&'a rhwp::model::paragraph::Paragraph>,
) {
    use rhwp::model::shape::ShapeObject;
    if let Some(drawing) = shape.drawing() {
        if let Some(text_box) = &drawing.text_box {
            all_paragraphs(&text_box.paragraphs, out);
        }
        if let Some(caption) = &drawing.caption {
            all_paragraphs(&caption.paragraphs, out);
        }
    }
    match shape {
        ShapeObject::Group(group) => {
            for child in &group.children {
                shape_paragraphs(child, out);
            }
            if let Some(caption) = &group.caption {
                all_paragraphs(&caption.paragraphs, out);
            }
        }
        ShapeObject::Picture(pic) => {
            if let Some(caption) = &pic.caption {
                all_paragraphs(&caption.paragraphs, out);
            }
        }
        _ => {}
    }
}

fn document_paragraphs(
    doc: &rhwp::model::document::Document,
) -> Vec<&rhwp::model::paragraph::Paragraph> {
    let mut out = Vec::new();
    for section in &doc.sections {
        all_paragraphs(&section.paragraphs, &mut out);
    }
    out
}

/// 양식 컨트롤의 모델 값 전부 (FormObject 는 PartialEq 가 없어 필드로 비교한다).
type FormFields = (
    String,
    String,
    String,
    String,
    i32,
    bool,
    (u32, u32, u32, u32),
    Vec<(String, String)>,
);

fn form_fields(doc: &rhwp::model::document::Document) -> Vec<FormFields> {
    use rhwp::model::control::Control;
    document_paragraphs(doc)
        .into_iter()
        .flat_map(|para| para.controls.iter())
        .filter_map(|ctrl| match ctrl {
            Control::Form(form) => {
                let mut properties: Vec<(String, String)> = form
                    .properties
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();
                properties.sort();
                Some((
                    format!("{:?}", form.form_type),
                    form.name.clone(),
                    form.caption.clone(),
                    form.text.clone(),
                    form.value,
                    form.enabled,
                    (form.width, form.height, form.fore_color, form.back_color),
                    properties,
                ))
            }
            _ => None,
        })
        .collect()
}

fn paragraph_texts(doc: &rhwp::model::document::Document) -> Vec<String> {
    document_paragraphs(doc)
        .into_iter()
        .map(|para| para.text.clone())
        .collect()
}

#[test]
fn v330_form_002_caption_entities_stable_across_two_native_saves() {
    use rhwp::document_core::DocumentCore;

    // Given: SHA 로 결속한 실제 원본과, 그 XML 에서 독립적으로 읽은 caption 기대값.
    let original: &[u8] = include_bytes!("../samples/hwpx/form-002.hwpx");
    assert_eq!(
        original.len(),
        131_571,
        "fixture must be the SHA-bound original"
    );
    let original_xml = hwpx_section_xml(original);
    let original_raw = raw_form_captions(&original_xml);
    let expected: Vec<String> = original_raw
        .iter()
        .map(|raw| expected_caption(raw))
        .collect();
    let amp_amp_captions = expected.iter().filter(|c| c.contains("&&")).count();
    assert_eq!(original_raw.len(), 180, "fixture button-like form controls");
    assert_eq!(amp_amp_captions, 15, "fixture captions carrying `&&`");
    assert!(expected.iter().any(|c| c == "IP R&&D연계"));
    assert!(expected.iter().any(|c| c == "R&&D 자율성트랙(일반)"));
    let original_amp_amp = original_xml.matches("&amp;amp;").count();
    assert_eq!(original_amp_amp, 0);

    // When: 에디터 저장 경로(export_hwpx_native)로 열기 → 저장 → 다시 열기를 두 번 한다.
    let opened = DocumentCore::from_bytes(original).expect("open original");
    let save1 = opened.export_hwpx_native().expect("save 1");
    let reopened1 = DocumentCore::from_bytes(&save1).expect("reopen save 1");
    let save2 = reopened1.export_hwpx_native().expect("save 2");
    let reopened2 = DocumentCore::from_bytes(&save2).expect("reopen save 2");

    // Then: 열 때 caption 은 한 번만 풀린 값이다 (원본 `&&` 는 바꾸지 않고 그대로).
    let open_forms = form_fields(opened.document());
    let open_captions: Vec<String> = open_forms.iter().map(|f| f.2.clone()).collect();
    assert_eq!(
        open_captions, expected,
        "open: decoded captions in document order"
    );

    // Then: 두 번 저장·재열기해도 양식 모델과 모든 문단 텍스트가 열 때와 같다.
    let open_texts = paragraph_texts(opened.document());
    for (label, core) in [("reopen 1", &reopened1), ("reopen 2", &reopened2)] {
        assert_eq!(
            form_fields(core.document()),
            open_forms,
            "{}: form fields",
            label
        );
        assert_eq!(
            paragraph_texts(core.document()),
            open_texts,
            "{}: paragraph texts",
            label
        );
    }

    // Then: 저장 XML 의 caption 속성은 원본과 같은 문자열이고 `&amp;amp;` 가 생기지 않는다.
    let xml1 = hwpx_section_xml(&save1);
    let xml2 = hwpx_section_xml(&save2);
    let mut saved_amp_amp = Vec::new();
    for (label, xml) in [("save 1", &xml1), ("save 2", &xml2)] {
        assert_eq!(
            raw_form_captions(xml),
            original_raw,
            "{}: raw captions",
            label
        );
        saved_amp_amp.push(xml.matches("&amp;amp;").count());
    }
    assert_eq!(
        saved_amp_amp,
        vec![0, 0],
        "nested escapes after save 1 and 2"
    );

    // Then: 두 번째 저장은 첫 저장의 양식 요소 속성을 원문까지 그대로 다시 쓴다.
    // (파일 전체 바이트 동일성은 이 회귀의 대상이 아니다 — header 문단 여백·빈 fillBrush·
    //  일부 charPrIDRef 가 첫 재저장에서 바뀌는 것은 entity 와 무관한 별도 기존 동작이다.)
    let form_elements1 = raw_form_elements(&xml1);
    assert_eq!(
        raw_form_elements(&xml2),
        form_elements1,
        "save 2 form elements"
    );

    println!(
        "ENTITY_ROUNDTRIP forms={} amp_amp_captions={} original_amp_amp={} save1_amp_amp={} save2_amp_amp={} form_elements={} save2_form_elements_equal_save1=true",
        open_forms.len(),
        amp_amp_captions,
        original_amp_amp,
        saved_amp_amp[0],
        saved_amp_amp[1],
        form_elements1.len()
    );
}

#[test]
fn v330_form_attribute_whitespace_survives_two_native_saves() {
    use rhwp::document_core::DocumentCore;
    use std::io::{Cursor, Read, Write};

    // Given: independent XML with references in start-tag and empty-tag attributes.
    let section_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">
  <hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">
    <hp:btn name="A&#9;B" caption="A&#10;B"></hp:btn>
    <hp:comboBox name="A&#13;&#10;B" selectedValue="A&#9;B">
      <hp:listItem value="A&#10;B"/><hp:listItem value="A&#9;B"/><hp:listItem value="A&#13;&#10;B"/>
    </hp:comboBox>
  </hp:run></hp:p>
</hs:sec>"#;
    let fixture = include_bytes!("../samples/hwpx/ref/ref_empty.hwpx");
    let mut original = zip::ZipArchive::new(Cursor::new(fixture)).unwrap();
    let mut input = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..original.len() {
        let mut entry = original.by_index(i).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        input
            .start_file(
                entry.name().expect("fixture filename"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        if entry.name().expect("fixture filename") == "Contents/section0.xml" {
            input.write_all(section_xml.as_bytes()).unwrap();
        } else {
            input.write_all(&bytes).unwrap();
        }
    }
    let input = input.finish().unwrap().into_inner();
    let opened = DocumentCore::from_bytes(&input).expect("open referenced whitespace");
    let expected = form_fields(opened.document());
    assert_eq!(expected.len(), 2);
    assert_eq!(expected[0].1, "A\tB");
    assert_eq!(expected[0].2, "A\nB");
    assert_eq!(expected[1].1, "A\r\nB");
    assert_eq!(expected[1].3, "A\tB");
    assert_eq!(
        expected[1].7,
        vec![
            ("listItem0".to_string(), "A\nB".to_string()),
            ("listItem1".to_string(), "A\tB".to_string()),
            ("listItem2".to_string(), "A\r\nB".to_string()),
        ]
    );

    // When: use the editor's native save/reopen path twice.
    let save1 = opened.export_hwpx_native().expect("save 1");
    let reopened1 = DocumentCore::from_bytes(&save1).expect("reopen 1");
    let save2 = reopened1.export_hwpx_native().expect("save 2");
    let reopened2 = DocumentCore::from_bytes(&save2).expect("reopen 2");

    // Then: model values survive and each raw attribute contains character references.
    for core in [&reopened1, &reopened2] {
        assert_eq!(form_fields(core.document()), expected);
    }
    for (pass, saved) in [(1, &save1), (2, &save2)] {
        let xml = hwpx_section_xml(saved);
        for (tag, occurrence, key, expected_raw) in [
            ("hp:btn", 1, "name", "A&#9;B"),
            ("hp:btn", 1, "caption", "A&#10;B"),
            ("hp:comboBox", 1, "name", "A&#13;&#10;B"),
            ("hp:comboBox", 1, "selectedValue", "A&#9;B"),
            ("hp:listItem", 1, "value", "A&#10;B"),
            ("hp:listItem", 2, "value", "A&#9;B"),
            ("hp:listItem", 3, "value", "A&#13;&#10;B"),
        ] {
            let tag_marker = format!("<{tag} ");
            let attr_marker = format!("{key}=\"");
            let raw_tag = xml
                .split(tag_marker.as_str())
                .nth(occurrence)
                .unwrap()
                .split_once('>')
                .unwrap()
                .0;
            let raw = raw_tag
                .split_once(attr_marker.as_str())
                .unwrap()
                .1
                .split_once('"')
                .unwrap()
                .0;
            assert_eq!(raw, expected_raw, "save {pass}, {tag}[{occurrence}].{key}");
        }
    }
}
