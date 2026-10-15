//! HWPX 재저장 header 안정 회귀 — form-002 를 두 번 저장·재열기해도 문단 여백·winBrush·첫 문단 run 글자모양이 원본과 같다.
//!
//! `samples/hwpx/form-002.hwpx` 는 GearUp `apps/demo/public/sample.hwpx` 와 바이트 동일한 실제
//! 원본이다 (SHA-256 5ab8f7c368e02538f75f1cd2bd82bbd8de2f925a54ba7b38ec9395b2cdb804d4).
//! 기대값은 원본 HWPX 를 zip 으로 열어 XML 을 직접 읽어 만든다. rhwp parser·`HwpDocument`·
//! `parse_color_str` 로 기대값을 만들지 않는다. 저장·재열기는 에디터 경로(`DocumentCore`)를 쓴다.

use rhwp::document_core::DocumentCore;
use rhwp::model::document::Document;
use rhwp::model::paragraph::Paragraph;
use rhwp::model::style::{FillType, ParaShape};

const FORM_002: &[u8] = include_bytes!("../samples/hwpx/form-002.hwpx");

/// 문단 여백 다섯 필드 순서 — `intent`(들여쓰기)·`left`·`right`·`prev`(위)·`next`(아래).
const MARGIN_FIELDS: [&str; 5] = ["intent", "left", "right", "prev", "next"];

// ─── 원본 XML 직접 읽기 ───

/// HWPX ZIP 엔트리 하나의 바이트.
fn zip_bytes(hwpx: &[u8], name: &str) -> Vec<u8> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(hwpx)).expect("HWPX ZIP");
    let mut entry = archive.by_name(name).expect("HWPX entry");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("read HWPX entry");
    bytes
}

fn zip_text(hwpx: &[u8], name: &str) -> String {
    String::from_utf8(zip_bytes(hwpx, name)).expect("utf-8 XML")
}

/// 이 시험의 작은 XML 트리. 요소·속성 이름은 접두어를 뗀 local 이름, 속성 값은 원문 그대로다.
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<usize>,
}

struct Tree {
    nodes: Vec<Node>,
}

fn local_name(raw: &[u8]) -> String {
    let local = raw.rsplit(|b| *b == b':').next().unwrap_or(raw);
    String::from_utf8(local.to_vec()).expect("utf-8 name")
}

impl Tree {
    fn parse(xml: &str) -> Tree {
        use quick_xml::events::{BytesStart, Event};
        fn node(e: &BytesStart) -> Node {
            let attrs = e
                .attributes()
                .map(|attr| {
                    let attr = attr.expect("well-formed attribute");
                    (
                        local_name(attr.key.as_ref()),
                        String::from_utf8(attr.value.to_vec()).expect("utf-8 value"),
                    )
                })
                .collect();
            Node {
                name: local_name(e.name().as_ref()),
                attrs,
                children: Vec::new(),
            }
        }
        let mut nodes = vec![Node {
            name: String::new(),
            attrs: Vec::new(),
            children: Vec::new(),
        }];
        let mut stack = vec![0usize];
        let mut reader = quick_xml::Reader::from_str(xml);
        loop {
            match reader.read_event().expect("well-formed XML") {
                Event::Start(e) => {
                    let id = nodes.len();
                    nodes.push(node(&e));
                    nodes[*stack.last().expect("open element")]
                        .children
                        .push(id);
                    stack.push(id);
                }
                Event::Empty(e) => {
                    let id = nodes.len();
                    nodes.push(node(&e));
                    nodes[*stack.last().expect("open element")]
                        .children
                        .push(id);
                }
                Event::End(_) => {
                    stack.pop();
                }
                Event::Eof => break,
                _ => {}
            }
        }
        Tree { nodes }
    }

    fn name(&self, id: usize) -> &str {
        &self.nodes[id].name
    }

