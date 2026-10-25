// HWP5 표의 TABLE 레코드 bit 0-1 을 문서 순서(전위)로 출력하는 진단 프로브
use rhwp::model::control::Control;
use rhwp::model::paragraph::Paragraph;

fn walk(paras: &[Paragraph], out: &mut Vec<String>) {
    for p in paras {
        for c in &p.controls {
            if let Control::Table(t) = c {
                out.push(format!("{}", t.raw_table_record_attr & 0x03));
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
