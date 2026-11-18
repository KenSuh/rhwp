// HWP5 표의 TABLE 레코드 attr·CTRL_HEADER attr·쪽나눔 방지를 문서 순서(전위)로 출력하는 진단 프로브
use rhwp::model::control::Control;
use rhwp::model::paragraph::Paragraph;

fn walk(paras: &[Paragraph], out: &mut Vec<String>) {
    for p in paras {
        for c in &p.controls {
            if let Control::Table(t) = c {
                out.push(format!(
                    "{}:{}:{}",
                    t.raw_table_record_attr, t.common.attr, t.common.prevent_page_break
                ));
                for cell in &t.cells {
                    walk(&cell.paragraphs, out);
                }
            }
        }
    }
}

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        match rhwp::parser::parse_document(&bytes) {
            Ok(doc) => {
                let mut out = Vec::new();
                for s in &doc.sections {
                    walk(&s.paragraphs, &mut out);
                }
                println!("{}\t{}", path, out.join(","));
            }
            Err(e) => println!("{}\tERR {}", path, e),
        }
    }
}
