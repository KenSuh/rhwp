//! HWPX 저장 충실도 레인 2 회귀 시험 (v353) — 문단 run·컨트롤 순서, 두 번 저장, 도형·header 잔여 결함.
//!
//! 표본은 `tests/fixtures/save-fidelity-l2/` 에 있다(출처·해시는 그 폴더 README). `hp:line` 은 레인 1 표본
//! T6 으로 잰다. 기대값은 한컴이 쓴 원본(C6·T6) XML 에서 낸다. 직렬화기가 만든 값으로 기대값을 만들지 않는다.
//!
//! 이 파일은 59c2285 에도 있는 공개 API 만 쓴다. 음성 대조(게이트 G5)에서 같은 시험을 옛 코드에
//! 그대로 돌려 결함을 실제로 잡는지 보기 위해서다.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use rhwp::document_core::DocumentCore;
use rhwp::model::control::{Control, Field, FieldType};
use rhwp::model::document::Document;
use rhwp::model::paragraph::{CharShapeRef, FieldRange, Paragraph};
use rhwp::model::shape::ShapeObject;
use rhwp::model::style::{LineSpacingType, ParaShape, TabDef, TabItem};
use rhwp::model::Point;

/// 한컴 원본 — 중소벤처기업부 공고 사업신청서(C6, 한컴 12 저장).
const C6: &[u8] = include_bytes!("fixtures/save-fidelity-l2/C6_hancom_original.hwpx");
/// GearUp 템플릿 — C6 를 G0 시기 rhwp WASM 이 내보낸 파일(표 82). 문단·글자는 C6 와 같다.
const FACF: &[u8] = include_bytes!("fixtures/save-fidelity-l2/FACF_gearup_template.hwpx");
/// 레인 1 표본 T6 — 한컴 `hp:line` 이 든 저장소 표본(출처 `fixtures/table-fidelity/README.md`).
const T6: &[u8] = include_bytes!("fixtures/table-fidelity/T6_table_giant_cell_overfill.hwpx");

const HWP_UNIT_CHAR_NS: &str = "http://www.hancom.co.kr/hwpml/2016/HwpUnitChar";

type Attrs = BTreeMap<String, String>;

fn local(name: &[u8]) -> &[u8] {
    name.rsplit(|b| *b == b':').next().unwrap_or(name)
}

fn name_of(e: &BytesStart) -> String {
    String::from_utf8(local(e.name().as_ref()).to_vec()).expect("utf-8 이름")
}

/// 속성을 원래 이름(접두어 포함) 그대로 담는다.
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

fn zip_text(hwpx: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).expect("HWPX ZIP");
    let mut entry = archive.by_name(name).expect("ZIP 항목");
    let mut text = String::new();
    entry.read_to_string(&mut text).expect("utf-8");
    text
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

/// HWPX → IR → HWPX (에디터의 열기 → 저장).
fn open_save(hwpx: &[u8]) -> Vec<u8> {
    DocumentCore::from_bytes(hwpx)
        .expect("HWPX 로드")
        .export_hwpx_native()
        .expect("HWPX 저장")
}

fn open_doc(hwpx: &[u8]) -> Document {
    DocumentCore::from_bytes(hwpx)
        .expect("HWPX 로드")
        .document()
        .clone()
}

fn replace_zip_text(hwpx: &[u8], name: &str, text: &str) -> Vec<u8> {
    let mut input = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).unwrap();
    let mut output = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        output
            .start_file(entry.name().unwrap(), zip::write::SimpleFileOptions::default())
            .unwrap();
        output
            .write_all(if entry.name().unwrap() == name {
                text.as_bytes()
            } else {
                &bytes
            })
            .unwrap();
    }
    output.finish().unwrap().into_inner()
}

// ---------- 최소 XML 트리 ----------

struct Node {
    name: String,
    attrs: Attrs,
    children: Vec<usize>,
    parent: usize,
}

/// 요소 이름(접두어 뗌)·속성·자식만 담는 시험용 트리. 0번은 문서 뿌리다.
struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    fn parse(xml: &str) -> Tree {
        let mut nodes = vec![Node {
            name: String::new(),
            attrs: Attrs::new(),
            children: Vec::new(),
            parent: 0,
        }];
        let mut open = vec![0usize];
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event().expect("well-formed XML") {
                Event::Start(e) => {
                    let id = Tree::push(&mut nodes, *open.last().unwrap(), &e);
                    open.push(id);
                }
                Event::Empty(e) => {
                    Tree::push(&mut nodes, *open.last().unwrap(), &e);
                }
                Event::End(_) => {
                    open.pop();
                }
                Event::Eof => break,
                _ => {}
            }
        }
        Tree { nodes }
    }

    fn push(nodes: &mut Vec<Node>, parent: usize, e: &BytesStart) -> usize {
        nodes.push(Node {
            name: name_of(e),
            attrs: attrs_of(e),
            children: Vec::new(),
            parent,
        });
        let id = nodes.len() - 1;
        nodes[parent].children.push(id);
        id
    }

    fn name(&self, id: usize) -> &str {
        &self.nodes[id].name
    }

    fn attr(&self, id: usize, key: &str) -> Option<&str> {
        self.nodes[id].attrs.get(key).map(String::as_str)
    }

    fn children_named(&self, id: usize, name: &str) -> Vec<usize> {
        self.nodes[id]
            .children
            .iter()
            .copied()
            .filter(|&c| self.name(c) == name)
            .collect()
    }

    /// `id` 아래(자신 제외) `name` 요소를 문서 순서로 모은다.
    fn descendants_named(&self, id: usize, name: &str) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack: Vec<usize> = self.nodes[id].children.iter().rev().copied().collect();
        while let Some(n) = stack.pop() {
            if self.name(n) == name {
                out.push(n);
            }
            stack.extend(self.nodes[n].children.iter().rev().copied());
        }
        out
    }

    fn child_names(&self, id: usize) -> Vec<String> {
        self.nodes[id]
            .children
            .iter()
            .map(|&c| self.name(c).to_string())
            .collect()
    }

    fn parent_name(&self, id: usize) -> &str {
        self.name(self.nodes[id].parent)
    }
}

// ---------- 문단 원자열 ----------

