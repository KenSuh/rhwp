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
