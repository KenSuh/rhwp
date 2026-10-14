//! HWPX(ZIP+XML) 직렬화 모듈 — `parser::hwpx`의 역방향.
//!
//! ## 단계 (#182)
//! - Stage 0 (완료): 기반 공사 — SerializeContext, IrDiff 하네스, canonical_defaults
//! - Stage 1: header.xml IR 기반 동적 생성
//! - Stage 2: section.xml 동적화 + charPrIDRef 매핑
//! - Stage 3: 표(Table)
//! - Stage 4: 그림(Picture) + BinData
//! - Stage 5: 도형·필드 + 대형 실문서 스모크

pub mod canonical_defaults;
pub mod content;
pub mod context;
pub mod field;
pub mod fixtures;
pub mod form;
pub mod header;
pub mod picture;
pub mod roundtrip;
pub mod section;
pub mod shape;
pub mod static_assets;
pub mod table;
pub mod utils;
pub mod writer;

use std::collections::HashSet;

use crate::model::document::Document;

use super::SerializeError;
use content::BinDataEntry as ContentBinDataEntry;
use context::SerializeContext;
use writer::HwpxZipWriter;

/// Document IR을 HWPX(ZIP+XML) 바이트로 직렬화한다.
///
/// Stage 0 이후: 빈 문서 특수 분기를 제거하고 **항상 동적 경로**를 탄다.
/// `SerializeContext`가 1-pass 스캔으로 ID 풀을 구성하고, 각 writer가 동일 컨텍스트를
/// 참조한다. 직렬화 종료 시 `assert_all_refs_resolved()`가 미등록 참조를 단언한다.
pub fn serialize_hwpx(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    use static_assets::*;

    // 1-pass: ID 풀 구성
    let mut ctx = SerializeContext::collect_from_document(doc);

    let mut z = HwpxZipWriter::new();

    // 1. mimetype (반드시 최초 엔트리, STORED, extra field 없음)
    z.write_stored("mimetype", b"application/hwp+zip")?;

    // 2. version.xml
    z.write_deflated("version.xml", VERSION_XML.as_bytes())?;

    // 3. Contents/header.xml — Stage 1 동적 생성 (IR 기반)
    let header_xml = header::write_header(doc, &ctx)?;
    z.write_deflated("Contents/header.xml", &header_xml)?;

    // 4. Contents/section{N}.xml — 실제 섹션만큼, 없으면 0개
    let section_hrefs: Vec<String> = (0..doc.sections.len())
        .map(|i| format!("Contents/section{}.xml", i))
        .collect();
    for (i, sec) in doc.sections.iter().enumerate() {
        let xml = section::write_section(sec, doc, i, &mut ctx)?;
        z.write_deflated(&section_hrefs[i], &xml)?;
    }

    // 5. Preview/PrvText.txt + Preview/PrvImage.png
    z.write_deflated("Preview/PrvText.txt", PRV_TEXT)?;
    z.write_deflated("Preview/PrvImage.png", PRV_IMAGE_PNG)?;

    // 6. settings.xml
    z.write_deflated("settings.xml", SETTINGS_XML.as_bytes())?;

    // 7. META-INF/container.rdf
    z.write_deflated("META-INF/container.rdf", META_INF_CONTAINER_RDF.as_bytes())?;

    // 8. BinData ZIP 엔트리 (Stage 4)
    //    `ctx.bin_data_map` 의 엔트리 순서대로 실제 바이너리를 ZIP에 추가.
    //    3-way 단언(binaryItemIDRef ↔ manifest ↔ ZIP entry) 의 1차 출력 지점.
    let bin_entries = ctx.bin_data_entries();
    let mut zip_bin_entries: HashSet<String> = HashSet::new();
    for entry in &bin_entries {
        let data = doc
            .bin_data_content
            .iter()
            .find(|b| b.id == entry.bin_data_id)
            .ok_or_else(|| {
                SerializeError::XmlError(format!(
                    "BinDataContent 누락: bin_data_id={}",
                    entry.bin_data_id
                ))
            })?;
        z.write_deflated(&entry.href, &data.data)?;
        zip_bin_entries.insert(entry.href.clone());
    }

    // 9. Contents/content.hpf — 항상 동적 경로 + BinData 매니페스트 엔트리
    let content_bin_entries: Vec<ContentBinDataEntry> = bin_entries
        .iter()
        .map(|e| ContentBinDataEntry {
            id: e.manifest_id.clone(),
            href: e.href.clone(),
            media_type: e.media_type.clone(),
        })
        .collect();
    let content_hpf = content::write_content_hpf(&section_hrefs, &content_bin_entries)?;
    z.write_deflated("Contents/content.hpf", &content_hpf)?;

    // 10. META-INF/container.xml
    z.write_deflated("META-INF/container.xml", META_INF_CONTAINER_XML.as_bytes())?;

    // 11. META-INF/manifest.xml
    z.write_deflated("META-INF/manifest.xml", META_INF_MANIFEST_XML.as_bytes())?;

    // 참조 정합성 단언 (Stage 1+)
    ctx.assert_all_refs_resolved()?;

    // 3-way BinData 단언 (Stage 4):
    //   - ctx.bin_data_map 의 manifest_id/href 집합
    //   - content.hpf opf:item (위에서 content_bin_entries 로 생성됨, 집합 동일)
    //   - ZIP entry (위에서 zip_bin_entries 로 기록됨)
    // 세 집합이 동일해야 한컴이 바인딩 오류 없이 그림을 표시함.
    assert_bin_data_3way(&bin_entries, &zip_bin_entries)?;

    z.finish()
}

