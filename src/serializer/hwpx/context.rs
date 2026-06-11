//! 직렬화 컨텍스트 — 1-pass 스캔으로 ID 풀을 구성하고 2-pass 쓰기에서 참조 정합성을 단언.
//!
//! ## 배경
//!
//! HWPX 직렬화에서 가장 큰 함정은 **한 파일(section.xml)에서 쓴 ID가 다른 파일(header.xml)에
//! 등록되지 않은** 상태로 출력되는 경우다. 예: `<hp:run charPrIDRef="3">` 를 썼는데
//! header의 `<hh:charPr id="3">` 가 누락되면 한컴2020이 조용히 스타일을 엉키게 렌더링한다.
//!
//! `SerializeContext`는 이를 구조적으로 방지한다:
//! 1. **1-pass**: Document IR을 훑어 모든 ID를 `registered`에 등록
//! 2. **2-pass**: 각 writer가 ID를 사용할 때 `reference`에 기록
//! 3. **단언**: `assert_all_refs_resolved()` 가 `referenced - registered` 가 공집합임을 확인
//!
//! Stage 0 에서는 뼈대 구조만 둔다. 실제 스캔 로직은 Stage 1~4에서 writer가 추가될 때 함께 확장한다.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use super::lossy::{classify_hwpx_lossy, LossyDrop, LossyKind, LossySurface};
use crate::model::control::Control;
use crate::model::document::Document;
use crate::serializer::SerializeError;

/// 양방향 ID 풀 — 등록된 ID와 참조된 ID를 추적한다.
#[derive(Debug, Default)]
pub struct IdPool<T: Copy + Eq + std::hash::Hash> {
    registered: HashSet<T>,
    referenced: HashSet<T>,
}

impl<T: Copy + Eq + std::hash::Hash> IdPool<T> {
    pub fn new() -> Self {
        Self {
            registered: HashSet::new(),
            referenced: HashSet::new(),
        }
    }

    /// header/DocInfo에서 정의되는 ID를 등록.
    pub fn register(&mut self, id: T) {
        self.registered.insert(id);
    }

    /// section/기타 writer가 ID를 참조할 때 호출.
    pub fn reference(&mut self, id: T) {
        self.referenced.insert(id);
    }

    pub fn is_registered(&self, id: &T) -> bool {
        self.registered.contains(id)
    }

    /// `referenced - registered`: 참조됐으나 등록되지 않은 ID.
    pub fn unresolved(&self) -> Vec<T> {
        self.referenced
            .difference(&self.registered)
            .copied()
            .collect()
    }

    pub fn registered_count(&self) -> usize {
        self.registered.len()
    }
}

/// HWPX manifest + ZIP entry용 BinData 엔트리.
#[derive(Debug, Clone)]
pub struct BinDataEntry {
    /// content.hpf 의 `opf:item id` (예: "image1")
    pub manifest_id: String,
    /// ZIP 엔트리 경로 (예: "BinData/image1.png")
    pub href: String,
    /// MIME 타입 (예: "image/png")
    pub media_type: String,
    /// IR 상의 bin_data_id (storage_id) — 매핑 역추적용
    pub bin_data_id: u16,
}

/// 1-pass 스캔으로 구축되는 직렬화 컨텍스트.
#[derive(Debug, Default)]
pub struct SerializeContext {
    pub char_shape_ids: IdPool<u32>,
    pub para_shape_ids: IdPool<u16>,
    pub border_fill_ids: IdPool<u16>,
    pub tab_pr_ids: IdPool<u16>,
    pub numbering_ids: IdPool<u16>,
    pub style_ids: IdPool<u16>,
    /// `bin_data_id` (IR) → manifest 엔트리 매핑
    pub bin_data_map: HashMap<u16, BinDataEntry>,
    /// 저장 시 emit 되지 않아 손실되는 컨트롤 수집(save-time hard warning 용). 각 drop site가
    /// `record_lossy` 로 채운다. 바이트 출력에는 영향 없음(관찰 전용).
    pub lossy: Vec<LossyDrop>,
    /// 현재 직렬화 중인 섹션 인덱스(손실 좌표 기록용). `write_section` 진입 시 설정.
    pub current_section_index: usize,
}

impl SerializeContext {
    /// Document IR 전체를 1-pass 스캔하여 ID 풀을 채운다.
    ///
    /// Stage 0에서는 최소 등록(header.xml 리소스만)만 수행한다. Stage 1~4에서
    /// 각 writer가 추가되면서 `reference()` 호출과 스캔 범위가 확장된다.
    pub fn collect_from_document(doc: &Document) -> Self {
        let mut ctx = Self::default();

        // CharShape, ParaShape, BorderFill, TabDef, Numbering, Style, Font
        // 목록은 배열 인덱스가 곧 HWPX `id` 속성이 된다.
        for (idx, _) in doc.doc_info.char_shapes.iter().enumerate() {
            ctx.char_shape_ids.register(idx as u32);
        }
        for (idx, _) in doc.doc_info.para_shapes.iter().enumerate() {
            ctx.para_shape_ids.register(idx as u16);
        }
        for (idx, _) in doc.doc_info.border_fills.iter().enumerate() {
            // HWPX borderFill id는 header writer에서 1-based로 출력된다.
            ctx.border_fill_ids.register((idx + 1) as u16);
        }
        for (idx, _) in doc.doc_info.tab_defs.iter().enumerate() {
            ctx.tab_pr_ids.register(idx as u16);
        }
        for (idx, _) in doc.doc_info.numberings.iter().enumerate() {
            ctx.numbering_ids.register(idx as u16);
        }
        for (idx, _) in doc.doc_info.styles.iter().enumerate() {
            ctx.style_ids.register(idx as u16);
        }

        // BinData: bin_data_content의 storage_id → manifest 엔트리 생성
        for (i, bd) in doc.bin_data_content.iter().enumerate() {
            let ext = if bd.extension.is_empty() {
                "bin"
            } else {
                bd.extension.as_str()
            };
            let manifest_id = format!("image{}", i + 1);
            let href = format!("BinData/{}.{}", manifest_id, ext);
            let media_type = mime_from_ext(ext);
            ctx.bin_data_map.insert(
                bd.id,
                BinDataEntry {
                    manifest_id,
                    href,
                    media_type: media_type.to_string(),
                    bin_data_id: bd.id,
                },
            );
        }

        ctx
    }

