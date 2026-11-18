//! 문서 핵심 도메인 모델
//!
//! HWP 문서의 도메인 상태와 로직을 캡슐화한다.
//! WASM/PyO3/MCP 등 어떤 어댑터에서도 독립적으로 사용할 수 있다.

pub(crate) mod helpers;
pub(crate) use helpers::*;

mod commands;
pub mod converters;
pub(crate) mod html_table_import;
mod queries;
pub mod table_calc;
pub mod validation;

use crate::model::document::Document;
use crate::model::event::DocumentEvent;
use crate::model::paragraph::{Paragraph, ParagraphEdit};
use crate::renderer::composer::ComposedParagraph;
use crate::renderer::height_measurer::{MeasuredSection, MeasuredTable};
use crate::renderer::layout::LayoutEngine;
use crate::renderer::pagination::PaginationResult;
use crate::renderer::render_tree::PageRenderTree;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::DEFAULT_DPI;
use std::cell::RefCell;
use std::collections::HashMap;

/// Snapshot-only storage: the editable model stays owned, while immutable byte
/// vectors are interned by value. Weak entries never keep discarded history alive.
#[derive(Default)]
struct SnapshotPayloadPool {
    buffers: HashMap<u64, Vec<std::sync::Weak<Vec<u8>>>>,
    previews: Vec<std::sync::Weak<Option<crate::model::document::Preview>>>,
}

impl SnapshotPayloadPool {
    fn capture_bytes(&mut self, bytes: &[u8]) -> std::sync::Arc<Vec<u8>> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hash);
        let bucket = self.buffers.entry(hash.finish()).or_default();
        bucket.retain(|entry| entry.strong_count() > 0);
        for entry in bucket.iter().filter_map(std::sync::Weak::upgrade) {
            if entry.as_slice() == bytes {
                return entry;
            }
        }
        let shared = std::sync::Arc::new(bytes.to_vec());
        bucket.push(std::sync::Arc::downgrade(&shared));
        shared
    }

    fn capture_preview(
        &mut self,
        preview: &Option<crate::model::document::Preview>,
    ) -> std::sync::Arc<Option<crate::model::document::Preview>> {
        self.previews.retain(|entry| entry.strong_count() > 0);
        for entry in self.previews.iter().filter_map(std::sync::Weak::upgrade) {
            let equal = match (entry.as_ref(), preview) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    a.text == b.text
                        && match (&a.image, &b.image) {
                            (None, None) => true,
                            (Some(a), Some(b)) => a.format == b.format && a.data == b.data,
                            _ => false,
                        }
                }
                _ => false,
            };
            if equal {
                return entry;
            }
        }
        let shared = std::sync::Arc::new(preview.clone());
        self.previews.push(std::sync::Arc::downgrade(&shared));
        shared
    }

    fn prune(&mut self) {
        self.buffers.retain(|_, bucket| {
            bucket.retain(|entry| entry.strong_count() > 0);
            !bucket.is_empty()
        });
        self.previews.retain(|entry| entry.strong_count() > 0);
    }
}

struct DocumentSnapshot {
    document: Document,
    bin_data: Vec<std::sync::Arc<Vec<u8>>>,
    preview: std::sync::Arc<Option<crate::model::document::Preview>>,
    extra_streams: Vec<std::sync::Arc<Vec<u8>>>,
    doc_info_raw: Option<std::sync::Arc<Vec<u8>>>,
    section_raw: Vec<Option<std::sync::Arc<Vec<u8>>>>,
}

impl DocumentSnapshot {
    fn capture(document: &Document, pool: &mut SnapshotPayloadPool) -> Self {
        use crate::model::document::{DocInfo, Section};
        // Explicit construction avoids cloning a payload only to clear it and
        // makes additions to Document/DocInfo/Section a compile-time obligation.
        let Document {
            header,
            doc_properties,
            doc_info,
            sections,
            preview,
            bin_data_content,
            extra_streams,
        } = document;
        let light_info = DocInfo {
            hwpx_track_change_flags: doc_info.hwpx_track_change_flags,
            bin_data_list: doc_info.bin_data_list.clone(),
            font_faces: doc_info.font_faces.clone(),
            border_fills: doc_info.border_fills.clone(),
            char_shapes: doc_info.char_shapes.clone(),
            tab_defs: doc_info.tab_defs.clone(),
            numberings: doc_info.numberings.clone(),
            bullets: doc_info.bullets.clone(),
            para_shapes: doc_info.para_shapes.clone(),
            styles: doc_info.styles.clone(),
            extra_records: doc_info.extra_records.clone(),
            raw_stream: None,
            bullet_count: doc_info.bullet_count,
            memo_shape_count: doc_info.memo_shape_count,
            distribute_doc_data_removed: doc_info.distribute_doc_data_removed,
            raw_stream_dirty: doc_info.raw_stream_dirty,
        };
        Self {
            document: Document {
                header: header.clone(),
                doc_properties: doc_properties.clone(),
                doc_info: light_info,
                sections: sections
                    .iter()
                    .map(|s| Section {
                        section_def: s.section_def.clone(),
                        paragraphs: s.paragraphs.clone(),
                        raw_stream: None,
                    })
                    .collect(),
                preview: None,
                bin_data_content: bin_data_content
                    .iter()
                    .map(|b| crate::model::bin_data::BinDataContent {
                        id: b.id,
                        extension: b.extension.clone(),
                        data: Vec::new(),
                    })
                    .collect(),
                extra_streams: extra_streams
                    .iter()
                    .map(|(name, _)| (name.clone(), Vec::new()))
                    .collect(),
            },
            bin_data: bin_data_content
                .iter()
                .map(|b| pool.capture_bytes(&b.data))
                .collect(),
            preview: pool.capture_preview(preview),
            extra_streams: extra_streams
                .iter()
                .map(|(_, bytes)| pool.capture_bytes(bytes))
                .collect(),
            doc_info_raw: doc_info
                .raw_stream
                .as_ref()
                .map(|bytes| pool.capture_bytes(bytes)),
            section_raw: sections
                .iter()
                .map(|s| s.raw_stream.as_ref().map(|bytes| pool.capture_bytes(bytes)))
                .collect(),
        }
    }

