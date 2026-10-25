//! HWPX 표 충실도 회귀 시험 (v353) — pageBreak 두 경로 보존, 편집 뒤 표시 높이, 폭 공식.
//!
//! 표본은 `tests/fixtures/table-fidelity/` 에 있다(출처·해시는 그 폴더 README).
//! 기대값은 원본 XML 이나 편집 전 원본 IR 에서 낸다. 직렬화기가 만든 값으로 기대값을 만들지 않는다.
//!
//! 이 파일은 v353 이전 커밋(a34d88ff)에도 있는 공개 API 만 쓴다. 음성 대조(게이트 G7)에서
//! 같은 시험을 옛 코드에 그대로 돌려 결함을 실제로 잡는지 보기 위해서다.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::model::document::Document;
use rhwp::model::table::Table;

const T1: &[u8] = include_bytes!("fixtures/table-fidelity/T1_press_release_cell_logo.hwpx");
const T2: &[u8] = include_bytes!("fixtures/table-fidelity/T2_2026_oss_rst.hwpx");
const T6: &[u8] = include_bytes!("fixtures/table-fidelity/T6_table_giant_cell_overfill.hwpx");

type Attrs = BTreeMap<String, String>;

/// HWPX 구역 XML 에서 읽은 `<hp:tbl>` 하나.
#[derive(Debug, Clone)]
struct TblXml {
    /// 바깥 표 안에 들어 있는 깊이 (본문 표 = 0)
    depth: usize,
    tbl: Attrs,
    sz: Attrs,
    /// 첫 행 셀들의 `cellSz@width` (열 순서)
    first_row_cell_widths: Vec<u32>,
    /// 모든 셀이 colSpan=rowSpan=1 인지
    unmerged: bool,
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

/// 모든 구역의 `<hp:tbl>` 을 문서 순서(중첩 표 포함, 전위)로 읽는다.
fn tables_xml(hwpx: &[u8]) -> Vec<TblXml> {
    /// 열린 표 하나의 읽기 상태. 셀 안 중첩 표가 끝난 뒤에도 바깥 셀 상태가 남도록 표마다 둔다.
    struct Open {
        idx: usize,
        /// 지금까지 연 `hp:tr` 수 (첫 행 = 1)
        row: usize,
        /// 이 표의 `hp:tc` 안인지
        in_tc: bool,
    }
    let mut out: Vec<TblXml> = Vec::new();
    for xml in section_xmls(hwpx) {
        let mut reader = Reader::from_str(&xml);
        let mut open: Vec<Open> = Vec::new();
        loop {
            match reader.read_event().expect("well-formed section XML") {
                Event::Start(e) => match local(e.name().as_ref()) {
                    b"tbl" => {
                        out.push(TblXml {
                            depth: open.len(),
                            tbl: attrs_of(&e),
                            sz: Attrs::new(),
                            first_row_cell_widths: Vec::new(),
                            unmerged: true,
                        });
                        open.push(Open {
                            idx: out.len() - 1,
                            row: 0,
                            in_tc: false,
                        });
                    }
                    b"tr" => {
                        if let Some(top) = open.last_mut() {
                            top.row += 1;
                        }
                    }
                    b"tc" => {
                        if let Some(top) = open.last_mut() {
                            top.in_tc = true;
                        }
                    }
                    _ => {}
                },
                Event::Empty(e) => {
                    let Some(top) = open.last() else {
                        continue;
                    };
                    let idx = top.idx;
                    match local(e.name().as_ref()) {
                        b"sz" if out[idx].sz.is_empty() => out[idx].sz = attrs_of(&e),
                        b"cellSpan" if top.in_tc => {
                            let a = attrs_of(&e);
                            if a.get("colSpan").map(String::as_str) != Some("1")
                                || a.get("rowSpan").map(String::as_str) != Some("1")
                            {
                                out[idx].unmerged = false;
                            }
                        }
                        b"cellSz" if top.in_tc && top.row == 1 => {
                            let w = attrs_of(&e)["width"].parse().expect("cellSz width");
                            out[idx].first_row_cell_widths.push(w);
                        }
                        _ => {}
                    }
                }
                Event::End(e) => match local(e.name().as_ref()) {
                    b"tbl" => {
                        open.pop();
                    }
                    b"tc" => {
                        if let Some(top) = open.last_mut() {
                            top.in_tc = false;
                        }
                    }
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
    }
    out
}

fn page_breaks(hwpx: &[u8]) -> Vec<String> {
    tables_xml(hwpx)
        .into_iter()
        .map(|t| t.tbl.get("pageBreak").cloned().unwrap_or_default())
        .collect()
}

/// 본문(최상위) 표의 (구역, 문단, 컨트롤) 위치를 문서 순서로.
fn top_level_tables(doc: &Document) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (si, section) in doc.sections.iter().enumerate() {
        for (pi, para) in section.paragraphs.iter().enumerate() {
            for (ci, ctrl) in para.controls.iter().enumerate() {
                if matches!(ctrl, Control::Table(_)) {
                    out.push((si, pi, ci));
                }
            }
        }
    }
    out
}

fn table_at(doc: &Document, (si, pi, ci): (usize, usize, usize)) -> &Table {
    match &doc.sections[si].paragraphs[pi].controls[ci] {
        Control::Table(t) => t,
        _ => panic!("({si},{pi},{ci}) 는 표가 아니다"),
    }
}

/// HWPX 를 다시 내보낸 결과에서 n번째 본문 표의 `sz` 를 읽는다.
fn exported_top_level_sz(hwpx: &[u8], nth: usize) -> Attrs {
    tables_xml(hwpx)
        .into_iter()
        .filter(|t| t.depth == 0)
        .nth(nth)
        .unwrap_or_else(|| panic!("본문 표 #{nth} 없음"))
        .sz
}

/// 표시 기준 행 높이 — 저장 행 높이를 표시 높이에 비례 배분하고, 반올림 오차는 마지막 행이 흡수한다.
/// 표시 높이가 셀 합 이하면 저장 행 높이 그대로다 (패킷 §3.2, 업스트림 1a76570e 와 같은 규칙).
fn display_row_heights(rows: &[u32], display: u32) -> Vec<u32> {
    let raw_sum: u64 = rows.iter().map(|h| *h as u64).sum();
    let target = display as u64;
    if rows.is_empty() || raw_sum == 0 || target <= raw_sum {
        return rows.to_vec();
    }
    let mut out: Vec<u32> = rows
        .iter()
        .map(|h| (((*h as u64 * target) + raw_sum / 2) / raw_sum).max(1) as u32)
        .collect();
    let scaled: u64 = out.iter().map(|h| *h as u64).sum();
    let last = out.last_mut().unwrap();
    if target > scaled {
        *last += (target - scaled) as u32;
    } else if scaled > target {
        *last = last.saturating_sub((scaled - target) as u32).max(1);
    }
    out
}

fn sum(rows: &[u32]) -> u32 {
    rows.iter().sum()
}

/// HWPX ZIP 안 구역 XML 하나를 바꿔 다시 묶는다.
fn rezip_with_section(hwpx: &[u8], section: &str, xml: &str) -> Vec<u8> {
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).expect("HWPX ZIP");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("ZIP entry");
        let name = entry.name().expect("ZIP entry name").to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).expect("ZIP entry bytes");
        let method = if name == "mimetype" {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        writer
            .start_file(&name, SimpleFileOptions::default().compression_method(method))
            .expect("start_file");
        if name == section {
            writer.write_all(xml.as_bytes()).expect("write section");
        } else {
            writer.write_all(&data).expect("write entry");
        }
    }
    writer.finish().expect("finish ZIP").into_inner()
}

/// HWPX → IR → HWPX (에디터의 열기 → 저장).
fn hwpx_ir_hwpx(hwpx: &[u8]) -> Vec<u8> {
    DocumentCore::from_bytes(hwpx)
        .expect("HWPX 로드")
        .export_hwpx_native()
        .expect("HWPX 저장")
}

/// HWPX → HWP5 → HWPX (HWP 로 저장한 뒤 그 HWP 를 다시 HWPX 로).
fn hwpx_hwp5_hwpx(hwpx: &[u8]) -> Vec<u8> {
    let mut core = DocumentCore::from_bytes(hwpx).expect("HWPX 로드");
    let hwp = core.export_hwp_with_adapter().expect("HWP5 저장");
    DocumentCore::from_bytes(&hwp)
        .expect("HWP5 재로드")
        .export_hwpx_native()
        .expect("HWPX 저장")
}

// ---------- pageBreak (게이트 G4) ----------
//
// 한컴은 HWPX TABLE·CELL·NONE 을 HWP5 값 1·2·0 으로 저장한다(같은 이름 HWP/HWPX 표본 쌍 대조).
// TABLE·CELL·NONE 은 두 경로에서 원 문자열을 보존해야 한다. ROW 는 한컴 값이 아닌 별칭이라
// RowBreak 로 읽고 CELL 로 정규화한다(Parent 처분 2 — HWP5 2비트로는 ROW 를 따로 담을 수 없다).

fn assert_has_all_page_break_values(original: &[String], name: &str) {
    for value in ["TABLE", "CELL", "NONE"] {
        assert!(
            original.iter().any(|v| v == value),
            "{name} 표본 전제: pageBreak={value} 표가 있어야 한다 ({original:?})"
        );
    }
}

#[test]
fn page_break_strings_survive_hwpx_ir_hwpx() {
    for (name, hwpx) in [("T2", T2), ("T6", T6)] {
        let original = page_breaks(hwpx);
        assert_has_all_page_break_values(&original, name);
        assert_eq!(
            page_breaks(&hwpx_ir_hwpx(hwpx)),
            original,
            "{name}: HWPX→IR→HWPX 에서 pageBreak 가 바뀌었다"
        );
    }
}

#[test]
fn page_break_strings_survive_hwpx_hwp5_hwpx() {
    // T2 만 쓴다. T6 은 HWP5 를 거치면 그림 BinData 가 빠져 마지막 HWPX 저장이 실패한다
    // ("binaryItemIDRef 미등록 bin_data_id=1") — 그림 경로(Phase 4) 결함이라 이 레인 밖이다.
    let original = page_breaks(T2);
    assert_has_all_page_break_values(&original, "T2");
    assert_eq!(
        page_breaks(&hwpx_hwp5_hwpx(T2)),
        original,
        "T2: HWPX→HWP5→HWPX 에서 pageBreak 가 바뀌었다"
    );
}

#[test]
fn page_break_writes_hancom_hwp5_bits() {
    // HWP5 HWPTAG_TABLE 속성 bit 0-1: TABLE=1, CELL=2, NONE=0.
    let expected: Vec<u32> = page_breaks(T2)
        .iter()
        .map(|v| match v.as_str() {
            "TABLE" => 1,
            "CELL" => 2,
            _ => 0,
        })
        .collect();
    let mut core = DocumentCore::from_bytes(T2).expect("HWPX 로드");
    let hwp = core.export_hwp_with_adapter().expect("HWP5 저장");
    let doc = rhwp::parser::parse_document(&hwp).expect("HWP5 파싱");
    let mut bits = Vec::new();
    fn walk(paras: &[rhwp::model::paragraph::Paragraph], out: &mut Vec<u32>) {
        for p in paras {
            for c in &p.controls {
                if let Control::Table(t) = c {
                    out.push(t.raw_table_record_attr & 0x03);
                    for cell in &t.cells {
                        walk(&cell.paragraphs, out);
                    }
                }
            }
        }
    }
    for section in &doc.sections {
        walk(&section.paragraphs, &mut bits);
    }
    assert_eq!(bits, expected, "HWP5 TABLE 속성 bit 0-1");
}

#[test]
fn page_break_row_alias_normalizes_to_cell_in_both_paths() {
    let section = section_xmls(T2).remove(0);
    let at = section
        .find(r#"pageBreak="CELL""#)
        .expect("T2 구역 0 에 pageBreak=CELL 표가 있어야 한다");
    let mut edited = section.clone();
    edited.replace_range(at..at + r#"pageBreak="CELL""#.len(), r#"pageBreak="ROW""#);
    let hwpx = rezip_with_section(T2, "Contents/section0.xml", &edited);
    let with_row = page_breaks(&hwpx);
    assert!(with_row.iter().any(|v| v == "ROW"), "ROW 를 심지 못했다");
    let normalized: Vec<String> = with_row
        .iter()
        .map(|v| if v == "ROW" { "CELL".to_string() } else { v.clone() })
        .collect();

    assert_eq!(page_breaks(&hwpx_ir_hwpx(&hwpx)), normalized, "HWPX→IR→HWPX");
    assert_eq!(page_breaks(&hwpx_hwp5_hwpx(&hwpx)), normalized, "HWPX→HWP5→HWPX");
}

// ---------- noAdjust 와 HWP5 ----------
//
// noAdjust 는 HWP5 HWPTAG_TABLE 속성 bit 3 이다(같은 이름 HWP/HWPX 표본 쌍 1,020표에서 1:1).

fn no_adjust_flags(hwpx: &[u8]) -> Vec<String> {
    tables_xml(hwpx)
        .into_iter()
        .map(|t| t.tbl.get("noAdjust").cloned().unwrap_or_default())
        .collect()
}

#[test]
fn no_adjust_survives_hwpx_hwp5_hwpx() {
    let original = no_adjust_flags(T2);
    assert!(
        original.iter().any(|v| v == "1"),
        "표본 전제: noAdjust=1 표가 있어야 한다"
    );
    assert_eq!(no_adjust_flags(&hwpx_hwp5_hwpx(T2)), original);
}

#[test]
fn no_adjust_table_pagebreak_edit_reaches_hwp5() {
    // noAdjust 표의 쪽 나눔을 표 속성에서 바꾼 뒤 HWP5 로 저장하면 바꾼 값이 나가야 한다.
    let xml_tops: Vec<TblXml> = tables_xml(T2).into_iter().filter(|t| t.depth == 0).collect();
    let k = xml_tops
        .iter()
        .position(|t| {
            t.tbl.get("noAdjust").map(String::as_str) == Some("1")
                && t.tbl.get("pageBreak").map(String::as_str) != Some("NONE")
        })
        .expect("표본 전제: 쪽 나눔이 있는 noAdjust=1 본문 표");

    let mut doc = rhwp::wasm_api::HwpDocument::from_bytes(T2).expect("T2 로드");
    let (si, pi, ci) = top_level_tables(doc.document())[k];
    doc.set_table_properties(si as u32, pi as u32, ci as u32, r#"{"pageBreak":0}"#)
        .expect("표 속성 설정");
    let hwp = doc.export_hwp_with_adapter().expect("HWP5 저장");

    let reloaded = DocumentCore::from_bytes(&hwp).expect("HWP5 재로드");
    // HWP5 저장 어댑터가 첫 문단에 구역 정의 컨트롤을 넣으므로 위치 대신 본문 표 순서로 찾는다.
    let loc = top_level_tables(reloaded.document())[k];
    let table = table_at(reloaded.document(), loc);
    assert_eq!(
        table.page_break,
        rhwp::model::table::TablePageBreak::None,
        "바꾼 쪽 나눔이 HWP5 에 실려야 한다 (raw attr=0x{:08x})",
        table.raw_table_record_attr
    );
    assert_ne!(table.raw_table_record_attr & 0x08, 0, "noAdjust 는 그대로");
}

// ---------- 표시 높이 (게이트 G5) ----------
//
// T6 첫 본문 표는 5행×1열, 표시 높이 73450 · 셀 높이 합 5830 이다(원본 XML).
// 행 삽입: 편집 전 표시 높이 + 새 행의 표시 기준 높이. 행 삭제: max(편집 전 − 삭제 행의 표시
// 기준 높이, 셀 합). 행 수가 변하지 않는 편집: 셀 합이 표시 높이를 넘을 때만 늘린다.

struct Before {
    loc: (usize, usize, usize),
    display: u32,
    rows: Vec<u32>,
}

/// `hwpx` 의 n번째 본문 표를 열고 편집 전 표시 높이·행 높이를 잰다.
fn open_table(hwpx: &[u8], nth: usize) -> (DocumentCore, Before) {
    let core = DocumentCore::from_bytes(hwpx).expect("HWPX 로드");
    let loc = top_level_tables(core.document())[nth];
    let table = table_at(core.document(), loc);
    let display = table.common.height;
    let rows = table.get_row_heights();
    // 원본 XML 의 표시 높이와 IR 이 같은지 먼저 본다.
    let xml_display: u32 = exported_top_level_sz(hwpx, nth)["height"].parse().unwrap();
    assert_eq!(display, xml_display, "본문 표 #{nth} 표시 높이 파싱");
    assert!(
        display > sum(&rows),
        "표본 전제: 표시 높이({display}) > 셀 합({})",
        sum(&rows)
    );
    (core, Before { loc, display, rows })
}

fn t6_table(nth: usize) -> (DocumentCore, Before) {
    open_table(T6, nth)
}

fn assert_display_height(core: &DocumentCore, before: &Before, nth: usize, expected: u32) {
    let table = table_at(core.document(), before.loc);
    assert_eq!(table.common.height, expected, "IR 표시 높이");
    let saved = core.export_hwpx_native().expect("HWPX 저장");
    let sz = exported_top_level_sz(&saved, nth);
    assert_eq!(sz["height"], expected.to_string(), "저장된 hp:sz@height");
}

#[test]
fn display_height_grows_by_display_row_height_on_row_insert() {
    let (mut core, before) = t6_table(0);
    let (si, pi, ci) = before.loc;
    let display_rows = display_row_heights(&before.rows, before.display);

    core.insert_table_row_native(si, pi, ci, 0, true).expect("행 삽입");

    let after_rows = table_at(core.document(), before.loc).get_row_heights();
    let expected = (before.display + display_rows[0]).max(sum(&after_rows));
    assert!(expected > before.display, "행 삽입 뒤 표시 높이는 늘어야 한다");
    assert_display_height(&core, &before, 0, expected);
}

#[test]
fn display_height_shrinks_by_display_row_height_on_row_delete() {
    let (mut core, before) = t6_table(0);
    let (si, pi, ci) = before.loc;
    let display_rows = display_row_heights(&before.rows, before.display);

    core.delete_table_row_native(si, pi, ci, 1).expect("행 삭제");

    let after_rows = table_at(core.document(), before.loc).get_row_heights();
    let expected = before
        .display
        .saturating_sub(display_rows[1])
        .max(sum(&after_rows));
    assert!(
        expected > sum(&after_rows),
        "삭제 뒤에도 표시 높이({expected})는 셀 합({})보다 커야 이 시험이 뜻을 가진다",
        sum(&after_rows)
    );
    assert_display_height(&core, &before, 0, expected);
}

#[test]
fn display_height_follows_text_edit_rule_on_overfilled_cell() {
    // T6 두 번째 본문 표는 1×1, 표시 높이 74182 · 셀 높이 36862 이고 셀 내용이 셀보다 크다
    // (표본 이름 그대로 "giant cell overfill"). 텍스트를 넣으면 셀이 내용 높이까지 자라
    // 셀 합이 표시 높이를 넘으므로, 규칙대로 표시 높이도 셀 합까지 늘어야 한다.
    let (mut core, before) = t6_table(1);
    let (si, pi, ci) = before.loc;

    core.insert_text_in_cell_native(si, pi, ci, 0, 0, 0, "가")
        .expect("셀 텍스트 삽입");

    let after_rows = table_at(core.document(), before.loc).get_row_heights();
    assert!(
        sum(&after_rows) > sum(&before.rows),
        "표본 전제: 편집이 셀 높이를 늘려야 한다 ({} → {})",
        sum(&before.rows),
        sum(&after_rows)
    );
    assert_display_height(&core, &before, 1, before.display.max(sum(&after_rows)));
}

#[test]
fn display_height_survives_cell_text_edit() {
    // T1 본문 표 #58 은 2×2, 표시 높이 14901 · 셀 높이 합 2713 이다(원본 XML).
    // 셀 1 에 20자를 넣으면 행이 자라지만 셀 합은 표시 높이보다 작다 → 표시 높이는 그대로다.
    let (mut core, before) = open_table(T1, 58);
    let (si, pi, ci) = before.loc;

    core.insert_text_in_cell_native(si, pi, ci, 1, 0, 0, &"가".repeat(20))
        .expect("셀 텍스트 삽입");

    let after_rows = table_at(core.document(), before.loc).get_row_heights();
    assert!(
        sum(&after_rows) > sum(&before.rows),
        "표본 전제: 편집이 셀 높이를 늘려야 한다 ({} → {})",
        sum(&before.rows),
        sum(&after_rows)
    );
    assert!(
        sum(&after_rows) < before.display,
        "표본 전제: 늘어난 셀 합({})이 표시 높이({})보다 작아야 한다",
        sum(&after_rows),
        before.display
    );
    assert_display_height(&core, &before, 58, before.display);
}

#[test]
fn display_height_survives_column_insert() {
    let (mut core, before) = t6_table(0);
    let (si, pi, ci) = before.loc;

    core.insert_table_column_native(si, pi, ci, 0, true)
        .expect("열 삽입");

    let after_rows = table_at(core.document(), before.loc).get_row_heights();
    assert_display_height(&core, &before, 0, before.display.max(sum(&after_rows)));
}

// ---------- 폭 공식 (게이트 G6) ----------
//
// hp:sz@width == Σ열 폭 + (colCnt + 1) × cellSpacing (병합 없는 표본 3,175표 중 3,174표).

fn assert_width_formula(t: &TblXml, label: &str) {
    let spacing: u32 = t.tbl["cellSpacing"].parse().unwrap();
    let cols: u32 = t.tbl["colCnt"].parse().unwrap();
    assert_eq!(t.first_row_cell_widths.len() as u32, cols, "{label}: 첫 행 셀 수");
    let expected = sum(&t.first_row_cell_widths) + (cols + 1) * spacing;
    assert_eq!(t.sz["width"], expected.to_string(), "{label}: sz.width");
}

/// T6 첫 본문 표(병합 없는 5×1)에 셀 간격을 주고, 그 상태의 선언 폭을 공식대로 맞춘 문서.
fn t6_with_cell_spacing(spacing: i16) -> (DocumentCore, (usize, usize, usize)) {
    let mut core = DocumentCore::from_bytes(T6).expect("T6 로드");
    let mut doc = core.document().clone();
    let loc = top_level_tables(&doc)[0];
    let (si, pi, ci) = loc;
    let Control::Table(table) = &mut doc.sections[si].paragraphs[pi].controls[ci] else {
        panic!("표가 아니다");
    };
    assert!(
        table.cells.iter().all(|c| c.col_span == 1 && c.row_span == 1),
        "표본 전제: 병합 없는 표"
    );
    table.cell_spacing = spacing;
    let columns: u32 = table.get_column_widths().iter().sum();
    table.common.width = columns + (table.col_count as u32 + 1) * spacing as u32;
    core.set_document(doc);
    (core, loc)
}

#[test]
fn width_formula_holds_after_column_insert_with_cell_spacing() {
    let (mut core, (si, pi, ci)) = t6_with_cell_spacing(200);
    core.insert_table_column_native(si, pi, ci, 0, true)
        .expect("열 삽입");
    let saved = core.export_hwpx_native().expect("HWPX 저장");
    let t = tables_xml(&saved).into_iter().find(|t| t.depth == 0).unwrap();
    assert_eq!(t.tbl["cellSpacing"], "200");
    assert_eq!(t.tbl["colCnt"], "2");
    assert_width_formula(&t, "열 삽입 뒤");
}

#[test]
fn width_formula_holds_after_column_delete_with_cell_spacing() {
    let (mut core, (si, pi, ci)) = t6_with_cell_spacing(150);
    core.insert_table_column_native(si, pi, ci, 0, true)
        .expect("열 삽입");
    core.insert_table_column_native(si, pi, ci, 1, true)
        .expect("열 삽입");
    core.delete_table_column_native(si, pi, ci, 0)
        .expect("열 삭제");
    let saved = core.export_hwpx_native().expect("HWPX 저장");
    let t = tables_xml(&saved).into_iter().find(|t| t.depth == 0).unwrap();
    assert_eq!(t.tbl["colCnt"], "2");
    assert_width_formula(&t, "열 삭제 뒤");
}

#[test]
fn width_formula_kept_after_row_insert() {
    let (mut core, (si, pi, ci)) = t6_with_cell_spacing(200);
    core.insert_table_row_native(si, pi, ci, 0, true)
        .expect("행 삽입");
    let saved = core.export_hwpx_native().expect("HWPX 저장");
    let t = tables_xml(&saved).into_iter().find(|t| t.depth == 0).unwrap();
    assert_width_formula(&t, "행 삽입 뒤");
}

// ---------- 한컴 확인용 산출물 (게이트 G12) ----------

/// 한컴에서 열어 볼 편집 전·후 HWPX 를 만든다. 기본 시험에서는 돌지 않는다.
///
/// ```text
/// V353_HANCOM_OUT=<폴더> V353_HANCOM_SET=head|base \
///   cargo test --test hwpx_table_fidelity write_hancom_review_files -- --ignored --nocapture
/// ```
///
/// `head` 는 편집 전 저장본과 편집 뒤 저장본 6개를, `base` 는 같은 행 삽입을 옛 코드로 저장한
/// 비교본 2개(`*_base.hwpx`)를 쓴다. 표마다 저장된 `hp:sz@height` 를 한 줄씩 찍는다.
#[test]
#[ignore]
fn write_hancom_review_files() {
    let out = std::env::var("V353_HANCOM_OUT").expect("V353_HANCOM_OUT");
    let set = std::env::var("V353_HANCOM_SET").expect("V353_HANCOM_SET=head|base");
    std::fs::create_dir_all(&out).expect("출력 폴더");
    let write = |name: &str, nth: usize, bytes: &[u8]| {
        let path = format!("{out}/{name}.hwpx");
        std::fs::write(&path, bytes).expect("산출물 쓰기");
        let sz = exported_top_level_sz(bytes, nth);
        println!("{name}.hwpx 본문 표 #{nth} sz.height={} sz.width={}", sz["height"], sz["width"]);
    };
    let suffix = if set == "base" { "_base" } else { "" };

    // T1 본문 표 #58 (2×2, 표시 높이 14901 · 셀 합 2713): 첫 행 아래에 행 삽입.
    let (mut core, before) = open_table(T1, 58);
    let (si, pi, ci) = before.loc;
    core.insert_table_row_native(si, pi, ci, 0, true).expect("행 삽입");
    write(&format!("T1_after_insert_row{suffix}"), 58, &core.export_hwpx_native().unwrap());

    // T6 본문 표 #0 (5×1, 표시 높이 73450 · 셀 합 5830): 첫 행 아래에 행 삽입.
    let (mut core, before) = open_table(T6, 0);
    let (si, pi, ci) = before.loc;
    core.insert_table_row_native(si, pi, ci, 0, true).expect("행 삽입");
    write(&format!("T6_after_insert_row{suffix}"), 0, &core.export_hwpx_native().unwrap());

    if set == "base" {
        return;
    }
    write("T1_before_edit", 58, &hwpx_ir_hwpx(T1));
    write("T6_before_edit", 0, &hwpx_ir_hwpx(T6));

    // T1 본문 표 #58: 셀 1 에 20자 (행은 자라지만 셀 합 < 표시 높이).
    let (mut core, before) = open_table(T1, 58);
    let (si, pi, ci) = before.loc;
    core.insert_text_in_cell_native(si, pi, ci, 1, 0, 0, &"가".repeat(20))
        .expect("셀 텍스트 삽입");
    write("T1_after_text_edit", 58, &core.export_hwpx_native().unwrap());

    // T6 본문 표 #0: 둘째 행 삭제.
    let (mut core, before) = open_table(T6, 0);
    let (si, pi, ci) = before.loc;
    core.delete_table_row_native(si, pi, ci, 1).expect("행 삭제");
    write("T6_after_delete_row", 0, &core.export_hwpx_native().unwrap());
}

#[test]
fn width_formula_matches_unmerged_fixture_tables() {
    // 원본 표본 자체로 공식을 확인한다: 병합 없는 표의 sz.width == Σ열 폭 + (colCnt+1)×간격.
    let mut checked = 0;
    for (name, hwpx) in [("T2", T2), ("T6", T6)] {
        for (i, t) in tables_xml(hwpx).iter().enumerate() {
            if t.unmerged {
                assert_width_formula(t, &format!("{name} 표 #{i}"));
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "병합 없는 표가 하나도 없다");
}

// ---------- R1 수정 회귀 (C1) ----------

fn irregular_grid_document() -> (rhwp::wasm_api::HwpDocument, (usize, usize, usize)) {
    let mut core = rhwp::wasm_api::HwpDocument::from_bytes(T6).unwrap();
    let mut doc = core.document().clone();
    let loc = top_level_tables(&doc)[0];
    let Control::Table(table) = &mut doc.sections[loc.0].paragraphs[loc.1].controls[loc.2] else {
        panic!("표가 아니다");
    };
    let template = table.cells[0].clone();
    table.cells.clear();
    for (idx, width) in [200, 4800, 4800, 200].into_iter().enumerate() {
        let mut cell = template.clone();
        cell.row = u16::try_from(idx / 2).unwrap();
        cell.col = u16::try_from(idx % 2).unwrap();
        cell.row_span = 1;
        cell.col_span = 1;
        cell.width = width;
        cell.height = 1000;
        table.cells.push(cell);
    }
    table.row_count = 2;
    table.col_count = 2;
    table.row_sizes = vec![2, 2];
    table.cell_spacing = 0;
    table.common.width = 5000;
    table.common.height = 5000;
    table.raw_ctrl_data.clear();
    table.rebuild_grid();
    core.set_document(doc);
    (core, loc)
}

fn assert_fitting_text_edit_height(display: u32, by_path: bool, cell_idx: usize) {
    // Given: 셀 합이 크고 한 글자가 들어간다. path는 px 올림이 없는 75-HU 배수를 쓴다.
    let (mut core, loc) = irregular_grid_document();
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[loc.0].paragraphs[loc.1].controls[loc.2] else {
        panic!("표가 아니다");
    };
    let row_height = if by_path || cell_idx != 0 {
        100_050
    } else {
        100_000
    };
    for cell in &mut table.cells {
        cell.height = row_height;
    }
    table.common.height = display;
    core.set_document(doc);

    // When: 일반 API 또는 cell-path API로 셀 안에 들어가는 한 글자를 삽입한다.
    if by_path {
        core.insert_text_in_cell_by_path(loc.0, loc.1, &[(loc.2, cell_idx, 0)], 0, "가")
            .unwrap();
    } else {
        core.insert_text_in_cell_native(loc.0, loc.1, loc.2, cell_idx, 0, 0, "가")
            .unwrap();
    }

    // Then: 셀 크기와 무관하게 표시 높이는 max(이전 표시 높이, 셀 합)이다.
    let after = table_at(core.document(), loc);
    assert_eq!(after.get_row_heights(), [row_height, row_height]);
    assert_eq!(after.common.height, display.max(2 * row_height));
    assert_eq!(
        after.common.width, 5000,
        "열 폭이 그대로면 선언 폭을 지킨다"
    );
    let saved = core.export_hwpx_native().unwrap();
    assert_eq!(
        exported_top_level_sz(&saved, 0)["height"],
        display.max(2 * row_height).to_string()
    );
}

#[test]
fn display_height_grows_when_fitting_text_exceeds_declared_height() {
    assert_fitting_text_edit_height(1000, false, 0);
}

#[test]
fn display_height_grows_after_reflow_when_fitting_text_exceeds_declaration() {
    assert_fitting_text_edit_height(1000, false, 1);
}

#[test]
fn display_height_grows_by_path_when_fitting_text_exceeds_declared_height() {
    assert_fitting_text_edit_height(1000, true, 0);
}

#[test]
fn display_height_keeps_larger_declaration_when_text_fits() {
    for by_path in [false, true] {
        assert_fitting_text_edit_height(300_000, by_path, 0);
    }
}

fn assert_ineffective_resize_preserves_width(by_path: bool) {
    for updates in [
        r#"[{"cellIdx":0,"widthDelta":-283}]"#,
        r#"[{"cellIdx":99,"widthDelta":500}]"#,
        r#"[{"cellIdx":0,"widthDelta":500},{"cellIdx":0,"widthDelta":-500}]"#,
        r#"[{"cellIdx":0,"widthDelta":500}]"#,
    ] {
        // Given: 행마다 경계가 다르고, 선언 폭 5000은 열별 최대 폭의 합 9600과 다르다.
        let (mut core, loc) = irregular_grid_document();
        let columns = table_at(core.document(), loc).get_column_widths();

        // When: clamp·무효 인덱스·상쇄 또는 열 최대 폭보다 작은 셀만의 크기 변경을 요청한다.
        if by_path {
            let path = format!(
                r#"[{{"controlIndex":{},"cellIndex":0,"cellParaIndex":0}}]"#,
                loc.2
            );
            core.resize_table_cells_by_path(
                u32::try_from(loc.0).unwrap(),
                u32::try_from(loc.1).unwrap(),
                &path,
                updates,
            )
            .unwrap();
        } else {
            core.resize_table_cells(
                u32::try_from(loc.0).unwrap(),
                u32::try_from(loc.1).unwrap(),
                u32::try_from(loc.2).unwrap(),
                updates,
            )
            .unwrap();
        }

        // Then: 실제 열 폭이 그대로면 선언 폭과 저장된 sz.width도 그대로다.
        let after = table_at(core.document(), loc);
        assert_eq!(
            after.get_column_widths(),
            columns,
            "{updates}, by_path={by_path}"
        );
        assert_eq!(after.common.width, 5000, "{updates}, by_path={by_path}");
        let saved = core.export_hwpx_native().unwrap();
        assert_eq!(exported_top_level_sz(&saved, 0)["width"], "5000");
    }
}

#[test]
fn width_formula_preserves_declaration_when_resize_keeps_columns() {
    assert_ineffective_resize_preserves_width(false);
}

#[test]
fn width_formula_preserves_declaration_when_path_resize_keeps_columns() {
    assert_ineffective_resize_preserves_width(true);
}

#[test]
fn width_formula_updates_when_resize_changes_columns() {
    for by_path in [false, true] {
        // Given: 첫 셀의 폭을 늘리면 해당 열의 최대 폭도 4800에서 5200으로 바뀐다.
        let (mut core, loc) = irregular_grid_document();
        let updates = r#"[{"cellIdx":0,"widthDelta":5000}]"#;

        // When: 열 폭을 실제로 바꾸는 resize를 적용한다.
        if by_path {
            let path = format!(
                r#"[{{"controlIndex":{},"cellIndex":0,"cellParaIndex":0}}]"#,
                loc.2
            );
            core.resize_table_cells_by_path(
                u32::try_from(loc.0).unwrap(),
                u32::try_from(loc.1).unwrap(),
                &path,
                updates,
            )
            .unwrap();
        } else {
            core.resize_table_cells(
                u32::try_from(loc.0).unwrap(),
                u32::try_from(loc.1).unwrap(),
                u32::try_from(loc.2).unwrap(),
                updates,
            )
            .unwrap();
        }

        // Then: 바뀐 열 폭의 합을 선언 폭과 HWPX 출력에 반영한다.
        let after = table_at(core.document(), loc);
        assert_eq!(after.get_column_widths(), [5200, 4800]);
        assert_eq!(after.common.width, 10_000);
        let saved = core.export_hwpx_native().unwrap();
        assert_eq!(exported_top_level_sz(&saved, 0)["width"], "10000");
    }
}

#[test]
fn width_formula_includes_new_spacing_after_column_split() {
    // Given: 셀 폭 1000과 간격 100을 포함한 선언 폭 1200의 1×1 표.
    let mut table = Table {
        row_count: 1,
        col_count: 1,
        cell_spacing: 100,
        cells: vec![rhwp::model::table::Cell::new_empty(0, 0, 1000, 1000, 1)],
        ..Default::default()
    };
    table.common.width = 1200;
    table.common.height = 1000;
    table.rebuild_grid();

    // When: 두 열로 나눈다.
    table.split_cell_into(0, 0, 1, 2, false, false).unwrap();

    // Then: 셀 폭 [500,500]과 세 간격을 포함해 선언 폭은 1300이다.
    assert_eq!(table.get_column_widths(), [500, 500]);
    assert_eq!(table.common.width, 1300);
}

#[test]
fn width_formula_updates_when_row_delete_changes_columns() {
    // Given: 첫 행을 삭제하면 둘째 열의 최대 폭이 4800에서 200으로 줄어든다.
    let (core, loc) = irregular_grid_document();
    let mut table = table_at(core.document(), loc).clone();
    table.common.width = 7000;

    // When: 열별 최대 폭을 제공하던 행을 삭제한다.
    table.delete_row(0).unwrap();

    // Then: 실제로 바뀐 열 폭의 합 5000으로 갱신한다.
    assert_eq!(table.get_column_widths(), [4800, 200]);
    assert_eq!(table.common.width, 5000);
}

#[test]
fn width_formula_preserves_declaration_when_absolute_widths_are_unchanged() {
    // Given: 모든 행의 열 폭이 같지만 선언 폭은 별도 값이다.
    let (core, loc) = irregular_grid_document();
    let mut table = table_at(core.document(), loc).clone();
    for cell in &mut table.cells {
        cell.width = if cell.col == 0 { 1000 } else { 2000 };
    }
    table.common.width = 7000;

    // When: 기존과 같은 열 폭을 절대값으로 설정한다.
    table.set_column_widths(&[1000, 2000]).unwrap();

    // Then: 실제 열 폭이 바뀌지 않았으므로 선언 폭을 지킨다.
    assert_eq!(table.get_column_widths(), [1000, 2000]);
    assert_eq!(table.common.width, 7000);
}

#[test]
fn width_formula_preserves_declaration_when_row_delete_keeps_columns() {
    // Given: 두 행이 같은 열 폭을 공유하지만 선언 폭은 셀 합과 다르다.
    let (core, loc) = irregular_grid_document();
    let mut table = table_at(core.document(), loc).clone();
    for cell in &mut table.cells {
        cell.width = if cell.col == 0 { 1000 } else { 2000 };
    }
    table.common.width = 7000;

    // When: 열 폭을 바꾸지 않는 행 삭제를 한다.
    table.delete_row(0).unwrap();

    // Then: 기존의 별도 선언 폭을 지킨다.
    assert_eq!(table.get_column_widths(), [1000, 2000]);
    assert_eq!(table.common.width, 7000);
}

#[test]
fn width_formula_preserves_declaration_when_row_insert_keeps_columns() {
    // Given: 열별 최대 폭 합 9600과 다른 선언 폭 5000의 표.
    let (core, loc) = irregular_grid_document();
    let mut table = table_at(core.document(), loc).clone();

    // When: 기존 열 폭을 복제하는 행을 삽입한다.
    table.insert_row(0, true).unwrap();

    // Then: 열 폭과 선언 폭을 그대로 둔다.
    assert_eq!(table.get_column_widths(), [4800, 4800]);
    assert_eq!(table.common.width, 5000);
}

#[test]
fn width_formula_preserves_declaration_when_row_split_keeps_columns() {
    // Given: 열 최대 폭과 다른 별도 선언 폭을 가진 표.
    let (core, loc) = irregular_grid_document();
    let mut table = table_at(core.document(), loc).clone();

    // When: 열 수·폭이 바뀌지 않게 행 방향으로만 분할한다.
    table.split_cell_into(0, 0, 2, 1, false, false).unwrap();

    // Then: 무조건 재계산하지 않고 실제 열 변화가 없음을 반영한다.
    assert_eq!(table.get_column_widths(), [4800, 4800]);
    assert_eq!(table.common.width, 5000);
}

#[test]
fn width_formula_uses_assigned_widths_when_all_columns_are_merged() {
    // Given: 열별 폭 추출은 기본값 1800을 쓰는 병합 셀뿐인 표.
    let mut cell = rhwp::model::table::Cell::new_empty(0, 0, 7000, 1000, 1);
    cell.col_span = 2;
    let mut table = Table {
        row_count: 1,
        col_count: 2,
        cell_spacing: 100,
        cells: vec![cell],
        ..Default::default()
    };
    table.common.width = 7200;
    table.rebuild_grid();

    // When: 절대 열 폭을 새 값으로 설정한다.
    table.set_column_widths(&[1000, 2000]).unwrap();

    // Then: 기본값이 아닌 실제 적용 폭과 새 간격으로 선언 폭을 계산한다.
    assert_eq!(table.cells[0].width, 3100);
    assert_eq!(table.common.width, 3300);
}

// ---------- R2 무변경 텍스트 요청 회귀 (C2) ----------

enum NoopCellEdit {
    InsertEmpty,
    DeleteZero,
    DeleteEnd,
}

fn assert_noop_text_keeps_display_height(by_path: bool, cell_idx: usize, edit: NoopCellEdit) {
    // Given: 셀 합 200100과 별도 표시 높이 1000을 가진 표. 셀 0은 reflow 우회, 셀 1은 가능하다.
    let (mut core, loc) = irregular_grid_document();
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[loc.0].paragraphs[loc.1].controls[loc.2] else {
        panic!("표가 아니다");
    };
    for cell in &mut table.cells {
        cell.height = 100_050;
    }
    table.common.height = 1000;
    table.dirty = false;
    core.set_document(doc);
    let before = table_at(core.document(), loc).clone();
    let text = &before.cells[cell_idx].paragraphs[0].text;
    let saved_before = core.export_hwpx_native().unwrap();

    // When: 빈 삽입·0개 삭제·문단 끝 삭제 중 하나를 일반 또는 path API로 요청한다.
    let path = [(loc.2, cell_idx, 0)];
    match edit {
        NoopCellEdit::InsertEmpty => {
            if by_path {
                core.insert_text_in_cell_by_path(loc.0, loc.1, &path, 0, "")
                    .unwrap();
            } else {
                core.insert_text_in_cell_native(loc.0, loc.1, loc.2, cell_idx, 0, 0, "")
                    .unwrap();
            }
        }
        NoopCellEdit::DeleteZero | NoopCellEdit::DeleteEnd => {
            let (offset, count) = match edit {
                NoopCellEdit::DeleteZero => (0, 0),
                NoopCellEdit::DeleteEnd => (text.chars().count(), 1),
                NoopCellEdit::InsertEmpty => unreachable!(),
            };
            if by_path {
                core.delete_text_in_cell_by_path(loc.0, loc.1, &path, offset, count)
                    .unwrap();
            } else {
                core.delete_text_in_cell_native(loc.0, loc.1, loc.2, cell_idx, 0, offset, count)
                    .unwrap();
            }
        }
    }

    // Then: 텍스트·행 높이뿐 아니라 표시 높이와 실제 HWPX 저장 바이트도 그대로다.
    let after = table_at(core.document(), loc);
    assert_eq!(after.cells[cell_idx].paragraphs[0].text, *text);
    assert_eq!(after.get_row_heights(), before.get_row_heights());
    assert_eq!(after.common.height, 1000);
    assert_eq!(after.dirty, before.dirty);
    let saved_after = core.export_hwpx_native().unwrap();
    assert_eq!(exported_top_level_sz(&saved_after, 0)["height"], "1000");
    assert!(saved_after == saved_before, "무변경 요청은 HWPX 바이트를 보존해야 한다");
}

#[test]
fn display_height_noop_insert_without_reflow() {
    assert_noop_text_keeps_display_height(false, 0, NoopCellEdit::InsertEmpty);
}

#[test]
fn display_height_noop_insert_with_reflow() {
    assert_noop_text_keeps_display_height(false, 1, NoopCellEdit::InsertEmpty);
}

#[test]
fn display_height_noop_insert_by_path() {
    assert_noop_text_keeps_display_height(true, 0, NoopCellEdit::InsertEmpty);
}

#[test]
fn display_height_noop_delete_zero_without_reflow() {
    assert_noop_text_keeps_display_height(false, 0, NoopCellEdit::DeleteZero);
}

#[test]
fn display_height_noop_delete_zero_with_reflow() {
    assert_noop_text_keeps_display_height(false, 1, NoopCellEdit::DeleteZero);
}

#[test]
fn display_height_noop_delete_zero_by_path() {
    assert_noop_text_keeps_display_height(true, 0, NoopCellEdit::DeleteZero);
}

#[test]
fn display_height_noop_delete_end_without_reflow() {
    assert_noop_text_keeps_display_height(false, 0, NoopCellEdit::DeleteEnd);
}

#[test]
fn display_height_noop_delete_end_by_path() {
    assert_noop_text_keeps_display_height(true, 0, NoopCellEdit::DeleteEnd);
}

#[test]
fn display_height_noop_cell_range_delete() {
    for cell_idx in [0, 1] {
        for count in [0, 1] {
            // Given: 실제 셀 텍스트 끝에서 삭제할 문자가 없는 표.
            let (mut core, loc) = irregular_grid_document();
            let mut doc = core.document().clone();
            let Control::Table(table) = &mut doc.sections[loc.0].paragraphs[loc.1].controls[loc.2]
            else {
                panic!("표가 아니다");
            };
            for cell in &mut table.cells {
                cell.height = 100_050;
            }
            table.common.height = 1000;
            table.dirty = false;
            let end = u32::try_from(table.cells[cell_idx].paragraphs[0].text.chars().count()).unwrap();
            core.set_document(doc);
            let saved_before = core.export_hwpx_native().unwrap();

            // When: 선택 영역 삭제 API로 길이 0 또는 텍스트 끝 뒤의 범위를 삭제한다.
            core.delete_range_in_cell(
                u32::try_from(loc.0).unwrap(),
                u32::try_from(loc.1).unwrap(),
                u32::try_from(loc.2).unwrap(),
                u32::try_from(cell_idx).unwrap(),
                0,
                end,
                0,
                end + count,
            )
            .unwrap();

            // Then: 별도 표시 높이와 HWPX 저장 바이트를 보존한다.
            assert_eq!(table_at(core.document(), loc).common.height, 1000);
            let saved_after = core.export_hwpx_native().unwrap();
            assert!(saved_after == saved_before, "cell={cell_idx}, count={count}");
        }
    }
}
