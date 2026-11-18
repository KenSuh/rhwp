//! 양식 caption `&&` 표시 접기 회귀 — 실제 form-002 SVG 는 `&` 로 그리고 모델·저장 XML 은 그대로 둔다.
//!
//! `samples/hwpx/form-002.hwpx` 는 GearUp `apps/demo/public/sample.hwpx` 와 바이트 동일한 실제
//! 원본이다 (SHA-256 5ab8f7c368e02538f75f1cd2bd82bbd8de2f925a54ba7b38ec9395b2cdb804d4).
//! 원본 caption `IP R&amp;&amp;D연계` 의 모델 값은 `IP R&&D연계` 이고, 한컴 관찰 출력은 양식
//! caption 의 `&&` 를 `&` 하나로 표시한다 (업스트림 Task #1562).
//! 기대값은 원본 XML 과 이 파일의 문자 규칙으로 만든다. 제품 helper 로 기대값을 만들지 않는다.

use rhwp::document_core::DocumentCore;

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
fn model_caption(raw: &str) -> String {
    assert_eq!(
        raw.matches('&').count(),
        raw.matches("&amp;").count(),
        "fixture caption has a reference other than &amp;: {}",
        raw
    );
    raw.replace("&amp;", "&")
}

/// 이 시험의 표시 규칙 — 왼쪽부터 겹치지 않는 `&&` 를 `&` 하나로 접는다 (제품 helper 와 독립).
fn expected_display(model: &str) -> String {
    model.replace("&&", "&")
}

