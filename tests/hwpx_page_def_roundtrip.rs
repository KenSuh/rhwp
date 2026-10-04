//! HWPX 쪽 설정(PageDef) 보존 회귀 테스트 — 원본 `<hp:pagePr>`·`<hp:margin>` 이 내보내기·재로드 뒤에도 그대로인지 본다.
//!
//! 고정 fixture
//! - `samples/hwpx/form-002.hwpx` — `apps/demo/public/sample.hwpx` 와 바이트 동일
//!   (SHA-256 5ab8f7c368e02538f75f1cd2bd82bbd8de2f925a54ba7b38ec9395b2cdb804d4).
//!   원본 여백 좌우 5669·위아래 2834 로 템플릿(8504/8504/5668/4252)과 다르다.
//! - `samples/hwpx/hwpx-h-01.hwpx` — 2구역 문서. 구역 0 좌우 5669, 구역 1 좌우 7086, 높이 84188.
//!
//! 기대값은 원본 XML 에서 손으로 옮긴 리터럴이거나 원본을 파싱한 IR 이다.
//! 직렬화기가 만든 값으로 기대값을 만들지 않는다.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use rhwp::document_core::DocumentCore;
use rhwp::model::document::{Document, Section};
use rhwp::model::page::{BindingMethod, PageDef};
use rhwp::model::paragraph::Paragraph;
use rhwp::parser::hwpx::parse_hwpx;
use rhwp::serializer::hwpx::serialize_hwpx;

const FORM_002: &[u8] = include_bytes!("../samples/hwpx/form-002.hwpx");
const HWPX_H_01: &[u8] = include_bytes!("../samples/hwpx/hwpx-h-01.hwpx");

type Attrs = BTreeMap<String, String>;

/// 구역 XML 하나에서 읽은 `<hp:pagePr>` 관측값.
#[derive(Debug, PartialEq)]
struct PagePrXml {
    /// 구역 XML 안 `pagePr` 요소 개수 (정상은 1)
    count: usize,
    /// 첫 `pagePr` 의 속성
    page_pr: Attrs,
    /// 첫 `pagePr` 의 자식 `margin` 속성
    margin: Attrs,
}

fn attrs(pairs: &[(&str, &str)]) -> Attrs {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn attrs_of(e: &BytesStart) -> Attrs {
    e.attributes()
        .map(|a| {
            let a = a.expect("well-formed attribute");
            (
                String::from_utf8(a.key.as_ref().to_vec()).expect("utf-8 key"),
                String::from_utf8(a.value.to_vec()).expect("utf-8 value"),
            )
        })
        .collect()
}

fn local(name: &[u8]) -> &[u8] {
    name.rsplit(|b| *b == b':').next().unwrap_or(name)
}

/// HWPX ZIP 의 `Contents/section{N}.xml` 을 0번부터 차례로 읽는다.
fn section_xmls(hwpx: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).expect("HWPX ZIP");
    let mut out = Vec::new();
    for i in 0.. {
        let mut entry = match archive.by_name(&format!("Contents/section{}.xml", i)) {
            Ok(entry) => entry,
            Err(_) => break,
        };
        let mut xml = String::new();
        entry.read_to_string(&mut xml).expect("section XML utf-8");
        out.push(xml);
    }
    out
}