    /// manifest·content.hpf 출력용 엔트리 목록 (삽입 순서 보존을 위해 `bin_data_id` 정렬).
    pub fn bin_data_entries(&self) -> Vec<BinDataEntry> {
        let mut v: Vec<_> = self.bin_data_map.values().cloned().collect();
        v.sort_by_key(|e| e.bin_data_id);
        v
    }

    /// `bin_data_id` → manifest id 조회 (Stage 4의 `<hc:img binaryItemIDRef="...">` 용).
    pub fn resolve_bin_id(&self, bin_data_id: u16) -> Option<&str> {
        self.bin_data_map
            .get(&bin_data_id)
            .map(|e| e.manifest_id.as_str())
    }

    /// 컨트롤 하나를 손실 분류하여 손실이면 `lossy` 에 기록한다(무손실이면 no-op).
    ///
    /// 본문 문단/표 셀 두 drop site 가 같은 분류(`classify_hwpx_lossy`)를 쓰도록 단일 진입점으로
    /// 둔다. `surface` 는 emit 집합이 다른 표면(본문/셀)을 구분한다. `para_index` 는 best-effort
    /// 좌표(표 셀은 셀-로컬 문단 인덱스일 수 있음).
    pub fn record_lossy(&mut self, ctrl: &Control, surface: LossySurface, para_index: usize) {
        if let Some(kind) = classify_hwpx_lossy(ctrl, surface) {
            self.record_lossy_kind(kind, para_index);
        }
    }

    /// 컨트롤이 아닌 섹션 단위 손실(예: SectionDef page_def 외 설정)을 직접 기록한다.
    pub fn record_lossy_kind(&mut self, kind: LossyKind, para_index: usize) {
        self.lossy.push(LossyDrop {
            kind,
            section_index: self.current_section_index,
            para_index,
        });
    }

    /// 모든 참조가 해소되었는지 단언. 해소되지 않은 ID가 있으면 `SerializeError::XmlError` 반환.
    pub fn assert_all_refs_resolved(&self) -> Result<(), SerializeError> {
        let mut missing: Vec<String> = Vec::new();
        let cs = self.char_shape_ids.unresolved();
        if !cs.is_empty() {
            missing.push(format!("charPrIDRef: {:?}", cs));
        }
        let ps = self.para_shape_ids.unresolved();
        if !ps.is_empty() {
            missing.push(format!("paraPrIDRef: {:?}", ps));
        }
        let bf = self.border_fill_ids.unresolved();
        if !bf.is_empty() {
            missing.push(format!("borderFillIDRef: {:?}", bf));
        }
        let tp = self.tab_pr_ids.unresolved();
        if !tp.is_empty() {
            missing.push(format!("tabPrIDRef: {:?}", tp));
        }
        let nm = self.numbering_ids.unresolved();
        if !nm.is_empty() {
            missing.push(format!("numberingIDRef: {:?}", nm));
        }
        let st = self.style_ids.unresolved();
        if !st.is_empty() {
            missing.push(format!("styleIDRef: {:?}", st));
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(SerializeError::XmlError(format!(
                "미등록 ID 참조 발견: {}",
                missing.join("; ")
            )))
        }
    }
}

fn mime_from_ext(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_doc_has_no_registered_ids() {
        let doc = Document::default();
        let ctx = SerializeContext::collect_from_document(&doc);
        assert_eq!(ctx.char_shape_ids.registered_count(), 0);
        assert_eq!(ctx.para_shape_ids.registered_count(), 0);
        assert!(ctx.bin_data_map.is_empty());
    }

    #[test]
    fn empty_doc_passes_ref_resolution() {
        let doc = Document::default();
        let ctx = SerializeContext::collect_from_document(&doc);
        ctx.assert_all_refs_resolved().expect("empty doc must pass");
    }

    #[test]
    fn unresolved_char_pr_fails() {
        let doc = Document::default();
        let mut ctx = SerializeContext::collect_from_document(&doc);
        ctx.char_shape_ids.reference(42); // 등록되지 않은 ID 참조
        let err = ctx.assert_all_refs_resolved().unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("charPrIDRef"),
            "error message should name charPrIDRef: {}",
            msg
        );
        assert!(
            msg.contains("42"),
            "error message should include id 42: {}",
            msg
        );
    }

    #[test]
    fn id_pool_register_reference_roundtrip() {
        let mut pool: IdPool<u32> = IdPool::new();
        pool.register(1);
        pool.register(2);
        pool.reference(1);
        pool.reference(3); // 미등록
        assert!(pool.is_registered(&1));
        assert!(!pool.is_registered(&3));
        assert_eq!(pool.unresolved(), vec![3]);
    }

    #[test]
    fn mime_from_ext_covers_common_formats() {
        assert_eq!(mime_from_ext("png"), "image/png");
        assert_eq!(mime_from_ext("PNG"), "image/png");
        assert_eq!(mime_from_ext("jpg"), "image/jpeg");
        assert_eq!(mime_from_ext("unknown"), "application/octet-stream");
    }
}