/// SVG 텍스트 내용 escape (`&`·`<`·`>`·`"`·`'` 다섯 글자).
fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// SVG 렌더러가 양식 글자를 그리는 `<text … dominant-baseline="central" font-family="'맑은 고딕',sans-serif">`
/// 요소의 내용을 모은다.
fn svg_form_texts(svg: &str) -> Vec<String> {
    const OPEN: &str = "dominant-baseline=\"central\" font-family=\"'맑은 고딕',sans-serif\">";
    let mut texts = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find(OPEN) {
        let after = &rest[start + OPEN.len()..];
        let end = after.find("</text>").expect("form text element is closed");
        texts.push(after[..end].to_string());
        rest = &after[end..];
    }
    texts
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

/// 모델의 양식 컨트롤 caption 을 문서 순서로 모은다.
fn model_form_captions(doc: &rhwp::model::document::Document) -> Vec<String> {
    use rhwp::model::control::Control;
    let mut paras = Vec::new();
    for section in &doc.sections {
        all_paragraphs(&section.paragraphs, &mut paras);
    }
    paras
        .into_iter()
        .flat_map(|para| para.controls.iter())
        .filter_map(|ctrl| match ctrl {
            Control::Form(form) => Some(form.caption.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn form_002_form_caption_display_folds_ampamp_in_svg_only() {
    // Given: SHA 로 결속한 실제 원본과, 그 XML 에서 독립적으로 읽은 caption 모델 값.
    let original: &[u8] = include_bytes!("../samples/hwpx/form-002.hwpx");
    assert_eq!(
        original.len(),
        131_571,
        "fixture must be the SHA-bound original"
    );
    let original_xml = hwpx_section_xml(original);
    let original_raw = raw_form_captions(&original_xml);
    assert_eq!(original_raw.len(), 180, "fixture button-like form controls");
    let expected_model: Vec<String> = original_raw.iter().map(|raw| model_caption(raw)).collect();
    let ampamp: Vec<&String> = expected_model.iter().filter(|c| c.contains("&&")).collect();

    let core = DocumentCore::from_bytes(original).expect("open original");
    let model_before = model_form_captions(core.document());
    assert_eq!(model_before, expected_model, "open: model captions");

    // When: 모든 페이지를 SVG 로 그린다.
    let pages = core.page_count();
    let mut form_texts = Vec::new();
    let mut ip_rnd_display = 0;
    for page in 0..pages {
        let svg = core.render_page_svg_native(page).expect("render page SVG");
        ip_rnd_display += svg.matches(">IP R&amp;D연계</text>").count();
        form_texts.extend(svg_form_texts(&svg));
    }

    // Then: `&&` caption 마다 접힌 표시가 그려지고 접히지 않은 원 값은 그려지지 않는다.
    let folded: Vec<String> = ampamp
        .iter()
        .map(|c| escape_text(&expected_display(c)))
        .collect();
    let unfolded: Vec<String> = ampamp.iter().map(|c| escape_text(c)).collect();
    let single_amp_lookalikes = expected_model
        .iter()
        .filter(|c| !c.contains("&&") && folded.contains(&escape_text(c)))
        .count();
    assert_eq!(
        single_amp_lookalikes, 0,
        "a non-`&&` caption already equals a folded display; F would be ambiguous"
    );
    let rendered_folded = form_texts.iter().filter(|t| folded.contains(t)).count();
    let rendered_unfolded = form_texts.iter().filter(|t| unfolded.contains(t)).count();
    println!(
        "SVG_CAPTION_FOLD pages={} model_ampamp_captions={} rendered_folded={} rendered_unfolded={} ip_rnd_display={} ",
        pages,
        ampamp.len(),
        rendered_folded,
        rendered_unfolded,
        ip_rnd_display
    );

    // Then: 렌더 뒤에도 모델 caption 은 원 값(`&&`) 그대로다.
    let model_after = model_form_captions(core.document());
    assert_eq!(
        model_after, expected_model,
        "render must not mutate the model"
    );
    let model_ampamp_after_render = model_after.iter().filter(|c| c.contains("&&")).count();

    // When: 렌더한 문서를 에디터 저장 경로로 두 번 저장·재열기한다.
    let save1 = core.export_hwpx_native().expect("save 1");
    let reopened1 = DocumentCore::from_bytes(&save1).expect("reopen save 1");
    let save2 = reopened1.export_hwpx_native().expect("save 2");
    let reopened2 = DocumentCore::from_bytes(&save2).expect("reopen save 2");
    assert_eq!(
        model_form_captions(reopened2.document()),
        expected_model,
        "reopen 2: model captions"
    );

    // Then: 저장 XML 의 caption 원문은 원본과 같고 `&amp;amp;` 는 생기지 않는다.
    let xml1 = hwpx_section_xml(&save1);
    let xml2 = hwpx_section_xml(&save2);
    let equal = |xml: &str| {
        let raw = raw_form_captions(xml);
        assert_eq!(raw.len(), original_raw.len(), "saved caption count");
        raw.iter()
            .zip(&original_raw)
            .filter(|(saved, original)| saved == original)
            .count()
    };
    let raw_equal = equal(&xml1).min(equal(&xml2));
    let save1_nested = xml1.matches("&amp;amp;").count();
    let save2_nested = xml2.matches("&amp;amp;").count();
    let save2_form_attrs_equal_save1 = raw_form_elements(&xml2) == raw_form_elements(&xml1);
    println!(
        "CAPTION_MODEL_SAVE model_ampamp_after_render={} save1_nested={} save2_nested={} raw_caption_equal_original={}/{} save2_form_attrs_equal_save1={}",
        model_ampamp_after_render,
        save1_nested,
        save2_nested,
        raw_equal,
        original_raw.len(),
        save2_form_attrs_equal_save1
    );

    assert_eq!(ampamp.len(), 15, "fixture captions carrying `&&`");
    assert_eq!(model_ampamp_after_render, 15);
    assert_eq!((save1_nested, save2_nested), (0, 0), "nested escapes");
    assert_eq!(raw_equal, original_raw.len(), "saved raw captions");
    assert!(save2_form_attrs_equal_save1, "save 2 form elements");
    assert_eq!(rendered_unfolded, 0, "unfolded `&&` captions drawn");
    assert_eq!(rendered_folded, ampamp.len(), "folded `&&` captions drawn");
    assert!(ip_rnd_display > 0, "`IP R&D연계` is displayed");
}