/// 구역 XML 을 XML 파서로 읽어 `pagePr`·`margin` 속성을 뽑는다 (문자열 정규식에 기대지 않는다).
fn page_pr_xml(section_xml: &str) -> PagePrXml {
    let mut reader = Reader::from_str(section_xml);
    let mut count = 0;
    let mut page_pr = Attrs::new();
    let mut margin = Attrs::new();
    let mut in_page_pr = false;
    loop {
        match reader.read_event().expect("well-formed section XML") {
            Event::Start(e) => match local(e.name().as_ref()) {
                b"pagePr" => {
                    count += 1;
                    if count == 1 {
                        page_pr = attrs_of(&e);
                    }
                    in_page_pr = true;
                }
                b"margin" if in_page_pr && count == 1 => margin = attrs_of(&e),
                _ => {}
            },
            Event::Empty(e) => match local(e.name().as_ref()) {
                b"pagePr" => {
                    count += 1;
                    if count == 1 {
                        page_pr = attrs_of(&e);
                    }
                }
                b"margin" if in_page_pr && count == 1 => margin = attrs_of(&e),
                _ => {}
            },
            Event::End(e) => {
                if local(e.name().as_ref()) == b"pagePr" {
                    in_page_pr = false;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    PagePrXml {
        count,
        page_pr,
        margin,
    }
}

/// PageDef 의 모델 필드 전부 — 크기, 여백 7개(좌·우·위·아래·머리말·꼬리말·제본), attr, 방향, 제책.
fn fields(pd: &PageDef) -> (u32, u32, [u32; 7], u32, bool, BindingMethod) {
    (
        pd.width,
        pd.height,
        [
            pd.margin_left,
            pd.margin_right,
            pd.margin_top,
            pd.margin_bottom,
            pd.margin_header,
            pd.margin_footer,
            pd.margin_gutter,
        ],
        pd.attr,
        pd.landscape,
        pd.binding,
    )
}

/// 구역마다 주어진 PageDef 를 단 문서. `true` 면 문단 하나, `false` 면 문단 없는 빈 구역.
fn doc_with_sections(defs: Vec<(PageDef, bool)>) -> Document {
    let mut doc = Document::default();
    for (page_def, with_paragraph) in defs {
        let mut section = Section::default();
        section.section_def.page_def = page_def;
        if with_paragraph {
            let mut para = Paragraph::default();
            para.text = "본문".to_string();
            section.paragraphs.push(para);
        }
        doc.sections.push(section);
    }
    doc
}

fn fixture_with_page_pr(page_pr: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(FORM_002)).expect("fixture ZIP");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("fixture member");
        if entry.name() == "Contents/section0.xml" {
            let mut xml = String::new();
            entry.read_to_string(&mut xml).expect("section XML");
            let start = xml.find("<hp:pagePr ").expect("fixture pagePr");
            let end = xml[start..].find("</hp:pagePr>").expect("pagePr close")
                + start
                + "</hp:pagePr>".len();
            xml.replace_range(start..end, page_pr);
            writer
                .start_file(entry.name(), zip::write::SimpleFileOptions::default())
                .expect("write section");
            writer.write_all(xml.as_bytes()).expect("section contents");
        } else {
            writer.raw_copy_file(entry).expect("copy fixture member");
        }
    }
    writer.finish().expect("finish fixture ZIP").into_inner()
}

fn nondefault_page_pr(landscape: &str, gutter_type: &str) -> String {
    format!(
        r#"<hp:pagePr landscape="{landscape}" width="59528" height="84186" gutterType="{gutter_type}"><hp:margin header="5555" footer="6666" gutter="7777" left="1111" right="2222" top="3333" bottom="4444"/></hp:pagePr>"#
    )
}

fn svg_dimensions(core: &DocumentCore) -> (f64, f64) {
    let svg = core.render_page_svg_native(0).expect("native SVG");
    let mut reader = Reader::from_str(&svg);
    loop {
        match reader.read_event().expect("SVG XML") {
            Event::Start(e) if local(e.name().as_ref()) == b"svg" => {
                let attrs = attrs_of(&e);
                return (
                    attrs["width"].parse().expect("SVG width"),
                    attrs["height"].parse().expect("SVG height"),
                );
            }
            Event::Eof => panic!("SVG root missing"),
            _ => {}
        }
    }
}

#[test]
fn file_roundtrip_preserves_orientation_and_all_binding_modes() {
    for (landscape_xml, landscape, gutter_xml, binding, attr) in [
        ("WIDELY", false, "LEFT_ONLY", BindingMethod::SingleSided, 0),
        ("WIDELY", false, "LEFT_RIGHT", BindingMethod::DuplexSided, 2),
        ("WIDELY", false, "TOP_BOTTOM", BindingMethod::TopFlip, 4),
        ("NARROWLY", true, "LEFT_ONLY", BindingMethod::SingleSided, 1),
        (
            "NARROWLY",
            true,
            "LEFT_RIGHT",
            BindingMethod::DuplexSided,
            3,
        ),
        ("NARROWLY", true, "TOP_BOTTOM", BindingMethod::TopFlip, 5),
    ] {
        // Given: real fixture members, with manually specified pagePr values.
        let input = fixture_with_page_pr(&nondefault_page_pr(landscape_xml, gutter_xml));
        let expected_xml = page_pr_xml(&section_xmls(&input)[0]);
        let expected_ir = (
            59528,
            84186,
            [1111, 2222, 3333, 4444, 5555, 6666, 7777],
            attr,
            landscape,
            binding,
        );

        // When: file import -> export -> reload -> re-export.
        let core = DocumentCore::from_bytes(&input).expect("import fixture");
        let exported = core.export_hwpx_native().expect("export fixture");
        let reloaded = DocumentCore::from_bytes(&exported).expect("reload export");
        let reexported = reloaded.export_hwpx_native().expect("re-export fixture");

        // Then: independently specified IR and input XML survive each boundary.
        for doc in [core.document(), reloaded.document()] {
            assert_eq!(fields(&doc.sections[0].section_def.page_def), expected_ir);
        }
        for bytes in [&exported, &reexported] {
            assert_eq!(page_pr_xml(&section_xmls(bytes)[0]), expected_xml);
        }
        println!("FILE_ROUNDTRIP {landscape_xml}/{gutter_xml} attr={attr} preserved");
    }
}

#[test]
fn native_render_interprets_landscape_once_without_rewriting_stored_dimensions() {
    // Given: Hancom's stored A4 dimensions, with landscape selected independently.
    let input = fixture_with_page_pr(&nondefault_page_pr("NARROWLY", "LEFT_RIGHT"));
    let core = DocumentCore::from_bytes(&input).expect("import landscape fixture");

    // When: native rendering consumes the imported PageDef.
    let (width, height) = svg_dimensions(&core);

    // Then: display is landscape, storage is unchanged, and body uses the same axes.
    assert!((width - 1122.48).abs() < 0.000_001);
    assert!((height - 793.706_666_666_666_7).abs() < 0.000_001);
    let pd = &core.document().sections[0].section_def.page_def;
    assert_eq!((pd.width, pd.height), (59528, 84186));
    let areas = rhwp::model::page::PageAreas::from_page_def(pd);
    assert_eq!(areas.body_area.width(), 73076);
    assert_eq!(areas.body_area.height(), 39530);
    println!(
        "NATIVE_LANDSCAPE stored=59528x84186 svg={width}x{height} pages={}",
        core.page_count()
    );
}

#[test]
fn landscape_does_not_infer_or_normalize_dimension_order() {
    // Given: an explicit flag with unusual stored dimensions; no heuristic repair.
    let page_pr = nondefault_page_pr("NARROWLY", "LEFT_ONLY").replace(
        "width=\"59528\" height=\"84186\"",
        "width=\"84186\" height=\"59528\"",
    );
    let input = fixture_with_page_pr(&page_pr);
    let core = DocumentCore::from_bytes(&input).expect("import unusual dimensions");

    // When: native rendering applies only the explicit orientation flag.
    let (width, height) = svg_dimensions(&core);

    // Then: one swap, with no short-side/long-side normalization or export swap.
    assert!((width - 793.706_666_666_666_7).abs() < 0.000_001);
    assert!((height - 1122.48).abs() < 0.000_001);
    let exported = core
        .export_hwpx_native()
        .expect("export unusual dimensions");
    assert_eq!(
        page_pr_xml(&section_xmls(&exported)[0]),
        page_pr_xml(&section_xmls(&input)[0])
    );
}

#[test]
fn missing_orientation_and_binding_keep_legacy_defaults_and_explicit_zero_margins() {
    // Given: attributes absent, while dimensions make the zero margins intentional.
    let input = fixture_with_page_pr(
        r#"<hp:pagePr width="59528" height="84186"><hp:margin header="0" footer="0" gutter="0" left="0" right="0" top="0" bottom="0"/></hp:pagePr>"#,
    );

    // When: importing and exporting the file.
    let core = DocumentCore::from_bytes(&input).expect("import missing attributes");
    let exported = core
        .export_hwpx_native()
        .expect("export missing attributes");
    let reloaded = parse_hwpx(&exported).expect("reload missing attributes");

    // Then: defaults stay portrait/single-sided, and zero margins remain zero.
    for doc in [core.document(), &reloaded] {
        assert_eq!(
            fields(&doc.sections[0].section_def.page_def),
            (59528, 84186, [0; 7], 0, false, BindingMethod::SingleSided)
        );
    }
}

#[test]
fn input_mutations_are_detected_without_changing_engine_source() {
    // Given: fixed expected IR and four independently corrupted input values.
    let valid = nondefault_page_pr("NARROWLY", "LEFT_RIGHT");
    let expected = (
        59528,
        84186,
        [1111, 2222, 3333, 4444, 5555, 6666, 7777],
        3,
        true,
        BindingMethod::DuplexSided,
    );
    for (name, mutant) in [
        ("orientation", valid.replace("NARROWLY", "WIDELY")),
        ("binding", valid.replace(" gutterType=\"LEFT_RIGHT\"", "")),
        (
            "dimensions",
            valid.replace(
                "width=\"59528\" height=\"84186\"",
                "width=\"84186\" height=\"59528\"",
            ),
        ),
        ("zero-margin", valid.replace("left=\"1111\"", "left=\"0\"")),
    ] {
        // When: importing the corrupted HWPX archive.
        let doc = parse_hwpx(&fixture_with_page_pr(&mutant)).expect("import input mutation");

        // Then: keeping the original expectation would fail for every corruption.
        assert_ne!(
            fields(&doc.sections[0].section_def.page_def),
            expected,
            "{name} mutation escaped"
        );
        println!("INPUT_MUTATION {name} detected");
    }
}

#[test]
fn form_002_noedit_export_keeps_original_page_pr() {
    let original = section_xmls(FORM_002);
    assert_eq!(original.len(), 1);
    let original_page_pr = page_pr_xml(&original[0]);
    assert_eq!(
        original_page_pr,
        PagePrXml {
            count: 1,
            page_pr: attrs(&[
                ("landscape", "WIDELY"),
                ("width", "59528"),
                ("height", "84186"),
                ("gutterType", "LEFT_ONLY"),
            ]),
            margin: attrs(&[
                ("header", "4252"),
                ("footer", "4252"),
                ("gutter", "0"),
                ("left", "5669"),
                ("right", "5669"),
                ("top", "2834"),
                ("bottom", "2834"),
            ]),
        },
        "fixture must keep its non-template margins"
    );

    // 호스트와 같은 경로: 바이트 로드 → 편집 없이 HWPX 내보내기
    let core = DocumentCore::from_bytes(FORM_002).expect("load form-002");
    let exported = core.export_hwpx_native().expect("no-edit export");
    let exported_sections = section_xmls(&exported);
    assert_eq!(exported_sections.len(), 1);
    assert_eq!(page_pr_xml(&exported_sections[0]), original_page_pr);
}

#[test]
fn form_002_noedit_reload_keeps_page_def_ir() {
    let original = parse_hwpx(FORM_002).expect("parse original");
    assert_eq!(
        fields(&original.sections[0].section_def.page_def),
        (
            59528,
            84186,
            [5669, 5669, 2834, 2834, 4252, 4252, 0],
            0,
            false,
            BindingMethod::SingleSided
        )
    );

    let core = DocumentCore::from_bytes(FORM_002).expect("load form-002");
    let reloaded = DocumentCore::from_bytes(&core.export_hwpx_native().expect("export"))
        .expect("reload export");
    let reloaded = reloaded.document();
    assert_eq!(reloaded.sections.len(), original.sections.len());
    assert_eq!(
        fields(&reloaded.sections[0].section_def.page_def),
        fields(&original.sections[0].section_def.page_def)
    );
}

#[test]
fn hwpx_h_01_each_section_keeps_its_own_page_pr() {
    let page_pr = attrs(&[
        ("landscape", "WIDELY"),
        ("width", "59528"),
        ("height", "84188"),
        ("gutterType", "LEFT_ONLY"),
    ]);
    let margin = |left_right: &str| {
        attrs(&[
            ("header", "2834"),
            ("footer", "2834"),
            ("gutter", "0"),
            ("left", left_right),
            ("right", left_right),
            ("top", "4251"),
            ("bottom", "4251"),
        ])
    };
    let expected = vec![
        PagePrXml {
            count: 1,
            page_pr: page_pr.clone(),
            margin: margin("5669"),
        },
        PagePrXml {
            count: 1,
            page_pr,
            margin: margin("7086"),
        },
    ];
    let original: Vec<PagePrXml> = section_xmls(HWPX_H_01)
        .iter()
        .map(|xml| page_pr_xml(xml))
        .collect();
    assert_eq!(original, expected, "fixture sections must differ");

    let core = DocumentCore::from_bytes(HWPX_H_01).expect("load hwpx-h-01");
    let exported = core.export_hwpx_native().expect("no-edit export");
    let exported_page_prs: Vec<PagePrXml> = section_xmls(&exported)
        .iter()
        .map(|xml| page_pr_xml(xml))
        .collect();
    assert_eq!(exported_page_prs, expected);

    let original_ir = parse_hwpx(HWPX_H_01).expect("parse original");
    let reloaded = DocumentCore::from_bytes(&exported).expect("reload export");
    let reloaded = reloaded.document();
    assert_eq!(reloaded.sections.len(), 2);
    for (i, (want, got)) in original_ir
        .sections
        .iter()
        .zip(&reloaded.sections)
        .enumerate()
    {
        assert_eq!(
            fields(&got.section_def.page_def),
            fields(&want.section_def.page_def),
            "section {} PageDef",
            i
        );
    }
}

#[test]
fn one_char_body_edit_keeps_original_page_def() {
    let original = parse_hwpx(FORM_002).expect("parse original");
    let mut core = DocumentCore::from_bytes(FORM_002).expect("load form-002");
    assert_eq!(core.document().sections[0].paragraphs[1].text, "");
    core.insert_text_native(0, 1, 0, "가")
        .expect("insert one char");

    let exported = core.export_hwpx_native().expect("export after edit");
    let reloaded = DocumentCore::from_bytes(&exported).expect("reload export");
    // 편집이 실제로 저장됐는지 먼저 본다 — 본문은 원본과 달라야 한다.
    assert_eq!(reloaded.document().sections[0].paragraphs[1].text, "가");
    assert_eq!(
        fields(&reloaded.document().sections[0].section_def.page_def),
        fields(&original.sections[0].section_def.page_def)
    );
    assert_eq!(
        page_pr_xml(&section_xmls(&exported)[0]),
        page_pr_xml(&section_xmls(FORM_002)[0])
    );
}

#[test]
fn every_page_def_field_is_written_to_its_own_attribute() {
    // 여백 7개가 모두 달라 속성 이름이 뒤바뀌면 드러난다. 구역 0 은 문단 있음, 구역 1 은 문단 없는 빈 구역.
    let page_def = || PageDef {
        width: 61001,
        height: 86002,
        margin_left: 1111,
        margin_right: 2222,
        margin_top: 3333,
        margin_bottom: 4444,
        margin_header: 5555,
        margin_footer: 6666,
        margin_gutter: 7777,
        ..Default::default()
    };
    let doc = doc_with_sections(vec![(page_def(), true), (page_def(), false)]);
    let bytes = serialize_hwpx(&doc).expect("serialize");

    let expected = PagePrXml {
        count: 1,
        page_pr: attrs(&[
            ("landscape", "WIDELY"),
            ("width", "61001"),
            ("height", "86002"),
            ("gutterType", "LEFT_ONLY"),
        ]),
        margin: attrs(&[
            ("header", "5555"),
            ("footer", "6666"),
            ("gutter", "7777"),
            ("left", "1111"),
            ("right", "2222"),
            ("top", "3333"),
            ("bottom", "4444"),
        ]),
    };
    let xmls = section_xmls(&bytes);
    assert_eq!(xmls.len(), 2);
    assert_eq!(page_pr_xml(&xmls[0]), expected, "section with a paragraph");
    assert_eq!(
        page_pr_xml(&xmls[1]),
        expected,
        "section without paragraphs"
    );

    let reloaded = parse_hwpx(&bytes).expect("reload");
    assert_eq!(reloaded.sections.len(), 2);
    for (i, section) in reloaded.sections.iter().enumerate() {
        assert_eq!(
            fields(&section.section_def.page_def),
            (
                61001,
                86002,
                [1111, 2222, 3333, 4444, 5555, 6666, 7777],
                0,
                false,
                BindingMethod::SingleSided
            ),
            "section {}",
            i
        );
    }
}

#[test]
fn landscape_and_binding_use_owpml_values() {
    // 한컴 실파일 관측: 세로 문서는 landscape="WIDELY", 가로 문서(1건)는 landscape="NARROWLY" 이고
    // 가로여도 width/height 는 세로형(짧은 변/긴 변) 그대로다. 모델(HWP 규약)과 같으므로 교환하지 않는다.
    let page_def = |landscape: bool, binding: BindingMethod| PageDef {
        width: 59528,
        height: 84186,
        margin_left: 8504,
        margin_right: 8504,
        margin_top: 5668,
        margin_bottom: 4252,
        margin_header: 4252,
        margin_footer: 4252,
        landscape,
        binding,
        ..Default::default()
    };
    let doc = doc_with_sections(vec![
        (page_def(false, BindingMethod::SingleSided), true),
        (page_def(true, BindingMethod::DuplexSided), true),
        (page_def(false, BindingMethod::TopFlip), true),
    ]);
    let observed: Vec<Attrs> = section_xmls(&serialize_hwpx(&doc).expect("serialize"))
        .iter()
        .map(|xml| page_pr_xml(xml).page_pr)
        .collect();
    assert_eq!(
        observed,
        vec![
            attrs(&[
                ("landscape", "WIDELY"),
                ("width", "59528"),
                ("height", "84186"),
                ("gutterType", "LEFT_ONLY"),
            ]),
            attrs(&[
                ("landscape", "NARROWLY"),
                ("width", "59528"),
                ("height", "84186"),
                ("gutterType", "LEFT_RIGHT"),
            ]),
            attrs(&[
                ("landscape", "WIDELY"),
                ("width", "59528"),
                ("height", "84186"),
                ("gutterType", "TOP_BOTTOM"),
            ]),
        ]
    );
}

#[test]
fn default_page_def_keeps_template_page_pr() {
    // PageDef 가 비어 있는(= 없는) 구역은 기존 동작대로 템플릿 값을 쓴다.
    let doc = doc_with_sections(vec![
        (PageDef::default(), true),
        (PageDef::default(), false),
    ]);
    let expected = PagePrXml {
        count: 1,
        page_pr: attrs(&[
            ("landscape", "WIDELY"),
            ("width", "59528"),
            ("height", "84186"),
            ("gutterType", "LEFT_ONLY"),
        ]),
        margin: attrs(&[
            ("header", "4252"),
            ("footer", "4252"),
            ("gutter", "0"),
            ("left", "8504"),
            ("right", "8504"),
            ("top", "5668"),
            ("bottom", "4252"),
        ]),
    };
    let xmls = section_xmls(&serialize_hwpx(&doc).expect("serialize"));
    assert_eq!(xmls.len(), 2);
    for (i, xml) in xmls.iter().enumerate() {
        assert_eq!(page_pr_xml(xml), expected, "section {}", i);
    }
}

#[test]
fn out_of_schema_page_def_values_are_written_verbatim() {
    // 크기 0(스키마 positiveInteger 밖)과 u32 최댓값도 모델 값 그대로 쓰고 그대로 다시 읽힌다.
    let zero_size = PageDef {
        width: 0,
        height: 0,
        margin_left: 4252,
        margin_right: 4252,
        margin_top: 5669,
        margin_bottom: 4252,
        margin_header: 2835,
        margin_footer: 2835,
        margin_gutter: 0,
        ..Default::default()
    };
    let max = PageDef {
        width: u32::MAX,
        height: u32::MAX,
        margin_left: u32::MAX,
        margin_right: u32::MAX,
        margin_top: u32::MAX,
        margin_bottom: u32::MAX,
        margin_header: u32::MAX,
        margin_footer: u32::MAX,
        margin_gutter: u32::MAX,
        ..Default::default()
    };
    let doc = doc_with_sections(vec![(zero_size, true), (max, true)]);
    let bytes = serialize_hwpx(&doc).expect("serialize");

    let xmls = section_xmls(&bytes);
    assert_eq!(xmls.len(), 2);
    assert_eq!(
        page_pr_xml(&xmls[0]),
        PagePrXml {
            count: 1,
            page_pr: attrs(&[
                ("landscape", "WIDELY"),
                ("width", "0"),
                ("height", "0"),
                ("gutterType", "LEFT_ONLY"),
            ]),
            margin: attrs(&[
                ("header", "2835"),
                ("footer", "2835"),
                ("gutter", "0"),
                ("left", "4252"),
                ("right", "4252"),
                ("top", "5669"),
                ("bottom", "4252"),
            ]),
        }
    );
    let max_text = "4294967295";
    assert_eq!(
        page_pr_xml(&xmls[1]),
        PagePrXml {
            count: 1,
            page_pr: attrs(&[
                ("landscape", "WIDELY"),
                ("width", max_text),
                ("height", max_text),
                ("gutterType", "LEFT_ONLY"),
            ]),
            margin: attrs(&[
                ("header", max_text),
                ("footer", max_text),
                ("gutter", max_text),
                ("left", max_text),
                ("right", max_text),
                ("top", max_text),
                ("bottom", max_text),
            ]),
        }
    );

    let reloaded = parse_hwpx(&bytes).expect("reload");
    assert_eq!(
        fields(&reloaded.sections[0].section_def.page_def),
        (
            0,
            0,
            [4252, 4252, 5669, 4252, 2835, 2835, 0],
            0,
            false,
            BindingMethod::SingleSided
        )
    );
    assert_eq!(
        fields(&reloaded.sections[1].section_def.page_def),
        (
            u32::MAX,
            u32::MAX,
            [u32::MAX; 7],
            0,
            false,
            BindingMethod::SingleSided
        )
    );
}
