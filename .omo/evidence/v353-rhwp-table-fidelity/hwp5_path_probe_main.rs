// 표본별 HWPX → HWP5 → HWPX 왕복이 끝까지 되는지 보는 관찰 프로브
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        let mut core = rhwp::DocumentCore::from_bytes(&bytes).unwrap();
        let r = core
            .export_hwp_with_adapter()
            .map_err(|e| format!("HWP5 저장: {e}"))
            .and_then(|hwp| rhwp::DocumentCore::from_bytes(&hwp).map_err(|e| format!("HWP5 재로드: {e}")))
            .and_then(|c| c.export_hwpx_native().map_err(|e| format!("HWPX 저장: {e}")));
        match r {
            Ok(_) => println!("OK\t{}", path.rsplit('/').next().unwrap()),
            Err(e) => println!("ERR\t{}\t{}", path.rsplit('/').next().unwrap(), e),
        }
    }
}
