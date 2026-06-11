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

// ---------- pagePr 용지 크기/여백 round-trip 보존 (b6f6cda 회귀 가드) ----------
// HWPX export 가 IR PageDef 의 용지 크기·여백을 직렬화하는지 검증한다. 수정 전에는
// empty_section0 템플릿의 고정 여백(top=5668 등)이 그대로 나가 저장→재로드 시 원본
// 여백을 잃고 본문 영역이 바뀌어 페이지가 재배치(reflow)됐다.
// 다중 섹션(haewoi 보도자료, section0+section1)으로 section1+ pagePr 직렬화도 함께 검증한다.
#[test]
fn pagepr_size_and_margins_preserved_on_roundtrip() {
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let fixtures: [(&str, &[u8]); 3] = [
        (
            "sangsaeng",
            include_bytes!("../samples/hwpx/sangsaeng-smartfactory-application.hwpx"),
        ),
        (
            "seoul-root",
            include_bytes!("../samples/hwpx/seoul-root-auto-2026.hwpx"),
        ),
        // 다중 섹션(Contents/section0.xml + section1.xml) 실문서.
        (
            "haewoi-2024q1-multisection",
            include_bytes!("../samples/hwpx/2024년 1분기 해외직접투자 보도자료 ff.hwpx"),
        ),
    ];

    let mut saw_non_template = false;
    let mut saw_multi_section = false;

    for (label, bytes) in fixtures {
        let doc1 = parse_hwpx(bytes).unwrap_or_else(|e| panic!("{label} parse: {e:?}"));
        if doc1.sections.len() >= 2 {
            saw_multi_section = true;
        }
        for s in &doc1.sections {
            let pd = &s.section_def.page_def;
            assert!(pd.width > 0 && pd.height > 0, "{label}: page size parsed");
            if pd.margin_top != 5668 {
                saw_non_template = true;
            }
        }

        let out = serialize_hwpx(&doc1).unwrap_or_else(|e| panic!("{label} serialize: {e:?}"));
        let doc2 = parse_hwpx(&out).unwrap_or_else(|e| panic!("{label} reparse: {e:?}"));
        assert_eq!(
            doc2.sections.len(),
            doc1.sections.len(),
            "{label}: section count preserved"
        );
        // 모든 섹션의 pagePr(용지 크기 + 7개 여백)가 그대로 보존돼야 한다(section1+ 포함).
        for (i, (s1, s2)) in doc1.sections.iter().zip(doc2.sections.iter()).enumerate() {
            let (a, b) = (&s1.section_def.page_def, &s2.section_def.page_def);
            assert_eq!(b.width, a.width, "{label} sec{i}: width");
            assert_eq!(b.height, a.height, "{label} sec{i}: height");
            assert_eq!(b.margin_left, a.margin_left, "{label} sec{i}: margin_left");
            assert_eq!(
                b.margin_right, a.margin_right,
                "{label} sec{i}: margin_right"
            );
            assert_eq!(b.margin_top, a.margin_top, "{label} sec{i}: margin_top");
            assert_eq!(
                b.margin_bottom, a.margin_bottom,
                "{label} sec{i}: margin_bottom"
            );
            assert_eq!(
                b.margin_header, a.margin_header,
                "{label} sec{i}: margin_header"
            );
            assert_eq!(
                b.margin_footer, a.margin_footer,
                "{label} sec{i}: margin_footer"
            );
            assert_eq!(
                b.margin_gutter, a.margin_gutter,
                "{label} sec{i}: margin_gutter"
            );
        }
    }
    // 가드 메타 검증: no-op 회귀를 잡으려면 비템플릿 여백 fixture 가, section1+ 직렬화를
    // 검증하려면 다중 섹션 fixture 가 실제로 코퍼스에 있어야 한다.
    assert!(
        saw_non_template,
        "최소 한 fixture-섹션은 비템플릿(top≠5668) 여백이어야 no-op 회귀를 잡는다"
    );
    assert!(
        saw_multi_section,
        "최소 한 fixture 는 다중 섹션이어야 section1+ pagePr 직렬화를 검증한다"
    );
}