    fn restore(&self) -> Document {
        let mut document = self.document.clone();
        for (bin, bytes) in document.bin_data_content.iter_mut().zip(&self.bin_data) {
            bin.data = bytes.as_ref().clone();
        }
        document.preview = self.preview.as_ref().clone();
        for ((_, data), bytes) in document.extra_streams.iter_mut().zip(&self.extra_streams) {
            *data = bytes.as_ref().clone();
        }
        document.doc_info.raw_stream = self
            .doc_info_raw
            .as_ref()
            .map(|bytes| bytes.as_ref().clone());
        for (section, bytes) in document.sections.iter_mut().zip(&self.section_raw) {
            section.raw_stream = bytes.as_ref().map(|bytes| bytes.as_ref().clone());
        }
        document
    }
}

mod mutation {
    use super::{DocumentSnapshot, SnapshotPayloadPool};
    use crate::error::HwpError;
    use crate::model::document::Document;
    use std::ops::{Deref, DerefMut};

    pub(super) struct InverseTicket {
        id: u64,
        outer: bool,
    }

    struct ActiveInverse {
        id: u64,
        depth: usize,
        expected_state: u64,
        produced_state: u64,
        /// Content at `expected_state`, taken lazily at the first mutation inside
        /// the scope so an aborted inverse can restore it exactly.
        checkpoint: Option<Box<DocumentSnapshot>>,
        /// Set by a nested abort: the outer close must roll back, never publish.
        poisoned: bool,
    }

    pub(super) struct DocumentMutation {
        document: Document,
        epoch: Result<String, String>,
        next_state: u64,
        current_state: u64,
        previous_state: u64,
        next_ticket_id: u64,
        active_inverse: Option<ActiveInverse>,
        /// True after a scope rollback until the owner rebuilds derived caches.
        rolled_back: bool,
        snapshot_payloads: SnapshotPayloadPool,
    }

    impl DocumentMutation {
        pub(super) fn new(document: Document) -> Self {
            let mut bytes = [0; 16];
            let epoch = getrandom::fill(&mut bytes)
                .map(|()| bytes.iter().map(|byte| format!("{byte:02x}")).collect())
                .map_err(|error| format!("EDIT_HISTORY_UNAVAILABLE: {error}"));
            Self {
                document,
                epoch,
                next_state: 0,
                current_state: 0,
                previous_state: 0,
                next_ticket_id: 0,
                active_inverse: None,
                rolled_back: false,
                snapshot_payloads: SnapshotPayloadPool::default(),
            }
        }

        pub(super) fn epoch(&self) -> Result<&str, &str> {
            self.epoch.as_deref().map_err(String::as_str)
        }

        pub(super) fn snapshot(&mut self) -> DocumentSnapshot {
            DocumentSnapshot::capture(&self.document, &mut self.snapshot_payloads)
        }

        pub(super) fn prune_snapshot_payloads(&mut self) {
            self.snapshot_payloads.prune();
        }

        pub(super) fn state_label(&self) -> Result<String, HwpError> {
            let epoch = self
                .epoch
                .as_ref()
                .map_err(|reason| HwpError::RenderError(reason.clone()))?;
            Ok(format!("{epoch}:{}", self.current_state))
        }

        pub(super) fn previous_state_label(&self) -> Result<String, HwpError> {
            let epoch = self
                .epoch
                .as_ref()
                .map_err(|reason| HwpError::RenderError(reason.clone()))?;
            Ok(format!("{epoch}:{}", self.previous_state))
        }

