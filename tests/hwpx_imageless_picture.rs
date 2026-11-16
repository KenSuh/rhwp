//! 그림 없는 그림 개체(`<hc:img binaryItemIDRef="">`)의 저장→재열기 왕복과 미등록 그림 참조 거부를 검증한다.
//!
//! 한컴은 그림을 지운 그림 개체를 빈 참조로 저장한다(T5 결재 양식의 두 번째 `<hp:pic>`). rhwp 는 이것을
//! bin_data_id 0 으로 읽고 같은 빈 참조로 쓴다. 비어 있지 않은데 등록되지 않은 참조는 계속 거부한다.

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

fn zip_entry(hwpx: &[u8], name: &str) -> Option<String> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).unwrap();
    let mut entry = archive.by_name(name).ok()?;
    let mut s = String::new();
    entry.read_to_string(&mut s).unwrap();
    Some(s)
}

fn zip_names(hwpx: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).unwrap();
    let mut names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().unwrap().to_string())
        .collect();
    names.sort();
    names
}

/// `<hc:img>` 의 binaryItemIDRef 값을 문서 순서로 모은다.
fn img_refs(section_xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(section_xml);
    let mut out = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).unwrap() {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"img" => {
                for a in e.attributes().flatten() {
                    if a.key.as_ref() == b"binaryItemIDRef" {
                        out.push(String::from_utf8_lossy(&a.value).into_owned());
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

fn picture(reference: &str) -> String {
    format!(
        r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:pic id="7" zOrder="5" numberingType="PICTURE" textWrap="TOP_AND_BOTTOM" textFlow="BOTH_SIDES" lock="0" dropcapstyle="None" href="" groupLevel="0" instid="8" reverse="0"><hp:offset x="0" y="0"/><hp:orgSz width="124125" height="77775"/><hp:curSz width="8443" height="4212"/><hp:flip horizontal="0" vertical="0"/><hp:rotationInfo angle="0" centerX="4221" centerY="2106" rotateimage="1"/><hp:renderingInfo><hc:transMatrix e1="1" e2="0" e3="0" e4="0" e5="1" e6="0"/><hc:scaMatrix e1="0.06802" e2="0" e3="0" e4="0" e5="0.054156" e6="0"/><hc:rotMatrix e1="1" e2="0" e3="0" e4="0" e5="1" e6="0"/></hp:renderingInfo><hp:imgRect><hc:pt0 x="0" y="0"/><hc:pt1 x="124125" y="0"/><hc:pt2 x="124125" y="77775"/><hc:pt3 x="0" y="77775"/></hp:imgRect><hp:imgClip left="0" right="76800" top="0" bottom="48060"/><hp:inMargin left="0" right="0" top="0" bottom="0"/><hp:imgDim dimwidth="76800" dimheight="48060"/><hc:img binaryItemIDRef="{reference}" bright="0" contrast="0" effect="REAL_PIC" alpha="0"/><hp:effects/><hp:sz width="8443" widthRelTo="ABSOLUTE" height="4212" heightRelTo="ABSOLUTE" protect="0"/><hp:pos treatAsChar="1" affectLSpacing="0" flowWithText="1" allowOverlap="0" holdAnchorAndSO="0" vertRelTo="PARA" horzRelTo="PARA" vertAlign="TOP" horzAlign="LEFT" vertOffset="0" horzOffset="0"/><hp:outMargin left="0" right="0" top="0" bottom="0"/></hp:pic><hp:t>뒤</hp:t></hp:run></hp:p>"#
    )
}

fn picture_bin_ids(doc: &rhwp::model::document::Document) -> Vec<u16> {
    fn walk(paras: &[rhwp::model::paragraph::Paragraph], out: &mut Vec<u16>) {
        for p in paras {
            for c in &p.controls {
                match c {
                    Control::Picture(pic) => out.push(pic.image_attr.bin_data_id),
                    Control::Table(t) => t.cells.iter().for_each(|c| walk(&c.paragraphs, out)),
                    _ => {}
                }
            }
        }
    }
    let mut out = Vec::new();
    for s in &doc.sections {
        walk(&s.paragraphs, &mut out);
    }
    out
}

#[test]
fn imageless_picture_round_trips_with_empty_reference_two_saves() {
    let input = fixture(&picture(""));
    let doc = parse_hwpx(&input).expect("image-less picture opens");
    assert_eq!(picture_bin_ids(&doc), vec![0]);
    assert!(doc.bin_data_content.is_empty());

    let saved = serialize_hwpx(&doc).expect("image-less picture must save");
    let section = zip_entry(&saved, "Contents/section0.xml").unwrap();
    // 같은 빈 참조를 쓰고, bin 항목을 새로 만들거나 다른 그림으로 가리키지 않는다.
    assert_eq!(img_refs(&section), vec![String::new()]);
    assert!(!zip_names(&saved).iter().any(|n| n.starts_with("BinData/")));
    assert!(!zip_entry(&saved, "Contents/content.hpf")
        .unwrap()
        .contains("BinData/"));

    let reopened = parse_hwpx(&saved).expect("reopen");
    assert_eq!(picture_bin_ids(&reopened), vec![0]);
    let saved2 = serialize_hwpx(&reopened).expect("second save");
    assert_eq!(
        zip_entry(&saved2, "Contents/section0.xml").unwrap(),
        section,
        "second save must be byte-identical"
    );
}

#[test]
fn non_empty_unregistered_picture_reference_is_still_refused() {
    // 숫자 참조(미등록 id 7), 숫자 없는 참조, 0 번 참조 — 모두 «그림 없음» 으로 바뀌지 않고 저장이 거부된다.
    for reference in ["image7", "foo", "image0"] {
        let doc = parse_hwpx(&fixture(&picture(reference))).expect("opens");
        assert_ne!(
            picture_bin_ids(&doc),
            vec![0],
            "{reference}: must not become image-less"
        );
        let err = serialize_hwpx(&doc)
            .err()
            .unwrap_or_else(|| panic!("{reference}: unregistered reference must be refused"));
        assert!(
            err.to_string().contains("binaryItemIDRef 미등록"),
            "{reference}: {err}"
        );
    }
}

#[test]
fn raw_t5_saves_and_keeps_its_image_and_imageless_pictures() {
    let original = img_refs(&zip_entry(T5, "Contents/section0.xml").unwrap());
    assert_eq!(original, vec!["image1".to_string(), String::new()]);

    let doc = parse_hwpx(T5).expect("T5 open");
    let saved = serialize_hwpx(&doc).expect("raw T5 must save");
    assert_eq!(
        img_refs(&zip_entry(&saved, "Contents/section0.xml").unwrap()),
        original
    );

    let core = rhwp::DocumentCore::from_bytes(T5).expect("T5 opens through DocumentCore");
    let exported = core.export_hwpx_native().expect("raw T5 must export");
    let reopened = parse_hwpx(&exported).expect("exported T5 reopens");
    assert_eq!(picture_bin_ids(&reopened), picture_bin_ids(&doc));
}