/// 3-way BinData 동기화 단언: `ctx.bin_data_entries()`, content.hpf manifest,
/// ZIP entry 의 href 집합이 모두 일치하는지 확인.
fn assert_bin_data_3way(
    bin_entries: &[context::BinDataEntry],
    zip_entries: &HashSet<String>,
) -> Result<(), SerializeError> {
    let ctx_hrefs: HashSet<String> = bin_entries.iter().map(|e| e.href.clone()).collect();
    if ctx_hrefs != *zip_entries {
        let missing_zip: Vec<_> = ctx_hrefs.difference(zip_entries).cloned().collect();
        let orphan_zip: Vec<_> = zip_entries.difference(&ctx_hrefs).cloned().collect();
        return Err(SerializeError::XmlError(format!(
            "3-way BinData 불일치: ctx(href) vs zip_entries — ctx에만 있음: {:?}, zip에만 있음: {:?}",
            missing_zip, orphan_zip
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hwpx::parse_hwpx;

    #[test]
    fn serialize_empty_doc_parses_back() {
        let doc = Document::default();
        let bytes = serialize_hwpx(&doc).expect("serialize empty");
        let parsed = parse_hwpx(&bytes).expect("parse back");
        assert_eq!(parsed.sections.len(), 0);
        assert!(parsed.bin_data_content.is_empty());
    }

    #[test]
    fn serialize_with_one_section_parses_back() {
        let mut doc = Document::default();
        doc.sections
            .push(crate::model::document::Section::default());
        let bytes = serialize_hwpx(&doc).expect("serialize one-section");
        let parsed = parse_hwpx(&bytes).expect("parse back");
        assert_eq!(parsed.sections.len(), 1);
    }

    #[test]
    fn serialize_text_paragraph_roundtrip() {
        let mut doc = Document::default();
        let mut section = crate::model::document::Section::default();
        let mut para = crate::model::paragraph::Paragraph::default();
        para.text = "안녕 Hello 123".to_string();
        section.paragraphs.push(para);
        doc.sections.push(section);

        let bytes = serialize_hwpx(&doc).expect("serialize text");
        // 직렬화된 XML에 텍스트가 그대로 들어갔는지 ZIP에서 추출해 확인
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("valid zip");
        let mut sec0 = archive.by_name("Contents/section0.xml").expect("section0");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut sec0, &mut xml).expect("read");
        assert!(
            xml.contains("<hp:t>안녕 Hello 123</hp:t>"),
            "text not injected into section0.xml"
        );

        // 라운드트립도 확인
        drop(sec0);
        let parsed = parse_hwpx(&bytes).expect("parse back");
        assert_eq!(parsed.sections.len(), 1);
        let p0 = &parsed.sections[0].paragraphs[0];
        assert!(
            p0.text.contains("안녕 Hello 123"),
            "text roundtrip failed: {:?}",
            p0.text
        );
    }

    #[test]
    fn tab_and_linebreak_emitted_inline() {
        let mut doc = Document::default();
        let mut section = crate::model::document::Section::default();
        let mut para = crate::model::paragraph::Paragraph::default();
        para.text = "A\tB\nC".to_string();
        section.paragraphs.push(para);
        doc.sections.push(section);

        let bytes = serialize_hwpx(&doc).expect("serialize");
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip");
        let mut sec0 = archive.by_name("Contents/section0.xml").expect("section0");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut sec0, &mut xml).expect("read");
        // Stage 2.3 (ref_mixed 기반): 혼합 콘텐츠 + tab 속성 포함
        assert!(
            xml.contains(
                r#"<hp:t>A<hp:tab width="4000" leader="0" type="1"/>B<hp:lineBreak/>C</hp:t>"#
            ),
            "mixed content not rendered: {}",
            xml
        );
    }

    #[test]
    fn linesegs_emitted_per_linebreak() {
        let mut doc = Document::default();
        let mut section = crate::model::document::Section::default();
        let mut para = crate::model::paragraph::Paragraph::default();
        para.text = "A\nB\nC".to_string();
        section.paragraphs.push(para);
        doc.sections.push(section);

        let bytes = serialize_hwpx(&doc).expect("serialize");
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip");
        let mut sec0 = archive.by_name("Contents/section0.xml").expect("section0");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut sec0, &mut xml).expect("read");

        // 3줄(소프트) → lineseg 3개, textpos=0/2/4, vertpos=0/1600/3200
        let count = xml.matches("<hp:lineseg ").count();
        assert_eq!(count, 3, "expected 3 linesegs, got {}: {}", count, xml);
        assert!(xml.contains(r#"textpos="0" vertpos="0""#));
        assert!(xml.contains(r#"textpos="2" vertpos="1600""#));
        assert!(xml.contains(r#"textpos="4" vertpos="3200""#));
    }

    #[test]
    fn multi_paragraph_emits_multiple_hp_p() {
        let mut doc = Document::default();
        let mut section = crate::model::document::Section::default();
        for t in ["첫째 줄", "둘째", "끝"] {
            let mut p = crate::model::paragraph::Paragraph::default();
            p.text = t.to_string();
            section.paragraphs.push(p);
        }
        doc.sections.push(section);
        let bytes = serialize_hwpx(&doc).expect("serialize");
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip");
        let mut sec0 = archive.by_name("Contents/section0.xml").expect("section0");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut sec0, &mut xml).expect("read");
        let p_count = xml.matches("<hp:p ").count();
        assert_eq!(p_count, 3, "expected 3 <hp:p>, got {}", p_count);
        assert!(xml.contains("<hp:t>첫째 줄</hp:t>"));
        assert!(xml.contains("<hp:t>둘째</hp:t>"));
        assert!(xml.contains("<hp:t>끝</hp:t>"));
    }

    #[test]
    fn xml_escape_applied_to_section_text() {
        let mut doc = Document::default();
        let mut section = crate::model::document::Section::default();
        let mut para = crate::model::paragraph::Paragraph::default();
        para.text = "a & b < c".to_string();
        section.paragraphs.push(para);
        doc.sections.push(section);

        let bytes = serialize_hwpx(&doc).expect("serialize");
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip");
        let mut sec0 = archive.by_name("Contents/section0.xml").expect("section0");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut sec0, &mut xml).expect("read");
        assert!(xml.contains("a &amp; b &lt; c"), "escape missing: {}", xml);
    }

    /// ZIP 안 `Contents/section0.xml` 을 읽는다.
    fn section0_xml(bytes: &[u8]) -> String {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip");
        let mut sec0 = archive.by_name("Contents/section0.xml").expect("section0");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut sec0, &mut xml).expect("read");
        xml
    }

    /// 양식 컨트롤마다 (종류, name, caption, text, listItem 순서대로) 를 모은다.
    fn form_fields(doc: &Document) -> Vec<(String, String, String, String, Vec<String>)> {
        use crate::model::control::Control;
        doc.sections
            .iter()
            .flat_map(|section| section.paragraphs.iter())
            .flat_map(|para| para.controls.iter())
            .filter_map(|ctrl| match ctrl {
                Control::Form(form) => {
                    let mut items: Vec<(usize, String)> = form
                        .properties
                        .iter()
                        .filter_map(|(key, value)| {
                            let index = key.strip_prefix("listItem")?.parse().ok()?;
                            Some((index, value.clone()))
                        })
                        .collect();
                    items.sort();
                    Some((
                        format!("{:?}", form.form_type),
                        form.name.clone(),
                        form.caption.clone(),
                        form.text.clone(),
                        items.into_iter().map(|(_, value)| value).collect(),
                    ))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn form_attribute_entities_survive_two_hwpx_saves() {
        use crate::parser::hwpx::section::parse_hwpx_section;

        // Given: 한컴 양식 속성 XML — 기대값은 이 XML 에서 손으로 옮긴 리터럴이다.
        let section_xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">
  <hp:p paraPrIDRef="0" styleIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:checkBtn caption="IP R&amp;&amp;D연계" value="CHECKED" name="Check&amp;Box &quot;1&quot;" foreColor="#000000" backColor="#FFFFFF" enabled="1"><hp:sz width="6803" height="1700"/></hp:checkBtn>
      <hp:btn caption="&lt;Say &quot;hi&quot;&gt; &apos;now&apos;" name="Btn&lt;1&gt;"><hp:sz width="5000" height="1500"/></hp:btn>
      <hp:radioBtn caption="&#xAC00;&#45208; &#38; done" name="Radio&#49;" value="UNCHECKED"><hp:sz width="5000" height="1500"/></hp:radioBtn>
      <hp:comboBox name="Combo literal &amp;amp;" selectedValue="A&amp;B &lt;sel&gt;"><hp:sz width="9000" height="1500"/><hp:listItem value="A&amp;B &lt;sel&gt;"/><hp:listItem value="literal &amp;amp; item"/><hp:listItem value="&quot;q&quot; &#x26; &apos;a&apos;"/></hp:comboBox>
      <hp:t>R&amp;D 본문</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"##;
        let no_items: Vec<String> = Vec::new();
        let expected = vec![
            (
                "CheckBox".to_string(),
                "Check&Box \"1\"".to_string(),
                "IP R&&D연계".to_string(),
                String::new(),
                no_items.clone(),
            ),
            (
                "PushButton".to_string(),
                "Btn<1>".to_string(),
                "<Say \"hi\"> 'now'".to_string(),
                String::new(),
                no_items.clone(),
            ),
            (
                "RadioButton".to_string(),
                "Radio1".to_string(),
                "가나 & done".to_string(),
                String::new(),
                no_items,
            ),
            (
                "ComboBox".to_string(),
                "Combo literal &amp;".to_string(),
                String::new(),
                "A&B <sel>".to_string(),
                vec![
                    "A&B <sel>".to_string(),
                    "literal &amp; item".to_string(),
                    "\"q\" & 'a'".to_string(),
                ],
            ),
        ];
        let mut doc = Document::default();
        doc.sections
            .push(parse_hwpx_section(section_xml).expect("parse form section XML"));
        assert_eq!(form_fields(&doc), expected, "open: attributes decode once");

        // When: export → reopen 을 두 번 반복한다.
        let save1 = serialize_hwpx(&doc).expect("save 1");
        let reopened1 = parse_hwpx(&save1).expect("reopen 1");
        let save2 = serialize_hwpx(&reopened1).expect("save 2");
        let reopened2 = parse_hwpx(&save2).expect("reopen 2");

        // Then: 모델 값은 매 경계에서 같고 본문 entity 도 그대로다.
        for (label, reopened) in [("reopen 1", &reopened1), ("reopen 2", &reopened2)] {
            assert_eq!(form_fields(reopened), expected, "{}", label);
            let body = &reopened.sections[0].paragraphs[0].text;
            assert!(body.contains("R&D 본문"), "{}: body text {:?}", label, body);
        }

        // Then: XML 은 한 번만 이스케이프된다 — 글자 그대로의 "&amp;" 두 곳만 `&amp;amp;` 로 남는다.
        let xml1 = section0_xml(&save1);
        let xml2 = section0_xml(&save2);
        for (label, xml) in [("save 1", &xml1), ("save 2", &xml2)] {
            for attr in [
                r#"caption="IP R&amp;&amp;D연계""#,
                r#"name="Check&amp;Box &quot;1&quot;""#,
                r#"caption="&lt;Say &quot;hi&quot;&gt; &apos;now&apos;""#,
                r#"name="Btn&lt;1&gt;""#,
                r#"caption="가나 &amp; done""#,
                r#"name="Combo literal &amp;amp;""#,
                r#"selectedValue="A&amp;B &lt;sel&gt;""#,
                r#"value="literal &amp;amp; item""#,
                r#"value="&quot;q&quot; &amp; &apos;a&apos;""#,
            ] {
                assert!(xml.contains(attr), "{}: missing {} in {}", label, attr, xml);
            }
            assert_eq!(
                xml.matches("&amp;amp;").count(),
                2,
                "{}: only the literal \"&amp;\" texts stay escaped: {}",
                label,
                xml
            );
        }
        assert_eq!(
            xml1, xml2,
            "second save must reproduce the first section XML"
        );
    }

    #[test]
    fn mimetype_is_first_entry() {
        let doc = Document::default();
        let bytes = serialize_hwpx(&doc).expect("serialize");
        assert_eq!(&bytes[0..4], b"PK\x03\x04", "ZIP signature");
        let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let name = &bytes[30..30 + name_len];
        assert_eq!(name, b"mimetype");
    }

    #[test]
    fn mimetype_stored_not_deflated() {
        let doc = Document::default();
        let bytes = serialize_hwpx(&doc).expect("serialize");
        let method = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(method, 0, "mimetype must be STORED (method=0)");
    }

    #[test]
    fn hancom_required_files_present() {
        let mut doc = Document::default();
        doc.sections
            .push(crate::model::document::Section::default());
        let bytes = serialize_hwpx(&doc).expect("serialize");
        // ZIP 파일 목록에 한컴 필수 11개가 모두 있는지 확인
        let cursor = std::io::Cursor::new(&bytes);
        let archive = zip::ZipArchive::new(cursor).expect("valid zip");
        let names: Vec<String> = archive.file_names().map(String::from).collect();
        let required = [
            "mimetype",
            "version.xml",
            "Contents/header.xml",
            "Contents/section0.xml",
            "Contents/content.hpf",
            "Preview/PrvText.txt",
            "Preview/PrvImage.png",
            "settings.xml",
            "META-INF/container.xml",
            "META-INF/container.rdf",
            "META-INF/manifest.xml",
        ];
        for r in &required {
            assert!(names.iter().any(|n| n == r), "missing required file: {}", r);
        }
    }
}