        fn parse_label(&self, label: &str) -> Result<u64, HwpError> {
            let epoch = self
                .epoch
                .as_ref()
                .map_err(|reason| HwpError::InvalidEditHandle(reason.clone()))?;
            let Some((prefix, raw_state)) = label.split_once(':') else {
                return Err(HwpError::InvalidEditHandle("invalid state label".into()));
            };
            if prefix != epoch {
                return Err(HwpError::InvalidEditHandle("document epoch differs".into()));
            }
            let state = raw_state
                .parse::<u64>()
                .map_err(|_| HwpError::InvalidEditHandle("invalid state revision".into()))?;
            if state > self.next_state {
                return Err(HwpError::InvalidEditHandle("future state label".into()));
            }
            Ok(state)
        }

        pub(super) fn begin_inverse(
            &mut self,
            expected_label: &str,
            produced_label: &str,
        ) -> Result<InverseTicket, HwpError> {
            let expected = self.parse_label(expected_label)?;
            let produced = self.parse_label(produced_label)?;
            if let Some(active) = &mut self.active_inverse {
                if active.expected_state != expected
                    || active.produced_state != produced
                    || self.current_state != expected
                {
                    return Err(HwpError::InvalidEditHandle(
                        "nested inverse label mismatch".into(),
                    ));
                }
                active.depth += 1;
                return Ok(InverseTicket {
                    id: active.id,
                    outer: false,
                });
            }
            if self.current_state != expected {
                return Err(HwpError::InvalidEditHandle(
                    "content state label differs".into(),
                ));
            }
            self.next_ticket_id = self
                .next_ticket_id
                .checked_add(1)
                .ok_or_else(|| HwpError::InvalidEditHandle("inverse ticket exhausted".into()))?;
            let id = self.next_ticket_id;
            self.active_inverse = Some(ActiveInverse {
                id,
                depth: 1,
                expected_state: expected,
                produced_state: produced,
                checkpoint: None,
                poisoned: false,
            });
            Ok(InverseTicket { id, outer: true })
        }

        /// Closes a scope. The outer close publishes `produced` only when every
        /// nested step finished; a poisoned scope restores the checkpoint and the
        /// `expected` label instead, so a refusal never changes bytes or label.
        pub(super) fn finish_inverse(&mut self, ticket: InverseTicket) -> Result<(), HwpError> {
            let Some(active) = &mut self.active_inverse else {
                return Err(HwpError::InvalidEditHandle(
                    "no active inverse scope".into(),
                ));
            };
            if active.id != ticket.id {
                return Err(HwpError::InvalidEditHandle(
                    "inverse ticket mismatch".into(),
                ));
            }
            if !ticket.outer {
                active.depth = active.depth.saturating_sub(1);
                self.current_state = active.produced_state;
                return Ok(());
            }
            let Some(closed) = self.active_inverse.take() else {
                return Err(HwpError::InvalidEditHandle(
                    "no active inverse scope".into(),
                ));
            };
            if closed.poisoned {
                if let Some(checkpoint) = closed.checkpoint {
                    self.document = checkpoint.restore();
                    self.rolled_back = true;
                }
                self.current_state = closed.expected_state;
                return Err(HwpError::InvalidEditHandle(
                    "inverse step failed; scope rolled back".into(),
                ));
            }
            self.current_state = closed.produced_state;
            Ok(())
        }

        /// Aborts a scope. A nested abort poisons the outer scope; an outer abort
        /// closes it through the rollback path of `finish_inverse`.
        pub(super) fn abort_inverse(&mut self, ticket: InverseTicket) {
            let Some(active) = &mut self.active_inverse else {
                return;
            };
            if active.id != ticket.id {
                return;
            }
            active.poisoned = true;
            if ticket.outer {
                let _rolled_back = self.finish_inverse(ticket);
            } else {
                active.depth = active.depth.saturating_sub(1);
            }
        }

        /// Reports (once) that a rollback replaced the document content.
        pub(super) fn take_rolled_back(&mut self) -> bool {
            std::mem::take(&mut self.rolled_back)
        }
    }

    impl Deref for DocumentMutation {
        type Target = Document;
        fn deref(&self) -> &Document {
            &self.document
        }
    }