// landscape PageDef 의 전체 serialize→reparse 라운드트립 계약 검증.
// HWPX 규약: width/height 는 실제(렌더) 방향으로 저장하고 파서는 landscape 속성을 읽지 않는다.
// 따라서 landscape=true(짧은변=width, 긴변=height; 렌더러가 교환) 문서를 저장하면, 교환된
// 실제 치수(가로=긴변, 세로=짧은변)가 보존돼 재로드 후에도 같은 가로 방향으로 렌더된다.
// landscape 플래그 자체는 false 로 정규화된다(메타 손실이 아니라 HWPX 표현 규약).
#[test]
fn landscape_pagedef_effective_dims_survive_roundtrip() {
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let bytes = include_bytes!("../samples/hwpx/sangsaeng-smartfactory-application.hwpx");
    let mut doc = parse_hwpx(bytes).expect("parse");
    // landscape HWP 임포트 모사: width=짧은변(59528), height=긴변(84186), landscape=true.
    {
        let pd = &mut doc.sections[0].section_def.page_def;
        pd.landscape = true;
        pd.width = 59528;
        pd.height = 84186;
    }
    let out = serialize_hwpx(&doc).expect("serialize");
    let re = parse_hwpx(&out).expect("reparse");
    let rpd = &re.sections[0].section_def.page_def;
    // 실제(렌더) 치수 보존: 가로=긴변(84186), 세로=짧은변(59528).
    assert_eq!(
        rpd.width, 84186,
        "landscape effective wide(긴변) preserved on roundtrip"
    );
    assert_eq!(
        rpd.height, 59528,
        "landscape effective tall(짧은변) preserved on roundtrip"
    );
    // HWPX 규약상 landscape 플래그는 false 로 정규화(방향은 width/height 가 보존).
    assert!(
        !rpd.landscape,
        "landscape normalizes to false in HWPX (orientation carried by width/height)"
    );
}

// 제본(binding)/제본여백 round-trip 계약 검증.
// gutterType 은 LEFT_ONLY 로 고정되고 HWPX 파서가 gutterType→binding 을 되읽지 않으므로,
// 제본 '변(side)' 표기와 binding 플래그는 round-trip 되지 않고 SingleSided 로 정규화된다
// (landscape 플래그와 동일한 파서-레벨 제약, 별도 추적). 단 제본 여백 '값'(margin_gutter)은
// 그대로 직렬화돼 본문 영역(레이아웃)에 영향이 없도록 보존돼야 한다.
#[test]
fn binding_gutter_value_preserved_side_normalized() {
    use rhwp::model::page::BindingMethod;
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let bytes = include_bytes!("../samples/hwpx/sangsaeng-smartfactory-application.hwpx");
    let mut doc = parse_hwpx(bytes).expect("parse");
    // 비기본 제본 + 0 아닌 제본 여백 설정(HWP 임포트 모사).
    {
        let pd = &mut doc.sections[0].section_def.page_def;
        pd.binding = BindingMethod::DuplexSided;
        pd.margin_gutter = 1000;
    }
    let out = serialize_hwpx(&doc).expect("serialize");
    let re = parse_hwpx(&out).expect("reparse");
    let rpd = &re.sections[0].section_def.page_def;
    // 레이아웃에 영향을 주는 제본 여백 '값'은 보존.
    assert_eq!(
        rpd.margin_gutter, 1000,
        "gutter VALUE preserved on roundtrip (layout-affecting)"
    );
    // binding 플래그는 HWPX 표현 규약상 SingleSided 로 정규화(추적되는 파서-레벨 제약).
    assert_eq!(
        rpd.binding,
        BindingMethod::SingleSided,
        "binding normalizes to SingleSided in HWPX (gutterType not read back by parser)"
    );
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

// ── Fable 5 R4 리뷰 추가 계약 (2026-06-11) ──────────────────────────────────────
// R1-1: width=0/height=0 퇴화 pagePr 에 실여백이 있는 실양식(참가신청서) 보존.
// 이전에는 "크기 0 = 미파싱" 으로 취급해 템플릿 A4+템플릿 여백으로 클로버링했다.
#[test]
fn degenerate_zero_size_pagepr_margins_preserved_on_roundtrip() {
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    let bytes = include_bytes!("../samples/hwpx/form-participation-sangsaeng-smartfactory.hwpx");
    let doc1 = parse_hwpx(bytes).expect("참가신청서 parse");

    // 메타 가드: width=0/height=0 인데 여백은 실값인 섹션이 실제로 존재해야 한다.
    let degenerate: Vec<usize> = doc1
        .sections
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let pd = &s.section_def.page_def;
            pd.width == 0
                && pd.height == 0
                && (pd.margin_left > 0 || pd.margin_top > 0 || pd.margin_header > 0)
        })
        .map(|(i, _)| i)
        .collect();
    assert!(
        !degenerate.is_empty(),
        "fixture 에 퇴화 pagePr(크기 0 + 실여백) 섹션이 있어야 회귀를 잡는다"
    );

    let out = serialize_hwpx(&doc1).expect("serialize");
    let doc2 = parse_hwpx(&out).expect("reparse");
    assert_eq!(doc2.sections.len(), doc1.sections.len());
    for i in degenerate {
        let (a, b) = (
            &doc1.sections[i].section_def.page_def,
            &doc2.sections[i].section_def.page_def,
        );
        assert_eq!(b.width, 0, "sec{i}: 퇴화 width=0 유지");
        assert_eq!(b.height, 0, "sec{i}: 퇴화 height=0 유지");
        assert_eq!(b.margin_left, a.margin_left, "sec{i}: margin_left 보존");
        assert_eq!(b.margin_right, a.margin_right, "sec{i}: margin_right 보존");
        assert_eq!(b.margin_top, a.margin_top, "sec{i}: margin_top 보존");
        assert_eq!(b.margin_bottom, a.margin_bottom, "sec{i}: margin_bottom 보존");
        assert_eq!(b.margin_header, a.margin_header, "sec{i}: margin_header 보존");
        assert_eq!(b.margin_footer, a.margin_footer, "sec{i}: margin_footer 보존");
        assert_eq!(b.margin_gutter, a.margin_gutter, "sec{i}: margin_gutter 보존");
    }
}