/// 문단 run 직속 내용 하나. run 경계는 보지 않는다(한컴과 rhwp 는 run 나누는 방식이 다르다).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    /// `<hp:secPr>`
    SecPr,
    /// `<hp:ctrl>` 직속 자식 (colPr·pageNum·fieldBegin·fieldEnd …)
    Ctrl(String),
    /// run 직속 개체 (tbl·pic·rect·container·line·양식 …)
    Obj(String),
    /// 이웃한 `<hp:t>` 글자를 이은 것. 빈 `<hp:t/>` 는 넣지 않는다.
    Text(String),
}

#[derive(Debug, Clone)]
struct ParaAtoms {
    /// 구역 안 최상위 문단 번호 (이 문단을 품은 최상위 문단)
    top: usize,
    /// 문단 중첩 깊이 (본문 = 0, 표 셀 = 1, 셀 안 표의 셀 = 2 …)
    depth: usize,
    atoms: Vec<Atom>,
}

fn push_text(atoms: &mut Vec<Atom>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(Atom::Text(prev)) = atoms.last_mut() {
        prev.push_str(text);
    } else {
        atoms.push(Atom::Text(text.to_string()));
    }
}

/// 모든 구역의 `<hp:p>` 를 전위 순서(표 셀·글상자 문단 포함)로 읽어 run 직속 원자열을 만든다.
fn paragraph_atoms(hwpx: &[u8]) -> Vec<ParaAtoms> {
    let mut out: Vec<ParaAtoms> = Vec::new();
    for xml in section_xmls(hwpx) {
        let mut reader = Reader::from_str(&xml);
        let mut stack: Vec<String> = Vec::new();
        let mut open_paras: Vec<usize> = Vec::new();
        let mut top = 0usize;
        let mut first_top = true;
        // run 직속 `<hp:t>` 를 읽는 중이면 그 글자
        let mut text: Option<String> = None;
        loop {
            let event = reader.read_event().expect("well-formed section XML");
            let (name, is_start, is_empty) = match &event {
                Event::Start(e) => (name_of(e), true, false),
                Event::Empty(e) => (name_of(e), true, true),
                Event::End(e) => (
                    String::from_utf8(local(e.name().as_ref()).to_vec()).unwrap(),
                    false,
                    false,
                ),
                Event::Text(t) => {
                    if let Some(buf) = text.as_mut() {
                        if stack.last().map(String::as_str) == Some("t") {
                            let raw = t.decode().expect("utf-8 text");
                            buf.push_str(&quick_xml::escape::unescape(&raw).expect("text"));
                        }
                    }
                    continue;
                }
                Event::GeneralRef(r) => {
                    if let Some(buf) = text.as_mut() {
                        let reference = format!("&{};", r.decode().expect("utf-8 ref"));
                        buf.push_str(&quick_xml::escape::unescape(&reference).expect("ref"));
                    }
                    continue;
                }
                Event::Eof => break,
                _ => continue,
            };
            if is_start {
                let n = stack.len();
                let parent = n.checked_sub(1).map(|i| stack[i].as_str());
                let grand = n.checked_sub(2).map(|i| stack[i].as_str());
                let great = n.checked_sub(3).map(|i| stack[i].as_str());
                if name == "p" {
                    if open_paras.is_empty() {
                        if first_top {
                            first_top = false;
                        } else {
                            top += 1;
                        }
                    }
                    out.push(ParaAtoms {
                        top,
                        depth: open_paras.len(),
                        atoms: Vec::new(),
                    });
                    open_paras.push(out.len() - 1);
                } else if parent == Some("run") && grand == Some("p") {
                    let atoms = &mut out[*open_paras.last().unwrap()].atoms;
                    match name.as_str() {
                        "t" => {
                            if !is_empty {
                                text = Some(String::new());
                            }
                        }
                        "secPr" => atoms.push(Atom::SecPr),
                        "ctrl" => {}
                        _ => atoms.push(Atom::Obj(name.clone())),
                    }
                } else if parent == Some("ctrl") && grand == Some("run") && great == Some("p") {
                    out[*open_paras.last().unwrap()]
                        .atoms
                        .push(Atom::Ctrl(name.clone()));
                } else if parent == Some("t") {
                    if let Some(buf) = text.as_mut() {
                        match name.as_str() {
                            "tab" => buf.push('\t'),
                            "lineBreak" | "columnBreak" => buf.push('\n'),
                            "nbSpace" => buf.push('\u{00A0}'),
                            "fwSpace" => buf.push('\u{2007}'),
                            _ => {}
                        }
                    }
                }
                if !is_empty {
                    stack.push(name);
                }
            } else {
                stack.pop();
                if name == "p" {
                    open_paras.pop();
                } else if name == "t" {
                    if let Some(buf) = text.take() {
                        push_text(&mut out[*open_paras.last().unwrap()].atoms, &buf);
                    }
                }
            }
        }
    }
    out
}

/// 필드 표지가 아닌 컨트롤·개체 원자인가.
fn is_inline_control(atom: &Atom) -> bool {
    match atom {
        Atom::Obj(_) => true,
        Atom::Ctrl(name) => name != "fieldBegin" && name != "fieldEnd",
        _ => false,
    }
}

/// 원본에서 글자 뒤에 (필드가 아닌) 컨트롤·개체가 오는 문단인가.
///
/// 이 저장소의 HWPX 파서는 필드가 아닌 컨트롤을 모두 char_offsets 앞머리에 모은다
/// (`src/parser/hwpx/section.rs` ctrl_offset). 그래서 IR 이 이런 순서를 담지 못한다 —
/// 레인 2 범위(§3.1) 밖의 알려진 한계다(Parent 처분 1, C6 의 최상위 p195 하나).
fn text_before_inline_control(atoms: &[Atom]) -> bool {
    let first_text = atoms.iter().position(|a| matches!(a, Atom::Text(_)));
    let last_inline = atoms.iter().rposition(is_inline_control);
    matches!((first_text, last_inline), (Some(t), Some(c)) if t < c)
}

/// IR 이 담는 순서 — 필드가 아닌 컨트롤·개체를 문단 앞으로 모은다(서로의 순서는 유지).
fn ir_order(atoms: &[Atom]) -> Vec<Atom> {
    let mut out: Vec<Atom> = atoms
        .iter()
        .filter(|a| is_inline_control(a))
        .cloned()
        .collect();
    for atom in atoms.iter().filter(|a| !is_inline_control(a)) {
        match atom {
            Atom::Text(text) => push_text(&mut out, text),
            other => out.push(other.clone()),
        }
    }
    out
}