    impl DerefMut for DocumentMutation {
        fn deref_mut(&mut self) -> &mut Document {
            #[expect(
                clippy::expect_used,
                reason = "a document cannot perform 2^64 mutations in its lifetime"
            )]
            {
                self.next_state = self
                    .next_state
                    .checked_add(1)
                    .expect("document revision exhausted");
            }
            match &mut self.active_inverse {
                Some(active) => {
                    if active.checkpoint.is_none() {
                        active.checkpoint = Some(Box::new(DocumentSnapshot::capture(
                            &self.document,
                            &mut self.snapshot_payloads,
                        )));
                    }
                }
                None => self.previous_state = self.current_state,
            }
            self.current_state = self.next_state;
            &mut self.document
        }
    }

    #[cfg(test)]
    mod tests {
        use super::DocumentMutation;
        use crate::model::document::{Document, Section};

        /// A gate whose content moved one revision forward (one section added),
        /// with the labels before and after that edit.
        fn edited_gate() -> (DocumentMutation, String, String) {
            let mut gate = DocumentMutation::new(Document::default());
            let before = gate.state_label().unwrap();
            gate.sections.push(Section::default());
            let after = gate.state_label().unwrap();
            (gate, before, after)
        }

        #[test]
        fn c7_poisoned_nested_step_rolls_back_partial_mutation() {
            let (mut gate, before, after) = edited_gate();
            let outer = gate.begin_inverse(&after, &before).unwrap();
            let nested = gate.begin_inverse(&after, &before).unwrap();
            gate.sections.clear();
            gate.abort_inverse(nested);
            assert!(
                gate.finish_inverse(outer).is_err(),
                "a scope with a failed step must not publish its produced label"
            );
            assert_eq!(gate.sections.len(), 1, "checkpoint content restored");
            assert_eq!(gate.state_label().unwrap(), after, "label stays expected");
            assert!(gate.take_rolled_back());
            assert!(!gate.take_rolled_back(), "rollback is reported once");

            let outer = gate.begin_inverse(&after, &before).unwrap();
            gate.sections.clear();
            gate.finish_inverse(outer).unwrap();
            assert!(gate.sections.is_empty());
            assert_eq!(gate.state_label().unwrap(), before);
            assert!(!gate.take_rolled_back());
        }

        #[test]
        fn c7_outer_abort_after_real_mutation_restores_content_and_label() {
            let (mut gate, before, after) = edited_gate();
            let outer = gate.begin_inverse(&after, &before).unwrap();
            gate.sections.clear();
            gate.sections.push(Section::default());
            gate.sections.push(Section::default());
            gate.abort_inverse(outer);
            assert_eq!(gate.sections.len(), 1);
            assert_eq!(gate.state_label().unwrap(), after);
            assert!(gate.take_rolled_back());

            let outer = gate.begin_inverse(&after, &before).unwrap();
            gate.abort_inverse(outer);
            assert_eq!(gate.state_label().unwrap(), after);
            assert!(!gate.take_rolled_back(), "no mutation, nothing to restore");
        }

        fn binary_document() -> Document {
            use crate::model::bin_data::BinDataContent;
            use crate::model::document::{Preview, PreviewImage, PreviewImageFormat};
            Document {
                bin_data_content: vec![BinDataContent {
                    id: 7,
                    extension: "png".into(),
                    data: vec![7; 1024 * 1024],
                }],
                preview: Some(Preview {
                    text: Some("preview".into()),
                    image: Some(PreviewImage {
                        format: PreviewImageFormat::Png,
                        data: vec![8; 1024],
                    }),
                }),
                extra_streams: vec![("extra".into(), vec![9; 1024])],
                doc_info: crate::model::document::DocInfo {
                    raw_stream: Some(vec![10; 1024]),
                    ..Default::default()
                },
                sections: vec![Section {
                    raw_stream: Some(vec![11; 1024]),
                    ..Default::default()
                }],
                ..Default::default()
            }
        }

        #[test]
        fn c9_snapshots_share_all_binary_payloads_without_mutating_the_document() {
            use std::sync::Arc;
            let mut gate = DocumentMutation::new(binary_document());
            let label = gate.state_label().unwrap();
            let first = gate.snapshot();
            for _ in 0..1025 {
                let next = gate.snapshot();
                assert!(Arc::ptr_eq(&first.bin_data[0], &next.bin_data[0]));
                assert!(Arc::ptr_eq(&first.preview, &next.preview));
                assert!(Arc::ptr_eq(&first.extra_streams[0], &next.extra_streams[0]));
                assert!(Arc::ptr_eq(
                    first.doc_info_raw.as_ref().unwrap(),
                    next.doc_info_raw.as_ref().unwrap()
                ));
                assert!(Arc::ptr_eq(
                    first.section_raw[0].as_ref().unwrap(),
                    next.section_raw[0].as_ref().unwrap()
                ));
                assert!(next.document.bin_data_content[0].data.is_empty());
                assert!(next.document.preview.is_none());
            }
            assert_eq!(gate.state_label().unwrap(), label);
            gate.bin_data_content[0].data[0] = 99;
            let changed = gate.snapshot();
            assert!(!Arc::ptr_eq(&first.bin_data[0], &changed.bin_data[0]));
            let restored = first.restore();
            assert_eq!(restored.bin_data_content[0].data, vec![7; 1024 * 1024]);
            assert_eq!(restored.bin_data_content[0].id, 7);
            assert_eq!(restored.bin_data_content[0].extension, "png");
            let preview = restored.preview.unwrap();
            assert_eq!(preview.text.as_deref(), Some("preview"));
            assert_eq!(preview.image.unwrap().data, vec![8; 1024]);
            assert_eq!(
                restored.extra_streams,
                vec![("extra".into(), vec![9; 1024])]
            );
            assert_eq!(restored.doc_info.raw_stream, Some(vec![10; 1024]));
            assert_eq!(restored.sections[0].raw_stream, Some(vec![11; 1024]));
        }

        #[test]
        fn c9_inverse_checkpoint_shares_payloads_and_rolls_back_changed_bytes() {
            use std::sync::Arc;
            let mut gate = DocumentMutation::new(binary_document());
            let before = gate.state_label().unwrap();
            gate.sections.push(Section::default());
            let after = gate.state_label().unwrap();
            let captured = gate.snapshot();
            let ticket = gate.begin_inverse(&after, &before).unwrap();
            gate.bin_data_content[0].data[0] = 99;
            let checkpoint = gate
                .active_inverse
                .as_ref()
                .unwrap()
                .checkpoint
                .as_ref()
                .unwrap();
            assert!(Arc::ptr_eq(&captured.bin_data[0], &checkpoint.bin_data[0]));
            gate.preview = None;
            gate.extra_streams.clear();
            gate.doc_info.raw_stream = None;
            gate.sections.clear();
            gate.abort_inverse(ticket);
            assert_eq!(gate.state_label().unwrap(), after);
            assert_eq!(gate.bin_data_content[0].data, vec![7; 1024 * 1024]);
            assert!(gate.preview.is_some());
            assert_eq!(gate.extra_streams[0].1, vec![9; 1024]);
            assert_eq!(gate.doc_info.raw_stream, Some(vec![10; 1024]));
            assert_eq!(gate.sections[0].raw_stream, Some(vec![11; 1024]));
            assert_eq!(gate.sections.len(), 2);
            drop(captured);
            gate.prune_snapshot_payloads();
            assert!(gate.snapshot_payloads.buffers.is_empty());
            assert!(gate.snapshot_payloads.previews.is_empty());
        }
    }
}