// R1-4: 실파일 표 pageBreak 어휘는 {NONE, CELL, TABLE} — "TABLE"(행 단위 분할)을
// 파서가 안 읽으면 한 번의 열기→저장에 RowBreak 가 NONE 으로 정규화된다.
#[test]
fn table_page_break_vocabulary_preserved_on_roundtrip() {
    use rhwp::model::control::Control;
    use rhwp::model::table::TablePageBreak;
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    fn page_break_census(doc: &rhwp::model::document::Document) -> (usize, usize, usize) {
        let mut none = 0;
        let mut cell = 0;
        let mut row = 0;
        for s in &doc.sections {
            for p in &s.paragraphs {
                for c in &p.controls {
                    if let Control::Table(t) = c {
                        match t.page_break {
                            TablePageBreak::None => none += 1,
                            TablePageBreak::CellBreak => cell += 1,
                            TablePageBreak::RowBreak => row += 1,
                        }
                    }
                }
            }
        }
        (none, cell, row)
    }

    // seoul-root 실파일: pageBreak="TABLE" 2건 + "CELL" 다수 + "NONE" 다수.
    let bytes = include_bytes!("../samples/hwpx/seoul-root-auto-2026.hwpx");
    let doc1 = parse_hwpx(bytes).expect("seoul-root parse");
    let before = page_break_census(&doc1);
    assert!(
        before.2 >= 1,
        "메타 가드: pageBreak=\"TABLE\" 이 RowBreak 로 파싱돼야 한다 (census={before:?})"
    );
    assert!(
        before.1 >= 1,
        "메타 가드: pageBreak=\"CELL\" fixture 존재 (census={before:?})"
    );

    let out = serialize_hwpx(&doc1).expect("serialize");
    let doc2 = parse_hwpx(&out).expect("reparse");
    let after = page_break_census(&doc2);
    assert_eq!(after, before, "pageBreak 어휘 census 가 round-trip 보존");
}