    fn attr(&self, id: usize, key: &str) -> Option<&str> {
        self.nodes[id]
            .attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
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
}

/// `<hh:margin>` 의 다섯 자식 `value` (각 자식이 정확히 하나여야 한다).
fn margin_children(tree: &Tree, margin: usize) -> [i32; 5] {
    let mut values = [0i32; 5];
    for (slot, field) in MARGIN_FIELDS.iter().enumerate() {
        let kids = tree.children_named(margin, field);
        assert_eq!(kids.len(), 1, "margin child <{}> count", field);
        values[slot] = tree
            .attr(kids[0], "value")
            .expect("margin child value")
            .parse()
            .expect("margin child value i32");
    }
    values
}

/// `case`/`default` 아래 margin 하나.
fn single_margin(tree: &Tree, branch: usize) -> usize {
    let margins = tree.children_named(branch, "margin");
    assert_eq!(margins.len(), 1, "margin per switch branch");
    margins[0]
}

/// 원본 winBrush 한 개 (borderFill id 와 속성 원문).
struct WinBrush {
    id: usize,
    face: String,
    hatch: String,
    alpha: String,
}

/// 원본 XML 에서 독립으로 만든 기대값.
struct Original {
    len: usize,
    /// paraPr id 순 `<hp:default>` margin (intent, left, right, prev, next).
    margins: Vec<[i32; 5]>,
    win_brushes: Vec<WinBrush>,
    tc: usize,
    fill_cells: usize,
    first_para_runs: String,
}

/// 원본 색 문자열 → 모델 ColorRef. 시험 자체 리터럴 규칙이다 (제품 `parse_color_str` 를 부르지 않는다).
/// `none` → 0xFFFFFFFF · `#RRGGBB` → 0x00BBGGRR · `#AARRGGBB` → AA 를 버리고 같은 규칙.
fn literal_color(s: &str) -> u32 {
    if s == "none" {
        return 0xFFFF_FFFF;
    }
    let hex = s.strip_prefix('#').expect("color starts with #");
    let rgb = match hex.len() {
        6 => hex,
        8 => &hex[2..],
        _ => panic!("unexpected color literal {}", s),
    };
    let v = u32::from_str_radix(rgb, 16).expect("hex color");
    let (r, g, b) = ((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF);
    (b << 16) | (g << 8) | r
}

/// 색 문자열의 RGB 부분(앞 AA 바이트 제외 · 대문자). `none` 은 그대로.
fn rgb_part(s: &str) -> String {
    if s.eq_ignore_ascii_case("none") {
        return "none".to_string();
    }
    let hex = s.trim_start_matches('#');
    let rgb = if hex.len() == 8 { &hex[2..] } else { hex };
    rgb.to_ascii_uppercase()
}

/// section XML 첫 최상위 `<hp:p>` 에서 secPr·pageNum·tbl 을 담은 첫 run 의 charPrIDRef.
fn first_para_runs(section_xml: &str) -> String {
    let tree = Tree::parse(section_xml);
    let sec = tree.children_named(0, "sec");
    assert_eq!(sec.len(), 1, "one <hs:sec> root");
    let paras = tree.children_named(sec[0], "p");
    let Some(&first) = paras.first() else {
        return "-,-,-".to_string();
    };
    // run 의 얕은 부분 트리(표 셀·글상자 문단으로 내려가지 않음)에 `target` 이 있는가.
    let contains = |run: usize, target: &str| -> bool {
        let mut stack: Vec<usize> = tree.nodes[run].children.clone();
        while let Some(n) = stack.pop() {
            if tree.name(n) == target {
                return true;
            }
            if !matches!(tree.name(n), "tbl" | "subList" | "p") {
                stack.extend(tree.nodes[n].children.iter().copied());
            }
        }
        false
    };
    let runs = tree.children_named(first, "run");
    ["secPr", "pageNum", "tbl"]
        .iter()
        .map(|target| {
            runs.iter()
                .find(|&&run| contains(run, target))
                .and_then(|&run| tree.attr(run, "charPrIDRef"))
                .unwrap_or("-")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn read_original(hwpx: &[u8]) -> Original {
    let header = Tree::parse(&zip_text(hwpx, "Contents/header.xml"));

    // paraPr: id 0.. 연속 · margin 을 담은 switch 정확히 하나 · case(HwpUnitChar)×2 == default.
    let para_prs = header.descendants_named(0, "paraPr");
    let mut margins = Vec::new();
    let mut scale_mismatch = 0usize;
    for (i, &pp) in para_prs.iter().enumerate() {
        assert_eq!(
            header.attr(pp, "id"),
            Some(i.to_string().as_str()),
            "paraPr ids are consecutive from 0"
        );
        let switches: Vec<usize> = header
            .children_named(pp, "switch")
            .into_iter()
            .filter(|&s| !header.descendants_named(s, "margin").is_empty())
            .collect();
        assert_eq!(switches.len(), 1, "paraPr {} margin switch count", i);
        let cases: Vec<usize> = header
            .children_named(switches[0], "case")
            .into_iter()
            .filter(|&c| {
                header
                    .attr(c, "required-namespace")
                    .is_some_and(|ns| ns.contains("HwpUnitChar"))
            })
            .collect();
        let defaults = header.children_named(switches[0], "default");
        assert_eq!(
            (cases.len(), defaults.len()),
            (1, 1),
            "paraPr {} HwpUnitChar case / default",
            i
        );
        let case_values = margin_children(&header, single_margin(&header, cases[0]));
        let default_values = margin_children(&header, single_margin(&header, defaults[0]));
        if case_values.map(|v| v * 2) != default_values {
            scale_mismatch += 1;
        }
        margins.push(default_values);
    }
    assert_eq!(scale_mismatch, 0, "case(HwpUnitChar) x2 == default");

    // borderFill: id 1.. 연속 · winBrush 원문.
    let border_fills = header.descendants_named(0, "borderFill");
    let mut win_brushes = Vec::new();
    for (i, &bf) in border_fills.iter().enumerate() {
        assert_eq!(
            header.attr(bf, "id"),
            Some((i + 1).to_string().as_str()),
            "borderFill ids are consecutive from 1"
        );
        let brushes = header.descendants_named(bf, "winBrush");
        assert!(brushes.len() <= 1, "borderFill {} winBrush count", i + 1);
        if let Some(&wb) = brushes.first() {
            let get = |key: &str| header.attr(wb, key).expect("winBrush attr").to_string();
            win_brushes.push(WinBrush {
                id: i + 1,
                face: get("faceColor"),
                hatch: get("hatchColor"),
                alpha: get("alpha"),
            });
        }
    }
    // 원본 alpha 는 모두 "0" 이다 — 모델 기대값은 불투명도 0.
    assert!(
        win_brushes.iter().all(|wb| wb.alpha == "0"),
        "fixture winBrush alpha"
    );

    let section_xml = zip_text(hwpx, "Contents/section0.xml");
    let section = Tree::parse(&section_xml);
    let cells = section.descendants_named(0, "tc");
    let fill_cells = cells
        .iter()
        .filter(|&&tc| {
            section
                .attr(tc, "borderFillIDRef")
                .and_then(|v| v.parse::<usize>().ok())
                .is_some_and(|id| win_brushes.iter().any(|wb| wb.id == id))
        })
        .count();

    Original {
        len: hwpx.len(),
        margins,
        win_brushes,
        tc: cells.len(),
        fill_cells,
        first_para_runs: first_para_runs(&section_xml),
    }
}

// ─── 모델 비교 ───

fn margins_of(ps: &ParaShape) -> [i32; 5] {
    [
        ps.indent,
        ps.margin_left,
        ps.margin_right,
        ps.spacing_before,
        ps.spacing_after,
    ]
}

/// 모델 `para_shapes[i]` 다섯 필드가 원본 `<hp:default>` 값과 같은 i 의 수.
fn margins_equal(doc: &Document, orig: &Original) -> usize {
    orig.margins
        .iter()
        .enumerate()
        .filter(|(i, expected)| {
            doc.doc_info
                .para_shapes
                .get(*i)
                .is_some_and(|ps| margins_of(ps) == **expected)
        })
        .count()
}

/// winBrush borderFill 마다 모델 fill 이 Solid 이고 색·alpha 가 리터럴 변환값과 같은 수.
fn fills_equal(doc: &Document, orig: &Original) -> usize {
    orig.win_brushes
        .iter()
        .filter(|wb| {
            let (face, hatch) = (literal_color(&wb.face), literal_color(&wb.hatch));
            doc.doc_info.border_fills.get(wb.id - 1).is_some_and(|bf| {
                bf.fill.fill_type == FillType::Solid
                    && bf.fill.alpha == 0
                    && bf
                        .fill
                        .solid
                        .is_some_and(|s| s.background_color == face && s.pattern_color == hatch)
            })
        })
        .count()
}

/// 본문·표 셀·글상자·캡션·머리말/꼬리말·각주/미주·숨은 설명 문단을 깊이 우선으로 모은다.
fn all_paragraphs<'a>(paras: &'a [Paragraph], out: &mut Vec<&'a Paragraph>) {
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

fn shape_paragraphs<'a>(shape: &'a rhwp::model::shape::ShapeObject, out: &mut Vec<&'a Paragraph>) {
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

fn document_paragraphs(doc: &Document) -> Vec<&Paragraph> {
    let mut out = Vec::new();
    for section in &doc.sections {
        all_paragraphs(&section.paragraphs, &mut out);
    }
    out
}

/// 글자마다 활성 글자모양 id — `char_offsets[i]` 이하에서 시작한 마지막 변경 (없으면 첫 변경).
fn active_char_shape_ids(para: &Paragraph) -> Vec<u32> {
    let first = para.char_shapes.first().map_or(0, |r| r.char_shape_id);
    (0..para.text.chars().count())
        .map(|i| {
            let pos = para.char_offsets.get(i).copied().unwrap_or(u32::MAX);
            para.char_shapes
                .iter()
                .take_while(|r| r.start_pos <= pos)
                .last()
                .map_or(first, |r| r.char_shape_id)
        })
        .collect()
}

/// 모든 문단의 글자마다 활성 글자모양 id 가 `base` 와 같은가와 비교한 글자 수.
fn char_shapes_equal(base: &Document, doc: &Document) -> (bool, usize) {
    let a = document_paragraphs(base);
    let b = document_paragraphs(doc);
    let mut equal = a.len() == b.len();
    let mut chars = 0usize;
    for (pa, pb) in a.iter().zip(&b) {
        if pa.text != pb.text {
            equal = false;
            continue;
        }
        let ia = active_char_shape_ids(pa);
        chars += ia.len();
        if ia != active_char_shape_ids(pb) {
            equal = false;
        }
    }
    (equal, chars)
}

/// 저장 header XML 의 winBrush 를 원본 문자열과 맞춘 한 줄.
fn fill_xml_line(save: usize, saved_header_xml: &str, orig: &Original) -> (String, [usize; 5]) {
    let tree = Tree::parse(saved_header_xml);
    let total = tree.descendants_named(0, "winBrush").len();
    let border_fills = tree.descendants_named(0, "borderFill");
    let (mut face_equal, mut face_none, mut hatch_equal, mut alpha_equal) = (0, 0, 0, 0);
    for wb in &orig.win_brushes {
        let Some(&bf) = border_fills
            .iter()
            .find(|&&bf| tree.attr(bf, "id") == Some(wb.id.to_string().as_str()))
        else {
            continue;
        };
        let brushes = tree.descendants_named(bf, "winBrush");
        let Some(&saved) = brushes.first() else {
            continue;
        };
        let face = tree.attr(saved, "faceColor").unwrap_or("");
        let hatch = tree.attr(saved, "hatchColor").unwrap_or("");
        let alpha = tree.attr(saved, "alpha").unwrap_or("");
        if face.eq_ignore_ascii_case(&wb.face) {
            face_equal += 1;
        }
        if face == "none" {
            face_none += 1;
        }
        if !hatch.is_empty() && rgb_part(hatch) == rgb_part(&wb.hatch) {
            hatch_equal += 1;
        }
        let as_f64 = |s: &str| s.parse::<f64>().ok();
        if as_f64(alpha).is_some() && as_f64(alpha) == as_f64(&wb.alpha) {
            alpha_equal += 1;
        }
    }
    let n = orig.win_brushes.len();
    (
        format!(
            "FILL_XML save={} winbrush={} face_equal={}/{} face_none={} hatch_rgb_equal={}/{} alpha_equal={}/{} ",
            save, total, face_equal, n, face_none, hatch_equal, n, alpha_equal, n
        ),
        [total, face_equal, face_none, hatch_equal, alpha_equal],
    )
}

// ─── 회귀 ───

#[test]
fn form_002_header_survives_two_save_reopen_cycles() {
    // Given: SHA 로 결속한 실제 원본과, 그 XML 에서 독립으로 만든 기대값 (§1.3).
    let orig = read_original(FORM_002);
    let nz = |slot: usize| orig.margins.iter().filter(|m| m[slot] != 0).count();
    let all_zero = orig
        .margins
        .iter()
        .filter(|m| m.iter().all(|v| *v == 0))
        .count();
    let face_none = orig
        .win_brushes
        .iter()
        .filter(|wb| wb.face == "none")
        .count();
    let orig_line = format!(
        "ORIG_EXPECT len={} parapr={} allzero={} nz_intent={} nz_left={} nz_right={} nz_prev={} nz_next={} winbrush={} face_none={} face_colored={} tc={} fill_cells={} first_para_runs={} ",
        orig.len,
        orig.margins.len(),
        all_zero,
        nz(0),
        nz(1),
        nz(2),
        nz(3),
        nz(4),
        orig.win_brushes.len(),
        face_none,
        orig.win_brushes.len() - face_none,
        orig.tc,
        orig.fill_cells,
        orig.first_para_runs
    );
    println!("{}", orig_line);

    // When: 원본을 열고 → 저장 → 재열기 → 저장 → 재열기 (에디터 경로).
    let core0 = DocumentCore::from_bytes(FORM_002).expect("open original");
    let save1 = core0.export_hwpx_native().expect("save 1");
    let core1 = DocumentCore::from_bytes(&save1).expect("reopen save 1");
    let save2 = core1.export_hwpx_native().expect("save 2");
    let core2 = DocumentCore::from_bytes(&save2).expect("reopen save 2");

    let header1 = zip_bytes(&save1, "Contents/header.xml");
    let header2 = zip_bytes(&save2, "Contents/header.xml");
    let cycles = [
        (
            core0.document(),
            orig.first_para_runs.clone(),
            "na".to_string(),
        ),
        (
            core1.document(),
            first_para_runs(&zip_text(&save1, "Contents/section0.xml")),
            "na".to_string(),
        ),
        (
            core2.document(),
            first_para_runs(&zip_text(&save2, "Contents/section0.xml")),
            (header2 == header1).to_string(),
        ),
    ];

    // Then (출력): 단언 전에 모든 측정 줄을 찍는다 (RED 에서도 남는다).
    let parapr = orig.margins.len();
    let fills = orig.win_brushes.len();
    let mut measured = Vec::new();
    for (k, (doc, runs, header_equal_prev)) in cycles.iter().enumerate() {
        let margins = margins_equal(doc, &orig);
        let fill = fills_equal(doc, &orig);
        let (chars_equal, text_chars) = char_shapes_equal(core0.document(), doc);
        println!(
            "RESAVE cycle={} margins_equal={}/{} fills_equal={}/{} first_para_runs={} char_shapes_equal_orig={} text_chars={} header_equal_prev={} ",
            k, margins, parapr, fill, fills, runs, chars_equal, text_chars, header_equal_prev
        );
        measured.push((margins, fill, runs.clone(), chars_equal, text_chars));
    }
    let (fill_line1, fill1) = fill_xml_line(
        1,
        &String::from_utf8(header1.clone()).expect("utf-8"),
        &orig,
    );
    let (fill_line2, fill2) = fill_xml_line(
        2,
        &String::from_utf8(header2.clone()).expect("utf-8"),
        &orig,
    );
    println!("{}", fill_line1);
    println!("{}", fill_line2);

    // Then (기대값): 원본 측정은 §1.3 리터럴과 같다.
    assert_eq!(
        orig_line,
        "ORIG_EXPECT len=131571 parapr=97 allzero=18 nz_intent=63 nz_left=15 nz_right=2 nz_prev=1 nz_next=5 winbrush=10 face_none=3 face_colored=7 tc=440 fill_cells=134 first_para_runs=9,9,88 ",
        "independent expectations from the original XML"
    );
    let win_ids: Vec<usize> = orig.win_brushes.iter().map(|wb| wb.id).collect();
    assert_eq!(win_ids, vec![2, 3, 5, 6, 7, 14, 18, 23, 24, 25]);

    // Then (회귀): 두 번 저장·재열기한 모델이 원본 여백·fill·첫 문단 run·글자모양을 지킨다.
    for (k, (margins, fill, runs, chars_equal, text_chars)) in measured.iter().enumerate() {
        assert_eq!(*margins, parapr, "cycle {}: para_shapes margins", k);
        assert_eq!(*fill, fills, "cycle {}: winBrush fills", k);
        assert_eq!(
            runs, "9,9,88",
            "cycle {}: first paragraph run charPrIDRef",
            k
        );
        assert!(*chars_equal, "cycle {}: active char shape per character", k);
        assert!(*text_chars > 0, "cycle {}: compared characters", k);
    }
    assert_eq!(header2, header1, "save 2 header.xml == save 1 header.xml");
    for fill in [fill1, fill2] {
        assert_eq!(fill, [10, 10, 3, 10, 10], "saved winBrush strings");
    }
}

// ─── parser: switch 밖 margin 자식 ───

/// paraPr 하나만 담은 header.xml 을 crate parser 로 읽는다.
fn parse_single_para_pr(body: &str) -> ParaShape {
    let xml = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
            r#"<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" "#,
            r#"xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" "#,
            r#"xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core">"#,
            r#"<hh:refList><hh:paraProperties itemCnt="1">"#,
            r#"<hh:paraPr id="0" tabPrIDRef="0" condense="0">{}</hh:paraPr>"#,
            r#"</hh:paraProperties></hh:refList></hh:head>"#
        ),
        body
    );
    let (info, _) = rhwp::parser::hwpx::header::parse_hwpx_header(&xml).expect("parse header.xml");
    assert_eq!(info.para_shapes.len(), 1, "one paraPr");
    info.para_shapes[0].clone()
}

#[test]
fn bare_margin_children_are_read_at_default_scale() {
    // rhwp writer 형태: switch 없는 <hh:margin> + hh: 자식 — 값 그대로(×2 없음).
    let ps = parse_single_para_pr(concat!(
        r#"<hh:align horizontal="JUSTIFY" vertical="BASELINE"/>"#,
        r#"<hh:margin><hh:intent unit="HWPUNIT" value="-2600"/><hh:left unit="HWPUNIT" value="3000"/>"#,
        r#"<hh:right unit="HWPUNIT" value="1200"/><hh:prev unit="HWPUNIT" value="400"/>"#,
        r#"<hh:next unit="HWPUNIT" value="800"/></hh:margin>"#,
        r#"<hh:lineSpacing type="PERCENT" value="160" unit="HWPUNIT"/>"#,
        r#"<hh:border borderFillIDRef="2" offsetLeft="0" offsetRight="0" offsetTop="0" offsetBottom="0"/>"#
    ));
    assert_eq!(margins_of(&ps), [-2600, 3000, 1200, 400, 800]);
    assert_eq!(ps.line_spacing, 160, "siblings after margin still parse");
    assert_eq!(ps.border_fill_id, 2, "siblings after margin still parse");

    // 한컴 접두어(hc:) 자식도 local 이름으로 같다.
    let ps = parse_single_para_pr(concat!(
        r#"<hh:margin><hc:intent value="-2600" unit="HWPUNIT"/><hc:left value="3000" unit="HWPUNIT"/>"#,
        r#"<hc:right value="1200" unit="HWPUNIT"/><hc:prev value="400" unit="HWPUNIT"/>"#,
        r#"<hc:next value="800" unit="HWPUNIT"/></hh:margin>"#
    ));
    assert_eq!(margins_of(&ps), [-2600, 3000, 1200, 400, 800]);
}

#[test]
fn switch_margin_is_read_only_by_the_switch_path() {
    // case(HwpUnitChar)×2 를 쓰고 default 는 버린다. default 를 일부러 case×2 와 다르게 둬
    // switch 안 margin 을 bare margin 경로가 다시 읽으면 드러나게 한다.
    let ps = parse_single_para_pr(concat!(
        r#"<hp:switch><hp:case hp:required-namespace="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar">"#,
        r#"<hh:margin><hc:intent value="-1300" unit="HWPUNIT"/><hc:left value="1500" unit="HWPUNIT"/>"#,
        r#"<hc:right value="600" unit="HWPUNIT"/><hc:prev value="200" unit="HWPUNIT"/>"#,
        r#"<hc:next value="400" unit="HWPUNIT"/></hh:margin>"#,
        r#"<hh:lineSpacing type="PERCENT" value="160" unit="HWPUNIT"/></hp:case>"#,
        r#"<hp:default><hh:margin><hc:intent value="-2601" unit="HWPUNIT"/><hc:left value="3001" unit="HWPUNIT"/>"#,
        r#"<hc:right value="1201" unit="HWPUNIT"/><hc:prev value="401" unit="HWPUNIT"/>"#,
        r#"<hc:next value="801" unit="HWPUNIT"/></hh:margin>"#,
        r#"<hh:lineSpacing type="PERCENT" value="160" unit="HWPUNIT"/></hp:default></hp:switch>"#
    ));
    assert_eq!(margins_of(&ps), [-2600, 3000, 1200, 400, 800]);
}

#[test]
fn self_closing_margin_attributes_still_parse() {
    // 속성 형태 <hh:margin …/> 는 종전처럼 속성만 읽고, 뒤 형제 요소에 영향이 없다.
    let ps = parse_single_para_pr(concat!(
        r#"<hh:margin left="100" right="200" indent="300" prev="400" next="500"/>"#,
        r#"<hh:lineSpacing type="PERCENT" value="130" unit="HWPUNIT"/>"#,
        r#"<hh:border borderFillIDRef="3" offsetLeft="0" offsetRight="0" offsetTop="0" offsetBottom="0"/>"#
    ));
    assert_eq!(margins_of(&ps), [300, 100, 200, 400, 500]);
    assert_eq!(ps.line_spacing, 130);
    assert_eq!(ps.border_fill_id, 3);
}