#[derive(Clone)]
pub(crate) struct EditRecord {
    handle: String,
    undo_from_label: Option<String>,
    undo_to_label: String,
    state: TextEditState,
}

/// A native text-edit stack change made inside an open command inverse. A scope
/// that rolls back replays these in reverse, so every handle that was applicable
/// before the scope is applicable again and nothing the scope created leaks.
pub(crate) enum EditJournalEntry {
    /// `undo_edit_native` consumed `original` and pushed its reciprocal.
    Moved {
        undoing: bool,
        original: EditRecord,
        reciprocal_handle: String,
    },
    /// A new edit cleared these redo records.
    RedoCleared(Vec<EditRecord>),
    /// `begin_text_edit_native` pushed this capture.
    Captured(String),
}

pub(crate) struct SnapshotRecord {
    pub(crate) id: u32,
    document: DocumentSnapshot,
    /// 저장 시점의 상태 라벨 — 이 스냅샷 내용의 정체. 짝(before/after)은 여기서 맺지 않고
    /// 복원 시 호출부가 명시한 라벨 쌍을 단일 역실행 게이트가 이 값과 대조한다.
    pub(crate) label: String,
}

#[derive(Clone)]
pub(crate) enum TextEditState {
    Paragraph {
        section: usize,
        paragraph: usize,
        inverse: ParagraphEdit,
        trailing_vpos: Vec<Vec<i32>>,
    },
    ParagraphList {
        section: usize,
        before: Vec<Paragraph>,
    },
}

pub(crate) struct TextEditHistory {
    next_sequence: u64,
    undo: Vec<EditRecord>,
    redo: Vec<EditRecord>,
    /// Some while a command inverse is open (see `EditJournalEntry`).
    journal: Option<Vec<EditJournalEntry>>,
}

impl TextEditHistory {
    fn new() -> Self {
        Self {
            next_sequence: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            journal: None,
        }
    }

    fn roll_back_journal(&mut self) {
        let Some(journal) = self.journal.take() else {
            return;
        };
        for entry in journal.into_iter().rev() {
            match entry {
                EditJournalEntry::Moved {
                    undoing,
                    original,
                    reciprocal_handle,
                } => {
                    let (source, target) = if undoing {
                        (&mut self.undo, &mut self.redo)
                    } else {
                        (&mut self.redo, &mut self.undo)
                    };
                    target.retain(|record| record.handle != reciprocal_handle);
                    source.push(original);
                }
                EditJournalEntry::RedoCleared(records) => {
                    let later = std::mem::replace(&mut self.redo, records);
                    self.redo.extend(later);
                }
                EditJournalEntry::Captured(handle) => {
                    self.undo.retain(|record| record.handle != handle);
                }
            }
        }
    }
}