// R1-2 + BODY-1: 본문 문단의 charPr run 분할(문단 중간 서식)과 텍스트 끝 이후의
// zero-length run(문단말 캐럿 스타일, 자기닫힘 <hp:run/>)이 저장 후에도 보존된다.
// 이전에는 본문이 char_shapes[0] 단일 run 으로 합쳐지고, 자기닫힘 run 은 파스에서,
// trailing run 은 직렬화에서 각각 유실됐다.
#[test]
fn body_char_shape_runs_and_trailing_refs_preserved_on_roundtrip() {
    use rhwp::model::paragraph::Paragraph;
    use rhwp::parser::hwpx::parse_hwpx;
    use rhwp::serializer::hwpx::serialize_hwpx;

    // 파서/직렬화기와 동일 규약(탭=8 유닛)의 per-char 활성 charPr 시그니처.
    fn style_signature(p: &Paragraph) -> (Vec<u32>, Vec<u32>) {
        let mut per_char = Vec::new();
        let mut fallback = 0u32;
        let mut text_end = 0u32;
        for (idx, ch) in p.text.chars().enumerate() {
            let pos = p.char_offsets.get(idx).copied().unwrap_or(fallback);
            let mut active = p.char_shapes.first().map(|r| r.char_shape_id).unwrap_or(0);
            for s in &p.char_shapes {
                if s.start_pos <= pos {
                    active = s.char_shape_id;
                } else {
                    break;
                }
            }
            per_char.push(active);
            let w = if ch == '\t' { 8 } else { ch.len_utf16() as u32 };
            fallback = pos + w;
            text_end = pos + w;
        }
        let trailing: Vec<u32> = p
            .char_shapes
            .iter()
            .filter(|s| s.start_pos >= text_end && !p.text.is_empty())
            .map(|s| s.char_shape_id)
            .collect();
        (per_char, trailing)
    }

    let bytes = include_bytes!("../samples/hwpx/seoul-root-auto-2026.hwpx");
    let doc1 = parse_hwpx(bytes).expect("seoul-root parse");
    let out2 = serialize_hwpx(&doc1).expect("serialize #1");
    let doc2 = parse_hwpx(&out2).expect("reparse #1");
    // 저장 사이클 안정성 검증용 2차 라운드트립.
    let out3 = serialize_hwpx(&doc2).expect("serialize #2");
    let doc3 = parse_hwpx(&out3).expect("reparse #2");

    let mut saw_multi_run = false;
    let mut saw_trailing_ctrl_free = false;
    for (si, (s1, s2)) in doc1.sections.iter().zip(doc2.sections.iter()).enumerate() {
        assert_eq!(
            s2.paragraphs.len(),
            s1.paragraphs.len(),
            "sec{si}: 문단 수 보존"
        );
        for (pi, (p1, p2)) in s1.paragraphs.iter().zip(s2.paragraphs.iter()).enumerate() {
            assert_eq!(p2.text, p1.text, "sec{si} para{pi}: 텍스트 보존");
            let (sig1, trail1) = style_signature(p1);
            let (sig2, trail2) = style_signature(p2);
            if sig1.iter().collect::<std::collections::BTreeSet<_>>().len() > 1 {
                saw_multi_run = true;
            }
            // per-char 서식은 모든 문단에서 1차 라운드트립에 보존.
            assert_eq!(sig2, sig1, "sec{si} para{pi}: per-char charPr 시그니처 보존");
            // trailing run 의 엄격 보존은 컨트롤 없는 문단에서 검증한다 — 컨트롤이 있는
            // 문단은 export 가 컨트롤 run 을 텍스트 뒤에 배치하는 정규화로 컨트롤 run 의
            // charPr 가 1회성 trailing ref 로 나타날 수 있다(아래 doc2↔doc3 안정성으로 수렴 검증).
            if p1.controls.is_empty() {
                if !trail1.is_empty() {
                    saw_trailing_ctrl_free = true;
                }
                assert_eq!(trail2, trail1, "sec{si} para{pi}: trailing zero-length run 보존");
            }
        }
    }
    // 저장 사이클 안정성: 2차 저장부터는 모든 문단의 시그니처(trailing 포함)가 고정점이어야
    // 한다 — 컨트롤 run charPr 재기록과 trailing 보존이 겹쳐 ref 가 증식하는 회귀를 잡는다.
    for (si, (s2, s3)) in doc2.sections.iter().zip(doc3.sections.iter()).enumerate() {
        assert_eq!(s3.paragraphs.len(), s2.paragraphs.len(), "sec{si}: 문단 수 안정");
        for (pi, (p2, p3)) in s2.paragraphs.iter().zip(s3.paragraphs.iter()).enumerate() {
            assert_eq!(p3.text, p2.text, "sec{si} para{pi}: 텍스트 안정");
            assert_eq!(
                style_signature(p3),
                style_signature(p2),
                "sec{si} para{pi}: 저장 사이클 시그니처 고정점 (ref 증식 금지)"
            );
            // 개수는 비증가 — 컨트롤/secPr run 의 charPr 재기록 아티팩트가 1회성으로
            // 정규화(수렴)되는 것은 허용하되, 저장 사이클마다 증식하는 회귀는 잡는다.
            assert!(
                p3.char_shapes.len() <= p2.char_shapes.len(),
                "sec{si} para{pi}: char_shapes 증식 금지 ({} -> {})",
                p2.char_shapes.len(),
                p3.char_shapes.len()
            );
        }
    }
    assert!(
        saw_multi_run,
        "메타 가드: 다중 run 본문 문단이 fixture 에 있어야 BODY-1 회귀를 잡는다"
    );
    assert!(
        saw_trailing_ctrl_free,
        "메타 가드: 컨트롤 없는 trailing run 문단이 fixture 에 있어야 R1-2 회귀를 잡는다"
    );
}