fn texts(paras: &[ParaAtoms]) -> Vec<String> {
    paras
        .iter()
        .map(|p| {
            p.atoms
                .iter()
                .filter_map(|a| match a {
                    Atom::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// 저장본의 문단마다 원자열이 원본(한컴 C6)과 같은지 본다.
///
/// IR 이 담지 못하는 원본 문단(`text_before_inline_control`)은 IR 순서를 기대한다. 그런 문단은
/// C6 의 최상위 p195(공백 5개 뒤 글자처럼 취급하는 표) 하나뿐이어야 한다.
fn assert_run_order(original: &[ParaAtoms], saved: &[ParaAtoms], label: &str) {
    assert_eq!(saved.len(), original.len(), "{label}: 문단 수");
    let limited: Vec<usize> = (0..original.len())
        .filter(|&i| text_before_inline_control(&original[i].atoms))
        .collect();
    let limited_desc: Vec<(usize, usize, &Vec<Atom>)> = limited
        .iter()
        .map(|&i| (original[i].top, original[i].depth, &original[i].atoms))
        .collect();
    assert_eq!(
        limited_desc,
        vec![(
            195,
            0,
            &vec![
                Atom::Text("     ".to_string()),
                Atom::Obj("tbl".to_string())
            ]
        )],
        "{label}: IR 이 담지 못하는 원본 문단은 C6 최상위 p195 하나"
    );
    let mut wrong = Vec::new();
    for i in 0..original.len() {
        let expected = if limited.contains(&i) {
            ir_order(&original[i].atoms)
        } else {
            original[i].atoms.clone()
        };
        if saved[i].atoms != expected {
            wrong.push((
                i,
                original[i].top,
                original[i].depth,
                expected,
                &saved[i].atoms,
            ));
        }
    }
    let mut by_depth: BTreeMap<usize, usize> = BTreeMap::new();
    for (_, _, depth, _, _) in &wrong {
        *by_depth.entry(*depth).or_default() += 1;
    }
    assert!(
        wrong.is_empty(),
        "{label}: run·컨트롤 순서가 원본과 다른 문단 {}개 (깊이별 {:?}). 처음 3개 (전위 번호, 최상위 문단, 깊이, 기대, 실제): {:?}",
        wrong.len(),
        by_depth,
        &wrong[..wrong.len().min(3)]
    );
}

// ---------- G2: 문단 run·컨트롤 순서 ----------

#[test]
fn run_order_c6_open_save_matches_hancom_original() {
    // Given: 한컴 원본 C6 의 문단 원자열 (본문·셀·중첩 표 셀)
    let original = paragraph_atoms(C6);
    // When: 에디터처럼 열고 저장한다
    let saved = paragraph_atoms(&open_save(C6));
    // Then: 문단마다 run·컨트롤 순서가 원본과 같다
    assert_run_order(&original, &saved, "C6 열고 저장");
}

#[test]
fn run_order_facf_open_save_restores_hancom_original() {
    // Given: FACF 는 C6 를 rhwp 가 내보낸 GearUp 템플릿이다 — 문단 수와 글자가 C6 와 같다.
    let original = paragraph_atoms(C6);
    let template = paragraph_atoms(FACF);
    assert_eq!(
        texts(&template),
        texts(&original),
        "FACF 와 C6 의 문단별 글자"
    );
    // When: 템플릿을 열고 저장한다
    let saved = paragraph_atoms(&open_save(FACF));
    // Then: 한컴 원본 순서로 돌아온다 (템플릿에 박힌 «글자 먼저» 순서가 남지 않는다)
    assert_run_order(&original, &saved, "FACF 열고 저장");
}

/// C6 IR 에 본문 문단 하나를 덧붙여 저장하고, 그 문단의 원자열을 돌려준다.
fn save_appended_paragraph(para: Paragraph) -> Vec<Atom> {
    let mut doc = open_doc(C6);
    doc.sections[0].paragraphs.push(para);
    let saved = rhwp::serializer::serialize_hwpx(&doc).expect("HWPX 저장");
    paragraph_atoms(&saved)
        .into_iter()
        .rfind(|p| p.depth == 0)
        .expect("덧붙인 문단")
        .atoms
}

/// C6 의 첫 본문 표 컨트롤.
fn first_table_control(doc: &Document) -> Control {
    doc.sections[0]
        .paragraphs
        .iter()
        .flat_map(|p| p.controls.iter())
        .find(|c| matches!(c, Control::Table(_)))
        .expect("본문 표")
        .clone()
}

fn table_paragraph(text: &str, char_offsets: Vec<u32>) -> Paragraph {
    let doc = open_doc(C6);
    Paragraph {
        text: text.to_string(),
        char_offsets,
        char_count: 13,
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        controls: vec![first_table_control(&doc)],
        ..Default::default()
    }
}

#[test]
fn run_order_offsets_gap_places_table_between_text() {
    // Given: HWP5 처럼 글자 사이 8 단위 빈칸에 표가 있는 문단 — "가나" [표] "다라"
    let para = table_paragraph("가나다라", vec![0, 1, 10, 11]);
    // When: 저장한다
    let atoms = save_appended_paragraph(para);
    // Then: 표가 빈칸 자리(글자 사이)에 쓰인다
    assert_eq!(
        atoms,
        vec![
            Atom::Text("가나".to_string()),
            Atom::Obj("tbl".to_string()),
            Atom::Text("다라".to_string()),
        ]
    );
}

#[test]
fn run_order_paragraph_without_offsets_keeps_text_first() {
    // Given: char_offsets 가 없는 합성 문단 (글자 위치를 모른다)
    let para = table_paragraph("가나다라", Vec::new());
    // When: 저장한다
    let atoms = save_appended_paragraph(para);
    // Then: 옛 순서(글자 → 컨트롤)를 지킨다
    assert_eq!(
        atoms,
        vec![
            Atom::Text("가나다라".to_string()),
            Atom::Obj("tbl".to_string()),
        ]
    );
}

#[test]
fn run_order_c1_binary_field_begin_precedes_table_at_same_position() {
    let mut para = table_paragraph("x", vec![16]);
    para.controls.insert(0, Control::Field(Field {
        field_type: FieldType::Hyperlink,
        command: "https://example.org".into(),
        ..Default::default()
    }));
    para.field_ranges.push(FieldRange {
        start_char_idx: 0,
        end_char_idx: 1,
        control_idx: 0,
    });
    assert_eq!(
        save_appended_paragraph(para),
        vec![
            Atom::Ctrl("fieldBegin".into()),
            Atom::Obj("tbl".into()),
            Atom::Text("x".into()),
            Atom::Ctrl("fieldEnd".into()),
        ]
    );
}

#[test]
fn run_order_c1_empty_fields_and_controls_keep_relative_order_two_saves() {
    let paragraph = |inner: &str| format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0">{inner}</hp:run></hp:p>"#);
    let table = |inner: &str| format!(r#"<hp:tbl rowCnt="1" colCnt="1" borderFillIDRef="1"><hp:sz width="10000" height="3000"/><hp:tr><hp:tc borderFillIDRef="1"><hp:subList>{inner}</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="10000" height="3000"/></hp:tc></hp:tr></hp:tbl>"#);
    let begin = |id| format!(r#"<hp:ctrl><hp:fieldBegin id="{id}" type="CLICK_HERE" name="empty" editable="1"/></hp:ctrl>"#);
    let end = |id| format!(r#"<hp:ctrl><hp:fieldEnd beginIDRef="{id}"/></hp:ctrl>"#);
    let (b1, b2, e1, e2) = (begin(1), begin(2), end(1), end(2));
    let object = table(&paragraph("<hp:t>cell</hp:t>"));
    let cases = [
        format!("{b1}{e1}{object}"),
        format!("{b1}{object}{e1}"),
        format!("{object}{b1}{e1}"),
        format!("{b1}{b2}{e2}{object}{e1}"),
        format!("{b1}{object}{b2}{e2}{e1}"),
        format!("{b1}{e1}{object}{b2}{e2}{object}"),
    ];
    let mut failures = Vec::new();
    for (case, raw) in cases.iter().enumerate() {
        let owner = paragraph(raw);
        let body = paragraph("<hp:t>prologue</hp:t>") + &owner
            + &paragraph(&table(&owner)) + &paragraph(&table(&paragraph(&table(&owner))));
        let xml = format!(r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">{body}</hs:sec>"#);
        let input = replace_zip_text(C6, "Contents/section0.xml", &xml);
        let expected: Vec<_> = paragraph_atoms(&input)
            .into_iter()
            .map(|p| p.atoms)
            .collect();
        let mut bytes = input;
        for pass in 1..=2 {
            bytes = open_save(&bytes);
            // The first paragraph additionally receives the serializer's section defaults.
            let actual: Vec<_> = paragraph_atoms(&bytes)
                .into_iter()
                .skip(1)
                .map(|p| p.atoms)
                .collect();
            if actual != expected[1..] {
                failures.push(format!("case {case}, save {pass}: {actual:?} != {:?}", &expected[1..]));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------- G3: 두 번 저장 — 첫 문단 구역 run 구성 ----------

/// 구역0 첫 최상위 `<hp:p>` 의 run 마다 직속 내용 종류 (secPr · ctrl:<자식> · t · t0(빈 t) · 개체 이름).
fn first_paragraph_runs(hwpx: &[u8]) -> Vec<Vec<String>> {
    let tree = Tree::parse(&section_xmls(hwpx)[0]);
    let sec = tree.children_named(0, "sec")[0];
    let first = tree.children_named(sec, "p")[0];
    tree.children_named(first, "run")
        .into_iter()
        .map(|run| {
            let mut kinds = Vec::new();
            for &child in &tree.nodes[run].children {
                match tree.name(child) {
                    "ctrl" => {
                        for &c in &tree.nodes[child].children {
                            kinds.push(format!("ctrl:{}", tree.name(c)));
                        }
                    }
                    "t" => kinds.push("t".to_string()),
                    other => kinds.push(other.to_string()),
                }
            }
            kinds
        })
        .collect()
}

/// 첫 글자 run 앞의 run 들 (구역·단·쪽 번호 정의).
fn leading_control_runs(runs: &[Vec<String>]) -> Vec<Vec<String>> {
    runs.iter()
        .take_while(|run| !run.iter().any(|k| k == "t"))
        .cloned()
        .collect()
}

#[test]
fn two_save_first_paragraph_keeps_hancom_section_runs() {
    // Given: 한컴 원본 첫 문단 — secPr 와 colPr 가 한 run, 그다음 pageNum run, 그다음 글자
    let hancom = leading_control_runs(&first_paragraph_runs(C6));
    assert_eq!(
        hancom,
        vec![
            vec!["secPr".to_string(), "ctrl:colPr".to_string()],
            vec!["ctrl:pageNum".to_string()],
        ],
        "원본 첫 문단 구성"
    );
    for (name, fixture) in [("C6", C6), ("FACF", FACF)] {
        // When: 열고 저장하기를 두 번 한다
        let save1 = open_save(fixture);
        let save2 = open_save(&save1);
        // Then: 두 저장본 모두 원본과 같은 run 구성이고, 첫 문단 XML 이 같다
        for (label, bytes) in [("저장 1", &save1), ("저장 2", &save2)] {
            assert_eq!(
                leading_control_runs(&first_paragraph_runs(bytes)),
                hancom,
                "{name} {label}: 첫 문단 구역 run 구성"
            );
        }
        assert_eq!(
            section_xmls(&save1),
            section_xmls(&save2),
            "{name}: 두 번째 저장이 첫 저장의 구역 XML 을 그대로 다시 쓴다"
        );
    }
}

#[test]
fn two_save_section_def_prefix_keeps_column_in_section_run() {
    // Given: HWP5 처럼 첫 문단 앞머리에 구역 정의 컨트롤이 있는 IR — [구역 정의, 단 정의, 쪽 번호] + 글자.
    // 구역 정의도 8 단위 자리를 차지하므로 글자 위치를 그만큼 민다.
    let mut doc = open_doc(C6);
    let first = &mut doc.sections[0].paragraphs[0];
    first
        .controls
        .insert(0, Control::SectionDef(Box::default()));
    for offset in &mut first.char_offsets {
        *offset += 8;
    }
    for shape in &mut first.char_shapes {
        shape.start_pos += 8;
    }
    first.char_count += 8;
    // When: 저장하고, 그 저장본을 한 번 더 열고 저장한다
    let save1 = rhwp::serializer::serialize_hwpx(&doc).expect("저장");
    let save2 = open_save(&save1);
    // Then: 단 정의는 구역 정의와 같은 run, 쪽 번호는 다음 run 이다 (한컴 원본 구성)
    for (label, bytes) in [("저장 1", &save1), ("저장 2", &save2)] {
        assert_eq!(
            leading_control_runs(&first_paragraph_runs(bytes)),
            leading_control_runs(&first_paragraph_runs(C6)),
            "{label}: 첫 문단 구역 run 구성"
        );
    }
}

// ---------- G4: header·도형 잔여 결함 ----------

/// paraPr 하나의 margin 다섯 값 (intent, left, right, prev, next).
fn margin_values(tree: &Tree, margin: usize) -> Vec<String> {
    ["intent", "left", "right", "prev", "next"]
        .iter()
        .map(|field| {
            let kids = tree.children_named(margin, field);
            assert_eq!(kids.len(), 1, "margin 자식 <{field}>");
            tree.attr(kids[0], "value").expect("value").to_string()
        })
        .collect()
}

/// paraPr 마다 (switch 수, case margin, default margin, case lineSpacing, default lineSpacing).
type ParaSwitch = (usize, Vec<String>, Vec<String>, Attrs, Attrs);

fn para_switches(header_xml: &str) -> Vec<ParaSwitch> {
    let tree = Tree::parse(header_xml);
    tree.descendants_named(0, "paraPr")
        .into_iter()
        .map(|pp| {
            let switches: Vec<usize> = tree
                .children_named(pp, "switch")
                .into_iter()
                .filter(|&s| !tree.descendants_named(s, "margin").is_empty())
                .collect();
            let Some(&switch) = switches.first() else {
                return (0, Vec::new(), Vec::new(), Attrs::new(), Attrs::new());
            };
            let case = tree
                .children_named(switch, "case")
                .into_iter()
                .find(|&c| tree.attr(c, "hp:required-namespace") == Some(HWP_UNIT_CHAR_NS))
                .expect("HwpUnitChar case");
            let default = tree.children_named(switch, "default")[0];
            let branch = |b: usize| {
                let margin = tree.children_named(b, "margin")[0];
                let spacing = tree.children_named(b, "lineSpacing")[0];
                (
                    margin_values(&tree, margin),
                    tree.nodes[spacing].attrs.clone(),
                )
            };
            let (case_margin, case_spacing) = branch(case);
            let (default_margin, default_spacing) = branch(default);
            (
                switches.len(),
                case_margin,
                default_margin,
                case_spacing,
                default_spacing,
            )
        })
        .collect()
}

#[test]
fn residual_header_para_margins_use_hancom_switch() {
    // Given: 한컴 원본은 paraPr 마다 margin·lineSpacing 을 HwpUnitChar case(절반) / default 로 쓴다
    let original_xml = zip_text(C6, "Contents/header.xml");
    let original = para_switches(&original_xml);
    assert_eq!(original.len(), 425, "원본 paraPr 수");
    assert!(
        original.iter().all(|p| p.0 == 1),
        "원본은 paraPr 마다 switch 하나"
    );
    let original_root = Tree::parse(&original_xml);
    let head = original_root.children_named(0, "head")[0];
    assert_eq!(
        original_root.attr(head, "xmlns:hwpunitchar"),
        Some(HWP_UNIT_CHAR_NS),
        "원본 header 뿌리의 HwpUnitChar 이름공간"
    );
    for (name, fixture) in [("C6", C6), ("FACF", FACF)] {
        // When: 열고 저장한다
        let saved_xml = zip_text(&open_save(fixture), "Contents/header.xml");
        // Then: 같은 switch 구조와 값을 쓴다 (평문 default 값을 한컴이 case 로 읽어 두 배가 되는 결함 E·D)
        assert_eq!(
            para_switches(&saved_xml),
            original,
            "{name}: paraPr margin switch"
        );
        let saved_root = Tree::parse(&saved_xml);
        let saved_head = saved_root.children_named(0, "head")[0];
        assert_eq!(
            saved_root.attr(saved_head, "xmlns:hwpunitchar"),
            Some(HWP_UNIT_CHAR_NS),
            "{name}: header 뿌리의 HwpUnitChar 이름공간"
        );
        assert!(
            saved_root
                .descendants_named(0, "paraPr")
                .iter()
                .all(|&pp| saved_root.children_named(pp, "margin").is_empty()),
            "{name}: switch 밖 평문 margin 이 남지 않는다"
        );
    }
}

/// paraPr `id` 의 HwpUnitChar case·default margin 자식 (value, unit) 다섯 개씩.
fn para_margin_branches(
    header_xml: &str,
    id: usize,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let tree = Tree::parse(header_xml);
    let pp = tree
        .descendants_named(0, "paraPr")
        .into_iter()
        .find(|&pp| tree.attr(pp, "id") == Some(id.to_string().as_str()))
        .expect("paraPr");
    let switch = *tree
        .children_named(pp, "switch")
        .first()
        .expect("paraPr margin switch (한컴 원본 표기)");
    let branch = |name: &str| {
        let b = tree
            .children_named(switch, name)
            .into_iter()
            .find(|&b| {
                name == "default" || tree.attr(b, "hp:required-namespace") == Some(HWP_UNIT_CHAR_NS)
            })
            .expect("switch 가지");
        let margin = tree.children_named(b, "margin")[0];
        ["intent", "left", "right", "prev", "next"]
            .iter()
            .map(|f| {
                let n = tree.children_named(margin, f)[0];
                (
                    tree.attr(n, "value").unwrap_or("").to_string(),
                    tree.attr(n, "unit").unwrap_or("").to_string(),
                )
            })
            .collect::<Vec<_>>()
    };
    (branch("case"), branch("default"))
}

#[test]
fn residual_header_odd_even_negative_margins_survive_two_saves() {
    // Given: 홀수·짝수·음수 여백 (IR = HWP5 척도). 한컴은 홀수 값을 case 에 unit=CHAR · (값-1)/2 로 쓴다
    // (`.omo/evidence/v353-rhwp-hwpx-save-l2/char-unit-evidence.json`: 한컴 12.30 저장본 43개 값, 예외 0).
    let margins = [-5409, 3001, 1500, -46, 161];
    let mut doc = open_doc(C6);
    let ps = &mut doc.doc_info.para_shapes[0];
    (
        ps.indent,
        ps.margin_left,
        ps.margin_right,
        ps.spacing_before,
        ps.spacing_after,
    ) = (margins[0], margins[1], margins[2], margins[3], margins[4]);
    let mut bytes = rhwp::serializer::serialize_hwpx(&doc).expect("저장");
    for pass in 1..=2 {
        // Then: 한컴과 같은 switch 표기
        let pair = |v: &str, u: &str| (v.to_string(), u.to_string());
        assert_eq!(
            para_margin_branches(&zip_text(&bytes, "Contents/header.xml"), 0),
            (
                vec![
                    pair("-2705", "CHAR"),
                    pair("1500", "CHAR"),
                    pair("750", "HWPUNIT"),
                    pair("-23", "HWPUNIT"),
                    pair("80", "CHAR"),
                ],
                vec![
                    pair("-5409", "HWPUNIT"),
                    pair("3001", "HWPUNIT"),
                    pair("1500", "HWPUNIT"),
                    pair("-46", "HWPUNIT"),
                    pair("161", "HWPUNIT"),
                ],
            ),
            "저장 {pass}: paraPr 0 margin switch"
        );
        // When: 다시 열고 저장한다
        let core = DocumentCore::from_bytes(&bytes).expect("다시 열기");
        let ps = &core.document().doc_info.para_shapes[0];
        // Then: 값이 정확히 돌아온다
        assert_eq!(
            [
                ps.indent,
                ps.margin_left,
                ps.margin_right,
                ps.spacing_before,
                ps.spacing_after
            ],
            margins,
            "읽기 {pass}: paraPr 0 여백"
        );
        bytes = core.export_hwpx_native().expect("다시 저장");
    }
}

/// tabPr 마다 tabItem 목록 — (case pos, case unit, default pos, type, leader). switch 밖 tabItem 은 case 없이.
fn tab_items(header_xml: &str) -> Vec<Vec<(String, String, String, String, String)>> {
    let tree = Tree::parse(header_xml);
    tree.descendants_named(0, "tabPr")
        .into_iter()
        .map(|tp| {
            let mut items = Vec::new();
            for &child in &tree.nodes[tp].children {
                match tree.name(child) {
                    "switch" => {
                        let case = tree.children_named(child, "case")[0];
                        let default = tree.children_named(child, "default")[0];
                        let c = tree.children_named(case, "tabItem")[0];
                        let d = tree.children_named(default, "tabItem")[0];
                        let get = |n: usize, k: &str| tree.attr(n, k).unwrap_or("").to_string();
                        items.push((
                            get(c, "pos"),
                            get(c, "unit"),
                            get(d, "pos"),
                            get(d, "type"),
                            get(d, "leader"),
                        ));
                    }
                    "tabItem" => {
                        let get = |k: &str| tree.attr(child, k).unwrap_or("").to_string();
                        items.push((
                            String::new(),
                            String::new(),
                            get("pos"),
                            get("type"),
                            get("leader"),
                        ));
                    }
                    _ => {}
                }
            }
            items
        })
        .collect()
}

#[test]
fn residual_header_tab_items_use_hancom_switch() {
    // Given: 한컴 원본은 tabItem 마다 HwpUnitChar case(절반, unit=HWPUNIT) / default switch 를 쓴다
    let original = tab_items(&zip_text(C6, "Contents/header.xml"));
    let total: usize = original.iter().map(Vec::len).sum();
    assert_eq!(total, 38, "원본 tabItem 수");
    assert!(
        original.iter().flatten().all(|t| !t.0.is_empty()),
        "원본 tabItem 은 모두 switch 안"
    );
    for (name, fixture) in [("C6", C6), ("FACF", FACF)] {
        // When: 열고 저장한다
        let saved = tab_items(&zip_text(&open_save(fixture), "Contents/header.xml"));
        // Then: 같은 switch 와 값 (평문 default 값이면 한컴이 탭 위치를 두 배로 읽는다)
        assert_eq!(saved, original, "{name}: tabItem switch");
    }
}

#[test]
fn residual_c1_tab_halving_path_survives_two_saves() {
    let values = [0, 1, 1000, 1001, 1 << 31, (1 << 31) + 1, u32::MAX - 1, u32::MAX];
    let mut doc = Document::default();
    doc.doc_info.tab_defs.push(TabDef {
        tabs: values.iter().map(|&position| TabItem {
            position,
            ..Default::default()
        }).collect(),
        ..Default::default()
    });
    for pass in 1..=2 {
        let bytes = rhwp::serializer::serialize_hwpx(&doc).unwrap();
        let branches = tab_items(&zip_text(&bytes, "Contents/header.xml"));
        assert_eq!(branches[0].len(), values.len());
        for (&value, branch) in values.iter().zip(&branches[0]) {
            assert_eq!(
                (&branch.0, branch.1.as_str(), &branch.2),
                (
                    &(value / 2).to_string(),
                    if value % 2 == 0 { "HWPUNIT" } else { "CHAR" },
                    &value.to_string(),
                ),
                "tab {value}, save {pass}"
            );
        }
        doc = rhwp::parser::hwpx::parse_hwpx(&bytes).unwrap();
        assert_eq!(
            doc.doc_info.tab_defs[0].tabs.iter().map(|t| t.position).collect::<Vec<_>>(),
            values,
            "tab read {pass}"
        );
    }
}

#[test]
fn residual_c1_signed_halving_paths_survive_two_saves() {
    let values = [-1001, -1000, -1, 0, 1, 1000, 1001, i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX];
    let types = [LineSpacingType::Fixed, LineSpacingType::SpaceOnly, LineSpacingType::Minimum, LineSpacingType::Percent];
    let mut doc = Document::default();
    for spacing_type in types {
        for value in values {
            doc.doc_info.para_shapes.push(ParaShape {
                indent: value,
                margin_left: value,
                margin_right: value,
                spacing_before: value,
                spacing_after: value,
                line_spacing_type: spacing_type,
                line_spacing: value,
                ..Default::default()
            });
        }
    }
    let mut failures = Vec::new();
    for pass in 1..=2 {
        let bytes = rhwp::serializer::serialize_hwpx(&doc).unwrap();
        doc = rhwp::parser::hwpx::parse_hwpx(&bytes).unwrap();
        assert_eq!(doc.doc_info.para_shapes.len(), types.len() * values.len());
        for (ps, (spacing_type, value)) in doc.doc_info.para_shapes.iter().zip(types.into_iter().flat_map(|t| values.into_iter().map(move |v| (t, v)))) {
            let actual = [
                ps.indent, ps.margin_left, ps.margin_right,
                ps.spacing_before, ps.spacing_after, ps.line_spacing,
            ];
            if actual != [value; 6] || ps.line_spacing_type != spacing_type {
                failures.push(format!("{spacing_type:?} {value}, read {pass}: {actual:?}, {:?}", ps.line_spacing_type));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn residual_c1_track_change_flags_survive_two_native_saves() {
    let header = zip_text(C6, "Contents/header.xml");
    assert!(header.contains(r#"<hh:trackchageConfig flags="56"/>"#));
    for flags in [56, 0, u32::MAX] {
        for paired in [false, true] {
            let element = if paired {
                format!(r#"<hh:trackchageConfig flags="{flags}"></hh:trackchageConfig>"#)
            } else {
                format!(r#"<hh:trackchageConfig flags="{flags}"/>"#)
            };
            let raw = header.replace(r#"<hh:trackchageConfig flags="56"/>"#, &element);
            let mut bytes = replace_zip_text(C6, "Contents/header.xml", &raw);
            for pass in 1..=2 {
                bytes = open_save(&bytes);
                let tree = Tree::parse(&zip_text(&bytes, "Contents/header.xml"));
                let config = tree.descendants_named(0, "trackchageConfig");
                assert_eq!(config.len(), 1);
                assert_eq!(
                    tree.attr(config[0], "flags"), Some(flags.to_string().as_str()),
                    "flags {flags}, paired {paired}, save {pass}"
                );
            }
        }
    }
}

#[test]
fn residual_c1_missing_track_change_config_keeps_existing_zero_output() {
    let header = zip_text(C6, "Contents/header.xml");
    let raw = header.replace(r#"<hh:trackchageConfig flags="56"/>"#, "");
    assert!(!raw.contains("trackchageConfig"));
    let input = replace_zip_text(C6, "Contents/header.xml", &raw);
    let mut zero = replace_zip_text(C6, "Contents/header.xml", &header.replace(r#"flags="56""#, r#"flags="0""#));
    let mut bytes = input;
    for pass in 1..=2 {
        bytes = open_save(&bytes);
        zero = open_save(&zero);
        let expected = zip_text(&zero, "Contents/header.xml");
        assert!(expected.contains(r#"<hh:trackchageConfig flags="0"/>"#));
        assert!(
            zip_text(&bytes, "Contents/header.xml") == expected,
            "missing config matches explicit zero, save {pass}"
        );
    }
}

/// 구역 XML 의 그리기 개체를 전위 순서로 — (이름, 묶음 안 자식인가, 직속 자식 이름들).
fn drawing_objects(hwpx: &[u8], names: &[&str]) -> Vec<(String, bool, Vec<String>)> {
    let mut out = Vec::new();
    for xml in section_xmls(hwpx) {
        let tree = Tree::parse(&xml);
        let mut stack: Vec<usize> = tree.nodes[0].children.iter().rev().copied().collect();
        while let Some(n) = stack.pop() {
            if names.contains(&tree.name(n)) {
                out.push((
                    tree.name(n).to_string(),
                    tree.parent_name(n) == "container",
                    tree.child_names(n),
                ));
            }
            stack.extend(tree.nodes[n].children.iter().rev().copied());
        }
    }
    out
}

#[test]
fn residual_group_children_match_hancom_structure() {
    // Given: 한컴 원본의 사각형·묶음·그림 — 묶음 안 자식은 sz/pos/outMargin 이 없다
    let names = ["rect", "container", "pic"];
    let original = drawing_objects(C6, &names);
    let children = original.iter().filter(|o| o.1).count();
    assert_eq!(children, 10, "원본 묶음 안 자식 개체 수");
    assert!(
        original.iter().filter(|o| o.1).all(|o| !o
            .2
            .iter()
            .any(|c| c == "sz" || c == "pos" || c == "outMargin")),
        "원본 묶음 자식은 sz/pos/outMargin 없음"
    );
    for (name, fixture) in [("C6", C6), ("FACF", FACF)] {
        // When: 열고 저장한다
        let saved = drawing_objects(&open_save(fixture), &names);
        assert_eq!(saved.len(), original.len(), "{name}: 개체 수");
        // Then: 개체마다 직속 자식 구성이 원본과 같다
        let wrong: Vec<_> = original
            .iter()
            .zip(&saved)
            .filter(|(o, s)| o.0 != s.0 || o.1 != s.1 || o.2 != s.2)
            .collect();
        assert!(
            wrong.is_empty(),
            "{name}: 원본과 구성이 다른 개체 {}개, 첫 개체 (원본, 저장본): {:?}",
            wrong.len(),
            wrong.first()
        );
    }
}

/// 구역 XML 의 사각형·묶음·그림마다 `hp:offset` 의 (x, y) 원문.
fn shape_offsets(hwpx: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for xml in section_xmls(hwpx) {
        let tree = Tree::parse(&xml);
        for name in ["rect", "container", "pic"] {
            for n in tree.descendants_named(0, name) {
                let offset = tree.children_named(n, "offset")[0];
                out.push((
                    tree.attr(offset, "x").unwrap_or("").to_string(),
                    tree.attr(offset, "y").unwrap_or("").to_string(),
                ));
            }
        }
    }
    out
}

#[test]
fn residual_negative_shape_offset_survives_second_save() {
    // Given: 한컴 원본 묶음 자식의 음수 offset 은 32비트 부호 없는 표기(4294966901 = -395)다
    let original = shape_offsets(C6);
    let negative = original
        .iter()
        .filter(|(x, y)| {
            [x, y]
                .iter()
                .any(|v| v.parse::<u64>().is_ok_and(|v| v >= 1 << 31))
        })
        .count();
    assert_eq!(negative, 4, "원본의 음수 offset 개체 수");
    // When: 열고 저장하기를 두 번 한다
    let save1 = open_save(C6);
    let save2 = open_save(&save1);
    // Then: 두 저장본 모두 원본과 같은 offset 을 쓴다 (부호 있는 표기는 다시 열 때 0 이 된다)
    assert_eq!(shape_offsets(&save1), original, "저장 1 offset");
    assert_eq!(shape_offsets(&save2), original, "저장 2 offset");
}

/// 문단(표 셀 문단 포함)에서 처음 나오는 선 도형.
fn first_line(paras: &mut [Paragraph]) -> Option<&mut rhwp::model::shape::LineShape> {
    for para in paras {
        for ctrl in &mut para.controls {
            match ctrl {
                Control::Shape(shape) => {
                    if let ShapeObject::Line(line) = shape.as_mut() {
                        return Some(line);
                    }
                }
                Control::Table(table) => {
                    for cell in &mut table.cells {
                        if let Some(line) = first_line(&mut cell.paragraphs) {
                            return Some(line);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[test]
fn residual_line_writes_start_and_end_points() {
    // Given: 한컴 원본 T6 의 선 — OWPML LineType 은 hc:startPt/hc:endPt 자식을 반드시 쓴다
    let original = drawing_objects(T6, &["line"]);
    assert_eq!(original.len(), 1, "T6 선 수");
    assert!(
        original[0].2.iter().any(|c| c == "startPt") && original[0].2.iter().any(|c| c == "endPt"),
        "원본 선의 시작·끝점 자식"
    );
    // When: 열고 저장한다
    let saved = drawing_objects(&open_save(T6), &["line"]);
    // Then: 직속 자식 구성이 원본과 같다
    assert_eq!(
        saved.iter().map(|o| &o.2).collect::<Vec<_>>(),
        original.iter().map(|o| &o.2).collect::<Vec<_>>(),
        "T6 선 자식 구성"
    );

    // Given: IR 선의 시작·끝점
    let mut doc = open_doc(T6);
    let line = doc
        .sections
        .iter_mut()
        .find_map(|s| first_line(&mut s.paragraphs))
        .expect("T6 의 선");
    line.start = Point { x: 100, y: 200 };
    line.end = Point { x: 300, y: 400 };
    // When: 저장한다
    let xml = section_xmls(&rhwp::serializer::serialize_hwpx(&doc).expect("저장")).join("");
    // Then: 점 자식에 IR 값이 그대로 쓰인다
    assert!(
        xml.contains(r#"<hc:startPt x="100" y="200"/>"#)
            && xml.contains(r#"<hc:endPt x="300" y="400"/>"#),
        "선 시작·끝점 자식"
    );
}

// ---------- 한컴 확인·측정용 산출물 (기본 시험에서는 돌지 않는다) ----------

/// 표본을 열고 저장한 파일을 쓴다.
///
/// ```text
/// L2_DUMP_OUT=<폴더> cargo test --test hwpx_save_fidelity_l2 dump_open_save -- --ignored
/// ```
#[test]
#[ignore]
fn dump_open_save() {
    let out = std::env::var("L2_DUMP_OUT").expect("L2_DUMP_OUT");
    std::fs::create_dir_all(&out).expect("출력 폴더");
    for (name, bytes) in [("C6", C6), ("FACF", FACF), ("T6", T6)] {
        let save1 = open_save(bytes);
        std::fs::write(format!("{out}/{name}_open_save2.hwpx"), open_save(&save1)).expect("쓰기");
        std::fs::write(format!("{out}/{name}_open_save.hwpx"), save1).expect("쓰기");
    }
}

/// 본문 표 (구역, 문단, 컨트롤) 목록.
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

/// 한컴에서 열어 볼 저장본 6개 — 표본 2개 × (열고 저장 / 행 삽입 / 셀 글자 입력).
///
/// 편집 대상 표는 Parent 의 `hancom-gearup/gearup_gen.rs` 규칙(행이 둘 이상이고 표시 높이 ≠ 셀 높이 합인
/// 첫 본문 표)으로 고른다. 행 삽입은 첫 행 아래, 글자 입력은 셀 0 문단 0 의 0 자리에 '가' 20 자다.
///
/// ```text
/// L2_HANCOM_OUT=<폴더> cargo test --test hwpx_save_fidelity_l2 write_hancom_review_files -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn write_hancom_review_files() {
    let out = std::env::var("L2_HANCOM_OUT").expect("L2_HANCOM_OUT");
    std::fs::create_dir_all(&out).expect("출력 폴더");
    for (name, fixture) in [("C6", C6), ("FACF", FACF)] {
        let base = DocumentCore::from_bytes(fixture).expect("HWPX 로드");
        let tables = top_level_tables(base.document());
        let pick = tables
            .iter()
            .position(|&(si, pi, ci)| {
                match &base.document().sections[si].paragraphs[pi].controls[ci] {
                    Control::Table(t) => {
                        let sum: i64 = t.get_row_heights().iter().map(|h| *h as i64).sum();
                        t.row_count >= 2 && i64::from(t.common.height) != sum
                    }
                    _ => false,
                }
            })
            .expect("편집 대상 표");
        let (si, pi, ci) = tables[pick];
        let write = |kind: &str, core: &DocumentCore| {
            let bytes = core.export_hwpx_native().expect("HWPX 저장");
            let path = format!("{out}/L2_{name}_{kind}.hwpx");
            std::fs::write(&path, &bytes).expect("산출물 쓰기");
            println!(
                "{path} 본문 표 #{pick} (구역 {si}, 문단 {pi}, 컨트롤 {ci}) {} 바이트",
                bytes.len()
            );
        };
        write(
            "open_save",
            &DocumentCore::from_bytes(fixture).expect("HWPX 로드"),
        );
        let mut core = DocumentCore::from_bytes(fixture).expect("HWPX 로드");
        core.insert_table_row_native(si, pi, ci, 0, true)
            .expect("행 삽입");
        write("insert_row", &core);
        let mut core = DocumentCore::from_bytes(fixture).expect("HWPX 로드");
        core.insert_text_in_cell_native(si, pi, ci, 0, 0, 0, &"가".repeat(20))
            .expect("셀 글자 입력");
        write("cell_text", &core);
    }
}