/// 기본 폰트 fallback 경로
pub const DEFAULT_FALLBACK_FONT: &str = "/usr/share/fonts/truetype/nanum/NanumGothic.ttf";

/// 내부 클립보드 데이터
pub(crate) struct ClipboardData {
    /// 복사된 문단들 (서식 정보 포함)
    pub(crate) paragraphs: Vec<Paragraph>,
    /// 플레인 텍스트
    pub(crate) plain_text: String,
}

/// HWP 문서 핵심 도메인 모델
///
/// 문서 데이터, 레이아웃 상태, 설정, 캐시를 포함한다.
/// WASM 바인딩 없이 순수 Rust 타입만 사용한다.
pub struct DocumentCore {
    /// IR 문서
    document: mutation::DocumentMutation,
    /// 페이지 분할 결과
    pub(crate) pagination: Vec<PaginationResult>,
    /// 해소된 스타일 세트
    pub(crate) styles: ResolvedStyleSet,
    /// 구역별 구성된 문단 목록
    pub(crate) composed: Vec<Vec<ComposedParagraph>>,
    /// DPI
    pub(crate) dpi: f64,
    /// 대체 폰트 경로
    pub(crate) fallback_font: String,
    /// 레이아웃 엔진 (자동 번호 카운터 포함)
    pub(crate) layout_engine: LayoutEngine,
    /// 내부 클립보드
    pub(crate) clipboard: Option<ClipboardData>,
    /// 문단부호(¶) 표시 여부
    pub(crate) show_paragraph_marks: bool,
    /// 조판부호 표시 여부 (개체 마커 [표]/[그림] 등, 문단부호 포함)
    pub(crate) show_control_codes: bool,
    /// 투명선 표시 여부
    pub(crate) show_transparent_borders: bool,
    /// 잘림 보기 (body/셀 클리핑 활성화 여부)
    pub(crate) clip_enabled: bool,
    /// 디버그 오버레이 표시 여부 (문단/표 경계 + pi/ci 라벨)
    pub(crate) debug_overlay: bool,
    /// 구역별 표 측정 데이터 (페이지네이션 결과 보존)
    pub(crate) measured_tables: Vec<Vec<MeasuredTable>>,
    /// 구역별 dirty 플래그 (true = 재페이지네이션 필요)
    pub(crate) dirty_sections: Vec<bool>,
    /// 구역별 측정 캐시 (증분 측정용)
    pub(crate) measured_sections: Vec<MeasuredSection>,
    /// 구역별 문단 dirty 비트맵.
    /// None = 전체 dirty (초기 로드 또는 전체 재구성 시).
    /// Some(vec) = vec[para_idx] = true이면 해당 문단만 재측정.
    pub(crate) dirty_paragraphs: Vec<Option<Vec<bool>>>,
    /// 구역별 문단→단 인덱스 매핑 (페이지네이션에서 결정)
    /// para_column_map[section_idx][para_idx] = column_index
    pub(crate) para_column_map: Vec<Vec<u16>>,
    /// 페이지별 렌더 트리 캐시 (지연 구축, 부분 무효화)
    pub(crate) page_tree_cache: RefCell<Vec<Option<PageRenderTree>>>,
    /// Batch 모드 플래그 — true이면 paginate() 스킵
    pub(crate) batch_mode: bool,
    /// 이벤트 로그 (Command 실행 시 누적)
    pub(crate) event_log: Vec<DocumentEvent>,
    /// 글상자 오버플로우 연결 캐시 (섹션별, 지연 계산)
    pub(crate) overflow_links_cache:
        RefCell<HashMap<usize, Vec<queries::doc_tree_nav::OverflowLink>>>,
    /// Undo/Redo용 Document 스냅샷 저장소 (ID → 상태 라벨 바인딩 스냅샷)
    pub(crate) snapshot_store: Vec<SnapshotRecord>,
    /// 다음 스냅샷 ID
    pub(crate) next_snapshot_id: u32,
    pub(crate) text_edit_history: TextEditHistory,
    /// 외부 기록 명령(서식/표 이동 등)의 단일 역실행 게이트 티켓
    pending_command_inverse: Option<(String, mutation::InverseTicket)>,
    next_command_inverse_seq: u64,
    /// 머리말/꼬리말 감추기: (global_page_index, is_header) 조합
    pub(crate) hidden_header_footer: std::collections::HashSet<(u32, bool)>,
    /// 파일 이름 (머리말/꼬리말 필드 치환용)
    pub(crate) file_name: String,
    /// 현재 활성 필드 위치 (커서가 진입한 누름틀 — 안내문 렌더링 스킵용)
    /// (section_idx, para_idx, field_control_idx)
    pub(crate) active_field: Option<ActiveFieldInfo>,
    /// 구역별 문단 인덱스 오프셋 (삽입=+N, 삭제=-N, 페이지네이션 수렴 감지용)
    /// paginate() 후 리셋.
    pub(crate) para_offset: Vec<i32>,
    /// 원본 파일 형식 (HWP/HWPX) — 저장 시 형식 분기용
    pub(crate) source_format: crate::parser::FileFormat,
    /// HWPX 비표준 감지 등 문서 검증 경고.
    /// `from_bytes` 에서 자동 생성되며, 사용자 고지·선택적 reflow 에 사용 (#177).
    pub(crate) validation_report: validation::ValidationReport,
}

/// 활성 필드 위치 정보
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveFieldInfo {
    pub section_idx: usize,
    pub para_idx: usize,
    /// field_ranges의 control_idx (controls[] 내 Field 컨트롤 인덱스)
    pub control_idx: usize,
    /// 셀 내부 필드인 경우의 전체 경로
    /// 단일 표: vec![(parent_para_idx, ctrl, cell)]
    /// 중첩 표: vec![(outer_ctrl, outer_cell, ..), (inner_ctrl, inner_cell, ..)]
    /// parent_para_idx는 별도 필드에 포함하지 않고 첫 번째 요소의 context로 사용
    pub cell_path: Option<Vec<(usize, usize, usize)>>, // Vec<(parent_para_idx_or_ctrl, ctrl_or_cell, cell_or_para)>
}

impl DocumentCore {
    pub(crate) fn mutate_document(&mut self) -> &mut Document {
        &mut self.document
    }

    pub fn state_label_native(&self) -> Option<String> {
        self.document.state_label().ok()
    }

    pub fn previous_state_label_native(&self) -> Option<String> {
        self.document.previous_state_label().ok()
    }

    pub fn begin_command_inverse_native(
        &mut self,
        expected_label: &str,
        produced_label: &str,
    ) -> Result<String, crate::error::HwpError> {
        if self.pending_command_inverse.is_some() {
            return Err(crate::error::HwpError::InvalidEditHandle(
                "command inverse already active".into(),
            ));
        }
        let ticket = self
            .document
            .begin_inverse(expected_label, produced_label)?;
        let epoch = self
            .document
            .epoch()
            .map_err(|reason| crate::error::HwpError::InvalidEditHandle(reason.to_string()))?
            .to_string();
        self.next_command_inverse_seq =
            self.next_command_inverse_seq
                .checked_add(1)
                .ok_or_else(|| {
                    crate::error::HwpError::InvalidEditHandle(
                        "command inverse sequence exhausted".into(),
                    )
                })?;
        let token = format!("{epoch}:cmd:{}", self.next_command_inverse_seq);
        self.pending_command_inverse = Some((token.clone(), ticket));
        self.text_edit_history.journal = Some(Vec::new());
        Ok(token)
    }

    pub fn finish_command_inverse_native(
        &mut self,
        scope_token: &str,
    ) -> Result<(), crate::error::HwpError> {
        let Some((expected_token, _)) = &self.pending_command_inverse else {
            return Err(crate::error::HwpError::InvalidEditHandle(
                "no active command inverse".into(),
            ));
        };
        if expected_token != scope_token {
            return Err(crate::error::HwpError::InvalidEditHandle(
                "command inverse token mismatch".into(),
            ));
        }
        let Some((_, ticket)) = self.pending_command_inverse.take() else {
            return Err(crate::error::HwpError::InvalidEditHandle(
                "no active command inverse".into(),
            ));
        };
        let result = self.document.finish_inverse(ticket);
        self.close_command_inverse(result.is_ok());
        result
    }

    /// Refuses the open command inverse atomically: content, label and native
    /// text-edit handles return to their state at `begin_command_inverse_native`.
    pub fn abort_command_inverse_native(&mut self, scope_token: &str) {
        if self
            .pending_command_inverse
            .as_ref()
            .is_some_and(|(token, _)| token == scope_token)
        {
            if let Some((_, ticket)) = self.pending_command_inverse.take() {
                self.document.abort_inverse(ticket);
                self.close_command_inverse(false);
            }
        }
    }

    fn close_command_inverse(&mut self, committed: bool) {
        if committed {
            self.text_edit_history.journal = None;
        } else {
            self.text_edit_history.roll_back_journal();
        }
        if self.document.take_rolled_back() {
            self.rebuild_after_inverse_rollback();
        }
    }

    /// Derived caches follow the restored checkpoint (same rebuild as a snapshot restore).
    fn rebuild_after_inverse_rollback(&mut self) {
        self.styles =
            crate::renderer::style_resolver::resolve_styles(&self.document.doc_info, self.dpi);
        self.composed = self
            .document
            .sections
            .iter()
            .map(crate::renderer::composer::compose_section)
            .collect();
        self.mark_all_sections_dirty();
        self.measured_tables.clear();
        self.measured_sections.clear();
        self.dirty_paragraphs.clear();
        self.para_column_map.clear();
        self.page_tree_cache.borrow_mut().clear();
        self.overflow_links_cache.borrow_mut().clear();
        self.paginate();
    }

    /// 총 페이지 수를 반환한다.
    pub fn page_count(&self) -> u32 {
        self.pagination
            .iter()
            .map(|pr| pr.pages.len() as u32)
            .sum::<u32>()
            .max(1)
    }

    /// 문서 정보를 JSON 문자열로 반환한다.
    pub fn get_document_info(&self) -> String {
        use crate::renderer::style_resolver::resolve_font_substitution;

        let mut fonts = std::collections::BTreeSet::new();
        for (lang_idx, lang_fonts) in self.document.doc_info.font_faces.iter().enumerate() {
            for font in lang_fonts {
                let resolved = resolve_font_substitution(&font.name, font.alt_type, lang_idx)
                    .unwrap_or(&font.name);
                fonts.insert(resolved.to_string());
            }
        }
        let fonts_json: Vec<String> = fonts
            .iter()
            .map(|f| {
                // 폰트 이름의 특수문자를 JSON 이스케이프 처리
                let escaped: String = f
                    .chars()
                    .flat_map(|c| match c {
                        '"' => vec!['\\', '"'],
                        '\\' => vec!['\\', '\\'],
                        '\n' => vec!['\\', 'n'],
                        '\r' => vec!['\\', 'r'],
                        '\t' => vec!['\\', 't'],
                        c if c < '\x20' => vec![],
                        c => vec![c],
                    })
                    .collect();
                format!("\"{}\"", escaped)
            })
            .collect();

        let escaped_fallback: String = self
            .fallback_font
            .chars()
            .flat_map(|c| match c {
                '"' => vec!['\\', '"'],
                '\\' => vec!['\\', '\\'],
                c => vec![c],
            })
            .collect();
        format!(
            "{{\"version\":\"{}.{}.{}.{}\",\"sectionCount\":{},\"pageCount\":{},\"encrypted\":{},\"fallbackFont\":\"{}\",\"fontsUsed\":[{}]}}",
            self.document.header.version.major,
            self.document.header.version.minor,
            self.document.header.version.build,
            self.document.header.version.revision,
            self.document.sections.len(),
            self.page_count(),
            self.document.header.encrypted,
            escaped_fallback,
            fonts_json.join(","),
        )
    }

    /// 이벤트 로그를 JSON 배열로 직렬화한다.
    pub fn serialize_event_log(&self) -> String {
        crate::model::event::serialize_event_log(&self.event_log)
    }

    /// DPI를 설정하고 스타일을 재해소한 후 재페이지네이션한다.
    pub fn set_dpi(&mut self, dpi: f64) {
        use crate::renderer::style_resolver::resolve_styles;
        self.dpi = dpi;
        self.styles = resolve_styles(&self.document.doc_info, dpi);
        self.paginate();
    }

    /// 빈 문서를 생성한다 (테스트/미리보기용).
    pub fn new_empty() -> Self {
        DocumentCore {
            document: mutation::DocumentMutation::new(Document::default()),
            pagination: Vec::new(),
            styles: ResolvedStyleSet::default(),
            composed: Vec::new(),
            dpi: DEFAULT_DPI,
            fallback_font: DEFAULT_FALLBACK_FONT.to_string(),
            layout_engine: LayoutEngine::new(DEFAULT_DPI),
            clipboard: None,
            show_paragraph_marks: false,
            show_control_codes: false,
            show_transparent_borders: false,
            clip_enabled: true,
            debug_overlay: false,
            measured_tables: Vec::new(),
            dirty_sections: Vec::new(),
            measured_sections: Vec::new(),
            dirty_paragraphs: Vec::new(),
            para_column_map: Vec::new(),
            page_tree_cache: RefCell::new(Vec::new()),
            batch_mode: false,
            event_log: Vec::new(),
            overflow_links_cache: RefCell::new(HashMap::new()),
            snapshot_store: Vec::new(),
            next_snapshot_id: 0,
            text_edit_history: TextEditHistory::new(),
            pending_command_inverse: None,
            next_command_inverse_seq: 0,
            hidden_header_footer: std::collections::HashSet::new(),
            file_name: String::new(),
            active_field: None,
            para_offset: Vec::new(),
            source_format: crate::parser::FileFormat::Hwp,
            validation_report: validation::ValidationReport::new(),
        }
    }

    /// 문서 검증 리포트에 대한 참조를 반환한다.
    ///
    /// `from_bytes` 시점에 HWPX 비표준 lineseg 감지가 수행되며, 경고가 있으면
    /// 사용자에게 고지되어야 한다. 자동 reflow 는 적용되지 않고 사용자가
    /// 명시적으로 `reflow_linesegs_on_demand()` 를 호출해야 보정된다.
    pub fn validation_report(&self) -> &validation::ValidationReport {
        &self.validation_report
    }
}
